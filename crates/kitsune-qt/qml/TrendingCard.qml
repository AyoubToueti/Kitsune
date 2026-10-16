// One trending item for the home rail: the vertical title runs up the left
// edge, the poster sits beside it with the rank number over its bottom-left
// corner. Mirrors `TrendingCard.svelte` -- deliberately NOT `PosterCard`.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: card

    property var anime: null
    property int rank: 1

    signal clicked()

    implicitWidth: 180
    implicitHeight: 300

    function displayTitle(item) {
        const t = item ? (item.title || {}) : {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // Trimmed to a fixed budget so the vertical line never runs too long.
    function shortTitle(item) {
        const full = displayTitle(item)
        return full.length > 25 ? full.slice(0, 22) + "..." : full
    }

    // Zero-padded rank string
    function rankLabel() {
        return rank < 10 ? "0" + rank : String(rank)
    }

    HoverHandler { id: hover }

    Row {
        anchors.fill: parent
        spacing: 8

        // --- Vertical Title Column ------------------------------------
        Item {
            width: 24
            height: parent.height
            clip: true

            Text {
                rotation: -90
                transformOrigin: Item.TopLeft
                x: 0
                y: card.height
                text: card.shortTitle(card.anime)
                color: hover.hovered ? Theme.accent : Theme.ink
                font.pixelSize: 14
                font.bold: true
                font.letterSpacing: 0.5

                Behavior on color { ColorAnimation { duration: 180 } }
            }
        }

        // --- Poster Frame ---------------------------------------------
        Item {
            width: parent.width - 32
            height: parent.height

            Rectangle {
                id: poster
                anchors.fill: parent
                radius: 12
                color: Theme.surfaceRaised
                clip: true
                border.width: 1
                border.color: hover.hovered ? Theme.accent : Theme.glassBorder

                Behavior on border.color { ColorAnimation { duration: 180 } }

                // Cover Image with smooth zoom
                Image {
                    anchors.fill: parent
                    source: card.anime ? (card.anime.coverImage || "") : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    sourceSize.width: 360
                    scale: hover.hovered ? 1.06 : 1.0

                    Behavior on scale { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                }

                // Dark Scrim Gradient Overlay
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.35) }
                        GradientStop { position: 0.55; color: "transparent" }
                        GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.88) }
                    }
                }

                // Glossy Reflection Overlay on Hover
                Rectangle {
                    anchors.fill: parent
                    color: "transparent"
                    opacity: hover.hovered ? 1.0 : 0.0
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0.12) }
                        GradientStop { position: 0.4; color: "transparent" }
                    }

                    Behavior on opacity { NumberAnimation { duration: 200 } }
                }

                // Large Rank Number (Bottom-Left)
                Text {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 10
                    anchors.bottomMargin: 4
                    text: card.rankLabel()
                    color: "white"
                    font.pixelSize: 38
                    font.bold: true
                    font.letterSpacing: -1
                    style: Text.Outline
                    styleColor: Qt.rgba(0, 0, 0, 0.6)
                }

                // Glassmorphic Score Badge (Top-Right)
                Rectangle {
                    visible: card.anime && card.anime.averageScore != null
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.margins: 8
                    radius: 6
                    color: Qt.rgba(0.07, 0.08, 0.12, 0.82)
                    border.width: 1
                    border.color: Theme.glassBorder
                    implicitWidth: trendScore.implicitWidth + 12
                    implicitHeight: 22

                    Text {
                        id: trendScore
                        anchors.centerIn: parent
                        text: "★ " + (card.anime ? card.anime.averageScore : "")
                        color: Theme.score
                        font.pixelSize: 11
                        font.bold: true
                    }
                }

                // Center Play Button Hover Overlay
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
        }
    }
}