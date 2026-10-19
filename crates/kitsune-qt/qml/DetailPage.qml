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
    property var anime: null

    // The shared Backend, passed in explicitly by the window when it pushes this page.
    property var backend: null

    signal back()
    signal openAnime(int id)
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
    property var episodeInfo: []

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
        if (a.episodeCount != null) out.push({ label: "Episodes", value: a.episodeCount + " eps" })
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

    Connections {
        target: page.backend

        // A work the provider already had from a list response. Paints the
        // page immediately, with no request and no skeleton.
        //
        // Deliberately does NOT call loadEpisodes / loadListEntry /
        // RetryState.reset: those have side effects, and `getEpisodes` hits
        // Jikan, which is not cached -- running it twice would double the
        // traffic. The authoritative `animeReady` below owns all of them.
        function onAnimePartial(json) {
            const parsed = JSON.parse(json)
            if (parsed === null || parsed.id !== page.animeId) return
            // A full reply may already have landed; never downgrade it.
            if (page.anime === null) page.anime = parsed
        }

        function onAnimeReady(json) {
            const parsed = JSON.parse(json)
            if (parsed === null) {
                page.anime = null
            } else if (parsed.id === page.animeId) {
                page.anime = parsed
                page.loadEpisodes()
                page.loadListEntry()
                RetryState.reset()
            }
        }
        function onAnimeFailed(message) {
            RetryState.reportFailure(message)
        }
        function onEpisodesReady(json) {
            page.episodeInfo = JSON.parse(json) || []
            page.rebuildEpisodes()
        }
        function onEpisodesFailed(message) {
            page.episodeInfo = []
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
            page.loadListEntry()
        }
        function onWatchPositionsReady(json) {
            const all = JSON.parse(json) || ({})
            const entry = all[String(page.animeId)]
            page.localEpisode = (entry && entry.episode) ? entry.episode : 0
        }
        function onLibraryChanged() {
            if (page.backend) page.backend.getWatchPositions()
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

        const json = backend.buildEpisodes(
            anime.episodeCount || 0,
            anime.status || "",
            anime.coverImage || "",
            JSON.stringify(episodeInfo))
        page.episodes = JSON.parse(json) || []
        page.episodeOffset = computeOffset(page.episodes)
    }

    function loadEpisodes() {
        page.episodeInfo = []
        page.rebuildEpisodes()

        if (!anime || !anime.idMal) return

        page.episodesLoading = true
        backend.getEpisodes(anime.idMal)
    }

    function loadListEntry() {
        if (!backend) return
        backend.authStatus()
    }

    function fetchListEntry() {
        if (backend && page.signedIn && page.anime) {
            backend.getListEntry(page.anime.id)
        }
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
            page.signedIn = false
            page.listStatus = ""
            page.listEntryId = 0
            backend.getAnime(animeId)
        }
    }

    function reload() {
        if (animeId > 0 && backend) {
            page.anime = null
            backend.getAnime(animeId)
        }
    }

    onAnimeIdChanged: maybeLoad()
    onBackendChanged: maybeLoad()

    Timer {
        id: statusBusyTimer
        interval: 1500
        onTriggered: page.statusBusy = false
    }

    DetailSkeleton {
        anchors.fill: parent
        visible: page.anime === null
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

                Image {
                    anchors.fill: parent
                    source: page.anime ? (page.anime.bannerImage || page.anime.coverImage || "") : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    opacity: 0.25
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
                        radius: Theme.radius
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
                            visible: page.anime !== null && page.anime.description
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
                        spacing: 12

                        Text {
                            text: "Episodes"
                            color: Theme.ink
                            font.pixelSize: 18
                            font.bold: true
                        }

                        // Season packs button placed directly next to "Episodes" title
                        Rectangle {
                            visible: page.anime !== null && page.anime.episodeCount !== 0
                            implicitWidth: packsRow.implicitWidth + 24
                            height: 28
                            radius: 14
                            color: packsMouse.containsMouse ? Theme.surfaceHover : "transparent"
                            border.width: 1
                            border.color: packsMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                            Behavior on border.color { ColorAnimation { duration: 150 } }

                            Row {
                                id: packsRow
                                anchors.centerIn: parent
                                spacing: 6

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

                        Text {
                            visible: page.anime
                                && page.anime.episodeCount != null
                                && page.episodes.length > 0
                                && page.anime.episodeCount > page.episodes.length
                            text: page.episodes.length + " of " + (page.anime ? page.anime.episodeCount : 0) + " aired"
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

                        model: page.episodes

                        delegate: Item {
                            required property var modelData
                            required property int index

                            readonly property int epNumber: modelData.number !== undefined
                                ? modelData.number : (index + 1)
                            readonly property bool watched: page.watchedThrough > 0
                                && epNumber < page.watchedThrough
                            readonly property bool current: page.watchedThrough > 0
                                && epNumber === page.watchedThrough

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
                                    radius: 10
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
                                        radius: 12
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

                                    Rectangle {
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
                                        text: page.episodeLabel(modelData, index)
                                        color: "white"
                                        font.pixelSize: 11
                                        font.bold: true
                                        elide: Text.ElideRight
                                        maximumLineCount: 2
                                        wrapMode: Text.WordWrap
                                    }
                                }

                                HoverHandler { id: epHover }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: page.triggerWatch(epNumber)
                                }
                            }
                        }
                    }

                    Text {
                        visible: page.episodes.length === 0 && page.anime !== null
                        text: "Episode info unavailable"
                        color: Theme.inkFaint
                        font.pixelSize: 13
                        topPadding: 8
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
                            radius: 10
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
                                        radius: 6
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
                                                color: "#337c5cff"
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
                                    onClicked: page.openAnime(modelData.id)
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
                        onOpenAnime: (id) => page.openAnime(id)
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