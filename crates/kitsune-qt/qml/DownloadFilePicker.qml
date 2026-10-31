// Include/exclude picker for a release with more than one file.
//
// A season pack has no episode to match on, so guessing is how the reader ends
// up with seven unwanted episodes. Instead this resolves the file list and lets
// them tick exactly what they want, defaulting to a sensible guess so the common
// case ("all of it") is one click.
//
// The list is fetched WITHOUT starting a download: `listReleaseFiles` adds the
// magnet, reads the metadata, then drops the torrent again.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: picker

    property var backend: null
    // The release being inspected, as the raw object from the search result.
    property var release: null
    // The episode the modal was opened for, used for the default ticks.
    property int episodeNumber: 0
    property int offset: 0

    // Echoed by the backend so a slow reply for a previous open is ignored.
    property string token: ""
    property bool loading: false
    property string error: ""
    property string releaseName: ""
    property var files: []
    // idx -> true. A plain object rather than an array so a sparse set of file
    // indices (season packs are numbered 0..N but not always contiguous) needs
    // no index juggling.
    property var checked: ({})

    readonly property var checkedIndices: {
        const out = []
        for (const f of files) if (checked[f.idx]) out.push(f.idx)
        return out
    }
    readonly property int checkedCount: checkedIndices.length
    readonly property real checkedBytes: {
        let total = 0
        for (const f of files) if (checked[f.idx]) total += (f.lengthBytes || 0)
        return total
    }

    modal: true
    closePolicy: Popup.CloseOnEscape
    padding: 0
    anchors.centerIn: parent
    width: Math.min(640, parent ? parent.width - 80 : 640)
    height: Math.min(560, parent ? parent.height - 80 : 560)

    /// Begin inspecting a release.
    ///
    /// Deliberately does NOT `open()` yet. The file count is unknown until the
    /// metadata arrives, and a single-file release must never flash a picker at
    /// the reader -- asking them to tick one checkbox they have no choice about
    /// is pure friction. The popup is opened from `onReleaseFilesReady`, only
    /// once there is a real choice to make.
    function openFor(releaseObject, episode, episodeOffset) {
        release = releaseObject
        episodeNumber = episode
        offset = episodeOffset || 0
        files = []
        checked = ({})
        error = ""
        releaseName = releaseObject ? (releaseObject.title || "") : ""
        loading = true
        token = String(Date.now()) + "-" + Math.floor(Math.random() * 100000)
        resolving()
        if (backend) backend.listReleaseFiles(token, JSON.stringify(releaseObject))
        else failure("Downloads are unavailable.")
    }

    /// Whether the reader has been given a list to choose from.
    ///
    /// Set once the popup actually opens, so a late `onReleaseFilesReady` from
    /// a previous inspection cannot reopen a closed picker.
    property bool showing: false

    /// Tell the modal what is happening, so it can put it in its status line.
    signal resolving()
    signal failure(string message)

    /// The playable files in the resolved list.
    readonly property var playableFiles: files.filter(function (f) {
        return picker.isPlayable(f)
    })

    /// The default ticks.
    ///
    /// One playable file means the release is a single episode: tick it. Several
    /// means a pack, where the reader almost always wants one episode, so tick
    /// the one the modal was opened for and leave the rest for them to add.
    function applyDefaultSelection() {
        const playable = files.filter(function (f) {
            return /\.(mkv|mp4|avi|webm|mov|m4v)$/i.test(f.name || "")
        })
        if (playable.length === 1) {
            const only = {}
            only[playable[0].idx] = true
            checked = only
            return
        }
        if (backend && playable.length > 1) {
            const picked = backend.pickFile(
                JSON.stringify(files), episodeNumber, offset)
            if (picked >= 0) {
                const one = {}
                one[picked] = true
                checked = one
                return
            }
        }
        // Nothing matched (a pack with no episode number): tick every playable
        // file rather than none, since "I downloaded this on purpose" is the
        // likelier reading of an empty choice.
        const all = {}
        for (const f of playable) all[f.idx] = true
        checked = all
    }

    function toggle(idx) {
        const next = Object.assign({}, checked)
        if (next[idx]) delete next[idx]
        else next[idx] = true
        checked = next
    }

    function checkAll() {
        const all = {}
        for (const f of files) if (isPlayable(f)) all[f.idx] = true
        checked = all
    }

    function checkNone() {
        checked = ({})
    }

    function isPlayable(file) {
        return /\.(mkv|mp4|avi|webm|mov|m4v)$/i.test(file.name || "")
    }

    function formatSize(bytes) {
        if (!bytes || bytes <= 0) return "\u2014"
        const units = ["B", "KiB", "MiB", "GiB", "TiB"]
        let v = bytes, u = 0
        while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
        return v.toFixed(v < 10 && u > 0 ? 1 : 0) + " " + units[u]
    }

    function confirm() {
        if (checkedCount === 0) return
        picker.acceptedDownload(release, checkedIndices.slice())
        close()
    }

    signal acceptedDownload(var release, var fileIndices)

    Connections {
        target: picker.backend
        function onReleaseFilesReady(readyToken, json) {
            if (readyToken !== picker.token) return
            const payload = JSON.parse(json)
            picker.loading = false
            if (payload === null) {
                picker.failure("This release returned no files.")
                return
            }
            picker.releaseName = payload.name || picker.releaseName
            picker.files = payload.files || []
            if (picker.files.length === 0) {
                picker.failure("This release returned no files.")
                return
            }

            // ONE playable file: there is nothing to choose, so start the
            // download straight away and never show a picker. This is the
            // ordinary single-episode release, which must feel like one click.
            const playable = picker.playableFiles
            if (playable.length === 1) {
                picker.acceptedDownload(picker.release, [playable[0].idx])
                picker.close()
                return
            }

            // Several files (or none playable) is a real decision, so show the
            // list. `applyDefaultSelection` pre-ticks the likeliest episode.
            picker.applyDefaultSelection()
            picker.showing = true
            picker.open()
        }
        function onReleaseFilesFailed(readyToken, message) {
            if (readyToken !== picker.token) return
            picker.loading = false
            picker.failure(message)
        }
    }

    background: GlassPanel {
        strong: true
        radius: 14
    }

    contentItem: ColumnLayout {
        spacing: 0

        // --- Header ---------------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 64

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 18
                anchors.rightMargin: 12
                spacing: 2

                Text {
                    text: "Choose files"
                    color: Theme.ink
                    font.pixelSize: 15
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    text: picker.releaseName
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }

            MouseArea {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                width: 24
                height: 24
                cursorShape: Qt.PointingHandCursor
                onClicked: picker.close()
                Text {
                    anchors.centerIn: parent
                    text: "\u2715"
                    color: closeMouse.containsMouse ? Theme.accent : Theme.inkMuted
                    font.pixelSize: 14
                }
                hoverEnabled: true
                id: closeMouse
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.separator
        }

        // --- Loading / error ------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.centerIn: parent
                visible: picker.loading
                spacing: 10
                PlayerIcon {
                    Layout.alignment: Qt.AlignHCenter
                    width: 26; height: 26
                    kind: "spinner"
                    color: Theme.inkMuted
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Reading the file list\u2026"
                    color: Theme.inkMuted
                    font.pixelSize: 12
                }
            }

            ColumnLayout {
                anchors.centerIn: parent
                width: parent.width - 48
                visible: !picker.loading && picker.error.length > 0
                spacing: 10
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Could not read this release"
                    color: Theme.ink
                    font.pixelSize: 13
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    text: picker.error
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            ListView {
                anchors.fill: parent
                anchors.margins: 10
                clip: true
                spacing: 4
                visible: !picker.loading && picker.error.length === 0
                model: picker.files

                delegate: Rectangle {
                    required property var modelData

                    readonly property bool playable: picker.isPlayable(modelData)
                    readonly property bool isChecked: picker.checked[modelData.idx] === true

                    width: ListView.view.width
                    height: 46
                    radius: 8
                    color: isChecked ? Theme.glassFillHover : "transparent"
                    border.width: 1
                    border.color: isChecked ? Theme.glassBorderHover : Theme.glassBorder
                    opacity: playable ? 1.0 : 0.55

                    MouseArea {
                        anchors.fill: parent
                        enabled: playable
                        cursorShape: Qt.PointingHandCursor
                        onClicked: picker.toggle(modelData.idx)
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 10

                        // Hand-drawn box rather than a CheckBox: the theme has
                        // no styled CheckBox and the default one is the wrong
                        // colour on both themes.
                        Rectangle {
                            width: 16; height: 16; radius: 4
                            color: isChecked ? Theme.accent : "transparent"
                            border.width: 1
                            border.color: isChecked ? Theme.accent : Theme.glassBorderHover
                            Text {
                                anchors.centerIn: parent
                                visible: isChecked
                                text: "\u2713"
                                color: "white"
                                font.pixelSize: 10
                                font.bold: true
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text {
                                Layout.fillWidth: true
                                text: modelData.name
                                color: Theme.ink
                                font.pixelSize: 11
                                elide: Text.ElideMiddle
                            }
                            Text {
                                text: picker.formatSize(modelData.lengthBytes)
                                    + (playable ? "" : "  \u00b7  not a video")
                                color: Theme.inkFaint
                                font.pixelSize: 10
                            }
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.separator
        }

        // --- Footer ---------------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 62

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                spacing: 8

                Text {
                    text: "All"
                    color: allMouse.containsMouse ? Theme.accent : Theme.inkMuted
                    font.pixelSize: 11
                    MouseArea {
                        id: allMouse
                        anchors.fill: parent
                        anchors.margins: -4
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: picker.checkAll()
                    }
                }
                Text {
                    text: "None"
                    color: noneMouse.containsMouse ? Theme.accent : Theme.inkMuted
                    font.pixelSize: 11
                    MouseArea {
                        id: noneMouse
                        anchors.fill: parent
                        anchors.margins: -4
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: picker.checkNone()
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: picker.checkedCount + " selected \u00b7 "
                        + picker.formatSize(picker.checkedBytes)
                    color: Theme.inkMuted
                    font.pixelSize: 11
                }

                Rectangle {
                    radius: 8
                    color: (picker.checkedCount > 0 && downloadMouse.containsMouse)
                        ? Theme.accent : "transparent"
                    border.width: 1
                    border.color: picker.checkedCount > 0 ? Theme.accent : Theme.glassBorder
                    implicitWidth: downloadLabel.implicitWidth + 26
                    implicitHeight: 32
                    opacity: picker.checkedCount > 0 ? 1.0 : 0.5

                    Text {
                        id: downloadLabel
                        anchors.centerIn: parent
                        text: "Download"
                        color: (picker.checkedCount > 0 && downloadMouse.containsMouse)
                            ? "white" : Theme.ink
                        font.pixelSize: 12
                        font.bold: true
                    }
                    MouseArea {
                        id: downloadMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        enabled: picker.checkedCount > 0
                        cursorShape: Qt.PointingHandCursor
                        onClicked: picker.confirm()
                    }
                }
            }
        }
    }
}