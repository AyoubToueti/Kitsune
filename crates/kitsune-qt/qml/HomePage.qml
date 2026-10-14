// The home screen: a top bar and a poster grid of trending titles.
import QtQuick
import QtQuick.Controls

Item {
    id: page

    // The list model lives here and is aliased out, so the owning window can
    // fill it when the async trending fetch resolves.
    property alias trendingModel: trendingModel
    property alias status: statusLabel.text

    signal openAnime(int id)
    signal openBrowse()
    signal openSchedule()
    signal openTop()
    signal openMyList()
    signal openSettings()

    ListModel { id: trendingModel }

    Column {
        anchors.fill: parent
        spacing: 0

        // Top bar
        Rectangle {
            width: parent.width
            height: 64
            color: Theme.surface

            Row {
                anchors.left: parent.left
                anchors.leftMargin: Theme.pad
                anchors.verticalCenter: parent.verticalCenter
                spacing: 12

                Text {
                    text: "Trending"
                    color: Theme.ink
                    font.pixelSize: 22
                    font.bold: true
                }

                Text {
                    id: statusLabel
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Loading…"
                    color: Theme.inkFaint
                    font.pixelSize: 13
                }
            }

            // Top / Schedule entries, right-aligned. Home, My List, Filter and
            // Settings live in the global navbar; these two have no slot there.
            Row {
                anchors.right: parent.right
                anchors.rightMargin: Theme.pad
                anchors.verticalCenter: parent.verticalCenter
                spacing: 10

                NavButton {
                    label: "Top Rated"
                    onClicked: page.openTop()
                }
                NavButton {
                    label: "Schedule"
                    onClicked: page.openSchedule()
                }
            }
        }

        // Poster grid
        GridView {
            id: grid
            width: parent.width
            height: parent.height - 64
            clip: true
            model: trendingModel
            cellWidth: 216
            cellHeight: 380
            leftMargin: Theme.pad
            topMargin: Theme.pad
            rightMargin: Theme.pad
            bottomMargin: Theme.pad

            delegate: PosterCard {
                width: 200
                title: model.title
                score: model.score
                cover: model.cover
                format: model.format
                onClicked: page.openAnime(model.id)
            }
        }
    }
}