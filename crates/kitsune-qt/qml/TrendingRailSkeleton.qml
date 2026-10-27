// The "Trending" rail placeholder, shown while the home page's trending data
// is in flight. Mirrors the trending section in `HomePage.qml`: an 18px accent
// heading over a horizontal row of `TrendingCard`-shaped blocks. The card shape
// follows `TrendingCardSkeleton`: a thin vertical title strip on the left and a
// 2:3 poster with a small rank square over its bottom-left corner.
import QtQuick

Column {
    id: rail

    spacing: 10

    // Heading, matching the real "Trending" label's size and colour.
    Text {
        x: Theme.pad
        text: "Trending"
        color: Theme.accent
        font.pixelSize: 18
        font.bold: true
    }

    // Six placeholders, the same count the Svelte rail shows.
    Row {
        x: Theme.pad
        spacing: Theme.gap

        Repeater {
            model: 6
            delegate: Item {
                width: 180
                height: 280

                Row {
                    anchors.fill: parent
                    spacing: 8

                    // Vertical title strip on the left edge.
                    Skeleton {
                        width: 24
                        height: parent.height * 0.6
                        radius: 4
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    // Poster with a rank square over its bottom-left corner.
                    Item {
                        width: 148
                        height: parent.height

                        Skeleton {
                            anchors.fill: parent
                            //radius: 12
                        }
                        Skeleton {
                            width: 28
                            height: 28
                            radius: 6
                            anchors.left: parent.left
                            anchors.bottom: parent.bottom
                            anchors.margins: 8
                        }
                    }
                }
            }
        }
    }
}