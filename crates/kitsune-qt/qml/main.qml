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
    }

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
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
        if (RetryState.active) {
            RetryState.reset()
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

    function openDetail(id) {
        root.navigate(detailComponent, { "animeId": id, "backend": root.backend })
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

            ctx.fillStyle = isReady ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.28) 
                                    : Qt.rgba(0.08, 0.10, 0.15, 0.95)

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
                    ctx.strokeStyle = isReady ? Theme.accent : "#FFFFFF"
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
                    ctx.strokeStyle = isReady ? Theme.accent : "#FFFFFF"
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
        onOpenAnime: (id) => root.openDetail(id)
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
        onOpenWatch: (animeId, episodeNumber) => {
            watchModal.animeId = animeId
            watchModal.malId = 0
            watchModal.episodeNumber = episodeNumber
            watchModal.absoluteEpisode = episodeNumber
            watchModal.titles = []
            watchModal.resumeLast = true
            watchModal.open()
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
        onOpenRequested: (animeId, episode) => {
            root.openDetail(animeId)
            watchModal.animeId = animeId
            watchModal.malId = 0
            watchModal.episodeNumber = episode
            watchModal.absoluteEpisode = episode
            watchModal.titles = []
            watchModal.resumeLast = true
            watchModal.open()
        }
    }

    RetryOverlay {
        id: retryOverlay
        anchors.fill: parent
        visible: RetryState.active
        z: 100
    }

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
            onBack: root.goBack()
            onOpenAnime: (id) => root.openDetail(id)
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
            onOpenAnime: (id) => root.openDetail(id)
        }
    }

    Component {
        id: recommendationsComponent
        RecommendationsPage {
            onBack: root.goBack()
            onOpenAnime: (id) => root.openDetail(id)
        }
    }

    Component {
        id: scheduleComponent
        SchedulePage {
            onOpenAnime: (id) => root.openDetail(id)
        }
    }

    Component {
        id: topComponent
        TopPage {
            onOpenAnime: (id) => root.openDetail(id)
        }
    }

    Component {
        id: myListComponent
        MyListPage {
            onOpenAnime: (id) => root.openDetail(id)
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
            watchModal.animeId = initialEpisodeAnime
            watchModal.episodeNumber = initialEpisodeNumber
            watchModal.absoluteEpisode = initialEpisodeNumber
            watchModal.open()
        } else if (initialWatchUrl.length > 0) {
            root.navigate(watchComponent, { "streamUrl": initialWatchUrl })
        }
    }
}