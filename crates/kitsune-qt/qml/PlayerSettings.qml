// The player's advanced settings, as a tabbed popup opened from the OSD gear.
//
// Stage 1 fills the Playback tab (speed, hardware decoding). The Video, Audio
// and Subtitles tabs are added by the following stages; until then they are
// present but empty, so the tab strip does not move when they fill in.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: popup

    // The live MpvObject, or null before the play stage creates it.
    property var player: null
    // Which tab is showing.
    property string tab: "playback"

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 460
    height: Math.min(implicitHeight, (parent ? parent.height : 800) - 80)
    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    // --- local mirror of the adjustable values ---------------------------
    //
    // mpv's properties are not Qt properties, so a QML binding cannot track
    // them. These are seeded from mpv when the popup opens and written back on
    // change. Nothing else in the app alters them, so a one-way seed is enough
    // and avoids fighting the 200ms poll.
    property string aspect: "-1"
    property int rotation: 0
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
    // Selected track ids; -1 means "auto"/none, the default.
    property int audioTrack: -1
    property int subTrack: -1
    property bool subVisible: true
    // Bumped when the track lists should be re-read.
    property int trackTick: 0

    onOpened: popup.seed()

    function num(name, fallback) {
        if (popup.player === null) return fallback
        const v = popup.player.getProperty(name)
        if (v === undefined || v === null || v === "") return fallback
        const n = Number(v)
        return isFinite(n) ? n : fallback
    }

    function str(name, fallback) {
        if (popup.player === null) return fallback
        const v = popup.player.getProperty(name)
        if (v === undefined || v === null || v === "") return fallback
        return String(v)
    }

    // Read every adjustable value off mpv into the local mirror.
    function seed() {
        popup.aspect = popup.str("video-aspect-override", "-1")
        popup.rotation = Math.round(popup.num("video-rotate", 0))
        popup.zoom = popup.num("video-zoom", 0)
        popup.panX = popup.num("video-pan-x", 0)
        popup.panY = popup.num("video-pan-y", 0)
        popup.brightness = popup.num("brightness", 0)
        popup.contrast = popup.num("contrast", 0)
        popup.saturation = popup.num("saturation", 0)
        popup.gamma = popup.num("gamma", 0)
        popup.audioDelay = popup.num("audio-delay", 0)
        popup.subDelay = popup.num("sub-delay", 0)
        popup.subScale = popup.num("sub-scale", 1)
        popup.audioTrack = Math.round(popup.num("aid", -1))
        popup.subTrack = Math.round(popup.num("sid", -1))
        popup.subVisible = popup.str("sub-visibility", "yes") !== "no"
        popup.trackTick++
    }

    function rate() { return popup.player !== null ? popup.player.speed : 1 }
    function hwdec() {
        if (popup.player === null) return "auto"
        const v = popup.player.getProperty("hwdec")
        return (v === undefined || v === null || v === "") ? "auto" : String(v)
    }

    /// The tracks of one kind ("audio" | "sub"), read field by field.
    ///
    /// `track-list` is a collection, which `getProperty` cannot return (it
    /// decodes scalars only), but mpv also exposes `track-list/N/field` paths
    /// -- so the list is rebuilt from `track-list/count` here.
    function tracks(kind) {
        if (popup.player === null) return []
        const count = popup.num("track-list/count", 0)
        const out = []
        for (let i = 0; i < count; i++) {
            if (popup.str("track-list/" + i + "/type", "") !== kind) continue
            out.push({
                "id": popup.num("track-list/" + i + "/id", 0),
                "lang": popup.str("track-list/" + i + "/lang", ""),
                "title": popup.str("track-list/" + i + "/title", ""),
                "selected": popup.player.getProperty("track-list/" + i + "/selected") === true
            })
        }
        return out
    }

    /// A human label for a track row.
    function trackLabel(t) {
        const parts = []
        if (t.lang) parts.push(t.lang)
        if (t.title) parts.push(t.title)
        if (parts.length === 0) parts.push("Track " + t.id)
        return parts.join(" · ")
    }

    background: Rectangle {
        radius: 14
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.borderSubtle
    }

    contentItem: ColumnLayout {
        spacing: 0

        // --- Header ------------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 54

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 20
                anchors.verticalCenter: parent.verticalCenter
                text: "Player settings"
                color: Theme.ink
                font.pixelSize: 16
                font.bold: true
            }
            Rectangle {
                anchors.right: parent.right
                anchors.rightMargin: 16
                anchors.verticalCenter: parent.verticalCenter
                width: 30; height: 30; radius: 8
                color: closeMouse.containsMouse ? Theme.surfaceHover : "transparent"
                border.width: 1
                border.color: Theme.borderSubtle
                Text {
                    anchors.centerIn: parent
                    text: "✕"
                    color: Theme.inkMuted
                    font.pixelSize: 12
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

        // --- Tab strip ---------------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 20
            Layout.rightMargin: 20
            spacing: 6

            Repeater {
                model: [
                    { "value": "playback", "label": "Playback" },
                    { "value": "video", "label": "Video" },
                    { "value": "audio", "label": "Audio" },
                    { "value": "subtitles", "label": "Subtitles" }
                ]
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool active: popup.tab === modelData.value
                    Layout.fillWidth: true
                    height: 32
                    radius: 8
                    color: active ? Theme.accent : "transparent"
                    border.width: 1
                    border.color: active ? Theme.accent : Theme.borderSubtle

                    Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: active ? "white" : Theme.inkMuted
                        font.pixelSize: 12
                        font.bold: active
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: popup.tab = modelData.value
                    }
                }
            }
        }

        // --- Body --------------------------------------------------------
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.topMargin: 14
            Layout.bottomMargin: 14
            contentWidth: width
            contentHeight: body.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            // ===== Playback tab ==========================================
            ColumnLayout {
                id: body
                width: parent.width - 40
                x: 20
                spacing: 16
                visible: popup.tab === "playback"

                // Speed.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Text {
                        text: "Playback speed"
                        color: Theme.ink
                        font.pixelSize: 12
                        font.bold: true
                    }
                    Flow {
                        Layout.fillWidth: true
                        Layout.preferredHeight: childrenRect.height
                        spacing: 6
                        Repeater {
                            model: ["0.25", "0.5", "0.75", "1", "1.25", "1.5", "1.75", "2", "3"]
                            delegate: Rectangle {
                                required property string modelData
                                readonly property bool active: String(popup.rate()) === modelData
                                radius: 12
                                color: active ? Theme.accent : Theme.surface
                                border.width: 1
                                border.color: active ? Theme.accent : Theme.borderSubtle
                                implicitWidth: speedLabel.implicitWidth + 22
                                implicitHeight: 28
                                Text {
                                    id: speedLabel
                                    anchors.centerIn: parent
                                    text: modelData + "×"
                                    color: active ? "white" : Theme.inkMuted
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (popup.player) popup.player.setProperty("speed", modelData)
                                    }
                                }
                            }
                        }
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.borderSubtle }

                // Hardware decoding.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Text {
                        text: "Hardware decoding"
                        color: Theme.ink
                        font.pixelSize: 12
                        font.bold: true
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "Applies to the next file loaded; some drivers only honour it on a restart."
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
                            delegate: Rectangle {
                                required property var modelData
                                readonly property bool active: popup.hwdec() === modelData.value
                                radius: 12
                                color: active ? Theme.accent : Theme.surface
                                border.width: 1
                                border.color: active ? Theme.accent : Theme.borderSubtle
                                implicitWidth: hwLabel.implicitWidth + 22
                                implicitHeight: 28
                                Text {
                                    id: hwLabel
                                    anchors.centerIn: parent
                                    text: modelData.label
                                    color: active ? "white" : Theme.inkMuted
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (popup.player) popup.player.setProperty("hwdec", modelData.value)
                                    }
                                }
                            }
                        }
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.borderSubtle }

                // Reset the picture adjustments to their defaults (no-ops until
                // the Video tab lands, but a single "reset" is where a reader
                // looks after fiddling).
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Rectangle {
                        width: 120; height: 32; radius: 8
                        color: resetMouse.containsMouse ? Theme.surfaceHover : "transparent"
                        border.width: 1
                        border.color: resetMouse.containsMouse ? Theme.accent : Theme.borderSubtle
                        Text {
                            anchors.centerIn: parent
                            text: "Reset picture"
                            color: resetMouse.containsMouse ? Theme.accent : Theme.ink
                            font.pixelSize: 12
                        }
                        MouseArea {
                            id: resetMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: popup.resetPicture()
                        }
                    }
                    Item { Layout.fillWidth: true }
                }
            }

            // ===== Video tab ============================================
            ColumnLayout {
                id: videoTab
                width: parent.width - 40
                x: 20
                spacing: 14
                visible: popup.tab === "video"

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
                        delegate: Chip {
                            required property var modelData
                            text: modelData.label
                            state: popup.aspect === modelData.value ? "include" : "off"
                            onClicked: {
                                popup.aspect = modelData.value
                                if (popup.player) popup.player.setProperty("video-aspect-override", modelData.value)
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
                        delegate: Chip {
                            required property int modelData
                            text: modelData + "°"
                            state: popup.rotation === modelData ? "include" : "off"
                            onClicked: {
                                popup.rotation = modelData
                                if (popup.player) popup.player.setProperty("video-rotate", String(modelData))
                            }
                        }
                    }
                }

                LabeledSlider {
                    label: "Zoom"
                    from: -1
                    to: 3
                    step: 0.05
                    value: popup.zoom
                    suffix: "×"
                    onMoved: function (v) {
                        popup.zoom = v
                        if (popup.player) popup.player.setProperty("video-zoom", String(v))
                    }
                }
                LabeledSlider {
                    label: "Pan X"
                    from: -1
                    to: 1
                    step: 0.05
                    value: popup.panX
                    onMoved: function (v) {
                        popup.panX = v
                        if (popup.player) popup.player.setProperty("video-pan-x", String(v))
                    }
                }
                LabeledSlider {
                    label: "Pan Y"
                    from: -1
                    to: 1
                    step: 0.05
                    value: popup.panY
                    onMoved: function (v) {
                        popup.panY = v
                        if (popup.player) popup.player.setProperty("video-pan-y", String(v))
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.borderSubtle }
                SettingLabel { text: "Colour equalizer" }

                LabeledSlider {
                    label: "Brightness"
                    from: -100
                    to: 100
                    step: 1
                    value: popup.brightness
                    onMoved: function (v) {
                        popup.brightness = v
                        if (popup.player) popup.player.setProperty("brightness", String(Math.round(v)))
                    }
                }
                LabeledSlider {
                    label: "Contrast"
                    from: -100
                    to: 100
                    step: 1
                    value: popup.contrast
                    onMoved: function (v) {
                        popup.contrast = v
                        if (popup.player) popup.player.setProperty("contrast", String(Math.round(v)))
                    }
                }
                LabeledSlider {
                    label: "Saturation"
                    from: -100
                    to: 100
                    step: 1
                    value: popup.saturation
                    onMoved: function (v) {
                        popup.saturation = v
                        if (popup.player) popup.player.setProperty("saturation", String(Math.round(v)))
                    }
                }
                LabeledSlider {
                    label: "Gamma"
                    from: -100
                    to: 100
                    step: 1
                    value: popup.gamma
                    onMoved: function (v) {
                        popup.gamma = v
                        if (popup.player) popup.player.setProperty("gamma", String(Math.round(v)))
                    }
                }
            }

            // ===== Audio tab ============================================
            ColumnLayout {
                id: audioTab
                width: parent.width - 40
                x: 20
                spacing: 14
                visible: popup.tab === "audio"

                SettingLabel { text: "Audio track" }
                TrackList {
                    kind: "audio"
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.borderSubtle }

                LabeledSlider {
                    label: "Audio delay"
                    from: -10
                    to: 10
                    step: 0.05
                    value: popup.audioDelay
                    suffix: " s"
                    onMoved: function (v) {
                        popup.audioDelay = v
                        if (popup.player) popup.player.setProperty("audio-delay", String(v))
                    }
                }
            }

            // ===== Subtitles tab ========================================
            ColumnLayout {
                id: subtitlesTab
                width: parent.width - 40
                x: 20
                spacing: 14
                visible: popup.tab === "subtitles"

                RowLayout {
                    Layout.fillWidth: true
                    SettingLabel { text: "Show subtitles"; Layout.fillWidth: true }
                    Switch {
                        id: subVis
                        checked: popup.subVisible
                        onToggled: {
                            popup.subVisible = checked
                            if (popup.player) popup.player.setProperty("sub-visibility", checked)
                        }
                    }
                }

                SettingLabel { text: "Subtitle track" }
                TrackList {
                    kind: "sub"
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.borderSubtle }

                LabeledSlider {
                    label: "Subtitle delay"
                    from: -10
                    to: 10
                    step: 0.05
                    value: popup.subDelay
                    suffix: " s"
                    onMoved: function (v) {
                        popup.subDelay = v
                        if (popup.player) popup.player.setProperty("sub-delay", String(v))
                    }
                }
                LabeledSlider {
                    label: "Subtitle size"
                    from: 0.5
                    to: 3
                    step: 0.1
                    value: popup.subScale
                    suffix: "×"
                    onMoved: function (v) {
                        popup.subScale = v
                        if (popup.player) popup.player.setProperty("sub-scale", String(v))
                    }
                }
            }
        }
    }

    // Put every picture adjustment back to its default, on mpv AND in the local
    // mirror (so the sliders move back too).
    function resetPicture() {
        popup.zoom = 0
        popup.panX = 0
        popup.panY = 0
        popup.rotation = 0
        popup.aspect = "-1"
        popup.brightness = 0
        popup.contrast = 0
        popup.saturation = 0
        popup.gamma = 0

        if (popup.player === null) return
        const p = popup.player
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

    // A small bold label above a control group.
    component SettingLabel: Text {
        color: Theme.ink
        font.pixelSize: 12
        font.bold: true
    }

    // A labelled slider that reports its value through `moved`, rather than
    // exposing `value` two-way: the parent already owns the value, and a
    // two-way binding would fight the seed-on-open.
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
            id: inner
            Layout.fillWidth: true
            from: sliderRoot.from
            to: sliderRoot.to
            stepSize: sliderRoot.step
            // Seeded from the parent; NOT a binding, so dragging is not undone
            // by a re-render.
            Component.onCompleted: value = sliderRoot.value
            onMoved: sliderRoot.moved(value)
        }
    }

    // The track picker for one kind. Reads the list off mpv and writes the
    // chosen id back, including the "auto"/"off" row.
    component TrackList: ColumnLayout {
        id: trackList
        property string kind: "audio"

        Layout.fillWidth: true
        spacing: 6

        Repeater {
            model: {
                // Depend on `trackTick` so the rows re-read when the popup opens
                // (mpv's track-list is not a Qt property, so nothing else would
                // trigger a re-evaluation).
                const _ = popup.trackTick
                const head = [{
                    "id": -1,
                    "lang": "",
                    "title": trackList.kind === "audio" ? "Auto (default)" : "Off / auto",
                    "selected": false
                }]
                return head.concat(popup.tracks(trackList.kind))
            }
            delegate: Rectangle {
                required property var modelData
                readonly property int current: trackList.kind === "audio"
                    ? popup.audioTrack : popup.subTrack
                readonly property bool active: modelData.id === current
                Layout.fillWidth: true
                height: 32
                radius: 8
                color: active ? Theme.accent : "transparent"
                border.width: 1
                border.color: active ? Theme.accent : Theme.borderSubtle

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    text: popup.trackLabel(modelData)
                    color: active ? "white" : Theme.inkMuted
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        if (trackList.kind === "audio") {
                            popup.audioTrack = modelData.id
                            if (popup.player) popup.player.setProperty("aid", String(modelData.id))
                        } else {
                            popup.subTrack = modelData.id
                            if (popup.player) popup.player.setProperty("sid", String(modelData.id))
                        }
                    }
                }
            }
        }
    }
}