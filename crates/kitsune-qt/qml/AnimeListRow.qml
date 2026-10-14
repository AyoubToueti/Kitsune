// The compact row used by the home page's list blocks: a small portrait
// thumbnail beside the title and a row of facts. Mirrors `AnimeListItem`.
import QtQuick

Item {
    id: row

    property var anime: null

    signal clicked()

    implicitHeight: 76

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    Rectangle {
        anchors.fill: parent
        radius: 8
        color: rowMouse.containsMouse ? Theme.surfaceHover : "transparent"

        Row {
            anchors.fill: parent
            anchors.margins: 8
            spacing: 12

            // Portrait thumbnail.
            Rectangle {
                width: 44
                height: 60
                radius: 4
                color: Theme.surfaceHover
                clip: true

                Image {
                    anchors.fill: parent
                    source: row.anime ? (row.anime.coverImage || "") : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                }
            }

            Column {
                width: parent.width - 56
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6

                Text {
                    width: parent.width
                    text: row.displayTitle(row.anime)
                    color: rowMouse.containsMouse ? Theme.accent : Theme.ink
                    font.pixelSize: 13
                    font.bold: true
                    elide: Text.ElideRight
                    maximumLineCount: 2
                    wrapMode: Text.WordWrap
                }

                // Chips: score, episode count, then the format.
                Row {
                    spacing: 6

                    Rectangle {
                        visible: row.anime && row.anime.averageScore != null
                        radius: 4
                        color: Theme.surfaceHover
                        implicitWidth: rowScore.implicitWidth + 12
                        implicitHeight: 18
                        Text {
                            id: rowScore
                            anchors.centerIn: parent
                            text: row.anime && row.anime.averageScore != null
                                ? String(row.anime.averageScore) : ""
                            color: Theme.score
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }

                    Rectangle {
                        visible: row.anime && row.anime.episodeCount != null
                        radius: 4
                        color: Theme.surfaceHover
                        implicitWidth: rowEps.implicitWidth + 12
                        implicitHeight: 18
                        Text {
                            id: rowEps
                            anchors.centerIn: parent
                            text: row.anime && row.anime.episodeCount != null
                                ? String(row.anime.episodeCount) : ""
                            color: Theme.inkMuted
                            font.pixelSize: 10
                        }
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: row.anime && row.anime.format
                        text: row.anime && row.anime.format ? row.anime.format : ""
                        color: Theme.inkFaint
                        font.pixelSize: 10
                    }
                }
            }
        }
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: row.clicked()
    }
}