// Top rated titles, ranked. Backed by getList("topRated"). Mirrors the Svelte
// /top route, which shows a ranked list rather than a poster grid.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    property var backend: null

    signal openAnime(int id)

    ListModel { id: rows }

    Connections {
        target: page.backend
        function onListReady(json) {
            const items = JSON.parse(json)
            rows.clear()
            for (let i = 0; i < items.length; i++) {
                const item = items[i]
                rows.append({
                    "id": item.id,
                    "rank": i + 1,
                    "title": displayTitle(item),
                    "score": item.averageScore || 0,
                    "cover": item.coverImage || "",
                    "format": item.format || "",
                    "episodes": item.episodeCount || 0,
                })
            }
            status.text = items.length + " titles"
        }
        function onListFailed(message) {
            status.text = "Failed: " + message
        }
    }

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    Component.onCompleted: {
        if (backend) backend.getList("topRated", 50)
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 64
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad

                Text {
                    text: "Top Rated"
                    color: Theme.accent
                    font.pixelSize: 24
                    font.bold: true
                }
                Text {
                    id: status
                    Layout.fillWidth: true
                    color: Theme.inkFaint
                    font.pixelSize: 13
                }
            }
        }

        // Ranked list
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: rows
            leftMargin: Theme.pad
            rightMargin: Theme.pad
            topMargin: Theme.gap
            bottomMargin: Theme.pad
            spacing: 8

            delegate: Rectangle {
                width: ListView.view.width - Theme.pad * 2
                height: 88
                radius: Theme.radius
                color: rowHover.hovered ? Theme.surfaceHover : Theme.surfaceRaised

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 14

                    // Rank number.
                    Text {
                        Layout.preferredWidth: 44
                        Layout.alignment: Qt.AlignVCenter
                        horizontalAlignment: Text.AlignRight
                        text: model.rank
                        color: Theme.accent
                        font.pixelSize: 26
                        font.bold: true
                    }

                    Rectangle {
                        Layout.preferredWidth: 50
                        Layout.preferredHeight: 72
                        radius: 8
                        color: Theme.surface
                        clip: true
                        Image {
                            anchors.fill: parent
                            source: model.cover
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3

                        Text {
                            Layout.fillWidth: true
                            text: model.title
                            color: Theme.ink
                            font.pixelSize: 16
                            font.bold: true
                            elide: Text.ElideRight
                        }
                        Text {
                            color: Theme.inkMuted
                            font.pixelSize: 13
                            text: {
                                const parts = []
                                if (model.format) parts.push(model.format)
                                if (model.episodes > 0) parts.push(model.episodes + " eps")
                                return parts.join("  ·  ")
                            }
                        }
                    }

                    Text {
                        Layout.alignment: Qt.AlignVCenter
                        text: model.score > 0 ? ("★ " + model.score) : ""
                        color: Theme.score
                        font.pixelSize: 18
                        font.bold: true
                    }
                }

                HoverHandler { id: rowHover }
                MouseArea {
                    anchors.fill: parent
                    onClicked: page.openAnime(model.id)
                }
            }
        }
    }
}