import QtQuick
import QtQuick.Controls
import com.kitsune.app

Window {
    id: root
    width: 1280
    height: 800
    visible: true
    title: "Kitsune"
    color: Theme.surface

    // Exposed so child pages (DetailPage) can reach the same Backend instance.
    property alias backend: backend

    // Set from Rust (KITSUNE_OPEN_ID): when > 0, the detail page is pushed on
    // startup so it can be screenshotted without a click.
    property int initialOpenId: 0

    // The Rust bridge. In QML a `#[qml_element]` QObject is instantiated by
    // its Rust type name.
    Backend {
        id: backend

        onTrendingReady: (json) => {
            home.trendingModel.clear()
            const items = JSON.parse(json)
            for (const item of items) {
                home.trendingModel.append({
                    "id": item.id,
                    "title": root.displayTitle(item),
                    "score": item.averageScore || 0,
                    "cover": item.coverImage || "",
                    "format": item.format || "",
                })
            }
            home.status = items.length + " trending"
        }
        onTrendingFailed: (message) => {
            home.status = "Failed: " + message
        }
    }

    // Mirrors `displayTitle` in src/lib/types.ts.
    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // The navigation stack: Home -> Detail -> ...
    StackView {
        id: stack
        anchors.fill: parent
        initialItem: home

        pushEnter: Transition { NumberAnimation { property: "x"; from: stack.width; to: 0; duration: 220; easing.type: Easing.OutCubic } }
        pushExit: Transition { NumberAnimation { property: "x"; from: 0; to: -stack.width * 0.3; duration: 220; easing.type: Easing.OutCubic } }
        popEnter: Transition { NumberAnimation { property: "x"; from: -stack.width * 0.3; to: 0; duration: 220; easing.type: Easing.OutCubic } }
        popExit: Transition { NumberAnimation { property: "x"; from: 0; to: stack.width; duration: 220; easing.type: Easing.OutCubic } }
    }

    HomePage {
        id: home
        visible: false
        onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
    }

    Component {
        id: detailComponent
        DetailPage {
            onBack: stack.pop()
            onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
        }
    }

    Component.onCompleted: {
        backend.getTrending(40)
        if (initialOpenId > 0) {
            stack.push(detailComponent, { "animeId": initialOpenId, "backend": root.backend })
        }
    }
}