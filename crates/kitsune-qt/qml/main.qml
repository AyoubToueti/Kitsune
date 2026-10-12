import QtQuick
import QtQuick.Controls
import com.kitsune.app

Window {
    width: 1280
    height: 800
    visible: true
    title: "Kitsune"
    color: "#0b0d12"

    // The Rust bridge. In QML a `#[qml_element]` QObject is instantiated by
    // its Rust type name.
    Backend {
        id: backend

        // Async results arrive as JSON, exactly the shape the Svelte frontend
        // consumed. Parse once here and drive the view off the model.
        onTrendingReady: (json) => {
            trendingModel.clear()
            const items = JSON.parse(json)
            for (const item of items) {
                trendingModel.append({
                    "title": item.title.userPreferred
                        || item.title.english
                        || item.title.romaji
                        || "(untitled)",
                    "score": item.averageScore || 0,
                })
            }
            status.text = items.length + " trending titles"
        }
        onTrendingFailed: (message) => {
            status.text = "Failed: " + message
        }
    }

    ListModel { id: trendingModel }

    Column {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 12

        Text {
            text: "Kitsune"
            color: "#7c5cff"
            font.pixelSize: 40
            font.bold: true
        }

        Text {
            id: status
            text: "Loading trending…"
            color: "#8a8f98"
            font.pixelSize: 14
        }

        ListView {
            width: parent.width
            height: parent.height - 120
            clip: true
            model: trendingModel
            delegate: Text {
                required property string title
                required property int score
                text: title + (score > 0 ? "  ★ " + score : "")
                color: "#e6e6e6"
                font.pixelSize: 15
            }
        }
    }

    Component.onCompleted: backend.getTrending(20)
}