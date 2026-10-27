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

    // Startup flags, set from Rust via engine.set_initial_properties
    property int initialOpenId: 0
    property bool initialBrowse: false
    property bool initialSchedule: false
    property bool initialTop: false
    property bool initialMyList: false
    property bool initialSettings: false
    property string initialWatchUrl: ""
    property int initialEpisodeAnime: 0
    property int initialEpisodeNumber: 0
    property bool autoPlay: false

    Backend {
        id: backend
        Component.onCompleted: backend.initDeepLinkNotify()
    }

    // A sign-in forwarded by a SECOND launch over D-Bus.
    //
    // The bus service cannot reach this object, so it parks the URL in a
    // process-wide slot and the Backend drains it. Doing that on activation is
    // exactly the right moment: the reader has just come back from the browser
    // tab where they authorised, which is also when the compositor gives this
    // window focus again.
    //
    // `Component.onCompleted` also drains, because the forwarded link can land
    // while the app is still starting up.
    function drainPendingAuth() {
        if (backend && backend.takePendingAuthUrl()) {
            root.raise()
            root.requestActivate()
        }
    }

    // Auto-raise when a deep link arrives while the app is running but not
    // focused. The D-Bus service emits this signal immediately, so the window
    // activates without requiring manual focus.
    Connections {
        target: backend
        function onPendingAuthArrived() {
            root.drainPendingAuth()
        }
    }

    onActiveChanged: if (active) root.drainPendingAuth()

    // Delegates to `TitlePreference` so the reader's chosen language applies
    // here too. The wrapper stays because every call site already spells it
    // this way.
    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    // Which segmented navbar pill is active
    property string navRoute: ""
    property int currentDetailId: 0

    function updateRoute() {
        const item = stack.currentItem
        root.navRoute = (item && item.pageRoute !== undefined) ? item.pageRoute : ""
        root.currentDetailId = (item && item.animeId !== undefined) ? item.animeId : 0
        RetryState.reset()
    }

    // --- History ----------------------------------------------------------
    property var pageEntries: []
    property var forwardEntries: []

    readonly property bool canGoBack: stack.depth > 1
    readonly property bool canGoForward: forwardEntries.length > 0

    function navigate(component, props) {
        forwardEntries = []
        pageEntries = pageEntries.concat([{ "component": component, "props": props }])
        stack.push(component, props)
    }

    function goTo(component, route, props) {
        if (route !== "" && root.navRoute === route) {
            return
        }
        root.navigate(component, props)
    }

    function goBack() {
        if (watchModal.settingsOpen) {
            watchModal.settingsOpen = false
            return
        }
        if (watchModal.opened) {
            watchModal.close()
            return
        }
        if (settingsPopup.opened) {
            settingsPopup.close()
            return
        }
        if (stack.depth <= 1) return

        const leaving = pageEntries[pageEntries.length - 1]
        if (leaving) {
            forwardEntries = forwardEntries.concat([leaving])
            pageEntries = pageEntries.slice(0, -1)
        }
        stack.pop()
    }

    function goForward() {
        if (forwardEntries.length === 0) return
        const next = forwardEntries[forwardEntries.length - 1]
        forwardEntries = forwardEntries.slice(0, -1)
        pageEntries = pageEntries.concat([next])
        stack.push(next.component, next.props)
    }

    function goHome() {
        forwardEntries = []
        pageEntries = []
        root.navRoute = ""
        stack.pop(null)
    }

    function onFilterPage() {
        const item = stack.currentItem
        return item !== null && item.pageRoute === "filter"
    }

    function openFilter(props, apply) {
        if (root.onFilterPage()) {
            apply(stack.currentItem)
        } else {
            root.navigate(browseComponent, props)
        }
    }

    // `provider` is the id's namespace ("anilist" / "kitsu"). It is REQUIRED
    // rather than inferred: the detail must come from the source that issued
    // the id, and a cold partial cache would otherwise default to AniList.
    function openDetail(id, provider) {
        root.navigate(detailComponent, {
            "animeId": id,
            "animeProvider": provider || "anilist",
            "backend": root.backend
        })
    }

    // --- Global navigation input ------------------------------------------
    MouseArea {
        id: navMouse

        anchors.fill: parent
        z: -1
        acceptedButtons: Qt.RightButton | Qt.BackButton | Qt.ForwardButton
        hoverEnabled: false

        property real pressX: 0
        property real pressY: 0
        property bool dragging: false

        readonly property int dragThreshold: 70
        readonly property real maxSlope: 0.6

        onPressed: function (mouse) {
            if (mouse.button === Qt.RightButton) {
                pressX = mouse.x
                pressY = mouse.y
                dragging = false
            }
        }

        onPositionChanged: function (mouse) {
            if (!pressed || (mouse.buttons & Qt.RightButton) === 0) return

            const dx = mouse.x - pressX
            const dy = Math.abs(mouse.y - pressY)
            const absDx = Math.abs(dx)

            // Initial qualification check for gesture locking
            if (!dragging) {
                if (absDx >= 5 && dy <= absDx * maxSlope) {
                    dragging = true
                    cueCanvas.direction = dx > 0 ? "back" : "forward"
                    cueCanvas.active = true
                }
            }

            // Once locked in dragging state, track position smoothly
            if (dragging) {
                cueCanvas.touchY = mouse.y
                var rawPull = cueCanvas.direction === "back" ? (mouse.x - pressX) : (pressX - mouse.x)
                cueCanvas.pullDistance = Math.max(0, rawPull)
            }
        }

        onReleased: function (mouse) {
            if (mouse.button === Qt.RightButton) {
                if (dragging) {
                    if (cueCanvas.pullDistance >= dragThreshold) {
                        if (cueCanvas.direction === "back") root.goBack()
                        else root.goForward()
                    }
                    dragging = false
                    cueCanvas.active = false
                    cueCanvas.pullDistance = 0
                }
                return
            }
            if (mouse.button === Qt.BackButton) root.goBack()
            else if (mouse.button === Qt.ForwardButton) root.goForward()
        }

        onCanceled: {
            dragging = false
            cueCanvas.active = false
            cueCanvas.pullDistance = 0
        }
    }

    // Android-style Flat Fluid Edge Wave Cue
    Canvas {
        id: cueCanvas

        property string direction: "back"
        readonly property bool isBack: direction === "back"
        property real pullDistance: 0
        property real touchY: parent.height / 2
        property bool active: false

        anchors.fill: parent
        z: 200
        visible: active || pullDistance > 0.5

        // Smooth interpolation for both vertical movement and un-stretching
        Behavior on touchY {
            NumberAnimation { duration: 100; easing.type: Easing.OutCubic }
        }
        Behavior on pullDistance {
            NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
        }

        onPullDistanceChanged: requestPaint()
        onTouchYChanged: requestPaint()
        onDirectionChanged: requestPaint()
        onActiveChanged: {
            if (!active) pullDistance = 0
            requestPaint()
        }

        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()

            if (pullDistance <= 0.5) return

            var pull = Math.min(pullDistance, 110)
            var peakWidth = pull * 0.55
            var waveSpan = 460
            var yCenter = touchY
            var yTop = yCenter - (waveSpan / 2)
            var yBottom = yCenter + (waveSpan / 2)
            var isReady = pullDistance >= 70

            ctx.fillStyle = isReady
                ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.28)
                : Qt.rgba(Theme.surfaceRaised.r, Theme.surfaceRaised.g, Theme.surfaceRaised.b, 0.95)

            ctx.beginPath()

            if (isBack) {
                // Smooth C1-continuous cubic Bezier curve from left edge
                ctx.moveTo(0, yTop)
                ctx.bezierCurveTo(0, yTop + waveSpan * 0.22, peakWidth, yCenter - waveSpan * 0.18, peakWidth, yCenter)
                ctx.bezierCurveTo(peakWidth, yCenter + waveSpan * 0.18, 0, yBottom - waveSpan * 0.22, 0, yBottom)
                ctx.closePath()
                ctx.fill()

                // Only render arrow once wave has expanded past 25px, fading it in smoothly
                var arrowOpacity = Math.max(0.0, Math.min(1.0, (pullDistance - 25) / 15.0))

                if (arrowOpacity > 0) {
                    ctx.save()
                    ctx.globalAlpha = arrowOpacity
                    ctx.strokeStyle = isReady ? Theme.accent : Theme.ink
                    ctx.lineWidth = 3.6
                    ctx.lineCap = "round"
                    ctx.lineJoin = "round"

                    var arrowX = (peakWidth * 0.45) - 4
                    ctx.beginPath()
                    ctx.moveTo(arrowX + 8, yCenter - 8)
                    ctx.lineTo(arrowX, yCenter)
                    ctx.lineTo(arrowX + 8, yCenter + 8)
                    ctx.stroke()
                    ctx.restore()
                }
            } else {
                // Smooth C1-continuous cubic Bezier curve from right edge
                var bulgeR = width - peakWidth
                ctx.moveTo(width, yTop)
                ctx.bezierCurveTo(width, yTop + waveSpan * 0.22, bulgeR, yCenter - waveSpan * 0.18, bulgeR, yCenter)
                ctx.bezierCurveTo(bulgeR, yCenter + waveSpan * 0.18, width, yBottom - waveSpan * 0.22, width, yBottom)
                ctx.closePath()
                ctx.fill()

                var arrowOpacityR = Math.max(0.0, Math.min(1.0, (pullDistance - 25) / 15.0))

                if (arrowOpacityR > 0) {
                    ctx.save()
                    ctx.globalAlpha = arrowOpacityR
                    ctx.strokeStyle = isReady ? Theme.accent : Theme.ink
                    ctx.lineWidth = 3.6
                    ctx.lineCap = "round"
                    ctx.lineJoin = "round"

                    var arrowXR = width - (peakWidth * 0.45) + 4
                    ctx.beginPath()
                    ctx.moveTo(arrowXR - 8, yCenter - 8)
                    ctx.lineTo(arrowXR, yCenter)
                    ctx.lineTo(arrowXR - 8, yCenter + 8)
                    ctx.stroke()
                    ctx.restore()
                }
            }
        }
    }

    // Keyboard navigation
    Shortcut {
        sequences: ["Alt+Left"]
        onActivated: root.goBack()
    }
    Shortcut {
        sequences: ["Alt+Right"]
        onActivated: root.goForward()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        NavBar {
            Layout.fillWidth: true
            activeRoute: root.navRoute
            canGoBack: root.canGoBack
            canGoForward: root.canGoForward
            onOpenHome: root.goHome()
            onGoBack: root.goBack()
            onGoForward: root.goForward()
            onOpenMyList: root.goTo(myListComponent, "list", { "backend": root.backend })
            onOpenFilter: root.goTo(browseComponent, "filter", { "backend": root.backend })
            onOpenDownloads: root.goTo(downloadsComponent, "downloads", { "backend": root.backend })
            onOpenSettings: settingsPopup.open()
            onSearch: (term) => root.openFilter(
                { "backend": root.backend, "initialSearch": term },
                (page) => page.applySearch(term))
        }

        StackView {
            id: stack
            Layout.fillWidth: true
            Layout.fillHeight: true
            initialItem: home

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
        onOpenAnime: (id, provider) => root.openDetail(id, provider)
        onOpenBrowse: () => root.goTo(browseComponent, "filter", { "backend": root.backend })
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
        // A Continue Watching resume: play the remembered release for this
        // work rather than searching again.
        onOpenWatch: (animeId, episodeNumber) => {
            watchModal.openFor(animeId, 0, episodeNumber, episodeNumber, "episodes", true)
        }
    }

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
        onOpenRequested: (animeId, episode, provider) => {
            root.openDetail(animeId, provider)
            watchModal.openFor(animeId, 0, episode, episode, "episodes", true, provider)
        }
    }



    // Shown only while a card is being dragged into a list. It carries its own
    // z (300) so it sits above the page stack and the back/forward cue, while
    // Qt's `Overlay.overlay` -- where the settings popup and watch modal live
    // -- still draws above it.
    DragToStatus {
        id: dragToStatus
        anchors.fill: parent
        visible: DragState.active || DragState.toastVisible
    }





    // The reader's display preferences, pushed into their singletons.
    //
    // `settings_ready` fires on the startup `getSettings()` above AND after
    // every `setSettings`, so saving in the Settings popup updates every open
    // page without any extra plumbing: both are properties, and every binding
    // reads them through `title()` / `Theme.dark`.
    Connections {
        target: root.backend
        function onSettingsReady(json) {
            const s = JSON.parse(json)
            if (s === null) return
            TitlePreference.language = s.titleLanguage || "english"
            Theme.choice = s.theme || "system"
        }
    }

    Component {
        id: detailComponent
        DetailPage {
            onBack: root.goBack()
            onOpenAnime: (id, provider) => root.openDetail(id, provider)
            // An episode click always searches afresh: `openFor` clears the
            // previous list and, critically, leaves `resumeLast` false so this
            // cannot replay the last release from another work.
            onOpenWatch: (animeId, malId, episodeNumber, absoluteEpisode) => {
                // `animeProvider` resolves to this DetailPage's own property,
                // so the watch modal fetches from the same source as the page.
                watchModal.openFor(animeId, malId, episodeNumber, absoluteEpisode,
                    "episodes", false, animeProvider)
            }
            // Season packs are a work-level search, so no episode travels.
            onOpenPacks: (animeId, malId) => {
                watchModal.openFor(animeId, malId, 0, 0, "packs", false, animeProvider)
            }
            onOpenGenre: (genre) => root.openFilter(
                { "backend": root.backend, "initialGenre": genre },
                (page) => page.applyGenre(genre))
            onOpenRecommendations: (animeId, title) => root.navigate(recommendationsComponent, {
                "animeId": animeId,
                "workTitle": title,
                "backend": root.backend
            })
        }
    }

    Component {
        id: browseComponent
        BrowsePage {
            onOpenAnime: (id, provider) => root.openDetail(id, provider)
        }
    }

    Component {
        id: recommendationsComponent
        RecommendationsPage {
            onBack: root.goBack()
            onOpenAnime: (id, provider) => root.openDetail(id, provider)
        }
    }

    Component {
        id: scheduleComponent
        SchedulePage {
            onOpenAnime: (id, provider) => root.openDetail(id, provider)
        }
    }

    Component {
        id: topComponent
        TopPage {
            onOpenAnime: (id, provider) => root.openDetail(id, provider)
        }
    }

    Component {
        id: myListComponent
        MyListPage {
            onOpenAnime: (id, provider) => root.openDetail(id, provider)
        }
    }

    Component {
        id: downloadsComponent
        DownloadsPage {
            // Reuse the watch modal's player, but feed it this anime's
            // completed local files instead of a release search.
            onOpenDownloads: (payload) => watchModal.openDownloads(payload)
            onOpenAnime: (id, provider) => root.openDetail(id, provider)
        }
    }

    SettingsPopup {
        id: settingsPopup
        backend: root.backend
    }

    EpisodeWatchModal {
        id: watchModal
        backend: root.backend
        autoPlay: root.autoPlay
        onRequestFullscreen: (on) => {
            if (on) root.showFullScreen()
            else root.showNormal()
        }
    }

    Component {
        id: watchComponent
        WatchPage {
            onBack: root.goBack()
        }
    }

    Component.onCompleted: {
        ListStatusStore.setBackend(root.backend)

        // The speed test is app-wide: the Settings card and the watch modal
        // share one measurement rather than each running its own.
        SpeedTestStore.setBackend(root.backend)

        // Read the title-language preference once at startup. `settings_ready`
        // also fires after every save, so the handler below keeps it current.
        root.backend.getSettings()

        // Reattach downloads left running when the app closed. Every torrent
        // comes back PAUSED so a launch never silently resumes transfers.
        root.backend.restoreDownloads()

        // Reconcile the local list with AniList exactly once per launch.
        //
        // This replaced a sync inside `authStatus()`, which three pages call
        // on startup and the detail page calls on every list change -- one
        // pull each, chained, until the rate limit tripped. A signed-out
        // reader is a no-op inside the backend.
        root.backend.syncMediaList()

        // A forwarded sign-in may already be waiting if the second launch beat
        // the window to the slot.
        root.drainPendingAuth()

        if (initialOpenId > 0) {
            root.navigate(detailComponent, { "animeId": initialOpenId, "backend": root.backend })
        } else if (initialBrowse) {
            root.navigate(browseComponent, { "backend": root.backend })
        } else if (initialSchedule) {
            root.navigate(scheduleComponent, { "backend": root.backend })
        } else if (initialTop) {
            root.navigate(topComponent, { "backend": root.backend })
        } else if (initialMyList) {
            root.navigate(myListComponent, { "backend": root.backend })
        } else if (initialSettings) {
            settingsPopup.open()
        } else if (initialEpisodeAnime > 0) {
            // Dev aid only, but still goes through `openFor` so it cannot
            // inherit state from a previous open.
            watchModal.openFor(initialEpisodeAnime, 0, initialEpisodeNumber,
                initialEpisodeNumber, "episodes", false, "anilist")
        } else if (initialWatchUrl.length > 0) {
            root.navigate(watchComponent, { "streamUrl": initialWatchUrl })
        }
    }
}