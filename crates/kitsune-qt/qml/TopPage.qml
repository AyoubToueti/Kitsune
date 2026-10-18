// Top rated titles, ranked. Backed by getList("topRated"). Mirrors the Svelte
// /top route, which shows a ranked list rather than a poster grid.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    property var backend: null

    signal openAnime(int id)

    // The ranked list. Loaded 50 at a time, capped at 100: reaching the end
    // of the visible rows fetches the next page until the cap or AniList runs
    // out, whichever comes first.
    readonly property int perPage: 50
    readonly property int maxItems: 100
    property int pageNumber: 1
    property bool loading: false
    property bool hasMore: true

    ListModel { id: rows }

    Connections {
        target: page.backend
        function onListReady(json) {
            const pageObj = JSON.parse(json)
            const items = (pageObj && pageObj.items) ? pageObj.items : []
            page.loading = false
            // A first page replaces; a later page appends, so the existing
            // rows keep their rank and scroll position.
            if (page.pageNumber === 1) rows.clear()
            const base = rows.count
            for (let i = 0; i < items.length; i++) {
                const item = items[i]
                rows.append({
                    "id": item.id,
                    "rank": base + i + 1,
                    "title": page.displayTitle(item),
                    "score": item.averageScore || 0,
                    "cover": item.coverImage || "",
                    "format": item.format || "",
                    "episodes": item.episodeCount || 0,
                })
            }
            // Stop when AniList says there is no next page, or the cap is hit.
            const info = pageObj ? pageObj.pageInfo : null
            page.hasMore = (info ? info.hasNextPage !== false : true)
                && rows.count < page.maxItems
            status.text = rows.count + " titles"
            RetryState.reset()
        }
        function onListFailed(message) {
            // Transient failures keep the skeleton pulsing behind the overlay;
            // a content failure drops it rather than pulsing forever.
            page.loading = RetryState.isTransient(message) && rows.count === 0
            // The overlay owns the wording -- no raw backend string here.
            status.text = ""
            RetryState.reportFailure(message)
        }
    }

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // Fetch a page of the ranked list. Guarded so a scroll burst does not
    // fire several overlapping requests.
    function loadPage(number) {
        if (!backend || loading) return
        if (number > 1 && !hasMore) return
        loading = true
        pageNumber = number
        backend.getListPaged("topRated", number, perPage)
    }

    // Re-run the first page. Called by the retry overlay.
    function reload() {
        rows.clear()
        hasMore = true
        pageNumber = 1
        loading = false
        loadPage(1)
    }

    Component.onCompleted: loadPage(1)

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

        // First-load placeholder: shown until the first page of rows arrives.
        // Later pages append to the list below and keep the footer "Loading…",
        // so a skeleton would only jitter the scroll.
        RankedListSkeleton {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            visible: page.loading && rows.count === 0
            count: 8
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
            visible: !(page.loading && rows.count === 0)

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

            // Fetch the next page when the last row comes into view. The cap
            // and the `hasMore` flag both live in `loadPage`, so an overshoot
            // here is harmless.
            onAtYEndChanged: {
                if (atYEnd && page.hasMore) page.loadPage(page.pageNumber + 1)
            }

            footer: Item {
                width: ListView.view.width
                height: page.loading ? 48 : 0
                Text {
                    anchors.centerIn: parent
                    text: "Loading…"
                    color: Theme.inkFaint
                    font.pixelSize: 13
                }
            }
        }
    }
}