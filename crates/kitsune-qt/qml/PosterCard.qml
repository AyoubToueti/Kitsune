// A single anime poster: cover art, a score badge, and a clamped title.
// Mirrors the Svelte `AnimeCard` / `TrendingCard`.
import QtQuick
import QtQuick.Controls
import com.kitsune.app

Item {
    id: card

    property int animeId: 0
    property string title
    property real score
    property url cover
    property string format
    property string animeStatus: ""

    signal clicked()

    implicitWidth: 180
    implicitHeight: width * 1.5 + 52

    HoverHandler { id: posterHover }

    Column {
        anchors.fill: parent
        spacing: 8

        // --- Poster Frame ---------------------------------------------
        Rectangle {
            id: poster
            width: parent.width
            height: parent.width * 1.5
            //radius: 12
            color: Theme.surfaceRaised
            clip: true
            // no border on this rectangle

            // Cover image with smooth scale zoom on hover
            Image {
                anchors.fill: parent
                source: card.cover
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 400
                scale: posterHover.hovered ? 1.06 : 1.0

                Behavior on scale { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
            }

            // Dark Scrim Gradient Overlay
            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.35) }
                    GradientStop { position: 0.5; color: "transparent" }
                    GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.88) }
                }
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
                    text: "\u2605 " + card.score
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

            // Mouse interaction for main card area
            MouseArea {
                id: cardMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                acceptedButtons: Qt.LeftButton | Qt.MiddleButton

                property bool dragCandidate: false
                property bool dragging: false
                property bool suppressClick: false
                property real pressX: 0
                property real pressY: 0

                onPressed: function (mouse) {
                    pressX = mouse.x
                    pressY = mouse.y
                    dragging = false
                    suppressClick = false
                    
                    dragCandidate = card.animeId > 0
                        && (mouse.button === Qt.MiddleButton
                            || (mouse.button === Qt.LeftButton
                                && (mouse.modifiers & Qt.AltModifier) !== 0))

                    preventStealing = dragCandidate
                }

                onPositionChanged: function (mouse) {
                    if (dragging) {
                        const held = (mouse.buttons & Qt.LeftButton) !== 0
                            || (mouse.buttons & Qt.MiddleButton) !== 0
                        if (!held) {
                            DragState.cancel()
                            dragging = false
                            dragCandidate = false
                            preventStealing = false
                            return
                        }
                        const moved = mapToItem(null, mouse.x, mouse.y)
                        DragState.move(moved.x, moved.y)
                        return
                    }

                    if (!dragCandidate) return

                    const dx = mouse.x - pressX
                    const dy = mouse.y - pressY
                    if (Math.sqrt(dx * dx + dy * dy) < 6) return

                    const point = mapToItem(null, mouse.x, mouse.y)
                    dragging = true
                    DragState.begin(card.animeId, card.title || "(untitled)",
                                    card.cover,
                                    point.x, point.y)
                }

                onReleased: function (mouse) {
                    if (dragging) {
                        DragState.finish()
                        suppressClick = true
                    }
                    dragCandidate = false
                    dragging = false
                    preventStealing = false
                }

                onCanceled: {
                    if (dragging) DragState.cancel()
                    dragCandidate = false
                    dragging = false
                    suppressClick = false
                    preventStealing = false
                }

                onClicked: function (mouse) {
                    if (suppressClick) {
                        suppressClick = false
                        return
                    }
                    if (mouse.button !== Qt.LeftButton) return
                    if ((mouse.modifiers & Qt.AltModifier) !== 0) return
                    card.clicked()
                }
            }

            // --- Reusable Quick Status Menu Component (Bottom-Right) -------------
            QuickStatusMenu {
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                animeId: card.animeId
                animeStatus: card.animeStatus
                z: 30
            }

            // ── Border overlay — on top of everything ─────────────────────
            Rectangle {
                anchors.fill: parent
                color: "transparent"
                border.width: 1
                border.color: posterHover.hovered ? Theme.accent : Theme.glassBorder
                z: 1000
                Behavior on border.color { ColorAnimation { duration: 180 } }
            }
        }

        // --- Title Label ----------------------------------------------
        Text {
            width: parent.width
            text: card.title
            color: posterHover.hovered ? Theme.accent : Theme.ink
            font.pixelSize: 13
            font.bold: true
            elide: Text.ElideRight
            maximumLineCount: 2
            wrapMode: Text.WordWrap

            Behavior on color { ColorAnimation { duration: 150 } }
        }
    }
}