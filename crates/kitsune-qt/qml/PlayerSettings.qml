import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: card

    property var player: null
    property bool open: false
    property string tab: "playback"
    // Current shortcut overrides, action -> key sequence, from the modal.
    property var shortcuts: ({})

    signal closeRequested()
    /// Emitted when the reader rebinds an action. An empty sequence clears the
    /// override so the action falls back to its default.
    signal shortcutChanged(string action, string sequence)

    // The actions the reader can rebind, with their built-in defaults. Kept in
    // sync with the `sc(...)` calls in EpisodeWatchModal.
    readonly property var shortcutActions: [
        { "key": "playPause",   "label": "Play / Pause",     "def": "Space" },
        { "key": "seekBack",    "label": "Seek back 5s",      "def": "Left" },
        { "key": "seekForward", "label": "Seek forward 5s",   "def": "Right" },
        { "key": "volumeUp",    "label": "Volume up",         "def": "Up" },
        { "key": "volumeDown",  "label": "Volume down",       "def": "Down" },
        { "key": "mute",        "label": "Mute",              "def": "M" },
        { "key": "fullscreen",  "label": "Toggle fullscreen", "def": "F" },
        { "key": "skip",        "label": "Skip OP / ED",      "def": "S" }
    ]

    /// The current binding for an action, falling back to its default.
    function shortcutFor(key, def) {
        const v = card.shortcuts[key]
        return (v && v.length > 0) ? v : def
    }

    /// Turn a key event into a Qt `Shortcut` sequence string, e.g. "Ctrl+Left".
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

    width: 340
    height: 440

    onOpenChanged: if (open) card.seed()

    property string aspect: "-1"
    property int videoRotation: 0
    property real zoom: 0
    property real panX: 0
    property real panY: 0
    property real brightness: 0
    property real contrast: 0
    property real saturation: 0
    property real gamma: 0
    property real audioDelay: 0
    property real subDelay: 0
    property real subScale: 1
    property int audioTrack: -1
    property int subTrack: -1
    property bool subVisible: true
    property int trackTick: 0

    function num(name, fallback) {
        if (card.player === null) return fallback
        const v = card.player.getProperty(name)
        if (v === undefined || v === null || v === "") return fallback
        const n = Number(v)
        return isFinite(n) ? n : fallback
    }

    function str(name, fallback) {
        if (card.player === null) return fallback
        const v = card.player.getProperty(name)
        if (v === undefined || v === null || v === "") return fallback
        return String(v)
    }

    function seed() {
        card.aspect = card.str("video-aspect-override", "-1")
        card.videoRotation = Math.round(card.num("video-rotate", 0))
        card.zoom = card.num("video-zoom", 0)
        card.panX = card.num("video-pan-x", 0)
        card.panY = card.num("video-pan-y", 0)
        card.brightness = card.num("brightness", 0)
        card.contrast = card.num("contrast", 0)
        card.saturation = card.num("saturation", 0)
        card.gamma = card.num("gamma", 0)
        card.audioDelay = card.num("audio-delay", 0)
        card.subDelay = card.num("sub-delay", 0)
        card.subScale = card.num("sub-scale", 1)
        card.audioTrack = Math.round(card.num("aid", -1))
        card.subTrack = Math.round(card.num("sid", -1))
        card.subVisible = card.str("sub-visibility", "yes") !== "no"
        card.trackTick++
    }

    function rate() { return card.player !== null ? card.player.speed : 1 }
    function hwdec() {
        if (card.player === null) return "auto"
        const v = card.player.getProperty("hwdec")
        return (v === undefined || v === null || v === "") ? "auto" : String(v)
    }

    function tracks(kind) {
        if (card.player === null) return []
        const count = card.num("track-list/count", 0)
        const out = []
        for (let i = 0; i < count; i++) {
            if (card.str("track-list/" + i + "/type", "") !== kind) continue
            out.push({
                "id": card.num("track-list/" + i + "/id", 0),
                "lang": card.str("track-list/" + i + "/lang", ""),
                "title": card.str("track-list/" + i + "/title", ""),
                "selected": card.player.getProperty("track-list/" + i + "/selected") === true
            })
        }
        return out
    }

    function trackLabel(t) {
        const parts = []
        if (t.lang) parts.push(t.lang)
        if (t.title) parts.push(t.title)
        if (parts.length === 0) parts.push("Track " + t.id)
        return parts.join(" · ")
    }

    function resetPicture() {
        card.zoom = 0; card.panX = 0; card.panY = 0
        card.videoRotation = 0; card.aspect = "-1"
        card.brightness = 0; card.contrast = 0; card.saturation = 0; card.gamma = 0

        if (card.player === null) return
        const p = card.player
        p.setProperty("video-zoom", "0")
        p.setProperty("video-pan-x", "0")
        p.setProperty("video-pan-y", "0")
        p.setProperty("video-rotate", "0")
        p.setProperty("video-aspect-override", "-1")
        p.setProperty("brightness", "0")
        p.setProperty("contrast", "0")
        p.setProperty("saturation", "0")
        p.setProperty("gamma", "0")
    }

    GlassPanel {
        anchors.fill: parent
        strong: true

        // Absorbs clicks inside the settings card so they don't trigger the dismiss backdrop
        MouseArea {
            anchors.fill: parent
            onClicked: {}
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            RowLayout {
                Layout.fillWidth: true

                Text {
                    Layout.fillWidth: true
                    text: "Player settings"
                    color: Theme.ink
                    font.pixelSize: 14
                    font.bold: true
                }
                Rectangle {
                    width: 26; height: 26; radius: 7
                    color: closeMouse.containsMouse ? Theme.glassFillHover : "transparent"
                    border.width: 1
                    border.color: Theme.glassBorder
                    PlayerIcon {
                        anchors.centerIn: parent
                        kind: "close"
                        color: Theme.ink
                        width: 12; height: 12
                    }
                    MouseArea {
                        id: closeMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            card.open = false
                            card.closeRequested()
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 4

                Repeater {
                    model: [
                        { "value": "playback", "label": "Playback" },
                        { "value": "video", "label": "Video" },
                        { "value": "audio", "label": "Audio" },
                        { "value": "subtitles", "label": "Subs" },
                        { "value": "shortcuts", "label": "Keys" }
                    ]
                    delegate: Rectangle {
                        required property var modelData
                        readonly property bool active: card.tab === modelData.value
                        Layout.fillWidth: true
                        height: 28
                        radius: 8
                        color: active ? Theme.accent : Theme.glassFillSubtle
                        border.width: 1
                        border.color: active ? Theme.accent : Theme.glassBorder

                        Text {
                            anchors.centerIn: parent
                            text: modelData.label
                            color: active ? "white" : Theme.inkMuted
                            font.pixelSize: 11
                            font.bold: active
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: card.tab = modelData.value
                        }
                    }
                }
            }

            Flickable {
                Layout.fillWidth: true
                Layout.fillHeight: true
                contentWidth: width
                contentHeight: body.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds

                ColumnLayout {
                    id: body
                    width: parent.width
                    spacing: 14
                    visible: card.tab === "playback"

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        SettingLabel { text: "Playback speed" }
                        Flow {
                            Layout.fillWidth: true
                            Layout.preferredHeight: childrenRect.height
                            spacing: 6
                            Repeater {
                                model: ["0.25", "0.5", "0.75", "1", "1.25", "1.5", "1.75", "2", "3"]
                                delegate: GlassChip {
                                    required property string modelData
                                    text: modelData + "×"
                                    active: String(card.rate()) === modelData
                                    onClicked: if (card.player) card.player.setProperty("speed", modelData)
                                }
                            }
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.glassBorder }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        SettingLabel { text: "Hardware decoding" }
                        Text {
                            Layout.fillWidth: true
                            text: "Applies to the next file loaded."
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            wrapMode: Text.WordWrap
                        }
                        Flow {
                            Layout.fillWidth: true
                            Layout.preferredHeight: childrenRect.height
                            spacing: 6
                            Repeater {
                                model: [
                                    { "value": "auto", "label": "Auto" },
                                    { "value": "auto-safe", "label": "Auto (safe)" },
                                    { "value": "nvdec", "label": "NVDEC" },
                                    { "value": "vaapi", "label": "VAAPI" },
                                    { "value": "d3d11va", "label": "D3D11VA" },
                                    { "value": "no", "label": "Off" }
                                ]
                                delegate: GlassChip {
                                    required property var modelData
                                    text: modelData.label
                                    active: card.hwdec() === modelData.value
                                    onClicked: if (card.player) card.player.setProperty("hwdec", modelData.value)
                                }
                            }
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.glassBorder }

                    RowLayout {
                        Layout.fillWidth: true
                        GlassButton {
                            label: "Reset picture"
                            onClicked: card.resetPicture()
                        }
                        Item { Layout.fillWidth: true }
                    }
                }

                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    visible: card.tab === "video"

                    SettingLabel { text: "Aspect ratio" }
                    Flow {
                        Layout.fillWidth: true
                        Layout.preferredHeight: childrenRect.height
                        spacing: 6
                        Repeater {
                            model: [
                                { "value": "-1", "label": "Original" },
                                { "value": "16:9", "label": "16:9" },
                                { "value": "4:3", "label": "4:3" },
                                { "value": "21:9", "label": "21:9" },
                                { "value": "2.35:1", "label": "2.35:1" },
                                { "value": "1:1", "label": "1:1" }
                            ]
                            delegate: GlassChip {
                                required property var modelData
                                text: modelData.label
                                active: card.aspect === modelData.value
                                onClicked: {
                                    card.aspect = modelData.value
                                    if (card.player) card.player.setProperty("video-aspect-override", modelData.value)
                                }
                            }
                        }
                    }

                    SettingLabel { text: "Rotation" }
                    Flow {
                        Layout.fillWidth: true
                        Layout.preferredHeight: childrenRect.height
                        spacing: 6
                        Repeater {
                            model: [0, 90, 180, 270]
                            delegate: GlassChip {
                                required property int modelData
                                text: modelData + "°"
                                active: card.videoRotation === modelData
                                onClicked: {
                                    card.videoRotation = modelData
                                    if (card.player) card.player.setProperty("video-rotate", String(modelData))
                                }
                            }
                        }
                    }

                    LabeledSlider {
                        label: "Zoom"; from: -1; to: 3; step: 0.05
                        value: card.zoom; suffix: "×"
                        onMoved: function (v) {
                            card.zoom = v
                            if (card.player) card.player.setProperty("video-zoom", String(v))
                        }
                    }
                    LabeledSlider {
                        label: "Pan X"; from: -1; to: 1; step: 0.05
                        value: card.panX
                        onMoved: function (v) {
                            card.panX = v
                            if (card.player) card.player.setProperty("video-pan-x", String(v))
                        }
                    }
                    LabeledSlider {
                        label: "Pan Y"; from: -1; to: 1; step: 0.05
                        value: card.panY
                        onMoved: function (v) {
                            card.panY = v
                            if (card.player) card.player.setProperty("video-pan-y", String(v))
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.glassBorder }
                    SettingLabel { text: "Colour equalizer" }

                    LabeledSlider {
                        label: "Brightness"; from: -100; to: 100; step: 1
                        value: card.brightness
                        onMoved: function (v) {
                            card.brightness = v
                            if (card.player) card.player.setProperty("brightness", String(Math.round(v)))
                        }
                    }
                    LabeledSlider {
                        label: "Contrast"; from: -100; to: 100; step: 1
                        value: card.contrast
                        onMoved: function (v) {
                            card.contrast = v
                            if (card.player) card.player.setProperty("contrast", String(Math.round(v)))
                        }
                    }
                    LabeledSlider {
                        label: "Saturation"; from: -100; to: 100; step: 1
                        value: card.saturation
                        onMoved: function (v) {
                            card.saturation = v
                            if (card.player) card.player.setProperty("saturation", String(Math.round(v)))
                        }
                    }
                    LabeledSlider {
                        label: "Gamma"; from: -100; to: 100; step: 1
                        value: card.gamma
                        onMoved: function (v) {
                            card.gamma = v
                            if (card.player) card.player.setProperty("gamma", String(Math.round(v)))
                        }
                    }
                }

                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    visible: card.tab === "audio"

                    SettingLabel { text: "Audio track" }
                    TrackList { kind: "audio" }

                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.glassBorder }

                    LabeledSlider {
                        label: "Audio delay"; from: -10; to: 10; step: 0.05
                        value: card.audioDelay; suffix: " s"
                        onMoved: function (v) {
                            card.audioDelay = v
                            if (card.player) card.player.setProperty("audio-delay", String(v))
                        }
                    }
                }

                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    visible: card.tab === "subtitles"

                    RowLayout {
                        Layout.fillWidth: true
                        SettingLabel { text: "Show subtitles"; Layout.fillWidth: true }
                        Switch {
                            checked: card.subVisible
                            onToggled: {
                                card.subVisible = checked
                                if (card.player) card.player.setProperty("sub-visibility", checked)
                            }
                        }
                    }

                    SettingLabel { text: "Subtitle track" }
                    TrackList { kind: "sub" }

                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.glassBorder }

                    LabeledSlider {
                        label: "Subtitle delay"; from: -10; to: 10; step: 0.05
                        value: card.subDelay; suffix: " s"
                        onMoved: function (v) {
                            card.subDelay = v
                            if (card.player) card.player.setProperty("sub-delay", String(v))
                        }
                    }
                    LabeledSlider {
                        label: "Subtitle size"; from: 0.5; to: 3; step: 0.1
                        value: card.subScale; suffix: "×"
                        onMoved: function (v) {
                            card.subScale = v
                            if (card.player) card.player.setProperty("sub-scale", String(v))
                        }
                    }
                }

                ColumnLayout {
                    width: parent.width
                    spacing: 8
                    visible: card.tab === "shortcuts"

                    Text {
                        Layout.fillWidth: true
                        text: "Click a key field, then press the key you want. Esc clears back to the default."
                        color: Theme.inkMuted
                        font.pixelSize: 10
                        wrapMode: Text.WordWrap
                    }

                    Repeater {
                        model: card.shortcutActions
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                Layout.fillWidth: true
                                text: modelData.label
                                color: Theme.ink
                                font.pixelSize: 11
                            }

                            TextField {
                                id: scField
                                Layout.preferredWidth: 110
                                readOnly: true
                                horizontalAlignment: TextInput.AlignHCenter
                                color: activeFocus ? Theme.accent : Theme.ink
                                text: activeFocus
                                    ? "Press a key…"
                                    : card.shortcutFor(modelData.key, modelData.def)
                                font.pixelSize: 11
                                font.family: "monospace"
                                background: Rectangle {
                                    radius: 8
                                    color: Theme.surface
                                    border.width: 1
                                    border.color: scField.activeFocus ? Theme.accent : Theme.glassBorder
                                }

                                Keys.onPressed: (event) => {
                                    if (event.key === Qt.Key_Escape) {
                                        card.shortcutChanged(modelData.key, "")
                                        focus = false
                                        event.accepted = true
                                        return
                                    }
                                    if (event.key === Qt.Key_Shift || event.key === Qt.Key_Control
                                            || event.key === Qt.Key_Alt || event.key === Qt.Key_Meta) {
                                        event.accepted = true
                                        return
                                    }
                                    const seq = card.sequenceFromEvent(event)
                                    if (seq.length > 0)
                                        card.shortcutChanged(modelData.key, seq)
                                    focus = false
                                    event.accepted = true
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    component SettingLabel: Text {
        color: Theme.ink
        font.pixelSize: 11
        font.bold: true
    }

    component GlassChip: Rectangle {
        property string text: ""
        property bool active: false
        signal clicked()

        radius: 12
        color: active ? Theme.accent : Theme.glassFillSubtle
        border.width: 1
        border.color: active ? Theme.accent : Theme.glassBorder
        implicitWidth: chipLabel.implicitWidth + 20
        implicitHeight: 26

        Text {
            id: chipLabel
            anchors.centerIn: parent
            text: parent.text
            color: parent.active ? "white" : Theme.inkMuted
            font.pixelSize: 11
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    component GlassButton: Rectangle {
        property string label: ""
        signal clicked()

        width: btnLabel.implicitWidth + 22
        height: 30
        radius: 9
        color: btnMouse.containsMouse ? Theme.glassFillHover : Theme.glassFillSubtle
        border.width: 1
        border.color: btnMouse.containsMouse ? Theme.accent : Theme.glassBorder

        Text {
            id: btnLabel
            anchors.centerIn: parent
            text: parent.label
            color: btnMouse.containsMouse ? Theme.accent : Theme.ink
            font.pixelSize: 11
        }
        MouseArea {
            id: btnMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    component LabeledSlider: ColumnLayout {
        id: sliderRoot
        property string label: ""
        property real from: 0
        property real to: 1
        property real step: 0.1
        property real value: 0
        property string suffix: ""
        signal moved(real value)

        Layout.fillWidth: true
        spacing: 2

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: sliderRoot.label
                color: Theme.inkMuted
                font.pixelSize: 11
            }
            Text {
                text: (Math.round(sliderRoot.value * 100) / 100) + sliderRoot.suffix
                color: Theme.accent
                font.pixelSize: 11
                font.bold: true
            }
        }
        Slider {
            Layout.fillWidth: true
            from: sliderRoot.from
            to: sliderRoot.to
            stepSize: sliderRoot.step
            Component.onCompleted: value = sliderRoot.value
            onMoved: sliderRoot.moved(value)
        }
    }

    component TrackList: ColumnLayout {
        id: trackList
        property string kind: "audio"

        Layout.fillWidth: true
        spacing: 6

        Repeater {
            model: {
                const _ = card.trackTick
                const head = [{
                    "id": -1,
                    "lang": "",
                    "title": trackList.kind === "audio" ? "Auto (default)" : "Off / auto",
                    "selected": false
                }]
                return head.concat(card.tracks(trackList.kind))
            }
            delegate: Rectangle {
                required property var modelData
                readonly property int current: trackList.kind === "audio"
                    ? card.audioTrack : card.subTrack
                readonly property bool active: modelData.id === current
                Layout.fillWidth: true
                height: 30
                radius: 8
                color: active ? Theme.accent : Theme.glassFillSubtle
                border.width: 1
                border.color: active ? Theme.accent : Theme.glassBorder

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    text: card.trackLabel(modelData)
                    color: active ? "white" : Theme.inkMuted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        if (trackList.kind === "audio") {
                            card.audioTrack = modelData.id
                            if (card.player) card.player.setProperty("aid", String(modelData.id))
                        } else {
                            card.subTrack = modelData.id
                            if (card.player) card.player.setProperty("sid", String(modelData.id))
                        }
                    }
                }
            }
        }
    }
}