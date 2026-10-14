// The player's OSD control bar: a seek timeline, transport buttons, speed,
// volume, and a gear opening the full settings popup. Sits UNDER the video
// surface (the layout the reader asked for).
//
// Everything is driven off the live `MpvObject`: play/pause, position,
// duration, speed and volume are read from its polled properties, and actions
// go through `command()` / `setProperty()`. No state is mirrored here, so the
// bar cannot drift from what the decoder is actually doing.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: bar

    // The live MpvObject, or null before the play stage creates it.
    property var player: null
    // The chosen file's downloaded fraction, 0..1: the buffer fill under the
    // played fill on the timeline.
    property real bufferedFraction: 0
    // Opens the settings popup; the modal owns the popup itself.
    signal openSettings()

    implicitHeight: 76

    // Bumped on every poll tick so the bindings below that read the player
    // re-evaluate. Reading `player.position` directly in a binding does NOT
    // work: the property lives on a C++ object and its NOTIFY is
    // `stateChanged`, not a per-property one.
    property int tick: 0

    Connections {
        target: bar.player
        function onStateChanged() {
            bar.tick++
        }
    }

    // Helpers reading the player, guarded for the null (not-yet-created) case.
    function pos() { return bar.player !== null ? bar.player.position : 0 }
    function dur() { return bar.player !== null ? bar.player.duration : 0 }
    function isPaused() { return bar.player !== null ? bar.player.paused : true }
    function rate() { return bar.player !== null ? bar.player.speed : 1 }
    function vol() { return bar.player !== null ? bar.player.volume : 100 }
    function isMuted() { return bar.player !== null ? bar.player.muted : false }

    // Seconds -> "h:mm:ss" / "m:ss".
    function fmt(seconds) {
        if (!seconds || seconds <= 0 || !isFinite(seconds)) return "0:00"
        const s = Math.floor(seconds % 60)
        const m = Math.floor((seconds / 60) % 60)
        const h = Math.floor(seconds / 3600)
        const ss = s < 10 ? "0" + s : "" + s
        if (h > 0) {
            const mm = m < 10 ? "0" + m : "" + m
            return h + ":" + mm + ":" + ss
        }
        return m + ":" + ss
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 4

        // --- Timeline ---------------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Text {
                text: bar.fmt(bar.pos())
                color: Theme.inkMuted
                font.pixelSize: 11
                Layout.preferredWidth: 48
                horizontalAlignment: Text.AlignRight
            }

            // The seek bar. A custom Item rather than a Slider: it needs two
            // fills (buffered + played) and a drag that does not fight the
            // 200ms position poll while the pointer is down.
            Item {
                id: seek
                Layout.fillWidth: true
                height: 18

                // While the reader is dragging, the displayed position is the
                // drag's, not the decoder's -- otherwise the poll yanks the
                // handle back under the finger.
                property bool dragging: false
                property real dragFraction: 0
                readonly property real fraction: {
                    if (seek.dragging) return seek.dragFraction
                    const d = bar.dur()
                    if (d <= 0) return 0
                    return Math.min(Math.max(bar.pos() / d, 0), 1)
                }

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width
                    height: 4
                    radius: 2
                    color: Theme.borderSubtle
                }

                // Buffered: how much of the file the torrent has fetched.
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width * Math.min(bar.bufferedFraction, 1)
                    height: 4
                    radius: 2
                    color: "#4a5568"
                }

                // Played.
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width * seek.fraction
                    height: 4
                    radius: 2
                    color: Theme.accent
                }

                // Handle.
                Rectangle {
                    visible: bar.dur() > 0
                    x: Math.min(parent.width - width, parent.width * seek.fraction - width / 2)
                    anchors.verticalCenter: parent.verticalCenter
                    width: 12
                    height: 12
                    radius: 6
                    color: "white"
                    border.width: 2
                    border.color: Theme.accent
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onPressed: function (mouse) {
                        seek.dragging = true
                        seek.dragFraction = Math.min(Math.max(mouse.x / width, 0), 1)
                    }
                    onPositionChanged: function (mouse) {
                        if (!seek.dragging) return
                        seek.dragFraction = Math.min(Math.max(mouse.x / width, 0), 1)
                    }
                    onReleased: {
                        seek.dragging = false
                        const d = bar.dur()
                        if (bar.player !== null && d > 0) {
                            // Absolute seek: mpv takes the target seconds.
                            bar.player.command(["seek", String(seek.dragFraction * d), "absolute"])
                        }
                    }
                }
            }

            Text {
                text: bar.fmt(bar.dur())
                color: Theme.inkMuted
                font.pixelSize: 11
                Layout.preferredWidth: 48
            }
        }

        // --- Transport row ----------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            ControlButton {
                glyph: "⏮"
                tip: "Previous frame"
                onClicked: {
                    if (bar.player) bar.player.command(["frame-back-step"])
                }
            }
            ControlButton {
                glyph: bar.isPaused() ? "▶" : "⏸"
                tip: bar.isPaused() ? "Play" : "Pause"
                accent: true
                onClicked: {
                    if (bar.player) bar.player.command(["cycle", "pause"])
                }
            }
            ControlButton {
                glyph: "⏹"
                tip: "Stop"
                onClicked: {
                    if (bar.player) bar.player.command(["stop"])
                }
            }
            ControlButton {
                glyph: "⏭"
                tip: "Next frame"
                onClicked: {
                    if (bar.player) bar.player.command(["frame-step"])
                }
            }

            Item { Layout.preferredWidth: 8 }

            ControlButton {
                glyph: "⏪"
                tip: "Back 10 seconds"
                onClicked: {
                    if (bar.player) bar.player.command(["seek", "-10", "relative"])
                }
            }
            ControlButton {
                glyph: "⏩"
                tip: "Forward 10 seconds"
                onClicked: {
                    if (bar.player) bar.player.command(["seek", "10", "relative"])
                }
            }

            Item { Layout.fillWidth: true }

            // Speed, as a small dropdown.
            ThemeSelect {
                preferredWidth: 84
                options: [
                    { "value": "0.5", "label": "0.5×" },
                    { "value": "0.75", "label": "0.75×" },
                    { "value": "1", "label": "1×" },
                    { "value": "1.25", "label": "1.25×" },
                    { "value": "1.5", "label": "1.5×" },
                    { "value": "2", "label": "2×" }
                ]
                // The current rate, as a wire string; falls back to "1".
                value: {
                    const r = String(bar.rate())
                    const known = ["0.5", "0.75", "1", "1.25", "1.5", "2"]
                    return known.indexOf(r) >= 0 ? r : "1"
                }
                onSelected: function (v) {
                    if (bar.player) bar.player.setProperty("speed", v)
                }
            }

            // Mute toggle.
            ControlButton {
                glyph: bar.isMuted() ? "🔇" : "🔊"
                tip: bar.isMuted() ? "Unmute" : "Mute"
                onClicked: {
                    if (bar.player) bar.player.setProperty("mute", !bar.isMuted())
                }
            }

            // Volume slider, 0..130 (above 100 boosts).
            Slider {
                Layout.preferredWidth: 110
                from: 0
                to: 130
                value: bar.vol()
                onMoved: {
                    if (bar.player) bar.player.setProperty("volume", value)
                }
            }

            // Gear -> settings popup.
            ControlButton {
                glyph: "⚙"
                tip: "Player settings"
                onClicked: bar.openSettings()
            }
        }
    }

    // A small square icon button, shared by the transport row.
    component ControlButton: Rectangle {
        property string glyph: ""
        property string tip: ""
        property bool accent: false
        signal clicked()

        width: 32
        height: 32
        radius: 8
        color: ctlMouse.containsMouse
            ? (accent ? Theme.accentHover : Theme.surfaceHover)
            : (accent ? Theme.accent : "transparent")
        border.width: accent ? 0 : 1
        border.color: Theme.borderSubtle

        Text {
            anchors.centerIn: parent
            text: parent.glyph
            color: parent.accent ? "white" : Theme.inkMuted
            font.pixelSize: parent.accent ? 14 : 13
        }

        MouseArea {
            id: ctlMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }

        ToolTip.visible: ctlMouse.containsMouse && tip.length > 0
        ToolTip.text: tip
        ToolTip.delay: 600
    }
}