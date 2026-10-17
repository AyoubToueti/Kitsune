import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Window {
    id: root
    width: 1280
    height: 800
    visible: true
    title: "Kitsune"
    color: Theme.surface
    // No `icon` here: a QML `Window` has no such property. The taskbar icon is
    // resolved from the window's app_id against a `.desktop` entry, which is
    // set by `kitsuneSetDesktopFileName` in main.rs.

    // Exposed so child pages can reach the same Backend instance.
    property alias backend: backend

    // Startup flags, set from Rust via engine.set_initial_properties so a
    // screen can be opened (and screenshotted) without a click.
    property int initialOpenId: 0
    property bool initialBrowse: false
    property bool initialSchedule: false
    property bool initialTop: false
    property bool initialMyList: false
    property bool initialSettings: false
    property string initialWatchUrl: ""
    property int initialEpisodeAnime: 0
    property int initialEpisodeNumber: 0
    // Dev aid: with this set, the watch modal picks the first release + file
    // on its own so the play stage can be screenshotted without clicking.
    property bool autoPlay: false

    // The Rust bridge. In QML a `#[qml_element]` QObject is instantiated by
    // its Rust type name.
    //
    // No signal handlers here: each page opens its own `Connections` to this
    // object, so a page that is not on screen never receives a result meant for
    // another (the home page's carousel and the top page both listen to lists,
    // and a shared handler could not tell them apart).
    Backend {
        id: backend
    }

    // The shared list-status cache the poster cards' quick-add controls read,
    // so one `userList()` fills every card on the page. Wired in the window's
    // own Component.onCompleted below.

    // Mirrors `displayTitle` in src/lib/types.ts.
    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // Which segmented navbar pill is active, so the indicator can follow.
    property string navRoute: ""
    // The anime id of the detail page on screen, or 0. The resume disc hides
    // on the work's own page, like the Svelte disc does.
    property int currentDetailId: 0

    // Re-derive the navbar's active route from whatever the stack is showing.
    //
    // Derived rather than set by `goTo`, because a detail page is pushed
    // DIRECTLY (not through `goTo`) and would otherwise leave `navRoute` stale
    // at the previous page's route -- so clicking Filter from a detail page
    // would wrongly think it was already on the filter page.
    function updateRoute() {
        const item = stack.currentItem
        root.navRoute = (item && item.pageRoute !== undefined) ? item.pageRoute : ""
        root.currentDetailId = (item && item.animeId !== undefined) ? item.animeId : 0
        // A navigation leaves the failed page behind, so any retry overlay it
        // raised must go with it -- otherwise the reader would see the previous
        // page's error over the new page.
        RetryState.reset()
    }

    // Push a page, unless that route is already the one showing.
    //
    // Re-pushing the same page replays the slide transition for no reason,
    // which reads as the page reloading. Pages opt in by declaring a
    // `pageRoute`; anything without one (`""` here) is always pushed, since
    // those are pushed by id and a second push is a genuine navigation.
    function goTo(component, route, props) {
        if (route !== "" && root.navRoute === route) {
            return
        }
        stack.push(component, props)
    }

    // Whether the filter page is the one currently showing.
    function onFilterPage() {
        const item = stack.currentItem
        return item !== null && item.pageRoute === "filter"
    }

    // Run `apply(item)` on the live filter page, or push a fresh one with
    // `props` when it is not showing. `goTo` refuses to re-push the same
    // route, so without this a search / genre / preset fired while already on
    // the filter page would silently do nothing.
    function openFilter(props, apply) {
        if (root.onFilterPage()) {
            apply(stack.currentItem)
        } else {
            stack.push(browseComponent, props)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        NavBar {
            Layout.fillWidth: true
            activeRoute: root.navRoute
            onOpenHome: {
                root.navRoute = ""
                stack.pop(null)
            }
            onOpenMyList: root.goTo(myListComponent, "list", { "backend": root.backend })
            onOpenFilter: root.goTo(browseComponent, "filter", { "backend": root.backend })
            onOpenSettings: settingsPopup.open()
            // Search works from any page: on the filter page the live instance
            // narrows in place, elsewhere a fresh page is pushed with the term.
            onSearch: (term) => root.openFilter(
                { "backend": root.backend, "initialSearch": term },
                (page) => page.applySearch(term))
        }

        // The navigation stack: Home -> Detail / Browse / ...
        StackView {
            id: stack
            Layout.fillWidth: true
            Layout.fillHeight: true
            initialItem: home

            // Keep the navbar's active pill in step with the visible page,
            // whatever pushed it (a navbar link, a card click, or a pop).
            onCurrentItemChanged: root.updateRoute()

            pushEnter: Transition { NumberAnimation { property: "x"; from: stack.width; to: 0; duration: 220; easing.type: Easing.OutCubic } }
            pushExit: Transition { NumberAnimation { property: "x"; from: 0; to: -stack.width * 0.3; duration: 220; easing.type: Easing.OutCubic } }
            popEnter: Transition { NumberAnimation { property: "x"; from: -stack.width * 0.3; to: 0; duration: 220; easing.type: Easing.OutCubic } }
            popExit: Transition { NumberAnimation { property: "x"; from: 0; to: stack.width; duration: 220; easing.type: Easing.OutCubic } }
        }
    }

    HomePage {
        id: home
        visible: false
        backend: root.backend
        onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
        onOpenBrowse: () => root.goTo(browseComponent, "filter", { "backend": root.backend })
        // A curated block's "View more" lands on the filter page preset to
        // that shelf, applying in place when the page is already showing.
        onOpenBrowseFiltered: (status, sort, reversed) => root.openFilter(
            {
                "backend": root.backend,
                "initialStatus": status,
                "initialSort": sort,
                "initialReversed": reversed
            },
            (page) => page.applyPreset(status, sort, reversed))
        onOpenSchedule: () => root.goTo(scheduleComponent, "", { "backend": root.backend })
        onOpenTop: () => root.goTo(topComponent, "", { "backend": root.backend })
        onOpenMyList: () => root.goTo(myListComponent, "list", { "backend": root.backend })
        onOpenSettings: settingsPopup.open()
        onOpenGenre: (genre) => root.openFilter(
            { "backend": root.backend, "initialGenre": genre },
            (page) => page.applyGenre(genre))
        // A Continue Watching resume opens the same modal the detail page uses,
        // at the saved episode. The modal fetches the work by id, so no malId
        // is needed here.
        onOpenWatch: (animeId, episodeNumber) => {
            watchModal.animeId = animeId
            watchModal.malId = 0
            watchModal.episodeNumber = episodeNumber
            watchModal.absoluteEpisode = episodeNumber
            watchModal.titles = []
            // Resume: play the remembered torrent/file directly, no picker.
            watchModal.resumeLast = true
            watchModal.open()
        }
    }

    // The floating resume disc, above the stack so it shows over any page.
    // Hidden while the player modal is open and on the work's own detail page.
    ResumeDisc {
        id: resumeDisc
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.rightMargin: 28
        anchors.bottomMargin: 45
        z: 50
        backend: root.backend
        allowed: !watchModal.opened
        currentDetailId: root.currentDetailId
        // The disc opens the work's detail page AND starts playback at the
        // saved episode, mirroring the Svelte disc (`/anime/{id}?ep=…`).
        onOpenRequested: (animeId, episode) => {
            stack.push(detailComponent, { "animeId": animeId, "backend": root.backend })
            watchModal.animeId = animeId
            watchModal.malId = 0
            watchModal.episodeNumber = episode
            watchModal.absoluteEpisode = episode
            watchModal.titles = []
            // Play the last torrent/file directly, no release picker.
            watchModal.resumeLast = true
            watchModal.open()
        }
    }

    // The retry overlay, over every page. Shown when a page reports a load
    // failure; its Retry button re-runs the current page via `performRetry`.
    RetryOverlay {
        id: retryOverlay
        anchors.fill: parent
        visible: RetryState.active
        z: 100
    }

    // Re-run whatever page is on screen. Each page that loads data exposes a
    // `reload()`; the overlay is only ever raised by such a page, so in
    // practice `currentItem` has one. Guarded anyway, and the overlay is
    // dismissed up front so a page that does not reload does not strand it.
    function performRetry() {
        const item = stack.currentItem
        RetryState.active = false
        if (item && typeof item.reload === "function") item.reload()
    }

    Connections {
        target: RetryState
        function onRetryRequested() { root.performRetry() }
    }

    Component {
        id: detailComponent
        DetailPage {
            onBack: stack.pop()
            onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
            // An episode click opens the watch modal OVER the detail page,
            // matching the web app: the reader keeps their place and closing
            // it returns exactly where they were.
            onOpenWatch: (animeId, malId, episodeNumber, absoluteEpisode) => {
                watchModal.animeId = animeId
                watchModal.malId = malId
                watchModal.episodeNumber = episodeNumber
                watchModal.absoluteEpisode = absoluteEpisode
                watchModal.titles = []
                watchModal.open()
            }
            onOpenGenre: (genre) => root.openFilter(
                { "backend": root.backend, "initialGenre": genre },
                (page) => page.applyGenre(genre))
            // "View more" on the inline recommendations row opens the full,
            // paged list for this work.
            onOpenRecommendations: (animeId, title) => stack.push(recommendationsComponent, {
                "animeId": animeId,
                "workTitle": title,
                "backend": root.backend
            })
        }
    }

    Component {
        id: browseComponent
        BrowsePage {
            onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
        }
    }

    // The full, paged recommendations list for one work, pushed from the
    // detail page's "View more".
    Component {
        id: recommendationsComponent
        RecommendationsPage {
            onBack: stack.pop()
            onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
        }
    }

    Component {
        id: scheduleComponent
        SchedulePage {
            onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
        }
    }

    Component {
        id: topComponent
        TopPage {
            onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
        }
    }

    Component {
        id: myListComponent
        MyListPage {
            onOpenAnime: (id) => stack.push(detailComponent, { "animeId": id, "backend": root.backend })
        }
    }

    // Settings is a modal popup over whatever page is showing, not a stack
    // page: opening it must not disturb the reader's place.
    SettingsPopup {
        id: settingsPopup
        backend: root.backend
    }

    // The episode watch flow: release picker -> files -> embedded player, as a
    // modal over the detail page.
    EpisodeWatchModal {
        id: watchModal
        backend: root.backend
        autoPlay: root.autoPlay
        // Fullscreen the real window, so the whole screen is the player and the
        // OS chrome (title bar) goes away. `showFullScreen()`/`showNormal()` are
        // METHODS on Window, not a writable property.
        onRequestFullscreen: (on) => {
            if (on) root.showFullScreen()
            else root.showNormal()
        }
    }

    // The standalone watch PAGE is kept for the KITSUNE_OPEN_WATCH debug path
    // (play a bare URL with no torrent behind it) and the KITSUNE_OPEN_EPISODE
    // smoke test. The normal path opens `watchModal` instead.
    Component {
        id: watchComponent
        WatchPage {
            onBack: stack.pop()
        }
    }

    Component.onCompleted: {
        // HomePage fetches its own trending in its own Component.onCompleted,
        // so there is no duplicate request here.

        // Start the shared list-status cache: it checks auth and, when signed
        // in, reads the whole list once so every poster card's quick-add knows
        // its status without its own lookup.
        ListStatusStore.setBackend(root.backend)

        if (initialOpenId > 0) {
            stack.push(detailComponent, { "animeId": initialOpenId, "backend": root.backend })
        } else if (initialBrowse) {
            stack.push(browseComponent, { "backend": root.backend })
        } else if (initialSchedule) {
            stack.push(scheduleComponent, { "backend": root.backend })
        } else if (initialTop) {
            stack.push(topComponent, { "backend": root.backend })
        } else if (initialMyList) {
            stack.push(myListComponent, { "backend": root.backend })
        } else if (initialSettings) {
            // A dev aid: KITSUNE_OPEN_SETTINGS=1 opens the popup on startup so
            // it can be screenshotted. Settings is not a stack route.
            settingsPopup.open()
        } else if (initialEpisodeAnime > 0) {
            // A dev aid: KITSUNE_OPEN_EPISODE=<id>:<n> opens the watch modal
            // on startup so it can be screenshotted without clicking.
            watchModal.animeId = initialEpisodeAnime
            watchModal.episodeNumber = initialEpisodeNumber
            watchModal.absoluteEpisode = initialEpisodeNumber
            watchModal.open()
        } else if (initialWatchUrl.length > 0) {
            stack.push(watchComponent, { "streamUrl": initialWatchUrl })
        }
    }
}