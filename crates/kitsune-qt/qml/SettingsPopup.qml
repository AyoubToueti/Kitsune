// The reader's preferences, as a modal popup rather than a page. Mirrors the
// Svelte SettingsDrawer: account, player, appearance, playback & downloads.
//
// A popup (not a stack page) so opening it never disturbs the page beneath:
// the reader keeps their place and the grid stays loaded.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

Popup {
    id: popup

    property var backend: null
    // The loaded settings object, edited in place and written back whole.
    property var current: null
    // Whether a token is stored, from `authStatus`.
    property bool signedIn: false

    // Centred over the window.
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 700
    height: Math.min(760, (parent ? parent.height : 800) - 60)
    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.CloseOnEscape

    // Fetch fresh on every open, so a change made elsewhere is not clobbered.
    onOpened: {
        if (backend) {
            backend.getSettings()
            backend.authStatus()
        }
    }

    Connections {
        target: popup.backend

        function onSettingsReady(json) {
            const s = JSON.parse(json)
            if (s === null) return
            popup.current = s
            playerField.text = s.player || ""
            playerArgsField.text = (s.playerArgs || []).join(" ")
            downloadField.text = s.downloadDir || ""
            thresholdSlider.value = s.readyFraction !== undefined ? s.readyFraction : 0.05
            seedersBox.value = s.minSeeders !== undefined ? s.minSeeders : 0
            askSwitch.checked = s.askEveryTime !== false
            popup.themeValue = s.theme || "system"
        }
        function onSettingsFailed(message) {
            status.text = "Failed: " + message
        }
        function onAuthStatusReady(signedInNow) {
            popup.signedIn = signedInNow
        }
        function onAuthError(message) {
            status.text = message
        }
    }

    // Which appearance segment is selected.
    property string themeValue: "system"

    function signIn() {
        if (!backend) return
        const url = backend.beginLogin()
        if (url && url.length > 0) Qt.openUrlExternally(url)
    }

    function disconnect() {
        if (backend) backend.logout()
    }

    // Write every edited field back as one Settings object. Untouched fields
    // ride along in `current`, so nothing else is reset.
    function save() {
        if (!current) return
        const s = current
        s.player = playerField.text
        s.playerArgs = playerArgsField.text.trim().length > 0
            ? playerArgsField.text.trim().split(/\s+/)
            : []
        s.downloadDir = downloadField.text.length > 0 ? downloadField.text : null
        s.readyFraction = thresholdSlider.value
        s.minSeeders = Math.round(seedersBox.value)
        s.askEveryTime = askSwitch.checked
        s.theme = popup.themeValue
        backend.setSettings(JSON.stringify(s))
    }

    FolderDialog {
        id: folderPicker
        title: "Choose download directory"
        onAccepted: {
            let p = selectedFolder.toString()
            if (p.startsWith("file://")) p = p.slice(7)
            downloadField.text = decodeURIComponent(p)
        }
    }

    background: Rectangle {
        radius: 16
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.borderSubtle
    }

    contentItem: ColumnLayout {
        spacing: 0

        // --- Header -----------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 60

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 24
                anchors.verticalCenter: parent.verticalCenter
                text: "Settings"
                color: Theme.ink
                font.pixelSize: 18
                font.bold: true
            }

            // Close X.
            Rectangle {
                anchors.right: parent.right
                anchors.rightMargin: 16
                anchors.verticalCenter: parent.verticalCenter
                width: 34
                height: 34
                radius: 9
                color: closeMouse.containsMouse ? Theme.surfaceHover : "transparent"
                border.width: 1
                border.color: Theme.borderSubtle

                Text {
                    anchors.centerIn: parent
                    text: "✕"
                    color: Theme.inkMuted
                    font.pixelSize: 14
                }
                MouseArea {
                    id: closeMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: { popup.save(); popup.close() }
                }
            }
        }

        // --- Body -------------------------------------------------------
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: bodyColumn.implicitHeight + 8
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            ColumnLayout {
                id: bodyColumn
                width: parent.width
                spacing: 16

                // ===== AniList account ==================================
                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    implicitHeight: accountCol.implicitHeight + 28
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: accountCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 14
                        spacing: 10

                        Text {
                            text: "ANILIST ACCOUNT"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.2
                        }

                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: popup.signedIn
                                    ? "●  Connected to AniList"
                                    : "○  Not signed in"
                                color: popup.signedIn ? "#4ade80" : Theme.inkMuted
                                font.pixelSize: 13
                            }
                            Item { Layout.fillWidth: true }

                            Rectangle {
                                width: 100
                                height: 30
                                radius: 8
                                color: acctMouse.containsMouse ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: acctMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                                Text {
                                    anchors.centerIn: parent
                                    text: popup.signedIn ? "Disconnect" : "Sign in"
                                    color: acctMouse.containsMouse ? Theme.accent : Theme.ink
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    id: acctMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: popup.signedIn ? popup.disconnect() : popup.signIn()
                                }
                            }
                        }
                    }
                }

                // ===== Player configuration =============================
                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    implicitHeight: playerCol.implicitHeight + 28
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: playerCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 14
                        spacing: 10

                        Text {
                            text: "PLAYER CONFIGURATION"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.2
                        }

                        Text { text: "Executable / Command"; color: Theme.ink; font.pixelSize: 12; font.bold: true }
                        TextField {
                            id: playerField
                            Layout.fillWidth: true
                            color: Theme.ink
                            placeholderText: "mpv"
                            placeholderTextColor: Theme.inkFaint
                            font.pixelSize: 12
                            background: Rectangle {
                                radius: 8; color: Theme.surface; border.width: 1
                                border.color: playerField.activeFocus ? Theme.accent : Theme.borderSubtle
                            }
                        }

                        Text { text: "Extra Arguments"; color: Theme.ink; font.pixelSize: 12; font.bold: true }
                        TextField {
                            id: playerArgsField
                            Layout.fillWidth: true
                            color: Theme.ink
                            placeholderText: "e.g. --fs --force-media-title"
                            placeholderTextColor: Theme.inkFaint
                            font.pixelSize: 12
                            background: Rectangle {
                                radius: 8; color: Theme.surface; border.width: 1
                                border.color: playerArgsField.activeFocus ? Theme.accent : Theme.borderSubtle
                            }
                        }
                        Text {
                            text: "Passed directly to player before stream URL"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                        }

                        Item { Layout.preferredHeight: 4 }

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                Layout.fillWidth: true
                                text: "Always prompt for media player choice before streaming"
                                color: Theme.ink
                                font.pixelSize: 12
                            }
                            Switch { id: askSwitch }
                        }
                    }
                }

                // ===== Appearance =======================================
                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    implicitHeight: themeCol.implicitHeight + 28
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: themeCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 14
                        spacing: 10

                        Text {
                            text: "APPEARANCE"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.2
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Repeater {
                                model: [
                                    { "value": "system", "label": "System" },
                                    { "value": "light", "label": "Light" },
                                    { "value": "dark", "label": "Dark" }
                                ]
                                delegate: Rectangle {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    height: 34
                                    radius: 8
                                    readonly property bool active: popup.themeValue === modelData.value
                                    color: themeMouse.containsMouse && !active
                                        ? Theme.surfaceHover : "transparent"
                                    border.width: 1
                                    border.color: active ? Theme.accent : Theme.borderSubtle

                                    Text {
                                        anchors.centerIn: parent
                                        text: modelData.label
                                        color: active ? Theme.accent : Theme.inkMuted
                                        font.pixelSize: 12
                                        font.bold: active
                                    }
                                    MouseArea {
                                        id: themeMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: popup.themeValue = modelData.value
                                    }
                                }
                            }
                        }
                    }
                }

                // ===== Playback & downloads =============================
                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    Layout.bottomMargin: 8
                    implicitHeight: playCol.implicitHeight + 28
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: playCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 14
                        spacing: 10

                        Text {
                            text: "PLAYBACK & DOWNLOADS"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.2
                        }

                        Text { text: "Download Directory"; color: Theme.ink; font.pixelSize: 12; font.bold: true }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            TextField {
                                id: downloadField
                                Layout.fillWidth: true
                                color: Theme.ink
                                placeholderText: "(system temp directory)"
                                placeholderTextColor: Theme.inkFaint
                                font.pixelSize: 12
                                background: Rectangle {
                                    radius: 8; color: Theme.surface; border.width: 1
                                    border.color: downloadField.activeFocus ? Theme.accent : Theme.borderSubtle
                                }
                            }
                            Rectangle {
                                width: 80; height: 32; radius: 8
                                color: browseMouse.containsMouse ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: browseMouse.containsMouse ? Theme.accent : Theme.borderSubtle
                                Text {
                                    anchors.centerIn: parent
                                    text: "Browse…"
                                    color: browseMouse.containsMouse ? Theme.accent : Theme.ink
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    id: browseMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: folderPicker.open()
                                }
                            }
                            Rectangle {
                                width: 60; height: 32; radius: 8
                                color: resetMouse.containsMouse ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: resetMouse.containsMouse ? Theme.accent : Theme.borderSubtle
                                Text {
                                    anchors.centerIn: parent
                                    text: "Reset"
                                    color: resetMouse.containsMouse ? Theme.accent : Theme.ink
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    id: resetMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: downloadField.text = ""
                                }
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 4
                            Text {
                                Layout.fillWidth: true
                                text: "Playback Buffer Threshold"
                                color: Theme.ink
                                font.pixelSize: 12
                            }
                            Text {
                                text: Math.round(thresholdSlider.value * 100) + "%"
                                color: Theme.accent
                                font.pixelSize: 12
                                font.bold: true
                            }
                        }
                        Slider {
                            id: thresholdSlider
                            Layout.fillWidth: true
                            from: 0.01
                            to: 1.0
                            stepSize: 0.01
                        }

                        Text {
                            Layout.topMargin: 4
                            text: "Minimum Seeders"
                            color: Theme.ink
                            font.pixelSize: 12
                            font.bold: true
                        }
                        SpinBox {
                            id: seedersBox
                            from: 0
                            to: 1000
                        }
                    }
                }
            }
        }

        // --- Footer -----------------------------------------------------
        // A solid background so body content scrolling under it does not show
        // through behind the Done button.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 60
            color: Theme.surfaceRaised

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 1
                color: Theme.borderSubtle
            }

            Text {
                id: status
                anchors.left: parent.left
                anchors.leftMargin: 24
                anchors.verticalCenter: parent.verticalCenter
                color: Theme.danger
                font.pixelSize: 11
            }

            Rectangle {
                anchors.right: parent.right
                anchors.rightMargin: 24
                anchors.verticalCenter: parent.verticalCenter
                width: 80
                height: 34
                radius: 8
                color: doneMouse.containsMouse ? Theme.accentHover : Theme.accent

                Text {
                    anchors.centerIn: parent
                    text: "Done"
                    color: "white"
                    font.pixelSize: 13
                    font.bold: true
                }
                MouseArea {
                    id: doneMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: { popup.save(); popup.close() }
                }
            }
        }
    }
}