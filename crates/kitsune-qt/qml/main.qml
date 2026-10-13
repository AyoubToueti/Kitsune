import QtQuick
import QtQuick.Controls
import com.kitsune.app

Window {
    width: 1280
    height: 800
    visible: true
    title: "Kitsune"
    color: Theme.surface

    // The Rust bridge. In QML a `#[qml_element]` QObject is instantiated by
    // its Rust type name.
    Backend {
        id: backend

        // Async results arrive as JSON, the shape the Svelte frontend already
        // consumed. Parse once here and feed the grid model.
        onTrendingReady: (json) => {
            trendingModel.clear()
            const items = JSON.parse(json)
            for (const item of items) {
                trendingModel.append({
                    "title": displayTitle(item),
                    "score": item.averageScore || 0,
                    "cover": item.coverImage || "",
                    "format": item.format || "",
                })
            }
            status.text = items.length + " trending"
        }
        onTrendingFailed: (message) => {
            status.text = "Failed: " + message
        }
    }

    // Mirrors `displayTitle` in src/lib/types.ts: prefer the user's preferred
    // form, then English, then romaji.
    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

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
                spacing: 10

                Text {
                    text: "Kitsune"
                    color: Theme.accent
                    font.pixelSize: 28
                    font.bold: true
                }

                Text {
                    id: status
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Loading…"
                    color: Theme.inkFaint
                    font.pixelSize: 13
                }
            }
        }

        // Poster grid
        GridView {
            id: grid
            width: parent.width
            height: parent.height - 64
            clip: true
            cellWidth: 216
            cellHeight: 380
            leftMargin: Theme.pad
            topMargin: Theme.pad
            rightMargin: Theme.pad
            bottomMargin: Theme.pad
            model: trendingModel

            delegate: PosterCard {
                width: 200
                title: model.title
                score: model.score
                cover: model.cover
                format: model.format
            }
        }
    }

    Component.onCompleted: backend.getTrending(40)
}