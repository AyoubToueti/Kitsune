// A single anime poster: cover art, a score badge, and a clamped title.
// Mirrors the Svelte `AnimeCard` / `TrendingCard`.
import QtQuick
import QtQuick.Controls
import com.kitsune.app

Item {
    id: card

    property string title
    property real score
    property url cover
    property string format

    signal clicked()

    implicitWidth: 180
    implicitHeight: width * 1.5 + 52

    HoverHandler { id: hover }

    Column {
        anchors.fill: parent
        spacing: 8

        // --- Poster Frame ---------------------------------------------
        Rectangle {
            id: poster
            width: parent.width
            height: parent.width * 1.5
            radius: 12
            color: Theme.surfaceRaised
            clip: true
            border.width: 1
            border.color: hover.hovered ? Theme.accent : Theme.glassBorder

            Behavior on border.color { ColorAnimation { duration: 180 } }

            // Cover image with smooth scale zoom on hover
            Image {
                anchors.fill: parent
                source: card.cover
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 400
                scale: hover.hovered ? 1.06 : 1.0

                Behavior on scale { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
            }

            // Dark Scrim Gradient Overlay
            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.35) }
                    GradientStop { position: 0.5; color: "transparent" }
                    GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.85) }
                }
            }

            // Glossy Shine/Reflection Overlay on Hover
            Rectangle {
                anchors.fill: parent
                color: "transparent"
                opacity: hover.hovered ? 1.0 : 0.0
                gradient: Gradient {
                    GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0.12) }
                    GradientStop { position: 0.5; color: "transparent" }
                }

                Behavior on opacity { NumberAnimation { duration: 200 } }
            }

            // Score Badge (Top-Right)
            Rectangle {
                visible: card.score > 0
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                radius: 6
                color: Qt.rgba(0.07, 0.08, 0.12, 0.82)
                border.width: 1
                border.color: Theme.glassBorder
                implicitWidth: scoreLabel.implicitWidth + 12
                implicitHeight: 22

                Text {
                    id: scoreLabel
                    anchors.centerIn: parent
                    text: "★ " + card.score
                    color: Theme.score
                    font.pixelSize: 11
                    font.bold: true
                }
            }

            // Format Badge (Bottom-Left)
            Rectangle {
                visible: card.format.length > 0
                anchors.left: parent.left
                anchors.bottom: parent.bottom
                anchors.margins: 8
                radius: 6
                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.88)
                implicitWidth: formatLabel.implicitWidth + 12
                implicitHeight: 20

                Text {
                    id: formatLabel
                    anchors.centerIn: parent
                    text: card.format
                    color: "white"
                    font.pixelSize: 10
                    font.bold: true
                    font.letterSpacing: 0.5
                }
            }

            // Center Play / Open Hover Overlay
            Rectangle {
                anchors.centerIn: parent
                width: 42
                height: 42
                radius: 21
                color: cardMouse.containsMouse ? Theme.accentHover : Theme.accent
                opacity: hover.hovered ? 1.0 : 0.0
                scale: hover.hovered ? 1.0 : 0.85

                Behavior on opacity { NumberAnimation { duration: 180 } }
                Behavior on scale { NumberAnimation { duration: 200; easing.type: Easing.OutBack } }

                PlayerIcon {
                    anchors.centerIn: parent
                    anchors.horizontalCenterOffset: 1
                    kind: "play"
                    color: "white"
                    width: 16
                    height: 16
                }
            }

            MouseArea {
                id: cardMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: card.clicked()
            }
        }

        // --- Title Label ----------------------------------------------
        Text {
            width: parent.width
            text: card.title
            color: hover.hovered ? Theme.accent : Theme.ink
            font.pixelSize: 13
            font.bold: true
            elide: Text.ElideRight
            maximumLineCount: 2
            wrapMode: Text.WordWrap

            Behavior on color { ColorAnimation { duration: 150 } }
        }
    }
}