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

    // Mirrors `displayTitle` in src/lib/types.ts.
    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // Which segmented navbar pill is active, so the indicator can follow.
    property string navRoute: ""

    // Re-derive the navbar's active route from whatever the stack is showing.
    //
    // Derived rather than set by `goTo`, because a detail page is pushed
    // DIRECTLY (not through `goTo`) and would otherwise leave `navRoute` stale
    // at the previous page's route -- so clicking Filter from a detail page
    // would wrongly think it was already on the filter page.
    function updateRoute() {
        const item = stack.currentItem
        root.navRoute = (item && item.pageRoute !== undefined) ? item.pageRoute : ""
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
            onSearch: (term) => root.goTo(browseComponent, "filter", {
                "backend": root.backend,
                "initialSearch": term
            })
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
        onOpenSchedule: () => root.goTo(scheduleComponent, "", { "backend": root.backend })
        onOpenTop: () => root.goTo(topComponent, "", { "backend": root.backend })
        onOpenMyList: () => root.goTo(myListComponent, "list", { "backend": root.backend })
        onOpenSettings: settingsPopup.open()
        onOpenGenre: (genre) => root.goTo(browseComponent, "filter", {
            "backend": root.backend,
            "initialGenre": genre
        })
        onOpenWatch: (animeId, episodeNumber) => stack.push(watchComponent, {
            "animeId": animeId,
            "episodeNumber": episodeNumber,
            "absoluteEpisode": episodeNumber,
            "backend": root.backend
        })
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
            onOpenGenre: (genre) => root.goTo(browseComponent, "filter", {
                "backend": root.backend,
                "initialGenre": genre
            })
        }
    }

    Component {
        id: browseComponent
        BrowsePage {
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