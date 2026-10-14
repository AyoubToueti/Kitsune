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

    // Re-read the player on each poll tick (the parent bumps its own counter).
    property int tick: 0

    function rate() { return popup.player !== null ? popup.player.speed : 1 }
    function hwdec() {
        if (popup.player === null) return "auto"
        const v = popup.player.getProperty("hwdec")
        return (v === undefined || v === null || v === "") ? "auto" : String(v)
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

            // The later tabs, so the strip does not lie about what exists.
            ColumnLayout {
                width: parent.width - 40
                x: 20
                spacing: 8
                visible: popup.tab !== "playback"
                Text {
                    text: popup.tab === "video" ? "Video adjustments"
                        : (popup.tab === "audio" ? "Audio tracks" : "Subtitles")
                    color: Theme.ink
                    font.pixelSize: 13
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    text: "Aspect, zoom, rotation, the colour equalizer, track selection and "
                        + "sync delays land in the next stage."
                    color: Theme.inkFaint
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }
            }
        }
    }

    // Put every picture adjustment back to its default. Safe to call before the
    // Video tab exists: each property is a no-op if mpv does not know it.
    function resetPicture() {
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
}