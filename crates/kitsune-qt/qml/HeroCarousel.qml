// The home page's spotlight carousel: the top few trending titles, one per
// slide, auto-advancing. Mirrors `HeroCarousel.svelte`.
//
// The slide is a fixed height so the carousel cannot resize as it moves, and
// each slide's synopsis is clamped so a long description does not push the
// facts row off the slide.
import QtQuick
import QtQuick.Controls

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
        radius: 12
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.borderSubtle
        clip: true

        ListView {
            id: track
            anchors.fill: parent
            orientation: ListView.Horizontal
            snapMode: ListView.SnapOneItem
            highlightRangeMode: ListView.StrictlyEnforceRange
            highlightMoveDuration: 400
            boundsBehavior: Flickable.StopAtBounds
            clip: true
            model: carousel.items

            delegate: Item {
                required property var modelData
                required property int index
                width: track.width
                height: track.height

                // Backdrop: the banner if the provider has one, else the cover.
                Image {
                    anchors.fill: parent
                    source: modelData.bannerImage || modelData.coverImage || ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                }

                // Two scrims: one across for text contrast, one up from the
                // bottom so the artwork fades into the page.
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0.0; color: "#f20b0d12" }
                        GradientStop { position: 0.55; color: "#d90b0d12" }
                        GradientStop { position: 1.0; color: "#330b0d12" }
                    }
                }
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: "transparent" }
                        GradientStop { position: 1.0; color: "#e60b0d12" }
                    }
                }

                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 32
                    anchors.right: parent.right
                    anchors.rightMargin: parent.width * 0.35
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 10

                    Text {
                        text: "#" + (index + 1) + " Spotlight"
                        color: Theme.accent
                        font.pixelSize: 13
                        font.bold: true
                    }

                    Text {
                        width: parent.width
                        text: carousel.displayTitle(modelData)
                        color: Theme.ink
                        font.pixelSize: 32
                        font.bold: true
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                    }

                    // Score in gold, then the plain facts.
                    Row {
                        spacing: 14

                        Text {
                            visible: modelData.averageScore != null
                            text: modelData.averageScore != null
                                ? String(modelData.averageScore) : ""
                            color: Theme.score
                            font.pixelSize: 14
                            font.bold: true
                        }

                        Repeater {
                            model: carousel.facts(modelData)
                            delegate: Text {
                                required property string modelData
                                text: modelData
                                color: Theme.inkMuted
                                font.pixelSize: 14
                            }
                        }
                    }

                    // Genre chips, first three.
                    Row {
                        spacing: 6
                        Repeater {
                            model: (modelData.genres || []).slice(0, 3)
                            delegate: Rectangle {
                                required property string modelData
                                radius: 10
                                color: Theme.surfaceHover
                                implicitWidth: heroGenre.implicitWidth + 18
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

                    Text {
                        width: parent.width
                        visible: modelData.description !== undefined
                            && modelData.description !== null
                        text: modelData.description
                            ? String(modelData.description).replace(/<[^>]*>/g, "")
                            : ""
                        color: Theme.inkMuted
                        font.pixelSize: 13
                        wrapMode: Text.WordWrap
                        maximumLineCount: 3
                        elide: Text.ElideRight
                        lineHeight: 1.3
                    }

                    Item { width: 1; height: 6 }

                    Rectangle {
                        width: 130
                        height: 38
                        radius: 19
                        color: detailsMouse.containsMouse ? Theme.accentHover : Theme.accent

                        Text {
                            anchors.centerIn: parent
                            text: "View details"
                            color: "white"
                            font.pixelSize: 13
                            font.bold: true
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

        // Arrow buttons on the right edge, shown only with more than one slide.
        Column {
            visible: carousel.items.length > 1
            anchors.right: parent.right
            anchors.rightMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8

            Rectangle {
                width: 34
                height: 34
                radius: 17
                color: nextMouse.containsMouse ? Theme.surfaceHover : "#cc0b0d12"
                Text {
                    anchors.centerIn: parent
                    text: "›"
                    color: Theme.ink
                    font.pixelSize: 20
                }
                MouseArea {
                    id: nextMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: carousel.step(1)
                }
            }
            Rectangle {
                width: 34
                height: 34
                radius: 17
                color: prevMouse.containsMouse ? Theme.surfaceHover : "#cc0b0d12"
                Text {
                    anchors.centerIn: parent
                    text: "‹"
                    color: Theme.ink
                    font.pixelSize: 20
                }
                MouseArea {
                    id: prevMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: carousel.step(-1)
                }
            }
        }

        // Dot indicators along the bottom-left.
        Row {
            visible: carousel.items.length > 1
            anchors.left: parent.left
            anchors.leftMargin: 32
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 16
            spacing: 8

            Repeater {
                model: carousel.items.length
                delegate: Rectangle {
                    required property int index
                    width: 8
                    height: 8
                    radius: 4
                    color: track.currentIndex === index ? Theme.accent : Theme.inkFaint
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: track.currentIndex = index
                    }
                }
            }
        }
    }

    // Wrap the index at both ends.
    function step(delta) {
        const count = items.length
        if (count === 0) return
        track.currentIndex = (track.currentIndex + delta + count) % count
    }

    // Auto-advance, restarting whenever the index changes so a manual jump
    // gets a fresh countdown.
    Timer {
        interval: carousel.advanceMs
        repeat: true
        running: carousel.visible && carousel.items.length > 1
        onTriggered: carousel.step(1)
    }
}