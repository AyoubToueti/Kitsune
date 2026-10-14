// One trending item for the home rail: the vertical title runs up the left
// edge, the poster sits beside it with the rank number over its bottom-left
// corner. Mirrors `TrendingCard.svelte` -- deliberately NOT `PosterCard`,
// which renders a title under the artwork (a different shape).
import QtQuick

Item {
    id: card

    property var anime: null
    property int rank: 1

    signal clicked()

    implicitWidth: 180
    implicitHeight: 300

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // Trimmed to a fixed budget so the vertical line never runs too long.
    function shortTitle(item) {
        const full = displayTitle(item)
        return full.length > 23 ? full.slice(0, 20) + "..." : full
    }

    // Zero-padded, so the number column stays aligned past nine.
    function rankLabel() {
        return rank < 10 ? "0" + rank : String(rank)
    }

    Row {
        anchors.fill: parent
        spacing: 6

        // The vertical title. `rotation: -90` makes it read bottom-to-top; the
        // container clips a long name so every card is the same size.
        Item {
            width: 22
            height: parent.height
            clip: true

            Text {
                // Rotated about its top-left, then moved to the bottom-left.
                rotation: -90
                transformOrigin: Item.TopLeft
                x: 0
                y: card.height
                text: card.shortTitle(card.anime)
                color: Theme.ink
                font.pixelSize: 15
                font.bold: true
            }
        }

        // Poster + rank badge + score pill.
        Item {
            width: parent.width - 28
            height: parent.height

            Rectangle {
                id: poster
                width: parent.width
                height: parent.height
                radius: 6
                color: Theme.surfaceHover
                clip: true

                Image {
                    anchors.fill: parent
                    source: card.anime ? (card.anime.coverImage || "") : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                }

                // Rank number, large, hung off the poster's bottom-left.
                Text {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 8
                    anchors.bottomMargin: 2
                    text: card.rankLabel()
                    color: "white"
                    font.pixelSize: 36
                    font.bold: true
                    style: Text.Outline
                    styleColor: "#cc000000"
                }

                // Score pill, top-right.
                Rectangle {
                    visible: card.anime && card.anime.averageScore != null
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.margins: 6
                    radius: 10
                    color: "#d90b0d12"
                    implicitWidth: trendScore.implicitWidth + 14
                    implicitHeight: 22

                    Text {
                        id: trendScore
                        anchors.centerIn: parent
                        text: card.anime && card.anime.averageScore != null
                            ? String(card.anime.averageScore) : ""
                        color: Theme.score
                        font.pixelSize: 12
                        font.bold: true
                    }
                }

                HoverHandler { id: hover }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: card.clicked()
                }
            }
        }
    }
}