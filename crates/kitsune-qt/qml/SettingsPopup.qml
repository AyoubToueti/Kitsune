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
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    // Fetch fresh on every open, so a change made elsewhere is not clobbered.
    onOpened: {
        if (backend) {
            backend.getSettings()
            backend.authStatus()
        }
    }

    // Auto-save whenever the popup is dismissed (Done button, Escape, or clicking outside).
    onClosed: {
        popup.save()
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
            popup.shortcuts = s.shortcuts || ({})
            popup.resumeValue = s.resumeBehavior || "ask"
            popup.rewindValue = (s.resumeRewindSeconds !== undefined && s.resumeRewindSeconds !== null)
                ? s.resumeRewindSeconds : 20
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

    // Resume behaviour: "ask" | "seek" | "fresh".
    property string resumeValue: "ask"
    // Seconds to rewind from the saved position on resume.
    property int rewindValue: 20

    // Player shortcut overrides, action -> key sequence. Edited by the
    // Shortcuts section and written back inside `current`.
    property var shortcuts: ({})

    // The actions the reader can rebind, with their built-in defaults. Kept in
    // sync with the `sc(...)` calls in EpisodeWatchModal.
    readonly property var shortcutActions: [
        { "key": "playPause",   "label": "Play / Pause",      "def": "Space" },
        { "key": "seekBack",    "label": "Seek back 5s",       "def": "Left" },
        { "key": "seekForward", "label": "Seek forward 5s",    "def": "Right" },
        { "key": "volumeUp",    "label": "Volume up",          "def": "Up" },
        { "key": "volumeDown",  "label": "Volume down",        "def": "Down" },
        { "key": "mute",        "label": "Mute",               "def": "M" },
        { "key": "fullscreen",  "label": "Toggle fullscreen",  "def": "F" },
        { "key": "skip",        "label": "Skip OP / ED",       "def": "S" }
    ]

    /// The current binding for an action, falling back to its default.
    function shortcutFor(key, def) {
        const v = popup.shortcuts[key]
        return (v && v.length > 0) ? v : def
    }

    /// Turn a key event into a Qt `Shortcut` sequence string, e.g. "Ctrl+Left".
    /// Mirrors the spellings QML accepts, so the saved value feeds straight
    /// back into a `Shortcut.sequence`.
    function sequenceFromEvent(event) {
        let mods = ""
        if (event.modifiers & Qt.ControlModifier) mods += "Ctrl+"
        if (event.modifiers & Qt.AltModifier) mods += "Alt+"
        if (event.modifiers & Qt.ShiftModifier) mods += "Shift+"
        if (event.modifiers & Qt.MetaModifier) mods += "Meta+"

        const text = event.text
        if (text && text.length > 0 && text.charCodeAt(0) >= 32
                && !(event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))) {
            return mods + text.toUpperCase()
        }
        const key = event.key
        const named = {
            32: "Space", 16777234: "Left", 16777236: "Right",
            16777235: "Up", 16777237: "Down", 16777220: "Return",
            16777217: "Tab", 16777216: "Escape", 16777219: "Backspace",
            16777223: "Delete"
        }
        if (named[key]) return mods + named[key]
        if (key >= 16777264 && key <= 16777299) return mods + "F" + (key - 16777263)
        if (text && text.length > 0) return mods + text.toUpperCase()
        return ""
    }

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
        s.resumeRewindSeconds = popup.rewindValue
        s.shortcuts = popup.shortcuts
        s.resumeBehavior = popup.resumeValue
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

            Rectangle {
                anchors.right: parent.right
                anchors.rightMargin: 20
                anchors.verticalCenter: parent.verticalCenter
                width: 32
                height: 32
                radius: 8
                color: closeMouse.containsMouse ? Theme.surfaceHover : "transparent"
                border.width: 1
                border.color: Theme.borderSubtle

                PlayerIcon {
                    anchors.centerIn: parent
                    kind: "close"
                    color: closeMouse.containsMouse ? Theme.ink : Theme.inkMuted
                    width: 12
                    height: 12
                }

                MouseArea {
                    id: closeMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: popup.close()
                }
            }
        }

        // --- Body -------------------------------------------------------
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: bodyColumn.implicitHeight + 16
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

                // ===== Player shortcuts =================================
                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    implicitHeight: scCol.implicitHeight + 28
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: scCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 14
                        spacing: 8

                        Text {
                            text: "PLAYER SHORTCUTS"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.2
                        }
                        Text {
                            text: "Click a field and press the key you want. Esc cancels."
                            color: Theme.inkFaint
                            font.pixelSize: 10
                        }

                        Repeater {
                            model: popup.shortcutActions
                            delegate: RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: 8

                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.label
                                    color: Theme.ink
                                    font.pixelSize: 12
                                }

                                TextField {
                                    id: scField
                                    Layout.preferredWidth: 120
                                    readOnly: true
                                    horizontalAlignment: TextInput.AlignHCenter
                                    color: activeFocus ? Theme.accent : Theme.ink
                                    text: activeFocus
                                        ? "Press a key…"
                                        : popup.shortcutFor(modelData.key, modelData.def)
                                    font.pixelSize: 12
                                    font.family: "monospace"
                                    background: Rectangle {
                                        radius: 8
                                        color: Theme.surface
                                        border.width: 1
                                        border.color: scField.activeFocus ? Theme.accent : Theme.borderSubtle
                                    }

                                    Keys.onPressed: (event) => {
                                        if (event.key === Qt.Key_Escape) {
                                            const next = Object.assign({}, popup.shortcuts)
                                            delete next[modelData.key]
                                            popup.shortcuts = next
                                            focus = false
                                            event.accepted = true
                                            return
                                        }
                                        if (event.key === Qt.Key_Shift || event.key === Qt.Key_Control
                                                || event.key === Qt.Key_Alt || event.key === Qt.Key_Meta) {
                                            event.accepted = true
                                            return
                                        }
                                        const seq = popup.sequenceFromEvent(event)
                                        if (seq.length > 0) {
                                            const next = Object.assign({}, popup.shortcuts)
                                            next[modelData.key] = seq
                                            popup.shortcuts = next
                                        }
                                        focus = false
                                        event.accepted = true
                                    }
                                }
                            }
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

                // ===== Resume ==========================================
                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    implicitHeight: resumeCol.implicitHeight + 28
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: resumeCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 14
                        spacing: 10

                        Text {
                            text: "CONTINUE WATCHING"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.2
                        }
                        Text {
                            text: "What to do when an episode has a saved position."
                            color: Theme.inkFaint
                            font.pixelSize: 10
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Repeater {
                                model: [
                                    { "value": "ask", "label": "Ask each time" },
                                    { "value": "seek", "label": "Seek automatically" },
                                    { "value": "fresh", "label": "Start fresh" }
                                ]
                                delegate: Rectangle {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    height: 34
                                    radius: 8
                                    readonly property bool active: popup.resumeValue === modelData.value
                                    color: resumeMouse.containsMouse && !active
                                        ? Theme.surfaceHover : "transparent"
                                    border.width: 1
                                    border.color: active ? Theme.accent : Theme.borderSubtle

                                    Text {
                                        anchors.centerIn: parent
                                        text: modelData.label
                                        color: active ? Theme.accent : Theme.inkMuted
                                        font.pixelSize: 11
                                        font.bold: active
                                    }
                                    MouseArea {
                                        id: resumeMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: popup.resumeValue = modelData.value
                                    }
                                }
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 4
                            Text {
                                text: "Rewind before resuming"
                                color: Theme.ink
                                font.pixelSize: 12
                                Layout.fillWidth: true
                            }
                            TextField {
                                id: rewindField
                                Layout.preferredWidth: 70
                                text: String(popup.rewindValue)
                                color: Theme.ink
                                font.pixelSize: 12
                                horizontalAlignment: TextInput.AlignHCenter
                                validator: IntValidator { bottom: 0; top: 3600 }
                                background: Rectangle {
                                    radius: 8; color: Theme.surface; border.width: 1
                                    border.color: rewindField.activeFocus ? Theme.accent : Theme.borderSubtle
                                }
                                onEditingFinished: {
                                    const n = parseInt(text, 10)
                                    popup.rewindValue = isFinite(n) && n >= 0 ? n : 0
                                    text = String(popup.rewindValue)
                                }
                            }
                            Text {
                                text: "seconds"
                                color: Theme.inkMuted
                                font.pixelSize: 11
                            }
                        }
                        Text {
                            text: "A saved position is rewound by this much so you get a run-up into the scene."
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
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

                // --- Internet speed test ---------------------------------
                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    implicitHeight: speedCol.implicitHeight + 28
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: speedCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 14
                        spacing: 10

                        Text {
                            text: "DIAGNOSTICS"
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.2
                        }

                        Text {
                            Layout.fillWidth: true
                            text: "Measure your connection against Cloudflare. Download speed and latency are what streaming depends on."
                            color: Theme.inkMuted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            Rectangle {
                                width: 150
                                height: 34
                                radius: 8
                                color: SpeedTestStore.running
                                    ? Theme.surfaceHover
                                    : (speedMouse.containsMouse ? Theme.accentHover : Theme.accent)

                                Text {
                                    anchors.centerIn: parent
                                    text: SpeedTestStore.running ? "Testing\u2026" : "Run speed test"
                                    color: SpeedTestStore.running ? Theme.inkFaint : "white"
                                    font.pixelSize: 12
                                    font.bold: true
                                }

                                MouseArea {
                                    id: speedMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    enabled: !SpeedTestStore.running
                                    cursorShape: SpeedTestStore.running
                                        ? Qt.ArrowCursor : Qt.PointingHandCursor
                                    onClicked: SpeedTestStore.run()
                                }
                            }

                            Item {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 8
                                visible: SpeedTestStore.running

                                readonly property real fraction: SpeedTestStore.progress
                                    ? Math.max(0, Math.min(1, SpeedTestStore.progress.percent / 100))
                                    : 0

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 4
                                    color: Theme.surface
                                    border.width: 1
                                    border.color: Theme.borderSubtle
                                }

                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.top: parent.top
                                    anchors.bottom: parent.bottom
                                    width: Math.max(parent.height, parent.width * parent.fraction)
                                    radius: 4
                                    color: Theme.accent

                                    Behavior on width { NumberAnimation { duration: 180 } }
                                }
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            visible: SpeedTestStore.error.length > 0
                            text: SpeedTestStore.error
                            color: Theme.danger
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10
                            visible: SpeedTestStore.result !== null

                            Repeater {
                                model: [
                                    { "label": "Latency", "value": SpeedTestStore.latencyText() },
                                    {
                                        "label": "Jitter",
                                        "value": (SpeedTestStore.result
                                            && SpeedTestStore.result.jitterMs !== undefined)
                                            ? Math.round(SpeedTestStore.result.jitterMs) + " ms"
                                            : ""
                                    },
                                    { "label": "Download", "value": SpeedTestStore.downloadText() }
                                ]

                                delegate: Rectangle {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    implicitHeight: tileCol.implicitHeight + 20
                                    radius: 8
                                    color: Theme.surface
                                    border.width: 1
                                    border.color: Theme.borderSubtle

                                    ColumnLayout {
                                        id: tileCol
                                        anchors.centerIn: parent
                                        spacing: 2

                                        Text {
                                            Layout.alignment: Qt.AlignHCenter
                                            text: modelData.label
                                            color: Theme.inkFaint
                                            font.pixelSize: 10
                                            font.bold: true
                                        }
                                        Text {
                                            Layout.alignment: Qt.AlignHCenter
                                            text: modelData.value
                                            color: Theme.accent
                                            font.pixelSize: 15
                                            font.bold: true
                                        }
                                    }
                                }
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            visible: SpeedTestStore.result !== null
                            text: SpeedTestStore.verdictText()
                            color: Theme.inkMuted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }
                    }
                }
            }
        }

        // --- Footer -----------------------------------------------------
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
                    onClicked: popup.close()
                }
            }
        }
    }
}