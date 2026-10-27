// A single title's detail page, mirroring the Svelte `/anime/[id]` page:
// hero backdrop + poster, a labelled metadata strip, an add-to-list menu,
// synopsis, an episode grid, a trailer card, a related-anime sidebar, and
// recommendations.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: page

    // The anime id to load. Setting it triggers the fetch.
    property int animeId: 0
    /// The id's NAMESPACE ("anilist" / "kitsu"), from the card that was
    /// clicked. Ids are per-source, so the page must ask the source that
    /// issued this one -- a Kitsu id sent to AniList fetches a different work.
    property string animeProvider: "anilist"
    property var anime: null

    // The work itself failed to load: the skeleton gives way to an error block
    // with a retry, rather than pulsing forever.
    property bool loadFailed: false
    property string loadError: ""

    // The work's episode numbers that currently have a downloaded,
    // on-disk record. Keys are the record's release expression (episode
    // number as a decimal string), values are one or more wire rows for that
    // episode: a reader can keep more than one source per episode.
    property var downloadedEpisodeReleases: ({})

    // The shared Backend, passed in explicitly by the window when it pushes this page.
    property var backend: null

    signal back()
    /// `provider` is the id's namespace, so the next page fetches from the
    /// source that issued it.
    signal openAnime(int id, string provider)
    // Emitted when an episode is chosen: pushes the watch page with the
    // numbers it needs to search a release and match a torrent file.
    signal openWatch(int animeId, int malId, int episodeNumber, int absoluteEpisode)
    // Season packs are a work-level search, so they get their own entry rather
    // than being buried behind a specific episode.
    signal openPacks(int animeId, int malId)
    // Emitted when a genre chip is clicked: opens the filter page for it.
    signal openGenre(string genre)
    // Emitted by the Recommendations "View more": opens the full, paged
    // recommendations list for this work.
    signal openRecommendations(int animeId, string title)

    // --- Episodes ---------------------------------------------------------
    property var episodes: []
    property int episodeOffset: 0
    property bool episodesLoading: false
    // The episode catalogue as `{ info: [...], aired: N }`. The aired count is
    // separate from the announced `episodeCount` because a releasing show has
    // more announced episodes than watchable ones.
    property var episodeInfo: []
    property int airedEpisodes: 0
    // Which work's episodes are already loaded, so a repeat `animeReady` for
    // the same id does not re-fetch. See `onAnimeReady`.
    property int episodesLoadedFor: 0
    property bool sortAscending: true

    // --- Jump-to-episode --------------------------------------------------
    property int jumpQuery: 0
    property string jumpError: ""
    property int highlightedIndex: -1

    // --- Watch progress ---------------------------------------------------
    property int aniProgress: 0
    property int localEpisode: 0
    readonly property int watchedThrough: Math.max(aniProgress, localEpisode)

    // --- List status ------------------------------------------------------
    property bool signedIn: false
    property string listStatus: ""
    property int listEntryId: 0
    property bool statusBusy: false
    property string statusError: ""

    readonly property var listStatuses: [
        { value: "current", label: "Watching" },
        { value: "completed", label: "Completed" },
        { value: "paused", label: "On hold" },
        { value: "dropped", label: "Dropped" },
        { value: "planning", label: "Plan to watch" }
    ]

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    function metaChips() {
        if (!page.anime) return []
        const a = page.anime
        const out = []
        if (a.format) out.push({ label: "Format", value: a.format })
        // An unreleased work says "0 eps" rather than hiding the row: the
        // reader asked how many episodes it has, and "none yet" is the honest
        // answer. The provider's own announced total is deliberately not
        // shown, because those episodes cannot be watched.
        if (a.status === "NOT_YET_RELEASED") out.push({ label: "Episodes", value: "0 eps" })
        else if (a.episodeCount != null) out.push({ label: "Episodes", value: a.episodeCount + " eps" })
        if (a.durationMinutes != null) out.push({ label: "Duration", value: a.durationMinutes + " min" })
        if (a.status) out.push({ label: "Status", value: a.status })
        if (a.seasonYear != null) out.push({ label: "Year", value: String(a.seasonYear) })
        if (a.averageScore != null) out.push({ label: "Score", value: String(a.averageScore), score: true })
        if (a.popularity != null) out.push({ label: "Popularity", value: Number(a.popularity).toLocaleString(Qt.locale()) })
        return out
    }

    function relationLabel(type) {
        if (!type) return ""
        return String(type).toLowerCase().split("_")
            .map(function (word) { return word.charAt(0).toUpperCase() + word.slice(1) })
            .join(" ")
    }

    function trailerWatchUrl(t) {
        if (!t || !t.site || !t.id) return ""
        const site = String(t.site).toLowerCase()
        if (site === "youtube") return "https://www.youtube.com/watch?v=" + t.id
        if (site === "dailymotion") return "https://www.dailymotion.com/video/" + t.id
        return ""
    }

    function episodeLabel(ep, index) {
        const number = ep.number !== undefined ? ep.number : index + 1
        const t = (ep.title || "").trim()
        if (t === "") return "Episode " + number
        if (/^ep(isode)?\.?\s*\d+/i.test(t)) return t
        return "Episode " + number + " - " + t
    }

    // The label for the trailing entry representing an episode that has not
    // aired yet. The date is formatted here rather than in Rust so it follows
    // the reader's locale; `airingAt` is unix seconds (UTC).
    function upcomingLabel(ep) {
        const number = ep.number !== undefined ? ep.number : 0
        const when = ep.airingAt
            ? Qt.formatDate(new Date(ep.airingAt * 1000), "d MMM")
            : ""
        if (when === "") return "Episode " + number
        return "Episode " + number + " · " + when
    }

    function computeOffset(list) {
        const numbers = list.map(function (e) { return e.number })
            .filter(function (n) { return n !== undefined })
        if (numbers.length === 0) return 0
        const lowest = Math.min.apply(null, numbers)
        return lowest > list.length ? lowest - 1 : 0
    }

    function triggerWatch(epNum) {
        const malId = page.anime && page.anime.idMal ? page.anime.idMal : 0
        page.openWatch(page.animeId, malId, epNum, epNum + page.episodeOffset)
    }

    function toggleEpisodeSort() {
        page.sortAscending = !page.sortAscending
        if (page.episodes && page.episodes.length > 0) {
            page.episodes = page.episodes.slice().reverse()
        }
    }

    function scrollToLastWatched() {
        if (page.watchedThrough <= 0 || !page.episodes || page.episodes.length === 0) return

        let found = -1
        for (let i = 0; i < page.episodes.length; i++) {
            const n = page.episodes[i].number !== undefined
                ? page.episodes[i].number : (i + 1)
            if (n === page.watchedThrough) { found = i; break }
        }

        if (found !== -1) {
            page.highlightedIndex = found
            episodeGrid.positionViewAtIndex(found, GridView.Center)
            highlightTimer.restart()
        }
    }

    Connections {
        target: page.backend

        function onAnimePartial(json) {
            const parsed = JSON.parse(json)
            if (parsed === null || parsed.id !== page.animeId) return
            if (page.anime === null) page.anime = parsed
        }

        function onAnimeReady(json) {
            const parsed = JSON.parse(json)
            if (parsed === null) {
                page.anime = null
            } else if (parsed.id === page.animeId) {
                page.anime = parsed
                page.loadFailed = false
                page.loadError = ""
                page.refreshDownloadedEpisodes()
                // Only fetch the episodes ONCE per work.
                //
                // `animeReady` is a signal on the shared Backend object, not
                // scoped to this page. The watch modal fetches the same work
                // when it opens (for its title forms), and that reply arrives
                // here too -- matching `animeId` and re-running the whole
                // episode load. That was the spinner and network round trip
                // behind the grid every time a release was picked or the
                // popup closed.
                if (page.episodesLoadedFor !== page.animeId) {
                    page.loadEpisodes()
                }
                page.loadListEntry()
                RetryState.reset()
            }
        }
        function onAnimeFailed(message) {
            page.loadFailed = true
            page.loadError = message
            RetryState.reportFailure(message)
        }
        function onEpisodesReady(json) {
            // `{ info, aired }`: the catalogue plus how many have actually
            // aired. A bare array here would silently lose the aired count and
            // re-introduce the "unaired episodes are playable" bug.
            const payload = JSON.parse(json) || ({})
            page.episodeInfo = payload.info || []
            page.airedEpisodes = payload.aired || 0
            page.episodesLoading = false
            page.rebuildEpisodes()
            page.refreshDownloadedEpisodes()
        }
        function onEpisodesFailed(message) {
            // No catalogue: fall back to the previous behaviour, which
            // synthesises from the announced count. Clearing the loading flag
            // is what un-hides the grid in that case.
            page.episodeInfo = []
            page.airedEpisodes = 0
            page.episodesLoading = false
            page.rebuildEpisodes()
        }

        function onAuthStatusReady(signedInNow) {
            page.signedIn = signedInNow
            if (signedInNow && page.anime) page.fetchListEntry()
            else if (!signedInNow) {
                page.listStatus = ""
                page.listEntryId = 0
            }
        }
        function onListEntryReady(json) {
            const entry = JSON.parse(json)
            page.listStatus = entry ? (entry.status || "") : ""
            page.listEntryId = entry ? (entry.id || 0) : 0
            page.aniProgress = entry ? (entry.progress || 0) : 0
        }
        function onListChanged() {
            // The STORE, not `authStatus()`. Calling `authStatus()` here made
            // the loop: authStatus started a sync, the sync's pull emitted
            // `listChanged`, this handler asked for authStatus again -- one
            // full 285-entry pull every couple of seconds until AniList
            // answered 429. `fetchListEntry()` only reads the local list.
            page.fetchListEntry()
        }
        function onWatchPositionsReady(json) {
            const all = JSON.parse(json) || ({})
            const entry = all[String(page.animeId)]
            page.localEpisode = (entry && entry.episode) ? entry.episode : 0
        }
        function onLibraryChanged() {
            if (page.backend) page.backend.getWatchPositions()
        }
        function onDownloadCompleted(json) {
            const parsed = JSON.parse(json) || null
            if (!parsed) return
            if (parsed.animeId !== page.animeId) return
            if (String(parsed.provider || "") !== page.animeProvider) return
            page.refreshDownloadedEpisodes()
        }
        function onAuthError(message) {
            page.statusError = message
        }
    }

    function rebuildEpisodes() {
        if (!backend || !anime) {
            page.episodes = []
            return
        }
        if (backend) backend.getWatchPositions()

        // `nextAiring` rides along with the catalogue: the grid is built in
        // one call, so a separate property would be a second source of truth
        // for the same list. It is only present on a still-airing work.
        // `aired` has TWO sources and the fallback matters:
        //
        // - `page.airedEpisodes` comes from the episode catalogue request, and
        //   is 0 when that request failed or the work has no mapping.
        // - `anime.airedEpisodes` comes from the DETAIL response, where AniList
        //   derives it from `nextAiringEpisode`.
        //
        // Only the first was passed before. A releasing work whose catalogue
        // fetch failed therefore arrived with an aired count of 0, the cap had
        // nothing to work with, and the announced total was drawn in full.
        const aired = page.airedEpisodes > 0
            ? page.airedEpisodes
            : (anime.airedEpisodes || 0)
        const json = backend.buildEpisodes(
            anime.episodeCount || 0,
            anime.status || "",
            anime.coverImage || "",
            JSON.stringify({
                info: page.episodeInfo,
                aired: aired,
                nextAiring: anime.nextAiring || null
            }))
        var list = JSON.parse(json) || []
        if (!page.sortAscending) {
            list.reverse()
        }
        page.episodes = list
        page.episodeOffset = computeOffset(page.episodes)
    }

    function loadEpisodes() {
        page.episodeInfo = []
        page.airedEpisodes = 0

        // Deliberately NO `rebuildEpisodes()` here. Rebuilding now would
        // synthesise a stand-in list from the announced count, paint it, and
        // then replace it when the real catalogue lands -- the visible "flash"
        // of placeholder cards. The grid stays empty (behind the loading fox)
        // until `onEpisodesReady` fills it.
        //
        // The id AND its namespace: a Kitsu id sent as an AniList id resolves
        // to no mapping and yields an empty grid. `anime.provider` says which
        // source issued it, so the backend does not have to guess.
        if (!anime || !anime.id) {
            // Nothing to fetch, so nothing to wait for. Rebuild immediately so
            // the synthesised path still works.
            page.episodesLoading = false
            page.rebuildEpisodes()
            return
        }

        page.episodesLoading = true
        page.episodesLoadedFor = anime.id
        backend.getEpisodes(anime.id, anime.provider || "anilist")
    }

    function loadListEntry() {
        if (!backend) return
        // Only to LEARN the sign-in state; this no longer starts a sync.
        backend.authStatus()
    }

    function fetchListEntry() {
        if (backend && page.signedIn && page.anime) {
            backend.getListEntry(page.anime.id)
        }
    }

    // Refreshed whenever the downloaded-episode cache can plausibly have
    // changed. It reads the store synchronously, so it is intentionally not
    // reactive on every download event.
    function refreshDownloadedEpisodes() {
        if (!backend || !animeId) {
            page.downloadedEpisodeReleases = ({}); return
        }
        const target = animeProvider || "anilist"
        let rows = []
        try { rows = JSON.parse(backend.downloadedForAnime(page.animeId, target)) || [] } catch (_) { rows = [] }

        const next = ({})
        for (const row of rows) {
            const key = String(row.episode !== undefined && row.episode !== null ? row.episode : 0)
            if (key === "0") continue
            if (!next[key]) next[key] = []
            next[key].push(row)
        }
        page.downloadedEpisodeReleases = next
    }

    function episodeIsDownloaded(epNum) {
        const key = String(epNum)
        const rows = page.downloadedEpisodeReleases[key]
        return Array.isArray(rows) && rows.length > 0
    }

    function chooseStatus(value) {
        if (!backend || !page.anime) return
        const previous = page.listStatus
        page.listStatus = value
        page.statusBusy = true
        page.statusError = ""
        backend.setListEntry(page.anime.id, value, 0)
        statusBusyTimer.restart()
        if (previous === value) return
    }

    function removeFromList() {
        if (!backend || page.listEntryId === 0) return
        const previousId = page.listEntryId
        page.listStatus = ""
        page.listEntryId = 0
        page.statusBusy = true
        page.statusError = ""
        backend.deleteListEntry(previousId)
        statusBusyTimer.restart()
    }

    function signIn() {
        if (!backend) return
        const url = backend.beginLogin()
        if (url && url.length > 0) Qt.openUrlExternally(url)
    }

    function maybeLoad() {
        if (animeId > 0 && backend) {
            page.anime = null
            page.loadFailed = false
            page.loadError = ""
            page.signedIn = false
            page.listStatus = ""
            page.listEntryId = 0
            backend.getAnime(animeId, page.animeProvider)
        }
    }

    function reload() {
        if (animeId > 0 && backend) {
            page.anime = null
            page.loadFailed = false
            page.loadError = ""
            backend.getAnime(animeId, page.animeProvider)
        }
    }

    onAnimeIdChanged: maybeLoad()
    onAnimeProviderChanged: maybeLoad()
    onBackendChanged: maybeLoad()

    Timer {
        id: statusBusyTimer
        interval: 1500
        onTriggered: page.statusBusy = false
    }

    ErrorState {
        anchors.fill: parent
        visible: page.loadFailed
        failureMessage: page.loadError
        retryAfter: RetryState.remaining
        onPrimaryClicked: page.reload()
    }

    DetailSkeleton {
        anchors.fill: parent
        visible: page.anime === null && !page.loadFailed
    }

    Flickable {
        id: scroller
        anchors.fill: parent
        contentWidth: width
        contentHeight: body.height
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        visible: page.anime !== null

        Column {
            id: body
            width: parent.width
            spacing: 0

            // --- Hero Section -----------------------------------------------
            Rectangle {
                width: parent.width
                height: 360
                color: Theme.surface
                clip: true

                // Same rule as the home hero: a missing banner becomes a
                // blurred wash instead of a cropped portrait. Kept at 0.25
                // opacity, which is what the scrim below was tuned against.
                //
                // The clear area starts further left here: this hero puts a
                // poster AND a text column on the left, so there is less room
                // the artwork has to fill.
                HeroBackdrop {
                    anchors.fill: parent
                    banner: page.anime ? (page.anime.bannerImage || "") : ""
                    cover: page.anime ? (page.anime.coverImage || "") : ""
                    clearStart: 0.28
                    opacity: 0.8
                }

                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: Theme.heroScrim }
                        GradientStop { position: 1.0; color: Theme.surface }
                    }
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.pad
                    spacing: 24

                    // Poster
                    Rectangle {
                        Layout.preferredWidth: 190
                        Layout.preferredHeight: 285
                        Layout.alignment: Qt.AlignVCenter
                        //radius: Theme.radius
                        color: Theme.surfaceRaised
                        clip: true

                        Image {
                            anchors.fill: parent
                            source: page.anime ? (page.anime.coverImage || "") : ""
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                        }
                    }

                    // Text column
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 10

                        Text {
                            Layout.fillWidth: true
                            text: page.anime ? displayTitle(page.anime) : "Loading…"
                            color: Theme.ink
                            font.pixelSize: 34
                            font.bold: true
                            wrapMode: Text.WordWrap
                        }

                        // Genre chips
                        Flow {
                            Layout.fillWidth: true
                            Layout.preferredHeight: childrenRect.height
                            spacing: 6
                            Repeater {
                                model: page.anime ? (page.anime.genres || []).slice(0, 8) : []
                                delegate: Rectangle {
                                    required property string modelData
                                    radius: 8
                                    color: genreMouse.containsMouse ? Theme.accent : Theme.surfaceRaised
                                    border.width: 1
                                    border.color: genreMouse.containsMouse ? Theme.accent : Theme.borderSubtle
                                    implicitWidth: genreText.implicitWidth + 20
                                    implicitHeight: 28

                                    Text {
                                        id: genreText
                                        anchors.centerIn: parent
                                        text: modelData
                                        color: genreMouse.containsMouse ? "white" : Theme.inkMuted
                                        font.pixelSize: 12
                                    }
                                    MouseArea {
                                        id: genreMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: page.openGenre(modelData)
                                    }
                                }
                            }
                        }

                        // --- Action bar: Add to List + Quick Watch CTA ---------
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 4
                            Layout.bottomMargin: 4
                            spacing: 12

                            // Primary Play CTA Button
                            Rectangle {
                                implicitWidth: playCtaRow.implicitWidth + 28
                                height: 40
                                radius: 20
                                color: playCtaMouse.containsMouse ? Theme.accentHover : Theme.accent

                                Behavior on color { ColorAnimation { duration: 150 } }

                                RowLayout {
                                    id: playCtaRow
                                    anchors.centerIn: parent
                                    spacing: 8

                                    PlayerIcon {
                                        kind: "play"
                                        color: "white"
                                        width: 14; height: 14
                                    }

                                    Text {
                                        text: page.watchedThrough > 0
                                            ? ("Resume Ep " + page.watchedThrough)
                                            : "Watch Ep 1"
                                        color: "white"
                                        font.pixelSize: 14
                                        font.bold: true
                                    }
                                }

                                MouseArea {
                                    id: playCtaMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        const targetEp = page.watchedThrough > 0 ? page.watchedThrough : 1
                                        page.triggerWatch(targetEp)
                                    }
                                }
                            }

                            // Add to list menu button
                            Rectangle {
                                implicitWidth: statusRow.implicitWidth + 28
                                height: 40
                                radius: 20
                                color: statusButtonMouse.containsMouse && !page.statusBusy
                                    ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: statusButtonMouse.containsMouse && !page.statusBusy
                                    ? Theme.accent : Theme.borderSubtle

                                Behavior on border.color { ColorAnimation { duration: 150 } }

                                Row {
                                    id: statusRow
                                    anchors.centerIn: parent
                                    spacing: 8

                                    Text {
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: "＋"
                                        color: Theme.ink
                                        font.pixelSize: 14
                                        font.bold: true
                                    }
                                    Text {
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: {
                                            if (!page.signedIn) return "Sign in to AniList"
                                            if (page.listStatus === "") return "Add to list"
                                            const found = page.listStatuses.find(
                                                function (s) { return s.value === page.listStatus })
                                            return found ? found.label : "Add to list"
                                        }
                                        color: Theme.ink
                                        font.pixelSize: 14
                                        font.bold: true
                                    }
                                }

                                MouseArea {
                                    id: statusButtonMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    enabled: !page.statusBusy
                                    onClicked: {
                                        if (!page.signedIn) page.signIn()
                                        else statusPopup.open()
                                    }
                                }
                            }

                            Item { Layout.fillWidth: true }
                        }

                        // A list add/remove failure. Compact and non-actionable:
                        // Retry cannot know which mutation failed, so the strip
                        // just reports it and clears on the next attempt.
                        ErrorState {
                            Layout.fillWidth: true
                            visible: page.statusError !== ""
                            compact: true
                            showArt: false
                            showActions: false
                            failureMessage: page.statusError
                            detail: page.statusError
                            title: "Couldn't update your list"
                            message: ""
                        }

                        // --- Metadata strip ----------------------------------
                        Flow {
                            Layout.fillWidth: true
                            Layout.preferredHeight: childrenRect.height
                            spacing: 20
                            Repeater {
                                model: page.anime ? page.metaChips() : []
                                delegate: Row {
                                    required property var modelData
                                    spacing: 6
                                    Text {
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: modelData.label
                                        color: Theme.inkFaint
                                        font.pixelSize: 14
                                    }
                                    Text {
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: modelData.value
                                        color: modelData.score ? Theme.score : Theme.ink
                                        font.pixelSize: 14
                                    }
                                }
                            }
                        }

                        // --- Synopsis ----------------------------------------
                        Text {
                            Layout.fillWidth: true
                            Layout.topMargin: 4
                            // `!!` is load-bearing: `a && b` yields `b`, so a
                            // missing `description` (undefined) would be
                            // assigned straight to a bool and warn.
                            visible: !!(page.anime && page.anime.description)
                            text: page.anime
                                ? String(page.anime.description).replace(/<[^>]*>/g, "")
                                : ""
                            color: Theme.inkMuted
                            font.pixelSize: 13
                            wrapMode: Text.WordWrap
                            lineHeight: 1.4
                            maximumLineCount: 5
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            // --- Two-column body ---------------------------------------------
            Row {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                topPadding: Theme.pad
                spacing: 32

                // ===== Left column: episodes ================================
                Column {
                    width: parent.width - 352
                    spacing: 12

                    RowLayout {
                        width: parent.width
                        spacing: 10

                        Text {
                            text: "Episodes"
                            color: Theme.ink
                            font.pixelSize: 18
                            font.bold: true
                        }

                        // Season packs button
                        Rectangle {
                            visible: page.anime !== null && page.anime.episodeCount !== 0
                            implicitWidth: packsRow.implicitWidth + 20
                            height: 28
                            radius: 14
                            color: packsMouse.containsMouse ? Theme.surfaceHover : "transparent"
                            border.width: 1
                            border.color: packsMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                            Behavior on border.color { ColorAnimation { duration: 150 } }

                            Row {
                                id: packsRow
                                anchors.centerIn: parent
                                spacing: 4

                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: "Season packs"
                                    color: packsMouse.containsMouse ? Theme.accent : Theme.inkMuted
                                    font.pixelSize: 12
                                    font.bold: true
                                }
                            }

                            MouseArea {
                                id: packsMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    const mal = page.anime && page.anime.idMal ? page.anime.idMal : 0
                                    page.openPacks(page.animeId, mal)
                                }
                            }
                        }

                        // Sort order toggle button (Oldest / Newest)
                        Rectangle {
                            visible: page.anime !== null && page.episodes.length > 1
                            implicitWidth: sortRow.implicitWidth + 20
                            height: 28
                            radius: 14
                            color: sortMouse.containsMouse ? Theme.surfaceHover : "transparent"
                            border.width: 1
                            border.color: sortMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                            Behavior on border.color { ColorAnimation { duration: 150 } }

                            Row {
                                id: sortRow
                                anchors.centerIn: parent
                                spacing: 4

                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: page.sortAscending ? "↓ Oldest" : "↑ Newest"
                                    color: sortMouse.containsMouse ? Theme.accent : Theme.inkMuted
                                    font.pixelSize: 12
                                    font.bold: true
                                }
                            }

                            MouseArea {
                                id: sortMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: page.toggleEpisodeSort()
                            }
                        }

                        // Scroll to last watched episode button
                        Rectangle {
                            visible: page.watchedThrough > 0 && page.episodes.length > 0
                            implicitWidth: lastWatchRow.implicitWidth + 20
                            height: 28
                            radius: 14
                            color: lastWatchMouse.containsMouse ? Theme.surfaceHover : "transparent"
                            border.width: 1
                            border.color: lastWatchMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                            Behavior on border.color { ColorAnimation { duration: 150 } }

                            Row {
                                id: lastWatchRow
                                anchors.centerIn: parent
                                spacing: 4

                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: "🎯 Ep " + page.watchedThrough
                                    color: lastWatchMouse.containsMouse ? Theme.accent : Theme.inkMuted
                                    font.pixelSize: 12
                                    font.bold: true
                                }
                            }

                            MouseArea {
                                id: lastWatchMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: page.scrollToLastWatched()
                            }
                        }

                        // `page.episodes.length` is what is RENDERED, not what
                        // has aired -- the list is capped and can be paged. The
                        // aired count is the honest number, so it comes from
                        // the provider rather than from the grid.
                        Text {
                            readonly property int aired: page.airedEpisodes > 0
                                ? page.airedEpisodes
                                : (page.anime ? (page.anime.airedEpisodes || 0) : 0)
                            readonly property int total: page.anime && page.anime.episodeCount != null
                                ? page.anime.episodeCount : 0
                            visible: total > 0 && aired > 0 && total > aired
                            text: aired + " of " + total + " aired"
                            color: Theme.inkFaint
                            font.pixelSize: 12
                            Layout.alignment: Qt.AlignVCenter
                        }

                        Item { Layout.fillWidth: true }

                        Row {
                            id: jumpRow
                            spacing: 8

                            Rectangle {
                                width: 130
                                height: 32
                                radius: 16
                                color: Theme.surfaceHover
                                border.width: 1
                                border.color: jumpField.activeFocus ? Theme.accent : Theme.borderSubtle

                                TextField {
                                    id: jumpField
                                    anchors.fill: parent
                                    anchors.leftMargin: 12
                                    anchors.rightMargin: 12
                                    placeholderText: "Episode #"
                                    color: Theme.ink
                                    placeholderTextColor: Theme.inkFaint
                                    font.pixelSize: 13
                                    background: Item {}
                                    inputMethodHints: Qt.ImhDigitsOnly
                                    validator: IntValidator { bottom: 1 }
                                    onTextChanged: page.jumpError = ""
                                    onAccepted: jumpToEpisode()
                                }
                            }

                            Rectangle {
                                width: 48
                                height: 32
                                radius: 16
                                color: goMouse.containsMouse ? Theme.accentHover : Theme.accent

                                Text {
                                    anchors.centerIn: parent
                                    text: "Go"
                                    color: "white"
                                    font.pixelSize: 13
                                    font.bold: true
                                }
                                MouseArea {
                                    id: goMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: jumpToEpisode()
                                }
                            }
                        }
                    }

                    Text {
                        visible: page.jumpError !== ""
                        text: page.jumpError
                        color: Theme.danger
                        font.pixelSize: 12
                    }

                    GridView {
                        id: episodeGrid
                        width: parent.width
                        height: Math.min(contentHeight, 420)
                        cellWidth: 216
                        cellHeight: 150
                        clip: true

                        // Hidden while the catalogue is in flight, so the
                        // reader sees the loading fox instead of a grid that
                        // fills with placeholders and then repaints.
                        visible: !page.episodesLoading && page.episodes.length > 0

                        model: page.episodes

                        delegate: Item {
                            required property var modelData
                            required property int index

                            readonly property int epNumber: modelData.number !== undefined
                                ? modelData.number : (index + 1)
                            // An episode that has not aired yet. It is drawn as
                            // a date, not a play affordance: there is nothing
                            // to watch, and a card that does nothing on click
                            // reads as broken.
                            readonly property bool upcoming: modelData.airingAt !== undefined
                                && modelData.airingAt !== null
                            readonly property bool watched: !upcoming && page.watchedThrough > 0
                                && epNumber < page.watchedThrough
                            readonly property bool current: !upcoming && page.watchedThrough > 0
                                && epNumber === page.watchedThrough
                            readonly property bool downloaded: page.episodeIsDownloaded(epNumber)

                            width: 200
                            height: 134

                            Item {
                                anchors.fill: parent
                                scale: epHover.hovered ? 1.03 : 1.0

                                Behavior on scale {
                                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                                }

                                Rectangle {
                                    id: epCard
                                    anchors.fill: parent
                                    //radius: 10
                                    color: Theme.surfaceHover
                                    border.width: (page.highlightedIndex === index || current) ? 2 : 1
                                    border.color: page.highlightedIndex === index
                                        ? Theme.accent
                                        : (current
                                            ? Theme.accent
                                            : (epHover.hovered ? Theme.accent : Theme.borderSubtle))
                                    clip: true

                                    Image {
                                        id: thumbImg
                                        anchors.fill: parent
                                        source: modelData.thumbnail || ""
                                        visible: source != ""
                                        fillMode: Image.PreserveAspectCrop
                                        asynchronous: true
                                        opacity: watched ? 0.45 : 1.0

                                        Behavior on opacity {
                                            NumberAnimation { duration: 150 }
                                        }
                                    }

                                    Rectangle {
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.bottom: parent.bottom
                                        height: parent.height * 0.65
                                        gradient: Gradient {
                                            GradientStop { position: 0.0; color: "transparent" }
                                            GradientStop { position: 1.0; color: "#e6000000" }
                                        }
                                    }

                                    Rectangle {
                                        anchors.fill: parent
                                        visible: watched
                                        color: "#40000000"
                                    }

                                    Rectangle {
                                        visible: watched
                                        anchors.right: parent.right
                                        anchors.top: parent.top
                                        anchors.margins: 6
                                        width: 24
                                        height: 24
                                        //radius: 12
                                        color: "#cc0b0d12"
                                        border.width: 1
                                        border.color: Theme.borderSubtle

                                        PlayerIcon {
                                            anchors.centerIn: parent
                                            kind: "eye-off"
                                            color: Theme.inkMuted
                                            width: 13; height: 13
                                        }
                                    }

                                    Rectangle {
                                        visible: current
                                        anchors.right: parent.right
                                        anchors.top: parent.top
                                        anchors.margins: 6
                                        radius: 4
                                        color: Theme.accent
                                        implicitWidth: lastWatchedText.implicitWidth + 10
                                        implicitHeight: lastWatchedText.implicitHeight + 4

                                        Text {
                                            id: lastWatchedText
                                            anchors.centerIn: parent
                                            text: "LAST WATCHED"
                                            color: "white"
                                            font.pixelSize: 9
                                            font.bold: true
                                        }
                                    }

                                    Rectangle {
                                        visible: downloaded
                                        anchors.right: parent.right
                                        anchors.top: parent.top
                                        anchors.rightMargin: 6
                                        anchors.topMargin: 32
                                        radius: 4
                                        color: Theme.healthGreen
                                        implicitWidth: downloadedText.implicitWidth + 10
                                        implicitHeight: downloadedText.implicitHeight + 4

                                        Text {
                                            id: downloadedText
                                            anchors.centerIn: parent
                                            text: "DOWNLOADED"
                                            color: "#ffffff"
                                            font.pixelSize: 9
                                            font.bold: true
                                        }
                                    }

                                    Rectangle {
                                        anchors.left: parent.left
                                        anchors.top: parent.top
                                        anchors.margins: 6
                                        radius: 4
                                        color: "#99000000"
                                        implicitWidth: epBadgeText.implicitWidth + 10
                                        implicitHeight: epBadgeText.implicitHeight + 4

                                        Text {
                                            id: epBadgeText
                                            anchors.centerIn: parent
                                            text: "EP " + epNumber
                                            color: "white"
                                            font.pixelSize: 10
                                            font.bold: true
                                        }
                                    }

                                    Rectangle {
                                        visible: modelData.filler === true
                                        anchors.left: epBadgeText.parent.right
                                        anchors.leftMargin: 6
                                        anchors.top: parent.top
                                        anchors.topMargin: 6
                                        radius: 4
                                        color: "#d9e5484d"
                                        implicitWidth: fillerText.implicitWidth + 8
                                        implicitHeight: fillerText.implicitHeight + 4

                                        Text {
                                            id: fillerText
                                            anchors.centerIn: parent
                                            text: "FILLER"
                                            color: "white"
                                            font.pixelSize: 9
                                            font.bold: true
                                        }
                                    }

                                    // Never shown for an upcoming episode:
                                    // the play affordance promises a stream
                                    // that does not exist yet.
                                    Rectangle {
                                        visible: !upcoming
                                        anchors.centerIn: parent
                                        width: 36
                                        height: 36
                                        radius: 18
                                        color: Qt.rgba(0.07, 0.07, 0.11, 0.85)
                                        border.width: 1
                                        border.color: Theme.accent
                                        opacity: epHover.hovered ? 1.0 : 0.0
                                        scale: epHover.hovered ? 1.0 : 0.8

                                        Behavior on opacity { NumberAnimation { duration: 150 } }
                                        Behavior on scale { NumberAnimation { duration: 150 } }

                                        PlayerIcon {
                                            anchors.centerIn: parent
                                            kind: "play"
                                            color: "white"
                                            width: 16; height: 16
                                        }
                                    }

                                    Text {
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.bottom: parent.bottom
                                        anchors.margins: 8
                                        text: upcoming
                                            ? page.upcomingLabel(modelData)
                                            : page.episodeLabel(modelData, index)
                                        color: upcoming ? Theme.accent : "white"
                                        font.pixelSize: 11
                                        font.bold: true
                                        elide: Text.ElideRight
                                        maximumLineCount: 2
                                        wrapMode: Text.WordWrap
                                    }
                                }

                                HoverHandler { id: epHover }

                                // Inert for an upcoming episode: there is no
                                // release to search for yet, so a click would
                                // open a watch page that can only fail.
                                MouseArea {
                                    anchors.fill: parent
                                    enabled: !upcoming
                                    cursorShape: upcoming ? Qt.ArrowCursor : Qt.PointingHandCursor
                                    onClicked: page.triggerWatch(epNumber)
                                }
                            }
                        }
                    }

                    // Loading / empty state for the grid.
                    //
                    // Three distinct states, because they mean different things
                    // to the reader:
                    //   - fetching the catalogue -> "loading" (thinking fox)
                    //   - fetched, but nothing to show -> "sleeping"
                    //   - no anime loaded at all -> not rendered
                    //
                    // Before this branch existed the page synthesised cards
                    // first and repainted when the real data landed, so the
                    // reader saw placeholder episodes flash past.
                    Rectangle {
                        visible: page.anime !== null
                            && (page.episodesLoading || page.episodes.length === 0)
                        width: parent.width
                        implicitHeight: 260
                        //radius: 14
                        color: Theme.surfaceRaised
                        border.width: 1
                        border.color: Theme.borderSubtle

                        ErrorState {
                            anchors.fill: parent
                            anchors.margins: 16
                            kind: page.episodesLoading ? "loading" : "sleeping"
                            artWidth: 120
                            showText: true
                            title: page.episodesLoading
                                ? "Fetching episodes…"
                                : "Nothing here yet"
                            message: page.episodesLoading
                                ? "Looking up this show's episode list."
                                : "There's nothing to show right now. Check back in a little while."
                        }
                    }
                }

                // ===== Right column: trailer + related ======================
                Column {
                    width: 320
                    spacing: 24

                    // --- Trailer ---------------------------------------------
                    Column {
                        width: parent.width
                        spacing: 10
                        visible: page.anime !== null
                            && page.anime.trailer !== undefined
                            && page.anime.trailer !== null
                            && page.trailerWatchUrl(page.anime.trailer).length > 0

                        Text {
                            text: "Trailer"
                            color: Theme.ink
                            font.pixelSize: 18
                            font.bold: true
                        }

                        Rectangle {
                            width: parent.width
                            height: parent.width * 9 / 16
                            //radius: 10
                            color: Theme.surfaceHover
                            border.width: 1
                            border.color: trailerMouse.containsMouse ? Theme.accent : Theme.borderSubtle
                            clip: true

                            Image {
                                anchors.fill: parent
                                source: page.anime && page.anime.trailer
                                    ? (page.anime.trailer.thumbnail || "") : ""
                                visible: source != ""
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                            }

                            Text {
                                anchors.centerIn: parent
                                text: "▶"
                                color: "#e6ffffff"
                                font.pixelSize: 40
                            }

                            MouseArea {
                                id: trailerMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    const url = page.trailerWatchUrl(page.anime.trailer)
                                    if (url.length > 0) Qt.openUrlExternally(url)
                                }
                            }
                        }
                    }

                    // --- Related anime ---------------------------------------
                    Column {
                        width: parent.width
                        spacing: 10
                        visible: page.anime !== null
                            && page.anime.relations !== undefined
                            && page.anime.relations.length > 0

                        Text {
                            text: "Related Anime"
                            color: Theme.ink
                            font.pixelSize: 18
                            font.bold: true
                        }

                        ListView {
                            width: parent.width
                            height: Math.min(contentHeight, 280)
                            clip: true
                            spacing: 8
                            model: page.anime ? page.anime.relations : []

                            delegate: Rectangle {
                                required property var modelData
                                width: 320
                                height: 88
                                radius: 10
                                color: relMouse.containsMouse ? Theme.surfaceHover : "transparent"

                                Row {
                                    anchors.fill: parent
                                    anchors.margins: 8
                                    spacing: 12

                                    Rectangle {
                                        width: 48
                                        height: 72
                                        //radius: 6
                                        color: Theme.surfaceHover
                                        clip: true

                                        Image {
                                            anchors.fill: parent
                                            source: modelData.coverImage || ""
                                            visible: source != ""
                                            fillMode: Image.PreserveAspectCrop
                                            asynchronous: true
                                        }
                                    }

                                    Column {
                                        width: parent.width - 68
                                        anchors.verticalCenter: parent.verticalCenter
                                        spacing: 6

                                        Text {
                                            width: parent.width
                                            text: page.displayTitle(modelData)
                                            color: relMouse.containsMouse ? Theme.accent : Theme.ink
                                            font.pixelSize: 13
                                            font.bold: true
                                            elide: Text.ElideRight
                                            maximumLineCount: 2
                                            wrapMode: Text.WordWrap
                                        }

                                        Row {
                                            spacing: 6

                                            Rectangle {
                                                radius: 4
                                                color: Qt.alpha(Theme.accent, 0.2)
                                                implicitWidth: relBadgeText.implicitWidth + 12
                                                implicitHeight: relBadgeText.implicitHeight + 4

                                                Text {
                                                    id: relBadgeText
                                                    anchors.centerIn: parent
                                                    text: page.relationLabel(modelData.relationType)
                                                    color: Theme.accent
                                                    font.pixelSize: 10
                                                    font.bold: true
                                                }
                                            }

                                            Rectangle {
                                                visible: modelData.episodeCount != null
                                                radius: 4
                                                color: Theme.surfaceHover
                                                implicitWidth: relEpText.implicitWidth + 12
                                                implicitHeight: relEpText.implicitHeight + 4

                                                Text {
                                                    id: relEpText
                                                    anchors.centerIn: parent
                                                    text: modelData.episodeCount != null
                                                        ? String(modelData.episodeCount) : ""
                                                    color: Theme.inkMuted
                                                    font.pixelSize: 10
                                                }
                                            }

                                            Text {
                                                anchors.verticalCenter: parent.verticalCenter
                                                visible: modelData.format !== undefined
                                                    && modelData.format !== ""
                                                text: modelData.format || ""
                                                color: Theme.inkFaint
                                                font.pixelSize: 10
                                            }
                                        }
                                    }
                                }

                                MouseArea {
                                    id: relMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    // A relation carries the SAME provider as
                                    // the page it sits on: both sources keep
                                    // their relations in their own namespace.
                                    onClicked: page.openAnime(modelData.id, page.animeProvider)
                                }
                            }
                        }
                    }
                }
            }

            // --- Recommendations -------------------------------------------
            Column {
                width: parent.width
                visible: page.anime
                    && page.anime.recommendations
                    && page.anime.recommendations.length > 0
                spacing: 12
                topPadding: Theme.pad
                bottomPadding: Theme.pad

                RowLayout {
                    x: Theme.pad
                    width: parent.width - Theme.pad * 2

                    Text {
                        text: "Recommendations"
                        color: Theme.ink
                        font.pixelSize: 18
                        font.bold: true
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: "View more ›"
                        color: recMoreMouse.containsMouse ? Theme.accent : Theme.inkMuted
                        font.pixelSize: 12
                        MouseArea {
                            id: recMoreMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.openRecommendations(
                                page.animeId, page.displayTitle(page.anime))
                        }
                    }
                }

                ListView {
                    width: parent.width
                    height: 380
                    orientation: ListView.Horizontal
                    leftMargin: Theme.pad
                    rightMargin: Theme.pad
                    spacing: Theme.gap
                    clip: true
                    model: page.anime ? page.anime.recommendations : []

                    delegate: RecommendationCard {
                        required property var modelData
                        width: 180
                        mediaId: page.animeId
                        votable: true
                        recommendation: modelData
                        backend: page.backend
                        onOpenAnime: (id, provider) => page.openAnime(id, provider)
                    }
                }
            }
        }
    }

    Popup {
        id: statusPopup
        parent: statusButtonMouse
        x: 0
        y: statusButtonMouse.height + 8
        width: 200
        padding: 6
        modal: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: Theme.surfaceRaised
            radius: 12
            border.width: 1
            border.color: Theme.borderSubtle
        }

        contentItem: Column {
            spacing: 2

            Repeater {
                model: page.listStatuses

                delegate: Rectangle {
                    required property var modelData
                    width: statusPopup.width - 12
                    height: 32
                    radius: 8
                    color: itemMouse.containsMouse ? Theme.surfaceHover : "transparent"

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.label
                        color: page.listStatus === modelData.value ? Theme.accent : Theme.ink
                        font.pixelSize: 13
                    }

                    MouseArea {
                        id: itemMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            statusPopup.close()
                            page.chooseStatus(modelData.value)
                        }
                    }
                }
            }

            Rectangle {
                visible: page.listStatus !== ""
                width: statusPopup.width - 12
                height: 1
                color: Theme.borderSubtle
            }

            Rectangle {
                visible: page.listStatus !== ""
                width: statusPopup.width - 12
                height: 32
                radius: 8
                color: removeMouse.containsMouse ? Theme.surfaceHover : "transparent"

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Remove from list"
                    color: Theme.danger
                    font.pixelSize: 13
                }

                MouseArea {
                    id: removeMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        statusPopup.close()
                        page.removeFromList()
                    }
                }
            }
        }
    }

    function jumpToEpisode() {
        const wanted = jumpField.text.length > 0 ? Number(jumpField.text) : 0
        if (!Number.isInteger(wanted) || wanted < 1) {
            page.jumpError = ""
            return
        }

        let found = -1
        for (let i = 0; i < page.episodes.length; i++) {
            const n = page.episodes[i].number !== undefined
                ? page.episodes[i].number : (i + 1)
            if (n === wanted) { found = i; break }
        }

        if (found === -1) {
            page.jumpError = "No episode " + wanted + " in this list."
            return
        }

        page.jumpError = ""
        page.highlightedIndex = found
        episodeGrid.positionViewAtIndex(found, GridView.Center)
        highlightTimer.restart()
    }

    Timer {
        id: highlightTimer
        interval: 3000
        onTriggered: page.highlightedIndex = -1
    }
}