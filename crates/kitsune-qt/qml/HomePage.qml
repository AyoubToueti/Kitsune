// The home screen, mirroring the Svelte `/` route: a spotlight carousel over a
// trending rail, then four curated list blocks. The global navbar handles
// navigation, so this page owns only its own content.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    property var backend: null

    signal openAnime(int id)
    signal openTop()
    signal openSchedule()
    signal openBrowse()
    // A curated block's "View more": open the filter page preset to that
    // shelf's own status/sort, rather than a generic browse.
    signal openBrowseFiltered(string status, string sort, bool reversed)
    signal openGenre(string genre)
    signal openWatch(int animeId, int episodeNumber)
    // Declared for the window's wiring even though the navbar now owns these
    // two routes: an unwired signal is harmless, a missing one is a load error.
    signal openMyList()
    signal openSettings()

    // --- trending: shared by the hero and the rail -------------------------
    //
    // Both draw on the same list: the hero rotates the top few, the rail ranks
    // the rest. One request, split between the two consumers.
    property var heroItems: []
    property var railItems: []
    property bool trendingLoading: true

    // --- the four curated blocks ------------------------------------------
    // Each is filled by the tagged list request that names it.
    property var topAiringItems: []
    property var mostPopularItems: []
    property var topRatedItems: []
    property var latestCompletedItems: []

    // --- continue watching -------------------------------------------------
    property bool signedIn: false
    property var continueItems: []
    // The locally last-opened work, so the row can lead with it. AniList's
    // `updatedAt` does not move when re-watching the same episode, so the local
    // record is the only reliable "most recent" signal.
    property var lastPlayedId: 0
    // The reader's saved positions, keyed by anime id, for the card's resume.
    property var watchPositions: ({})

    // --- top 100 preview ---------------------------------------------------
    // A separate, larger request than the "Top rated" block: ten rows rather
    // than five, ranked by score.
    property var top100Items: []

    readonly property int heroLimit: 5
    readonly property int trendingLimit: 20
    readonly property int blockLimit: 5
    readonly property int top100Limit: 10

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    Connections {
        target: page.backend

        function onTrendingReady(json) {
            const all = JSON.parse(json) || []
            page.heroItems = all.slice(0, page.heroLimit)
            page.railItems = all.slice(page.heroLimit)
            page.trendingLoading = false
        }
        function onTrendingFailed(message) {
            page.heroItems = []
            page.railItems = []
            page.trendingLoading = false
        }

        // The four blocks share one signal; `tag` names which one this is.
        function onTaggedListReady(tag, json) {
            const items = JSON.parse(json) || []
            if (tag === "topAiring") page.topAiringItems = items
            else if (tag === "mostPopular") page.mostPopularItems = items
            else if (tag === "topRated") page.topRatedItems = items
            else if (tag === "latestCompleted") page.latestCompletedItems = items
            else if (tag === "top100") page.top100Items = items
        }
        function onTaggedListFailed(tag, message) {
            // A failed block stays empty rather than taking the page down.
        }

        // The whole home screen arrives in ONE reply: trending (split between
        // the hero and the rail) plus the five curated blocks. This is the
        // path `Component.onCompleted` uses; the per-shelf handlers above are
        // kept only as a fallback.
        function onHomeReady(json) {
            const shelves = JSON.parse(json) || ({})
            const trending = shelves.trending || []
            page.heroItems = trending.slice(0, page.heroLimit)
            page.railItems = trending.slice(page.heroLimit)
            page.trendingLoading = false
            page.topAiringItems = shelves.topAiring || []
            page.mostPopularItems = shelves.mostPopular || []
            page.topRatedItems = shelves.topRated || []
            page.latestCompletedItems = shelves.latestCompleted || []
            page.top100Items = shelves.top100 || []
            // A clean load clears any retry overlay and resets its backoff.
            RetryState.reset()
        }
        function onHomeFailed(message) {
            // Leave the skeleton up (data never arrived) and raise the retry
            // overlay, so the reader can retry in place instead of restarting.
            // `trendingLoading` stays true, which keeps every placeholder
            // visible behind the overlay.
            RetryState.reportFailure(message)
        }

        function onAuthStatusReady(signedInNow) {
            page.signedIn = signedInNow
            if (signedInNow) page.backend.continueWatching(12)
        }
        function onContinueWatchingReady(json) {
            page.continueItems = page.sortedContinue(JSON.parse(json) || [])
        }

        function onLastPlayedReady(json) {
            const record = JSON.parse(json)
            page.lastPlayedId = (record && record.animeId) ? record.animeId : 0
            // Re-sort now that we know which work was opened most recently.
            page.continueItems = page.sortedContinue(page.continueItems)
        }

        function onWatchPositionsReady(json) {
            page.watchPositions = JSON.parse(json) || ({})
            // Re-sort: the local watch times are the primary sort key, so a
            // fresh positions reply can reorder the whole row.
            page.continueItems = page.sortedContinue(page.continueItems)
        }

        // Any local resume write (a play, a position save) re-reads the row,
        // so a just-watched work leads it without a restart.
        function onLibraryChanged() {
            if (!page.backend) return
            page.backend.getLastPlayed()
            page.backend.getWatchPositions()
            if (page.signedIn) page.backend.continueWatching(12)
        }

        // An AniList LIST write -- a status change, or a removal from this
        // very row -- re-reads it too.
        //
        // Deliberately separate from `onLibraryChanged`: that fires for LOCAL
        // stores (last played, watch positions), while this fires for the
        // remote list. Without it, removing a work left its card on screen
        // until the page was left and re-entered, because nothing told the row
        // to re-read.
        function onListChanged() {
            if (!page.backend || !page.signedIn) return
            page.backend.continueWatching(12)
        }
    }

    // Order the row by when each work was last watched, most recent first.
    //
    // The local watch-position store carries an `at` timestamp per work, which
    // is the only reliable recency signal: AniList's `updatedAt` does not move
    // when the reader re-watches the episode they are already on, so its order
    // goes stale. A work with no local position (watched on another device, or
    // played before this store existed) has no timestamp, so it falls to the
    // end while keeping AniList's order among its peers -- a stable sort, so
    // equal keys never shuffle.
    function sortedContinue(items) {
        if (!items || items.length === 0) return []
        const ranked = items.map(function (item, index) {
            const id = (item && item.anime) ? item.anime.id : 0
            const pos = page.watchPositions[String(id)]
            let at = (pos && pos.at) ? pos.at : 0
            // A just-opened work whose position write has not landed yet still
            // leads, rather than falling in with the untimed entries.
            if (id !== 0 && id === page.lastPlayedId && at === 0) {
                at = Number.MAX_SAFE_INTEGER
            }
            return { "item": item, "at": at, "index": index }
        })
        ranked.sort(function (a, b) {
            if (b.at !== a.at) return b.at - a.at
            return a.index - b.index
        })
        return ranked.map(function (entry) { return entry.item })
    }

    // The saved position for a work, or null, for the card's resume readout.
    function positionFor(animeId) {
        return page.watchPositions[String(animeId)] || null
    }

    // Re-run the home load. Called by the window's retry overlay; the skeleton
    // is put back first so the reader sees the page reloading rather than the
    // previous (empty) state.
    function reload() {
        if (!backend) return
        page.trendingLoading = true
        page.heroItems = []
        page.railItems = []
        page.topAiringItems = []
        page.mostPopularItems = []
        page.topRatedItems = []
        page.latestCompletedItems = []
        page.top100Items = []
        backend.getHome()
    }

    Component.onCompleted: {
        if (!backend) return
        // ONE request fills every shelf: trending + the five curated blocks.
        // The old path fired six separate AniList queries at once, which is
        // what tripped the rate limit on startup.
        backend.getHome()
        backend.authStatus()
        // Resume ordering needs the local last-played record; the positions
        // give the cards their progress readout.
        backend.getLastPlayed()
        backend.getWatchPositions()
    }

    Flickable {
        id: scroller
        anchors.fill: parent
        contentWidth: width
        contentHeight: body.height
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        Column {
            id: body
            width: parent.width
            spacing: 24
            topPadding: Theme.pad
            bottomPadding: Theme.pad

            // --- Hero carousel ------------------------------------------
            // While the single home request is in flight, a skeleton stands in
            // for the carousel so the page reads as loading rather than blank.
            HeroSkeleton {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                visible: page.trendingLoading
            }

            HeroCarousel {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                items: page.heroItems
                visible: !page.trendingLoading
                onOpenAnime: (id) => page.openAnime(id)
            }

            // --- Trending rail ------------------------------------------
            TrendingRailSkeleton {
                width: parent.width
                visible: page.trendingLoading
            }

            Column {
                width: parent.width
                visible: page.railItems.length > 0
                spacing: 10

                Text {
                    x: Theme.pad
                    text: "Trending"
                    color: Theme.accent
                    font.pixelSize: 18
                    font.bold: true
                }

                ListView {
                    id: rail
                    width: parent.width
                    height: 300
                    orientation: ListView.Horizontal
                    leftMargin: Theme.pad
                    rightMargin: Theme.pad
                    spacing: Theme.gap
                    clip: true
                    model: page.railItems

                    delegate: TrendingCard {
                        width: 180
                        height: 280
                        anime: modelData
                        // Numbered from 01 within the rail itself, matching the
                        // original. It used to continue the hero's numbering
                        // (`index + heroLimit + 1`), so the first card read 06.
                        rank: index + 1
                        onClicked: page.openAnime(modelData.id)
                    }
                }
            }

            // --- Continue watching (signed in only) ---------------------
            Column {
                width: parent.width
                visible: page.continueItems.length > 0
                spacing: 10

                Text {
                    x: Theme.pad
                    text: "Continue Watching"
                    color: Theme.accent
                    font.pixelSize: 18
                    font.bold: true
                }

                ListView {
                    width: parent.width
                    height: 240
                    orientation: ListView.Horizontal
                    leftMargin: Theme.pad
                    rightMargin: Theme.pad
                    spacing: Theme.gap
                    clip: true
                    model: page.continueItems

                    delegate: ContinueWatchingCard {
                        required property var modelData
                        entry: modelData
                        position: page.positionFor(modelData.anime.id)
                        onOpenAnime: (id) => page.openAnime(id)
                        onOpenWatch: (animeId, episode) => page.openWatch(animeId, episode)
                    }
                }
            }

            // --- Four curated blocks ------------------------------------
            // Placeholder grid while the home reply is in flight. The four
            // shelves arrive together in the one `getHome()` response, so this
            // shares the `trendingLoading` flag with the hero.
            Grid {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                columns: 4
                spacing: 16
                visible: page.trendingLoading

                Repeater {
                    model: ["Top airing", "Most popular", "Top rated", "Latest completed"]
                    delegate: ListBlockSkeleton {
                        required property string modelData
                        width: (parent.width - 48) / 4
                        title: modelData
                    }
                }
            }

            Grid {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                columns: 4
                spacing: 16
                visible: !page.trendingLoading

                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Top airing"
                    items: page.topAiringItems
                    onOpenAnime: (id) => page.openAnime(id)
                    // Airing, most popular first -- the shelf's own filter.
                    onViewMore: page.openBrowseFiltered("releasing", "popularity", false)
                }
                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Most popular"
                    items: page.mostPopularItems
                    onOpenAnime: (id) => page.openAnime(id)
                    onViewMore: page.openBrowseFiltered("", "popularity", false)
                }
                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Top rated"
                    items: page.topRatedItems
                    onOpenAnime: (id) => page.openAnime(id)
                    onViewMore: page.openBrowseFiltered("", "score", false)
                }
                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Latest completed"
                    items: page.latestCompletedItems
                    onOpenAnime: (id) => page.openAnime(id)
                    // Finished, most recently started first.
                    onViewMore: page.openBrowseFiltered("finished", "newest", false)
                }
            }

            // --- Estimated schedule -------------------------------------
            Column {
                width: parent.width
                spacing: 12

                Text {
                    x: Theme.pad
                    text: "Estimated schedule"
                    color: Theme.accent
                    font.pixelSize: 18
                    font.bold: true
                }

                ScheduleWidgetSkeleton {
                    x: Theme.pad
                    width: parent.width - Theme.pad * 2
                    visible: page.trendingLoading
                }

                ScheduleWidget {
                    x: Theme.pad
                    width: parent.width - Theme.pad * 2
                    visible: !page.trendingLoading
                    backend: page.backend
                    onOpenAnime: (id) => page.openAnime(id)
                }
            }

            // --- Top 100 preview ----------------------------------------
            // Placeholder while the home reply is in flight: a heading bar and
            // ten ranked rows inside the same bordered shell the real preview
            // uses, so the swap does not shift the page.
            Column {
                width: parent.width
                spacing: 12
                visible: page.trendingLoading

                RowLayout {
                    x: Theme.pad
                    width: parent.width - Theme.pad * 2
                    Skeleton { width: 150; height: 18; radius: 4 }
                    Item { Layout.fillWidth: true }
                    Skeleton { width: 60; height: 12; radius: 4 }
                }

                Rectangle {
                    x: Theme.pad
                    width: parent.width - Theme.pad * 2
                    radius: 12
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle
                    implicitHeight: topSkeleton.implicitHeight + 8

                    RankedListSkeleton {
                        id: topSkeleton
                        x: 4
                        y: 4
                        width: parent.width - 8
                        count: 10
                    }
                }
            }

            Column {
                width: parent.width
                spacing: 12
                visible: !page.trendingLoading

                RowLayout {
                    x: Theme.pad
                    width: parent.width - Theme.pad * 2

                    Text {
                        text: "Top 100 Anime"
                        color: Theme.ink
                        font.pixelSize: 18
                        font.bold: true
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: "View All ›"
                        color: viewAllMouse.containsMouse ? Theme.accent : Theme.inkMuted
                        font.pixelSize: 12
                        MouseArea {
                            id: viewAllMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.openTop()
                        }
                    }
                }

                Rectangle {
                    x: Theme.pad
                    width: parent.width - Theme.pad * 2
                    radius: 12
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle
                    implicitHeight: topList.implicitHeight + 8

                    TopList {
                        id: topList
                        x: 4
                        y: 4
                        width: parent.width - 8
                        items: page.top100Items
                        onOpenAnime: (id) => page.openAnime(id)
                    }
                }
            }
        }
    }
}