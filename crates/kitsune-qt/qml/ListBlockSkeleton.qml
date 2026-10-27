// A home-page list block placeholder, shown while the curated shelves are in
// flight. Mirrors `ListBlock.qml`: a titled, bordered card of five compact rows
// with a "View more" footer pinned to the bottom. Each row matches
// `AnimeListRow`'s shape (a 44x60 portrait thumbnail beside two text lines and
// a chip row), so the real block swaps in without a shift.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: block

    // The real block's title, so the placeholder reads as that shelf.
    property string title: ""

    Layout.fillWidth: true
    //radius: 12
    color: "transparent"
    border.width: 1
    border.color: Theme.borderSubtle
    implicitHeight: content.implicitHeight + 24

    ColumnLayout {
        id: content
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 6

        Text {
            text: block.title
            color: Theme.accent
            font.pixelSize: 14
            font.bold: true
        }

        // Five row placeholders, matching the real block's count.
        Repeater {
            model: 5
            delegate: Item {
                Layout.fillWidth: true
                implicitHeight: 76

                Row {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 12

                    // Portrait thumbnail.
                    Skeleton {
                        width: 44
                        height: 60
                        radius: 4
                    }

                    Column {
                        width: parent.width - 56
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 8

                        // Title line.
                        Skeleton {
                            width: parent.width * 0.8
                            height: 12
                            radius: 4
                        }

                        // Chip row: score, episodes, format.
                        Row {
                            spacing: 6
                            Skeleton { width: 34; height: 16; radius: 4 }
                            Skeleton { width: 28; height: 16; radius: 4 }
                            Skeleton { width: 44; height: 16; radius: 4 }
                        }
                    }
                }
            }
        }

        Item { Layout.fillHeight: true; Layout.preferredHeight: 4 }

        // "View more" footer.
        Skeleton {
            width: 76
            height: 12
            radius: 4
        }
    }
}