// A single title's detail page, mirroring the Svelte `/anime/[id]` page:
// hero backdrop + poster, a labelled metadata strip, an add-to-list menu,
// synopsis, an episode grid, a trailer card, a related-anime sidebar, and
// recommendations.
//
// The two columns are the biggest structural difference from the first Qt
// version: episodes on the left, trailer + relations in a fixed sidebar on the
// right, recommendations spanning the full width below.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    // The anime id to load. Setting it triggers the fetch.
    property int animeId: 0
    property var anime: null

    // The shared Backend, passed in explicitly by the window when it pushes
    // this page. Relying on `Window.window.backend` here is fragile: the
    // attached property does not always resolve for an item inside a StackView.
    property var backend: null

    signal back()
    signal openAnime(int id)
    // Emitted when an episode is chosen: pushes the watch page with the
    // numbers it needs to search a release and match a torrent file.
    signal openWatch(int animeId, int malId, int episodeNumber, int absoluteEpisode)
    // Emitted when a genre chip is clicked: opens the filter page for it.
    signal openGenre(string genre)
    // Emitted by the Recommendations "View more": opens the full, paged
    // recommendations list for this work.
    signal openRecommendations(int animeId, string title)

    // --- Episodes ---------------------------------------------------------

    // The episode list to render, from Jikan's catalogue (via `buildEpisodes`)
    // or synthesised from `episodeCount` when Jikan has nothing.
    property var episodes: []
    // How much to add to an episode's own number to get the work's absolute
    // number, for a later cour whose releases carry the running total.
    property int episodeOffset: 0
    property bool episodesLoading: false
    // Jikan's per-episode catalogue, or empty when there is none.
    property var episodeInfo: []

    // --- Jump-to-episode --------------------------------------------------
    property int jumpQuery: 0
    property string jumpError: ""
    property int highlightedIndex: -1

    // --- List status ------------------------------------------------------

    property bool signedIn: false
    // The reader's list status wire value ("current", ...), or "" when the
    // work is not on their list.
    property string listStatus: ""
    // The list entry's id, needed to remove it.
    property int listEntryId: 0
    property bool statusBusy: false
    property string statusError: ""

    // The five lists, in the order they are offered. Labels are the reader's
    // words; values are AniList's own enum members (the wire format).
    readonly property var listStatuses: [
        { value: "current", label: "Watching" },
        { value: "completed", label: "Completed" },
        { value: "paused", label: "On hold" },
        { value: "dropped", label: "Dropped" },
        { value: "planning", label: "Plan to watch" }
    ]

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // The labelled facts shown in the metadata strip. Mirrors `MetadataStrip`.
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

    // `SIDE_STORY` -> "Side Story". Generic so an unknown type still reads.
    function relationLabel(type) {
        if (!type) return ""
        return String(type).toLowerCase().split("_")
            .map(function (word) { return word.charAt(0).toUpperCase() + word.slice(1) })
            .join(" ")
    }

    // Where a trailer can be watched, or "" for a site we cannot link to.
    // Mirrors `trailerWatchUrl` in src/lib/trailer.ts.
    function trailerWatchUrl(t) {
        if (!t || !t.site || !t.id) return ""
        const site = String(t.site).toLowerCase()
        if (site === "youtube") return "https://www.youtube.com/watch?v=" + t.id
        if (site === "dailymotion") return "https://www.dailymotion.com/video/" + t.id
        return ""
    }

    // The caption under an episode thumbnail. A title that already announces
    // its own number is left alone, so "Episode 7" does not become
    // "Episode 7 - Episode 7".
    function episodeLabel(ep, index) {
        const number = ep.number !== undefined ? ep.number : index + 1
        const t = (ep.title || "").trim()
        if (t === "") return "Episode " + number
        if (/^ep(isode)?\.?\s*\d+/i.test(t)) return t
        return "Episode " + number + " - " + t
    }

    // The lowest number minus one, but only when it exceeds the list length
    // (a later cour). Mirrors `absoluteOffset` in the core.
    function computeOffset(list) {
        const numbers = list.map(function (e) { return e.number })
            .filter(function (n) { return n !== undefined })
        if (numbers.length === 0) return 0
        const lowest = Math.min.apply(null, numbers)
        return lowest > list.length ? lowest - 1 : 0
    }

    Connections {
        target: page.backend

        function onAnimeReady(json) {
            const parsed = JSON.parse(json)
            if (parsed === null) {
                page.anime = null
            } else if (parsed.id === page.animeId) {
                page.anime = parsed
                page.loadEpisodes()
                page.loadListEntry()
            }
        }
        function onEpisodesReady(json) {
            page.episodeInfo = JSON.parse(json) || []
            page.rebuildEpisodes()
        }
        function onEpisodesFailed(message) {
            // A failed catalogue is not fatal: the synthesised list stands.
            page.episodeInfo = []
            page.rebuildEpisodes()
        }

        function onAuthStatusReady(signedInNow) {
            page.signedIn = signedInNow
            // Fetch the entry for THIS work once the work is known. Does NOT
            // call authStatus() again -- that emitted this signal and would
            // recurse forever.
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
        }
        function onListChanged() {
            // Any successful write re-reads, so the button is never stale.
            page.loadListEntry()
        }
        function onAuthError(message) {
            page.statusError = message
        }
    }

    // Ask the core to build the display list from the work + catalogue. The
    // rules live in kitsune-core, so Qt and the web app cannot disagree.
    function rebuildEpisodes() {
        if (!backend || !anime) {
            page.episodes = []
            return
        }
        const json = backend.buildEpisodes(
            anime.episodeCount || 0,
            anime.status || "",
            anime.coverImage || "",
            JSON.stringify(episodeInfo))
        page.episodes = JSON.parse(json) || []
        page.episodeOffset = computeOffset(page.episodes)
    }

    function loadEpisodes() {
        // Build from the count first, so the grid is usable immediately even
        // while Jikan is in flight (and if it never answers).
        page.episodeInfo = []
        page.rebuildEpisodes()

        if (!anime || !anime.idMal) return

        // Jikan refines the list with real titles and air dates when it lands.
        page.episodesLoading = true
        backend.getEpisodes(anime.idMal)
    }

    // Re-check the token. The answer arrives via `authStatusReady`, which is
    // where the entry itself is fetched -- calling `getListEntry` here as well
    // would double-fetch on every load.
    function loadListEntry() {
        if (!backend) return
        backend.authStatus()
    }

    // Fetch this work's list entry. Safe to call repeatedly; it does NOT
    // re-check auth, so it cannot loop back through `authStatusReady`.
    function fetchListEntry() {
        if (backend && page.signedIn && page.anime) {
            backend.getListEntry(page.anime.id)
        }
    }

    function chooseStatus(value) {
        if (!backend || !page.anime) return
        const previous = page.listStatus
        page.listStatus = value          // optimistic
        page.statusBusy = true
        page.statusError = ""
        backend.setListEntry(page.anime.id, value, 0)
        // The backend answers with listChanged (success) or authError. Clear
        // busy either way after a moment so the button cannot stick.
        statusBusyTimer.restart()
        if (previous === value) return
    }

    function removeFromList() {
        if (!backend || page.listEntryId === 0) return
        const previousId = page.listEntryId
        page.listStatus = ""             // optimistic: off the list
        page.listEntryId = 0
        page.statusBusy = true
        page.statusError = ""
        backend.deleteListEntry(previousId)
        // A failure answers with authError; the button re-enables either way.
        statusBusyTimer.restart()
    }

    function signIn() {
        if (!backend) return
        const url = backend.beginLogin()
        if (url && url.length > 0) Qt.openUrlExternally(url)
    }

    // Start the fetch once BOTH `animeId` and `backend` are set. StackView
    // applies the initial properties in an unspecified order, so keying the
    // fetch off `animeId` alone would silently miss when `animeId` lands first
    // and `backend` is still null.
    function maybeLoad() {
        if (animeId > 0 && backend) {
            page.anime = null
            page.signedIn = false
            page.listStatus = ""
            page.listEntryId = 0
            backend.getAnime(animeId)
        }
    }

    onAnimeIdChanged: maybeLoad()
    onBackendChanged: maybeLoad()

    // Clears the busy flag a beat after a write, so a dropped signal cannot
    // leave the button disabled forever.
    Timer {
        id: statusBusyTimer
        interval: 1500
        onTriggered: page.statusBusy = false
    }

    // Whole-page scroll: the content is taller than the window.
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
            spacing: 0

            // --- Hero -------------------------------------------------------
            Rectangle {
                width: parent.width
                height: 360
                color: Theme.surface
                clip: true

                // Blurred backdrop.
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
                        GradientStop { position: 0.0; color: "#cc0b0d12" }
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

                        // Genre chips, each linking to the filter page.
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

                        // --- Action bar: add to list --------------------------
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.topMargin: 4
                            Layout.bottomMargin: 4
                            implicitHeight: 52
                            color: "transparent"

                            Rectangle {
                                anchors.left: parent.left
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.topMargin: 8
                                anchors.bottomMargin: 8
                                visible: true
                                width: statusRow.implicitWidth + 28
                                height: 40
                                radius: 20
                                color: statusButtonMouse.containsMouse && !page.statusBusy
                                    ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: statusButtonMouse.containsMouse && !page.statusBusy
                                    ? Theme.accent : Theme.borderSubtle

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
                                        if (!page.signedIn) {
                                            page.signIn()
                                        } else {
                                            statusPopup.open()
                                        }
                                    }
                                }
                            }
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

                            width: 200
                            height: 134

                            Rectangle {
                                id: epCard
                                anchors.fill: parent
                                radius: 10
                                color: Theme.surfaceHover
                                border.width: page.highlightedIndex === index ? 2 : 1
                                border.color: page.highlightedIndex === index
                                    ? Theme.accent
                                    : (epHover.hovered ? Theme.accent : Theme.borderSubtle)
                                clip: true

                                Image {
                                    anchors.fill: parent
                                    source: modelData.thumbnail || ""
                                    visible: source != ""
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                }

                                // A bottom gradient so the caption stays readable.
                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    height: parent.height * 0.6
                                    gradient: Gradient {
                                        GradientStop { position: 0.0; color: "transparent" }
                                        GradientStop { position: 1.0; color: "#d9000000" }
                                    }
                                }

                                // EP badge, top-left.
                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.top: parent.top
                                    anchors.margins: 6
                                    radius: 4
                                    color: "#99000000"
                                    implicitWidth: epBadgeText.implicitWidth + 12
                                    implicitHeight: epBadgeText.implicitHeight + 6

                                    Text {
                                        id: epBadgeText
                                        anchors.centerIn: parent
                                        text: "EP " + (modelData.number !== undefined
                                            ? modelData.number : (index + 1))
                                        color: "white"
                                        font.pixelSize: 10
                                        font.bold: true
                                    }
                                }

                                // FILLER flag, top-right.
                                Rectangle {
                                    visible: modelData.filler === true
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 6
                                    radius: 4
                                    color: "#d9e5484d"
                                    implicitWidth: fillerText.implicitWidth + 12
                                    implicitHeight: fillerText.implicitHeight + 6

                                    Text {
                                        id: fillerText
                                        anchors.centerIn: parent
                                        text: "FILLER"
                                        color: "white"
                                        font.pixelSize: 9
                                        font.bold: true
                                    }
                                }

                                Text {
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    anchors.margins: 6
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
                                onClicked: {
                                    const number = (modelData.number !== undefined
                                        ? modelData.number : (index + 1))
                                    const malId = page.anime && page.anime.idMal
                                        ? page.anime.idMal : 0
                                    page.openWatch(page.animeId, malId, number,
                                        number + page.episodeOffset)
                                }
                            }
                        }
                    }

                    // Shown when neither Jikan nor the count produced anything.
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

                    // Each card shows the recommended work's cover and score
                    // and the community vote tally, with its own quick-add.
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

    // --- Add-to-list menu -----------------------------------------------
    //
    // Anchored to the button so it follows it, and drawn in the window's
    // overlay so the Flickable's clipping cannot cut it off.
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

    // Jump to an episode number, scrolling it into view and flagging a miss.
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