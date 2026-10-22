// The full recommendations list for one work, paged as the reader scrolls.
// Pushed from the detail page's "View more". Mirrors the Svelte
// `/anime/[id]/recommendations` route: a heading with the work's title, a Back
// link, and a grid of RecommendationCards (score + votes + quick-add).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: page

    property var backend: null
    property int animeId: 0
    property string workTitle: ""

    signal back()
    // `provider` is the id's namespace, forwarded so the detail page fetches
    // from the source that issued the id.
    signal openAnime(int id, string provider)

    readonly property int perPage: 24

    property int pageNumber: 1
    property bool loading: false
    property bool hasMore: true
    property string error: ""

    // A LATER page failed: the loaded grid stays and a retry row appears under
    // it. Kept separate from `error`, which is the empty-state message.
    property string loadError: ""
    property int loadErrorPage: 0
    property int retryIn: 0

    // A FIRST page failed: no grid to preserve, so the results area becomes an
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

    // The loaded recommendations, as a ListModel of `{ recJson }` rows.
    //
    // A ListModel (not a plain array) so appending a page emits an incremental
    // `rowsInserted` signal and the GridView KEEPS its scroll position.
    // Reassigning a plain array replaces the whole model, which resets the
    // view to the top -- the "scrolls me back up on the next page" bug.
    //
    // Each row stores the recommendation as a JSON STRING rather than the
    // object itself: a ListModel coerces a role holding a nested object to the
    // string "[object Object]", which made every card render "Untitled". A
    // string survives the round-trip; the delegate parses it back.
    ListModel { id: recModel }

    // An inline back chevron. URL-encoded, NOT base64: `Qt.btoa` is deprecated
    // in Qt 6 (its output differs from the standard Web API).
    //
    // The fill comes from the theme: the button sits on the header bar, which
    // is light in light mode, so a hard-coded white chevron disappeared.
    function getBackArrowSvg() {
        var xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='"
            + Theme.hex(Theme.ink)
            + "'><path d='M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z'/></svg>"
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    Connections {
        target: page.backend
        function onRecommendationsReady(id, json) {
            if (id !== page.animeId) return
            const pageObj = JSON.parse(json)
            const raw = (pageObj && pageObj.items) ? pageObj.items : []
            page.loading = false
            // A first page replaces; a later page appends. `append` (not a
            // model reassignment) is what keeps the scroll position.
            if (page.pageNumber === 1) recModel.clear()
            for (let i = 0; i < raw.length; i++) {
                recModel.append({ "recJson": JSON.stringify(raw[i]) })
            }
            const info = pageObj ? pageObj.pageInfo : null
            page.hasMore = info ? info.hasNextPage !== false : false
            // A successful page clears the inline failure and any lock.
            page.loadError = ""
            page.retryIn = 0
            page.loadFailed = false
            page.pageError = ""
            inlineRetryTimer.stop()
            RetryState.reset()
        }
        function onRecommendationsFailed(id, message) {
            if (id !== page.animeId) return
            // A first-page failure has nothing to preserve, so the results
            // area becomes an error block. A LATER page must not take the grid.
            if (recModel.count === 0) {
                page.loading = false
                page.error = ""
                page.loadFailed = true
                page.pageError = message
                RetryState.reportFailure(message)
                return
            }

            page.loading = false
            page.loadError = message
            // `loadPage` sets `pageNumber` before requesting.
            page.loadErrorPage = page.pageNumber
            page.retryIn = RetryState.isRateLimit(message) ? 5 : 0
            if (page.retryIn > 0) inlineRetryTimer.restart()
        }
    }

    function loadPage(number) {
        if (!backend || loading || animeId <= 0) return
        if (number > 1 && !hasMore) return
        loading = true
        error = ""
        pageNumber = number
        backend.getRecommendations(animeId, number, perPage)
    }

    function maybeLoad() {
        if (animeId > 0 && backend) {
            recModel.clear()
            hasMore = true
            pageNumber = 1
            loadPage(1)
        }
    }

    // Re-run the first page. Called by the retry overlay, which can now only
    // appear for a page-1 failure.
    function reload() {
        recModel.clear()
        hasMore = true
        pageNumber = 1
        loading = false
        error = ""
        page.loadError = ""
        page.retryIn = 0
        page.loadFailed = false
        page.pageError = ""
        inlineRetryTimer.stop()
        loadPage(1)
    }

    // Re-request ONLY the page that failed, keeping the loaded grid.
    function retryFailedPage() {
        if (!backend || loading || animeId <= 0) return
        if (page.retryIn > 0) return
        if (page.loadErrorPage <= 0) return

        // `loadError` deliberately STAYS set: the row must remain visible and
        // switch to "Retrying…" rather than vanishing, so the footer does not
        // jump and the reader can see the retry is in flight.
        page.retryIn = 0
        inlineRetryTimer.stop()
        loadPage(page.loadErrorPage)
    }
    onAnimeIdChanged: maybeLoad()
    onBackendChanged: maybeLoad()

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Glassmorphic Header Bar
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 76

            Rectangle {
                anchors.fill: parent
                color: Qt.rgba(Theme.surface.r, Theme.surface.g, Theme.surface.b, 0.6)
                border.width: 1
                border.color: Theme.glassBorder
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad
                spacing: 16

                // Back Button Frame
                Rectangle {
                    Layout.preferredWidth: 38
                    Layout.preferredHeight: 38
                    radius: 12
                    color: backMouse.containsMouse ? Theme.surfaceHover : Qt.rgba(1, 1, 1, 0.04)
                    border.width: 1
                    border.color: backMouse.containsMouse ? Theme.accent : Theme.glassBorder
                    scale: backMouse.containsMouse ? 1.04 : 1.0

                    Behavior on scale { NumberAnimation { duration: 150 } }
                    Behavior on border.color { ColorAnimation { duration: 150 } }

                    Image {
                        anchors.centerIn: parent
                        width: 16
                        height: 16
                        source: page.getBackArrowSvg()
                        sourceSize.width: 32
                        sourceSize.height: 32
                        smooth: true
                    }

                    MouseArea {
                        id: backMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.back()
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        Layout.fillWidth: true
                        text: page.workTitle.length > 0
                            ? ("Recommended for " + page.workTitle)
                            : "Recommendations"
                        color: Theme.ink
                        font.pixelSize: 19
                        font.bold: true
                        elide: Text.ElideRight
                    }

                    Text {
                        // The failure is reported by the error block below, so
                        // the header only ever counts what actually arrived.
                        text: recModel.count + " suggestions found"
                        color: Theme.inkMuted
                        font.pixelSize: 12
                        visible: !page.loadFailed
                    }
                }
            }
        }

        // A first-page failure replaces the results area.
        ErrorState {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: page.loadFailed
            failureMessage: page.pageError
            retryAfter: page.retryIn
            onPrimaryClicked: page.reload()
        }

        // First-load placeholder: shown until the first page of recommendations
        // arrives. Later pages append and keep the footer "Loading…", so a
        // skeleton would only jitter the scroll.
        PosterGridSkeleton {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            visible: page.loading && recModel.count === 0 && !page.loadFailed
            count: 8
            cellWidth: 200
            cellHeight: 356
        }

        // Results Grid
        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: recModel
            cellWidth: 200
            cellHeight: 356
            leftMargin: Theme.pad
            rightMargin: Theme.pad
            topMargin: Theme.gap
            bottomMargin: Theme.pad
            visible: !(page.loading && recModel.count === 0) && !page.loadFailed

            delegate: RecommendationCard {
                required property string recJson
                width: 184
                mediaId: page.animeId
                votable: true
                // The row holds the recommendation as a JSON string; parse it
                // back into the object the card expects.
                recommendation: JSON.parse(recJson)
                backend: page.backend
                onOpenAnime: (id, provider) => page.openAnime(id, provider)
            }

            onAtYEndChanged: {
                // The `loadError` guard is essential: a failed page leaves the
                // sentinel visible, so without it the failure would re-fire on
                // every scroll tick and hammer the rate limit.
                if (atYEnd && page.hasMore && !page.loading
                    && page.loadError === "") {
                    page.loadPage(page.pageNumber + 1)
                }
            }

            // One footer for both states, mutually exclusive: the "Loading…"
            // row while a page is in flight, the retry row once it failed.
            footer: Item {
                // The height is CONSTANT, and that is load-bearing: a footer's
                // height feeds contentHeight, which drives `atYEnd`, whose
                // handler sets `page.loading`. A height that depended on
                // `loading` closed that loop.
                readonly property bool showLoading: page.loading
                    && page.loadError === ""

                width: GridView.view.width
                // Tall enough for the compact `InlineRetry` block, and still
                // CONSTANT for the `atYEnd` reason spelled above.
                height: 96

                LoadingMore {
                    id: loadingRow
                    anchors.left: parent.left
                    anchors.right: parent.right
                    visible: parent.showLoading
                    message: "Loading recommendations\u2026"
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

        // Empty State Banner
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 120
            // Hidden while the retry overlay is up: the page has not concluded
            // "no suggestions", it has failed to ask, and the overlay says so.
            visible: !page.loading && recModel.count === 0
                && page.error.length === 0 && !page.loadFailed

            Rectangle {
                anchors.centerIn: parent
                implicitWidth: emptyText.implicitWidth + 32
                implicitHeight: 40
                radius: 20
                color: Theme.fieldFill
                border.width: 1
                border.color: Theme.borderSubtle

                Text {
                    id: emptyText
                    anchors.centerIn: parent
                    text: "No recommendations found for this title yet."
                    color: Theme.inkMuted
                    font.pixelSize: 13
                }
            }
        }
    }
}