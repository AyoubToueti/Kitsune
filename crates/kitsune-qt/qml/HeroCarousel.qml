// The home page's spotlight carousel: top trending titles auto-advancing with a smooth,
// infinite forward loop and seamless full-bleed background integration.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: carousel

    property var items: []

    // `provider` is the id's namespace, carried through from the model.
    signal openAnime(int id, string provider)

    /** How long each slide is shown before advancing. */
    readonly property int advanceMs: 7000

    implicitHeight: 360

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

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
        color: Theme.surface
        clip: true

        PathView {
            id: swipe
            anchors.fill: parent
            clip: true

            model: carousel.items
            interactive: carousel.items.length > 1

            pathItemCount: 3
            preferredHighlightBegin: 0.5
            preferredHighlightEnd: 0.5
            highlightRangeMode: PathView.StrictlyEnforceRange
            snapMode: PathView.SnapOneItem
            highlightMoveDuration: 600
            flickDeceleration: 1500

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

                // Full-bleed artwork. A real banner is shown as-is; a work
                // with no wide art gets its portrait blurred into a wash
                // rather than cropped across the strip. See `HeroBackdrop`.
                HeroBackdrop {
                    anchors.fill: parent
                    banner: modelData.bannerImage || ""
                    cover: modelData.coverImage || ""
                }

                // 1. Horizontal vignette overlay:
                // Fades in on the left for text readability, clears through the center,
                // and mirrors the bottom fade style across the right edge (80% to 100%).
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0.00; color: Theme.heroFadeStart }
                        GradientStop { position: 0.22; color: Theme.heroFadeStart }
                        GradientStop { position: 0.38; color: Theme.heroFadeMid }
                        GradientStop { position: 0.55; color: "transparent" }
                        GradientStop { position: 0.80; color: "transparent" }
                        GradientStop { position: 1.00; color: Theme.heroFadeStart }
                    }
                }

                // 2. Vertical vignette overlay:
                // Smoothly fades only the bottom edge (80% to 100%) into the surface background.
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop { position: 0.00; color: "transparent" }
                        GradientStop { position: 0.80; color: "transparent" }
                        GradientStop { position: 1.00; color: Theme.heroFadeStart }
                    }
                }

                // Compact left text content column
                ColumnLayout {
                    anchors.left: parent.left
                    anchors.leftMargin: 36
                    anchors.right: parent.right
                    anchors.rightMargin: parent.width * 0.64
                    anchors.top: parent.top
                    anchors.topMargin: 24
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 48
                    spacing: 6

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
                        font.pixelSize: 24
                        font.bold: true
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                    }

                    // Score & facts
                    RowLayout {
                        spacing: 10

                        Rectangle {
                            visible: modelData.averageScore != null
                            radius: 6
                            color: Theme.fieldFill
                            border.width: 1
                            border.color: Theme.borderSubtle
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
                                color: Theme.fieldFill
                                border.width: 1
                                border.color: Theme.borderSubtle
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

                    // Compact Synopsis
                    // Compact Synopsis
                    Text {
                        Layout.fillWidth: true
                        visible: modelData.description !== undefined && modelData.description !== null
                        text: modelData.description ? String(modelData.description).replace(/<[^>]*>/g, "") : ""
                        color: Theme.inkMuted
                        font.pixelSize: 12
                        wrapMode: Text.WordWrap
                        maximumLineCount: 4
                        elide: Text.ElideRight
                        lineHeight: 1.25
                    }

                    Item { Layout.fillHeight: true }

                    // Action button: View Details
                    Rectangle {
                        height: 36
                        radius: 18
                        color: detailsMouse.containsMouse ? Theme.accentHover : Theme.accent
                        implicitWidth: btnRow.implicitWidth + 24
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
                            onClicked: carousel.openAnime(modelData.id, modelData.provider || "anilist")
                        }
                    }
                }
            }
        }

        // Active slide pill indicators
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
                    color: swipe.currentIndex === index ? Theme.accent : Theme.inkFaint
                    opacity: swipe.currentIndex === index ? 1.0 : 0.45

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

    Timer {
        interval: carousel.advanceMs
        repeat: true
        running: carousel.visible && carousel.items.length > 1
                 && !swipe.dragging && !swipe.flicking
        onTriggered: swipe.incrementCurrentIndex()
    }
}