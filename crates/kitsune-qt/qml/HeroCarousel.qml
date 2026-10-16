// The home page's spotlight carousel: the top few trending titles, one per
// slide, auto-advancing with a smooth, infinite forward loop.
// Mirrors `HeroCarousel.svelte`.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: carousel

    // The anime to feature, in order. The page passes the top few trending.
    property var items: []

    signal openAnime(int id)

    /** How long each slide is shown before advancing. */
    readonly property int advanceMs: 7000

    implicitHeight: 360

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // The inline facts row under the title.
    function facts(item) {
        const out = []
        if (item.format) out.push(item.format)
        if (item.durationMinutes != null) out.push(item.durationMinutes + "m")
        if (item.seasonYear != null) out.push(String(item.seasonYear))
        if (item.episodeCount != null) out.push(item.episodeCount + " eps")
        return out
    }

    visible: items.length > 0

    Rectangle {
        id: frame
        anchors.fill: parent
        radius: 16
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.glassBorder
        clip: true

        // PathView is circular, so stepping forward from the last slide
        // slides smoothly into the first one (no snap back).
        PathView {
            id: swipe
            anchors.fill: parent
            clip: true

            model: carousel.items
            interactive: carousel.items.length > 1

            // Current slide plus one on each side, so the next slide is
            // already in place when wrapping from last to first.
            pathItemCount: 3
            preferredHighlightBegin: 0.5
            preferredHighlightEnd: 0.5
            highlightRangeMode: PathView.StrictlyEnforceRange
            snapMode: PathView.SnapOneItem
            highlightMoveDuration: 600
            flickDeceleration: 1500

            // A straight horizontal line, 3 slides long. Slides sit at
            // -0.5w, 0.5w and 1.5w, so the middle one fills the frame.
            path: Path {
                startX: -swipe.width
                startY: swipe.height / 2
                PathLine {
                    x: swipe.width * 2
                    y: swipe.height / 2
                }
            }

            delegate: Item {
                required property var modelData
                required property int index

                width: swipe.width
                height: swipe.height

                // Backdrop banner/cover image
                Image {
                    anchors.fill: parent
                    source: modelData.bannerImage || modelData.coverImage || ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                }

                // Horizontal gradient for text contrast
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0.0; color: Qt.rgba(0.05, 0.06, 0.09, 0.96) }
                        GradientStop { position: 0.55; color: Qt.rgba(0.05, 0.06, 0.09, 0.82) }
                        GradientStop { position: 1.0; color: Qt.rgba(0.05, 0.06, 0.09, 0.25) }
                    }
                }

                // Bottom gradient blend
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: "transparent" }
                        GradientStop { position: 1.0; color: Qt.rgba(0.05, 0.06, 0.09, 0.9) }
                    }
                }

                // Strictly bounded content column
                ColumnLayout {
                    anchors.left: parent.left
                    anchors.leftMargin: 36
                    anchors.right: parent.right
                    anchors.rightMargin: parent.width * 0.28
                    anchors.top: parent.top
                    anchors.topMargin: 24
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 52
                    spacing: 8

                    // Spotlight rank badge
                    Rectangle {
                        radius: 6
                        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2)
                        border.width: 1
                        border.color: Theme.accent
                        implicitWidth: spotText.implicitWidth + 16
                        implicitHeight: 22

                        Text {
                            id: spotText
                            anchors.centerIn: parent
                            text: "#" + (index + 1) + " SPOTLIGHT"
                            color: Theme.accent
                            font.pixelSize: 10
                            font.bold: true
                            font.letterSpacing: 1.0
                        }
                    }

                    // Title
                    Text {
                        Layout.fillWidth: true
                        text: carousel.displayTitle(modelData)
                        color: Theme.ink
                        font.pixelSize: 26
                        font.bold: true
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                    }

                    // Score & facts
                    RowLayout {
                        spacing: 12

                        Rectangle {
                            visible: modelData.averageScore != null
                            radius: 6
                            color: Qt.rgba(0.07, 0.08, 0.12, 0.75)
                            border.width: 1
                            border.color: Theme.glassBorder
                            implicitWidth: scoreText.implicitWidth + 12
                            implicitHeight: 22

                            Text {
                                id: scoreText
                                anchors.centerIn: parent
                                text: "★ " + (modelData.averageScore != null ? modelData.averageScore : "")
                                color: Theme.score
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        Repeater {
                            model: carousel.facts(modelData)
                            delegate: Text {
                                required property string modelData
                                text: modelData
                                color: Theme.inkMuted
                                font.pixelSize: 12
                                font.bold: true
                            }
                        }
                    }

                    // Genre chips
                    Row {
                        spacing: 6
                        Repeater {
                            model: (modelData.genres || []).slice(0, 3)
                            delegate: Rectangle {
                                required property string modelData
                                radius: 8
                                color: Qt.rgba(1, 1, 1, 0.08)
                                border.width: 1
                                border.color: Theme.glassBorder
                                implicitWidth: heroGenre.implicitWidth + 16
                                implicitHeight: 22

                                Text {
                                    id: heroGenre
                                    anchors.centerIn: parent
                                    text: modelData
                                    color: Theme.inkMuted
                                    font.pixelSize: 11
                                }
                            }
                        }
                    }

                    // Synopsis (clamped to 2 lines)
                    Text {
                        Layout.fillWidth: true
                        visible: modelData.description !== undefined && modelData.description !== null
                        text: modelData.description ? String(modelData.description).replace(/<[^>]*>/g, "") : ""
                        color: Theme.inkMuted
                        font.pixelSize: 12
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        lineHeight: 1.3
                    }

                    Item { Layout.fillHeight: true }

                    // Action button: View Details
                    Rectangle {
                        height: 38
                        radius: 19
                        color: detailsMouse.containsMouse ? Theme.accentHover : Theme.accent
                        implicitWidth: btnRow.implicitWidth + 28
                        scale: detailsMouse.containsMouse ? 1.03 : 1.0

                        Behavior on scale { NumberAnimation { duration: 150 } }
                        Behavior on color { ColorAnimation { duration: 150 } }

                        RowLayout {
                            id: btnRow
                            anchors.centerIn: parent
                            spacing: 8

                            PlayerIcon {
                                kind: "play"
                                color: "white"
                                width: 12
                                height: 12
                            }

                            Text {
                                text: "View Details"
                                color: "white"
                                font.pixelSize: 13
                                font.bold: true
                            }
                        }

                        MouseArea {
                            id: detailsMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: carousel.openAnime(modelData.id)
                        }
                    }
                }
            }
        }

        // Expanding pill indicators (bottom-left)
        Row {
            visible: carousel.items.length > 1
            anchors.left: parent.left
            anchors.leftMargin: 36
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 18
            spacing: 6
            z: 10

            Repeater {
                model: carousel.items.length
                delegate: Rectangle {
                    required property int index
                    width: swipe.currentIndex === index ? 24 : 8
                    height: 8
                    radius: 4
                    color: swipe.currentIndex === index ? Theme.accent : Qt.rgba(1, 1, 1, 0.3)

                    Behavior on width { NumberAnimation { duration: 200; easing.type: Easing.OutQuad } }
                    Behavior on color { ColorAnimation { duration: 200 } }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: swipe.currentIndex = index
                    }
                }
            }
        }
    }

    // Auto-advance: always steps forward and loops seamlessly.
    // Paused while the user is dragging or the view is still moving.
    Timer {
        interval: carousel.advanceMs
        repeat: true
        running: carousel.visible && carousel.items.length > 1
                 && !swipe.dragging && !swipe.flicking
        onTriggered: swipe.incrementCurrentIndex()
    }
}
