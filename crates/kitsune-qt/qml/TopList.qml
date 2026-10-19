// A ranked list of titles: rank, poster, title, genre chips, then score /
// format / season columns. Mirrors `TopList.svelte`. A plain Column, so the
// home page scrolls it rather than it scrolling itself.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Column {
    id: list

    property var items: []
    // Rank of the first item, so a later page continues the numbering.
    property int startRank: 1

    signal openAnime(int id)

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    // A stable hue per genre, derived from the name so a genre is the same
    // colour everywhere without a palette to maintain.
    function genreColor(genre) {
        let hash = 0
        for (let i = 0; i < genre.length; i++) {
            hash = (hash * 31 + genre.charCodeAt(i)) % 360
        }
        return Qt.hsla(hash / 360, 0.45, 0.24, 1.0)
    }
    function genreTextColor(genre) {
        let hash = 0
        for (let i = 0; i < genre.length; i++) {
            hash = (hash * 31 + genre.charCodeAt(i)) % 360
        }
        return Qt.hsla(hash / 360, 0.80, 0.76, 1.0)
    }

    // "FALL 2023" -> "Fall 2023"; either half may be missing.
    function formatSeason(season, year) {
        if (!season && !year) return ""
        const label = season
            ? (season.charAt(0).toUpperCase() + season.slice(1).toLowerCase())
            : ""
        if (label && year) return label + " " + year
        if (label) return label
        return String(year)
    }

    spacing: 0

    Repeater {
        model: list.items

        delegate: Rectangle {
            required property var modelData
            required property int index

            width: list.width
            height: 84
            color: rowMouse.containsMouse ? Theme.surfaceHover : "transparent"

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 16

                // Rank, fixed width so the posters line up past nine.
                Text {
                    Layout.preferredWidth: 24
                    horizontalAlignment: Text.AlignRight
                    text: String(list.startRank + index)
                    color: Theme.inkFaint
                    font.pixelSize: 14
                    font.bold: true
                }

                // Poster.
                Rectangle {
                    Layout.preferredWidth: 40
                    Layout.preferredHeight: 60
                    radius: 4
                    color: Theme.surfaceHover
                    clip: true

                    Image {
                        anchors.fill: parent
                        source: modelData.coverImage || ""
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                    }
                }

                // Title + genre chips.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 5

                    Text {
                        Layout.fillWidth: true
                        text: list.displayTitle(modelData)
                        color: Theme.ink
                        font.pixelSize: 14
                        elide: Text.ElideRight
                    }

                    Row {
                        spacing: 4
                        Repeater {
                            model: (modelData.genres || []).slice(0, 5)
                            delegate: Rectangle {
                                required property string modelData
                                radius: 9
                                color: list.genreColor(modelData)
                                implicitWidth: genreLabel.implicitWidth + 16
                                implicitHeight: 18
                                Text {
                                    id: genreLabel
                                    anchors.centerIn: parent
                                    text: modelData
                                    color: list.genreTextColor(modelData)
                                    font.pixelSize: 10
                                }
                            }
                        }
                    }
                }

                // Score + popularity.
                ColumnLayout {
                    Layout.preferredWidth: 96
                    spacing: 1
                    visible: modelData.averageScore != null

                    Text {
                        Layout.alignment: Qt.AlignRight
                        text: modelData.averageScore != null
                            ? modelData.averageScore + "%" : ""
                        color: Theme.score
                        font.pixelSize: 14
                        font.bold: true
                    }
                    Text {
                        Layout.alignment: Qt.AlignRight
                        visible: modelData.popularity != null
                        text: modelData.popularity != null
                            ? Number(modelData.popularity).toLocaleString(Qt.locale()) + " users"
                            : ""
                        color: Theme.inkFaint
                        font.pixelSize: 10
                    }
                }

                // Format + episode count.
                ColumnLayout {
                    Layout.preferredWidth: 96
                    spacing: 1

                    Text {
                        Layout.alignment: Qt.AlignRight
                        text: modelData.format || ""
                        color: Theme.ink
                        font.pixelSize: 13
                    }
                    Text {
                        Layout.alignment: Qt.AlignRight
                        visible: modelData.episodeCount != null
                        text: modelData.episodeCount != null
                            ? modelData.episodeCount + " episodes" : ""
                        color: Theme.inkFaint
                        font.pixelSize: 10
                    }
                }

                // Season + status.
                ColumnLayout {
                    Layout.preferredWidth: 96
                    spacing: 1
                    visible: list.formatSeason(modelData.season, modelData.seasonYear) !== ""

                    Text {
                        Layout.alignment: Qt.AlignRight
                        text: list.formatSeason(modelData.season, modelData.seasonYear)
                        color: Theme.ink
                        font.pixelSize: 13
                    }
                    Text {
                        Layout.alignment: Qt.AlignRight
                        text: modelData.status || ""
                        color: Theme.inkFaint
                        font.pixelSize: 10
                    }
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: Theme.borderSubtle
            }

            MouseArea {
                id: rowMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: list.openAnime(modelData.id)
            }
        }
    }
}