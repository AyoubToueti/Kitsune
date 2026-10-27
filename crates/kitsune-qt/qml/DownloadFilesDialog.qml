// Manage which files a downloaded torrent keeps.
//
// Unlike the pre-download picker, this edits an EXISTING group: adding a file
// creates a new download row, removing one drops its row (a completed library
// file is left on disk). The backend reattaches the torrent paused when it has
// no live handle, changes `only_files`, then resumes only if it was running.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: dialog

    property var backend: null
    property string groupKey: ""
    property string groupName: ""
    property string token: ""
    property bool loading: false
    property bool saving: false
    property bool pendingSave: false
    property string error: ""
    property var files: []
    // idx -> true. Sparse object so non-contiguous torrent indices need no
    // index juggling.
    property var checked: ({})

    readonly property var checkedIndices: {
        const out = []
        for (const file of files) if (checked[file.idx]) out.push(file.idx)
        return out
    }
    readonly property int checkedCount: checkedIndices.length
    readonly property real checkedBytes: {
        let total = 0
        for (const file of files)
            if (checked[file.idx]) total += (file.lengthBytes || 0)
        return total
    }

    signal filesApplied()

    modal: true
    closePolicy: Popup.CloseOnEscape
    padding: 0
    anchors.centerIn: parent
    width: Math.min(680, parent ? parent.width - 80 : 680)
    height: Math.min(600, parent ? parent.height - 80 : 600)

    function openFor(key, name) {
        groupKey = key
        groupName = name || ""
        files = []
        checked = ({})
        error = ""
        loading = true
        saving = false
        pendingSave = false
        token = String(Date.now()) + "-" + Math.floor(Math.random() * 100000)
        if (backend) backend.listDownloadFiles(token, key)
        else error = "Downloads are unavailable."
        open()
    }

    function toggle(idx) {
        const next = Object.assign({}, checked)
        if (next[idx]) delete next[idx]
        else next[idx] = true
        checked = next
    }

    function checkAll() {
        const all = {}
        for (const file of files) all[file.idx] = true
        checked = all
    }

    function checkNone() {
        checked = ({})
    }

    function save() {
        if (!backend || checkedCount === 0 || saving) return
        saving = true
        pendingSave = true
        error = ""
        backend.setDownloadFiles(
            token, groupKey, JSON.stringify(checkedIndices.slice()))
    }

    function formatSize(bytes) {
        if (!bytes || bytes <= 0) return "\u2014"
        const units = ["B", "KiB", "MiB", "GiB", "TiB"]
        let v = bytes, u = 0
        while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
        return v.toFixed(v < 10 && u > 0 ? 1 : 0) + " " + units[u]
    }

    Connections {
        target: dialog.backend

        function onDownloadFilesReady(readyToken, json) {
            if (readyToken !== dialog.token) return
            const payload = JSON.parse(json)
            dialog.loading = false
            if (payload === null || !payload.files) {
                dialog.saving = false
                dialog.pendingSave = false
                dialog.error = "This torrent returned no files."
                return
            }

            dialog.files = payload.files
            dialog.groupName = payload.name || dialog.groupName

            if (dialog.pendingSave) {
                dialog.pendingSave = false
                dialog.saving = false
                dialog.filesApplied()
                dialog.close()
                return
            }

            const selected = {}
            for (const file of dialog.files)
                if (file.selected) selected[file.idx] = true
            dialog.checked = selected
        }

        function onDownloadFilesFailed(readyToken, message) {
            if (readyToken !== dialog.token) return
            dialog.loading = false
            dialog.saving = false
            dialog.pendingSave = false
            dialog.error = message
        }
    }

    background: GlassPanel {
        strong: true
        radius: 14
    }

    contentItem: ColumnLayout {
        spacing: 0

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 64

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 18
                anchors.rightMargin: 48
                spacing: 2

                Text {
                    text: "Manage files"
                    color: Theme.ink
                    font.pixelSize: 15
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    text: dialog.groupName
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }

            MouseArea {
                id: closeMouse
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                width: 24
                height: 24
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: dialog.close()
                Text {
                    anchors.centerIn: parent
                    text: "\u2715"
                    color: closeMouse.containsMouse ? Theme.accent : Theme.inkMuted
                    font.pixelSize: 14
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.separator
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.centerIn: parent
                visible: dialog.loading
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
                visible: !dialog.loading && dialog.error.length > 0
                spacing: 10
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Could not update this download"
                    color: Theme.ink
                    font.pixelSize: 13
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    text: dialog.error
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: retryText.implicitWidth + 24
                    height: 30
                    radius: 8
                    color: retryMouse.containsMouse ? Theme.glassFillHover : "transparent"
                    border.width: 1
                    border.color: Theme.glassBorder
                    Text {
                        id: retryText
                        anchors.centerIn: parent
                        text: "Try again"
                        color: Theme.ink
                        font.pixelSize: 11
                    }
                    MouseArea {
                        id: retryMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: dialog.openFor(dialog.groupKey, dialog.groupName)
                    }
                }
            }

            ListView {
                anchors.fill: parent
                anchors.margins: 10
                clip: true
                spacing: 4
                visible: !dialog.loading && dialog.error.length === 0
                model: dialog.files

                delegate: Rectangle {
                    required property var modelData

                    readonly property bool isChecked: dialog.checked[modelData.idx] === true

                    width: ListView.view.width
                    height: 48
                    radius: 8
                    color: isChecked ? Theme.glassFillHover : "transparent"
                    border.width: 1
                    border.color: isChecked ? Theme.glassBorderHover : Theme.glassBorder

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: dialog.toggle(modelData.idx)
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 10

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
                                text: dialog.formatSize(modelData.lengthBytes)
                                    + (modelData.selected ? "  \u00b7  already selected" : "")
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
                        onClicked: dialog.checkAll()
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
                        onClicked: dialog.checkNone()
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: dialog.checkedCount + " selected \u00b7 "
                        + dialog.formatSize(dialog.checkedBytes)
                    color: Theme.inkMuted
                    font.pixelSize: 11
                }

                Rectangle {
                    radius: 8
                    color: (dialog.checkedCount > 0 && saveMouse.containsMouse)
                        ? Theme.accent : "transparent"
                    border.width: 1
                    border.color: dialog.checkedCount > 0
                        ? Theme.accent : Theme.glassBorder
                    implicitWidth: saveLabel.implicitWidth + 26
                    implicitHeight: 32
                    opacity: dialog.checkedCount > 0 && !dialog.saving ? 1.0 : 0.5

                    Text {
                        id: saveLabel
                        anchors.centerIn: parent
                        text: dialog.saving ? "Saving\u2026" : "Save changes"
                        color: (dialog.checkedCount > 0 && saveMouse.containsMouse)
                            ? "white" : Theme.ink
                        font.pixelSize: 12
                        font.bold: true
                    }
                    MouseArea {
                        id: saveMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        enabled: dialog.checkedCount > 0 && !dialog.saving
                        cursorShape: Qt.PointingHandCursor
                        onClicked: dialog.save()
                    }
                }
            }
        }
    }
}
