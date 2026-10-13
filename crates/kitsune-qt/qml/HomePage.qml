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
                    text: "Kitsune"
                    color: Theme.accent
                    font.pixelSize: 28
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

            // Browse entry, right-aligned.
            Rectangle {
                anchors.right: parent.right
                anchors.rightMargin: Theme.pad
                anchors.verticalCenter: parent.verticalCenter
                radius: 8
                color: browseHover.hovered ? Theme.accent : Theme.surfaceRaised
                implicitWidth: browseLabel.implicitWidth + 24
                implicitHeight: 36

                Text {
                    id: browseLabel
                    anchors.centerIn: parent
                    text: "Browse"
                    color: Theme.ink
                    font.pixelSize: 14
                    font.bold: true
                }

                HoverHandler { id: browseHover }
                MouseArea {
                    anchors.fill: parent
                    onClicked: page.openBrowse()
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