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

    readonly property int heroLimit: 5
    readonly property int trendingLimit: 20
    readonly property int blockLimit: 5

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
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
        }
        function onTaggedListFailed(tag, message) {
            // A failed block stays empty rather than taking the page down.
        }

        function onAuthStatusReady(signedInNow) {
            page.signedIn = signedInNow
            if (signedInNow) page.backend.continueWatching(12)
        }
        function onContinueWatchingReady(json) {
            page.continueItems = JSON.parse(json) || []
        }
    }

    Component.onCompleted: {
        if (!backend) return
        backend.getTrending(page.trendingLimit)
        backend.getListTagged("topAiring", "topAiring", page.blockLimit)
        backend.getListTagged("mostPopular", "mostPopular", page.blockLimit)
        backend.getListTagged("topRated", "topRated", page.blockLimit)
        backend.getListTagged("latestCompleted", "latestCompleted", page.blockLimit)
        backend.authStatus()
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
            HeroCarousel {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                items: page.heroItems
                onOpenAnime: (id) => page.openAnime(id)
            }

            // --- Trending rail ------------------------------------------
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
                        rank: index + page.heroLimit + 1
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

                    delegate: Item {
                        required property var modelData
                        width: 130
                        height: 230

                        Rectangle {
                            width: 130
                            height: 180
                            radius: Theme.radius
                            color: Theme.surfaceRaised
                            clip: true

                            Image {
                                anchors.fill: parent
                                source: modelData.anime ? (modelData.anime.coverImage || "") : ""
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                            }
                        }

                        Text {
                            anchors.top: parent.top
                            anchors.topMargin: 186
                            width: parent.width
                            text: page.displayTitle(modelData.anime)
                            color: Theme.inkMuted
                            font.pixelSize: 12
                            elide: Text.ElideRight
                            maximumLineCount: 2
                            wrapMode: Text.WordWrap
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.openAnime(modelData.anime.id)
                        }
                    }
                }
            }

            // --- Four curated blocks ------------------------------------
            Grid {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                columns: 4
                spacing: 16

                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Top airing"
                    items: page.topAiringItems
                    onOpenAnime: (id) => page.openAnime(id)
                    onViewMore: page.openTop()
                }
                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Most popular"
                    items: page.mostPopularItems
                    onOpenAnime: (id) => page.openAnime(id)
                    onViewMore: page.openBrowse()
                }
                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Top rated"
                    items: page.topRatedItems
                    onOpenAnime: (id) => page.openAnime(id)
                    onViewMore: page.openTop()
                }
                ListBlock {
                    width: (parent.width - 48) / 4
                    title: "Latest completed"
                    items: page.latestCompletedItems
                    onOpenAnime: (id) => page.openAnime(id)
                    onViewMore: page.openBrowse()
                }
            }

            // --- Secondary links ----------------------------------------
            Row {
                x: Theme.pad
                spacing: 10

                NavButton {
                    label: "Top Rated"
                    onClicked: page.openTop()
                }
                NavButton {
                    label: "Schedule"
                    onClicked: page.openSchedule()
                }
            }
        }
    }
}