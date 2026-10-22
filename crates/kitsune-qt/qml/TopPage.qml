// Top rated titles, ranked. Backed by getList("topRated"). Mirrors the Svelte
// /top route, which shows a ranked list rather than a poster grid.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    property var backend: null

    // `provider` is the id's namespace, forwarded so the detail page fetches
    // from the source that issued the id.
    signal openAnime(int id, string provider)

    // The ranked list. Loaded 50 at a time, capped at 100: reaching the end
    // of the visible rows fetches the next page until the cap or AniList runs
    // out, whichever comes first.
    readonly property int perPage: 50
    readonly property int maxItems: 100
    property int pageNumber: 1
    property bool loading: false
    property bool hasMore: true

    // A LATER page failed: the rows already loaded stay put and a retry row
    // appears beneath them. `loadErrorPage` is what a retry re-requests, so it
    // never resets the reader back to the top.
    property string loadError: ""
    property int loadErrorPage: 0
    property int retryIn: 0

    // A FIRST page failed: nothing to preserve, so the list area becomes an
    // error block with a retry for page 1.
    property bool loadFailed: false
    property string pageError: ""

    Timer {
        id: inlineRetryTimer
        interval: 1000
        repeat: true
        onTriggered: {
            page.retryIn -= 1
            if (page.retryIn <= 0) {
                page.retryIn = 0
                stop()
            }
        }
    }

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
                    "titlesJson": JSON.stringify(item.title || {}),
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
            // A successful page clears the inline failure and any lock.
            page.loadError = ""
            page.retryIn = 0
            page.loadFailed = false
            page.pageError = ""
            inlineRetryTimer.stop()
            RetryState.reset()
        }
        function onListFailed(message) {
            // A first-page failure has nothing to preserve, so the list area
            // becomes an error block. A LATER page must not take the rows away.
            if (rows.count === 0) {
                page.loading = false
                status.text = ""
                page.loadFailed = true
                page.pageError = message
                RetryState.reportFailure(message)
                return
            }

            page.loading = false
            page.loadError = message
            // `loadPage` sets `pageNumber` before requesting, so it already
            // names the page that failed.
            page.loadErrorPage = page.pageNumber
            page.retryIn = RetryState.isRateLimit(message) ? 5 : 0
            if (page.retryIn > 0) inlineRetryTimer.restart()
        }
    }

    function displayTitle(item) {
        return TitlePreference.title(item)
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

    // Re-run the first page. Called by the retry overlay, which can now only
    // appear for a page-1 failure.
    function reload() {
        rows.clear()
        hasMore = true
        pageNumber = 1
        loading = false
        page.loadError = ""
        page.retryIn = 0
        page.loadFailed = false
        page.pageError = ""
        inlineRetryTimer.stop()
        loadPage(1)
    }

    // Re-request ONLY the page that failed, keeping the rows already shown.
    function retryFailedPage() {
        if (!backend || loading) return
        if (page.retryIn > 0) return
        if (page.loadErrorPage <= 0) return

        // `loadError` deliberately STAYS set: the row must remain visible and
        // switch to "Retrying…" rather than vanishing, so the footer does not
        // jump and the reader can see the retry is in flight.
        page.retryIn = 0
        inlineRetryTimer.stop()
        loadPage(page.loadErrorPage)
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

        // A first-page failure replaces the list area.
        ErrorState {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: page.loadFailed
            failureMessage: page.pageError
            retryAfter: page.retryIn
            onPrimaryClicked: page.reload()
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
            visible: page.loading && rows.count === 0 && !page.loadFailed
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
            visible: !(page.loading && rows.count === 0) && !page.loadFailed

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
                            text: TitlePreference.title(JSON.parse(model.titlesJson))
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
                    onClicked: page.openAnime(model.id, model.provider || "anilist")
                }
            }

            // Fetch the next page when the last row comes into view. The cap
            // and the `hasMore` flag both live in `loadPage`, so an overshoot
            // here is harmless.
            onAtYEndChanged: {
                // The `loadError` guard is essential: a failed page leaves the
                // sentinel visible, so without it the failure would re-fire on
                // every scroll tick and hammer the rate limit.
                if (atYEnd && page.hasMore && !page.loading
                    && page.loadError === "") {
                    page.loadPage(page.pageNumber + 1)
                }
            }

            // One footer for both states, mutually exclusive: "Loading…" while
            // a page is in flight, the retry row once it has failed.
            footer: Item {
                // The height is CONSTANT, and that is load-bearing: a footer's
                // height feeds contentHeight, which drives `atYEnd`, whose
                // handler sets `page.loading`. A height that depended on
                // `loading` closed that loop.
                readonly property bool showLoading: page.loading
                    && page.loadError === ""

                width: ListView.view.width
                // Tall enough for the compact `InlineRetry` block, and still
                // CONSTANT for the `atYEnd` reason spelled above.
                height: 96

                LoadingMore {
                    id: loadingRow
                    anchors.left: parent.left
                    anchors.right: parent.right
                    visible: parent.showLoading
                }

                InlineRetry {
                    id: inlineRetry
                    anchors.left: parent.left
                    anchors.right: parent.right
                    visible: page.loadError !== ""
                    message: page.loadError
                    retryIn: page.retryIn
                    busy: page.loading
                    onRetry: page.retryFailedPage()
                }
            }
        }
    }
}