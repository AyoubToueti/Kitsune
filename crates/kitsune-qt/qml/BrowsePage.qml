// The filter page, mirroring the Svelte /filter route: a "Filter anime" title,
// a toolbar with a Filters button (and active count) on the left and a
// reverse + sort group on the right, a bordered active-filters bar, and the
// paged results grid. The Filters dropdown holds Search/Type/Status/Score/
// Season/Year plus the Advanced Genre & Tag catalogue.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    // Which navbar pill this page lights up. Read by the window to derive the
    // active route, so re-opening this page does not replay the slide.
    readonly property string pageRoute: "filter"

    // The shared Backend, passed in by the window.
    property var backend: null
    // Optional genre to pre-select (from a genre chip on the detail page).
    property string initialGenre: ""
    // Optional search text to pre-fill (from the navbar search box).
    property string initialSearch: ""
    // Optional preset filters (from a home-page "View more" or a preset link).
    property string initialStatus: ""
    property string initialSort: "popularity"
    property bool initialReversed: false

    // `provider` is the id's namespace, forwarded so the detail page fetches
    // from the source that issued the id.
    signal openAnime(int id, string provider)

    // --- Filter state --------------------------------------------------------
    property string searchText: initialSearch
    property string sort: initialSort.length > 0 ? initialSort : "popularity"
    property bool reversed: initialReversed
    property string format: ""
    property string status: initialStatus
    property string season: ""
    property int seasonYear: 0
    property int minScore: 0
    // Selected genres (include only).
    property var genres: initialGenre.length > 0 ? [initialGenre] : []
    // tag name -> "include" | "exclude"
    property var tagStates: ({})

    // --- Catalogue + results -------------------------------------------------
    property var genreOptions: []
    property var tagGroups: []
    property bool filtersOpen: false
    property bool showCatalogue: false
    property string tagFilter: ""
    property bool loading: false
    property int pageNumber: 1

    // A LATER page failed. The grid stays usable and a retry row appears
    // beneath it; `loadErrorPage` remembers which page to ask for again, so a
    // retry never resets the reader back to the top.
    property string loadError: ""
    property int loadErrorPage: 0
    // Seconds left on a rate-limit lock for that retry row.
    property int retryIn: 0

    // A FIRST page failed: there is no grid to keep, so the page shows a
    // full-area error block with a retry that re-runs the same query.
    property bool loadFailed: false
    property string pageError: ""

    // Tick the inline lock down. Started only on a rate-limit failure; stopped
    // by `retryFailedPage` and on any successful page.
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
    property string resultStatus: ""

    ListModel { id: results }

    // --- Static option lists -------------------------------------------------
    readonly property var formatOptions: [
        { "value": "", "label": "All" },
        { "value": "tv", "label": "TV" },
        { "value": "movie", "label": "Movie" },
        { "value": "ova", "label": "OVA" },
        { "value": "ona", "label": "ONA" },
        { "value": "special", "label": "Special" },
        { "value": "music", "label": "Music" },
    ]
    readonly property var statusOptions: [
        { "value": "", "label": "All" },
        { "value": "releasing", "label": "Currently airing" },
        { "value": "finished", "label": "Finished airing" },
        { "value": "notYetReleased", "label": "Not yet aired" },
    ]
    readonly property var seasonOptions: [
        { "value": "", "label": "All" },
        { "value": "winter", "label": "Winter" },
        { "value": "spring", "label": "Spring" },
        { "value": "summer", "label": "Summer" },
        { "value": "fall", "label": "Fall" },
    ]
    readonly property var scoreOptions: [
        { "value": "0", "label": "All" },
        { "value": "50", "label": "50+" },
        { "value": "60", "label": "60+" },
        { "value": "70", "label": "70+" },
        { "value": "80", "label": "80+" },
        { "value": "90", "label": "90+" },
    ]
    readonly property var sortOptions: [
        { "value": "titleAz", "label": "Title" },
        { "value": "popularity", "label": "Popularity" },
        { "value": "score", "label": "Average Score" },
        { "value": "trending", "label": "Trending" },
        { "value": "favorites", "label": "Favorites" },
        { "value": "dateAdded", "label": "Date Added" },
        { "value": "newest", "label": "Release Date" },
    ]

    // Years newest-first, from next year back to 1960.
    readonly property var yearOptions: {
        const newest = new Date().getFullYear() + 1
        const out = [{ "value": "0", "label": "Any" }]
        for (let y = newest; y >= 1960; y--) out.push({ "value": String(y), "label": String(y) })
        return out
    }

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    // --- Query building ------------------------------------------------------

    function includedTags() {
        return Object.keys(tagStates).filter((name) => tagStates[name] === "include")
    }
    function excludedTags() {
        return Object.keys(tagStates).filter((name) => tagStates[name] === "exclude")
    }

    function buildQuery() {
        const q = { "sort": sort, "reversed": reversed }
        if (searchText.length > 0) q.search = searchText
        if (genres.length > 0) q.genres = genres
        const inc = includedTags()
        if (inc.length > 0) q.tags = inc
        const exc = excludedTags()
        if (exc.length > 0) q.excludedTags = exc
        if (format.length > 0) q.format = format
        if (status.length > 0) q.status = status
        if (season.length > 0) q.season = season
        if (seasonYear > 0) q.seasonYear = seasonYear
        if (minScore > 0) q.minScore = minScore
        return q
    }

    function runSearch(resetPage) {
        if (!backend || loading) return
        if (resetPage) pageNumber = 1
        loading = true
        backend.getBrowse(JSON.stringify(buildQuery()), pageNumber, 40)
    }

    // Re-run the current query from the first page. Called by the retry overlay,
    // which can now only appear for a page-1 failure.
    function reload() {
        results.clear()
        pageNumber = 1
        loading = false
        page.loadError = ""
        page.retryIn = 0
        page.loadFailed = false
        page.pageError = ""
        inlineRetryTimer.stop()
        runSearch(true)
    }

    // Re-request ONLY the page that failed, leaving the loaded pages alone.
    function retryFailedPage() {
        if (!backend || loading) return
        if (page.retryIn > 0) return
        if (page.loadErrorPage <= 0) return

        // `loadError` deliberately STAYS set: the row must remain visible and
        // switch to "Retrying…" rather than vanishing, so the footer does not
        // jump and the reader can see the retry is in flight. Success clears it
        // in `onBrowseReady`; another failure re-records it.
        page.retryIn = 0
        inlineRetryTimer.stop()
        page.loadError = "" // Clear error so state switches to LoadingMore
        page.stickToEnd = false
        page.loading = true
        backend.getBrowse(JSON.stringify(buildQuery()), page.loadErrorPage, 40)
    }

    // While true, any growth of the content keeps the view glued to the bottom,
    // so the footer's height animation can't leave the error card cut off.
    property bool stickToEnd: false

    function pinToEnd() {
        grid.contentY = Math.max(grid.originY,
            grid.originY + grid.contentHeight - grid.height + grid.bottomMargin)
    }

    function scrollToBottom() {
        stickToEnd = true
        Qt.callLater(pinToEnd)
    }

    function applyFilters() {
        runSearch(true)
    }

    // --- In-place navigation ---------------------------------------------
    function applySearch(term) {
        searchText = term
        applyFilters()
    }

    function applyGenre(genre) {
        resetFilters()
        genres = genre.length > 0 ? [genre] : []
        applyFilters()
    }

    function applyPreset(statusValue, sortValue, reversedValue) {
        resetFilters()
        if (statusValue && statusValue.length > 0) status = statusValue
        if (sortValue && sortValue.length > 0) sort = sortValue
        reversed = reversedValue === true
        applyFilters()
    }

    function resetFilters() {
        searchText = ""
        format = ""
        status = ""
        season = ""
        seasonYear = 0
        minScore = 0
        genres = []
        tagStates = ({})
        sort = "popularity"
        reversed = false
    }

    // --- Tag helpers ---------------------------------------------------------
    function cycleTag(name) {
        const next = Object.assign({}, tagStates)
        if (next[name] === undefined) next[name] = "include"
        else if (next[name] === "include") next[name] = "exclude"
        else delete next[name]
        tagStates = next
        applyFilters()
    }

    function categoryLabel(category) {
        const at = category.indexOf("-")
        if (at === -1) return category
        return category.slice(0, at) + " / " + category.slice(at + 1)
    }

    function groupTags(tags) {
        const groups = []
        const index = {}
        for (const tag of tags) {
            if (index[tag.category] === undefined) {
                index[tag.category] = groups.length
                groups.push({ "category": tag.category, "tags": [] })
            }
            groups[index[tag.category]].tags.push(tag)
        }
        return groups
    }

    // --- Active filters ------------------------------------------------------
    function activeFilters() {
        const list = []
        if (searchText.length > 0) list.push({ "label": "“" + searchText + "”", "excluded": false, "clear": "search" })
        for (const g of genres) list.push({ "label": g, "excluded": false, "clear": "genre:" + g })
        for (const t of includedTags()) list.push({ "label": t, "excluded": false, "clear": "tag:" + t })
        for (const t of excludedTags()) list.push({ "label": "Not " + t, "excluded": true, "clear": "exclude_tag:" + t })
        if (format.length > 0) list.push({ "label": format.toUpperCase(), "excluded": false, "clear": "format" })
        if (status.length > 0) list.push({ "label": status, "excluded": false, "clear": "status" })
        if (season.length > 0) list.push({ "label": season, "excluded": false, "clear": "season" })
        if (seasonYear > 0) list.push({ "label": String(seasonYear), "excluded": false, "clear": "year" })
        if (minScore > 0) list.push({ "label": "Score " + minScore + "+", "excluded": false, "clear": "score" })
        return list
    }

    function clearFilter(kind) {
        if (kind === "search") searchText = ""
        else if (kind === "format") format = ""
        else if (kind === "status") status = ""
        else if (kind === "season") season = ""
        else if (kind === "year") seasonYear = 0
        else if (kind === "score") minScore = 0
        else if (kind.startsWith("genre:")) genres = genres.filter((g) => g !== kind.slice(6))
        else if (kind.startsWith("tag:")) {
            const next = Object.assign({}, tagStates); delete next[kind.slice(4)]; tagStates = next
        } else if (kind.startsWith("exclude_tag:")) {
            const next = Object.assign({}, tagStates); delete next[kind.slice(12)]; tagStates = next
        }
        applyFilters()
    }

    function clearAll() {
        searchText = ""
        format = ""
        status = ""
        season = ""
        seasonYear = 0
        minScore = 0
        genres = []
        tagStates = ({})
        applyFilters()
    }

    // --- Backend wiring ------------------------------------------------------
    Connections {
        target: page.backend

        function onGenresReady(json) {
            page.genreOptions = JSON.parse(json) || []
        }
        function onTagsReady(json) {
            page.tagGroups = page.groupTags(JSON.parse(json) || [])
        }
        function onBrowseReady(json) {
            const pageObj = JSON.parse(json)
            const items = pageObj.items || []
            page.loading = false
            if (page.pageNumber === 1) results.clear()
            for (const item of items) {
                results.append({
                    "id": item.id,
                    "titlesJson": JSON.stringify(item.title || {}),
                    "score": item.averageScore || 0,
                    "cover": item.coverImage || "",
                    "format": item.format || "",
                    "status": item.status || "",
                })
            }
            page.resultStatus = results.count + " results"
            // A successful page clears the inline failure, whichever page it
            // was, and cancels any lock still ticking.
            page.loadError = ""
            page.retryIn = 0
            page.loadFailed = false
            page.pageError = ""
            inlineRetryTimer.stop()
            RetryState.reset()
        }
        function onBrowseFailed(message) {
            // A first-page failure has nothing to preserve, so the whole
            // results area becomes an error block. A LATER page must not take
            // the grid away.
            if (results.count === 0) {
                page.loading = false
                page.resultStatus = ""
                page.loadFailed = true
                page.pageError = message
                RetryState.reportFailure(message)
                return
            }

            page.loading = false
            page.loadError = message
            // The caller advanced `pageNumber` before the request, so it
            // already names the page that failed.
            page.loadErrorPage = page.pageNumber
            page.retryIn = RetryState.isRateLimit(message) ? 5 : 0
            if (page.retryIn > 0) inlineRetryTimer.restart()
            page.scrollToBottom()
        }
    }

    Component.onCompleted: {
        if (backend) {
            backend.getGenres()
            backend.getTags()
        }
        runSearch(true)
    }

    // --- Layout --------------------------------------------------------------
    Column {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        topPadding: Theme.pad
        spacing: 14

        Text {
            x: Theme.pad
            text: "Filter anime"
            color: Theme.ink
            font.pixelSize: 20
            font.bold: true
        }

        // --- Toolbar ------------------------------------------------------
        Item {
            x: Theme.pad
            width: parent.width - Theme.pad * 2
            height: 40

            Rectangle {
                id: filtersButton
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: filtersRow.implicitWidth + 28
                height: 40
                radius: 12
                color: Theme.surfaceRaised
                border.width: 1
                border.color: (page.filtersOpen || page.activeFilters().length > 0)
                    ? Theme.accent : Theme.borderSubtle

                Row {
                    id: filtersRow
                    anchors.centerIn: parent
                    spacing: 8

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "☰"
                        color: Theme.ink
                        font.pixelSize: 14
                        font.bold: true
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Filters"
                        color: Theme.ink
                        font.pixelSize: 13
                        font.bold: true
                    }
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: page.activeFilters().length > 0
                        width: 20
                        height: 20
                        radius: 10
                        color: Theme.accent
                        Text {
                            anchors.centerIn: parent
                            text: page.activeFilters().length
                            color: "white"
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: page.filtersOpen = !page.filtersOpen
                }
            }

            Text {
                anchors.left: filtersButton.right
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: page.resultStatus
                color: Theme.inkFaint
                font.pixelSize: 12
            }

            Rectangle {
                id: sortGroup
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                width: sortRow.implicitWidth + 10
                height: 40
                radius: 12
                color: Theme.surfaceRaised
                border.width: 1
                border.color: Theme.borderSubtle

                Row {
                    id: sortRow
                    anchors.centerIn: parent
                    spacing: 4

                    // Reverse toggle with flip animation
                    Rectangle {
                        width: 34
                        height: 32
                        radius: 9
                        color: reverseMouse.containsMouse ? Theme.surfaceHover : "transparent"

                        Text {
                            id: reverseIcon
                            anchors.centerIn: parent
                            text: "⇅"
                            color: page.reversed ? Theme.accent : Theme.inkMuted
                            font.pixelSize: 15
                            rotation: page.reversed ? 180 : 0

                            Behavior on rotation {
                                NumberAnimation {
                                    duration: 250
                                    easing.type: Easing.InOutQuad
                                }
                            }
                        }

                        MouseArea {
                            id: reverseMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: { 
                                page.reversed = !page.reversed
                                page.applyFilters() 
                            }
                        }
                    }

                    Rectangle {
                        width: 1
                        height: 18
                        anchors.verticalCenter: parent.verticalCenter
                        color: Theme.borderSubtle
                    }

                    ThemeSelect {
                        anchors.verticalCenter: parent.verticalCenter
                        preferredWidth: 180
                        borderless: true
                        accentText: true
                        options: page.sortOptions
                        value: page.sort
                        onSelected: (v) => { page.sort = v; page.applyFilters() }
                    }
                }
            }
        }

        // --- Active filters bar -------------------------------------------
        Rectangle {
            x: Theme.pad
            width: parent.width - Theme.pad * 2
            height: 46
            radius: 12
            visible: page.activeFilters().length > 0
            color: "transparent"
            border.width: 1
            border.color: Theme.borderSubtle

            Row {
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                spacing: 8

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Active"
                    color: Theme.inkFaint
                    font.pixelSize: 12
                }

                Repeater {
                    model: page.activeFilters()

                    delegate: Rectangle {
                        required property var modelData
                        anchors.verticalCenter: parent.verticalCenter
                        height: 26
                        radius: 13
                        width: pillLabel.implicitWidth + 22
                        color: Theme.accent

                        Text {
                            id: pillLabel
                            anchors.centerIn: parent
                            text: modelData.label + "  ×"
                            color: "white"
                            font.pixelSize: 11
                            font.bold: true
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.clearFilter(modelData.clear)
                        }
                    }
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: page.activeFilters().length > 1
                    text: "Clear all"
                    color: clearMouse.containsMouse ? Theme.accent : Theme.inkMuted
                    font.pixelSize: 12
                    MouseArea {
                        id: clearMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.clearAll()
                    }
                }
            }
        }
    }

    // --- Results grid ------------------------------------------------------
    // A first-page failure replaces the results area: there is no grid to
    // keep, so the reader retries the same query in place.
    ErrorState {
        anchors.top: header.bottom
        anchors.topMargin: Theme.gap
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        visible: page.loadFailed
        failureMessage: page.pageError
        retryAfter: page.retryIn
        onPrimaryClicked: page.reload()
    }

    PosterGridSkeleton {
        anchors.top: header.bottom
        anchors.topMargin: Theme.gap
        anchors.left: parent.left
        anchors.leftMargin: Theme.pad
        anchors.right: parent.right
        anchors.rightMargin: Theme.pad
        anchors.bottom: parent.bottom
        clip: true
        visible: page.loading && results.count === 0 && !page.loadFailed
        count: 12
        cellWidth: 200
        cellHeight: 350
    }

    GridView {
        id: grid
        anchors.top: header.bottom
        anchors.topMargin: Theme.gap
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        clip: true
        model: results
        cellWidth: 200
        cellHeight: 350
        leftMargin: Theme.pad
        rightMargin: Theme.pad
        bottomMargin: Theme.pad
        visible: !(page.loading && results.count === 0) && !page.loadFailed

        // Auto-scroll to show the error card as soon as it appears
        Connections {
            target: page
            function onLoadErrorChanged() {
                if (page.loadError !== "") page.scrollToBottom()
            }
        }

        // Follow the footer while it grows/animates.
        onContentHeightChanged: if (page.stickToEnd) page.pinToEnd()
        // The user took over scrolling: stop pinning.
        onMovementStarted: page.stickToEnd = false

        delegate: Item {
            width: grid.cellWidth
            height: grid.cellHeight

            PosterCard {
                anchors.centerIn: parent
                width: 180
                height: 336
                animeId: model.id
                title: TitlePreference.title(JSON.parse(model.titlesJson))
                score: model.score
                cover: model.cover
                format: model.format
                animeStatus: model.status
                onClicked: page.openAnime(model.id, model.provider || "anilist")
            }
        }

        onAtYEndChanged: {
            // `loadError === ""` is essential: a failed page leaves the
            // sentinel visible, so without this the failure would re-fire on
            // every scroll tick and hammer the rate limit.
            if (atYEnd && !page.loading && results.count > 0
                && page.loadError === "") {
                page.pageNumber += 1
                page.runSearch(false)
            }
        }

        // A failed LATER page retries here, under the loaded results, rather
        // than taking over the screen.
        // One footer for both states, mutually exclusive: the spinner while a
        // page is in flight, the retry row once it has failed. `loading` alone
        // was not enough before -- a later page set it with nothing rendered.
        footer: Item {
            readonly property bool showLoading: page.loading
                && results.count > 0 && page.loadError === ""

            width: GridView.view.width
            // Must equal the row heights: InlineRetry = 180, LoadingMore = 118.
            height: page.loadError !== "" ? 180 : 118

            Behavior on height {
                NumberAnimation { duration: 200; easing.type: Easing.InOutQuad }
            }

            LoadingMore {
                id: loadingRow
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                visible: parent.showLoading
            }

            InlineRetry {
                id: inlineRetry
                anchors.top: parent.top
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

    // --- Filters dropdown --------------------------------------------------
    Rectangle {
        id: filterPanel
        visible: page.filtersOpen
        anchors.top: header.bottom
        anchors.left: parent.left
        anchors.leftMargin: Theme.pad
        width: Math.min(920, parent.width - Theme.pad * 2)
        height: Math.min(implicitHeight, parent.height - header.height - 20)
        implicitHeight: panelColumn.implicitHeight + 32
        radius: 12
        z: 10
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.borderSubtle

        Flickable {
            anchors.fill: parent
            anchors.margins: 16
            contentWidth: width
            contentHeight: panelColumn.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            ColumnLayout {
                id: panelColumn
                width: parent.width
                spacing: 14

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    Text { Layout.preferredWidth: 170; text: "Type"; color: Theme.inkFaint; font.pixelSize: 11 }
                    Text { Layout.preferredWidth: 190; text: "Status"; color: Theme.inkFaint; font.pixelSize: 11 }
                    Text { Layout.preferredWidth: 150; text: "Score"; color: Theme.inkFaint; font.pixelSize: 11 }
                    Text { Layout.preferredWidth: 150; text: "Season"; color: Theme.inkFaint; font.pixelSize: 11 }
                    Text { Layout.preferredWidth: 150; text: "Year"; color: Theme.inkFaint; font.pixelSize: 11 }
                    Item { Layout.fillWidth: true }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    ThemeSelect { preferredWidth: 170; options: page.formatOptions; value: page.format
                        onSelected: (v) => { page.format = v; page.applyFilters() } }
                    ThemeSelect { preferredWidth: 190; options: page.statusOptions; value: page.status
                        onSelected: (v) => { page.status = v; page.applyFilters() } }
                    ThemeSelect { preferredWidth: 150; options: page.scoreOptions; value: String(page.minScore)
                        onSelected: (v) => { page.minScore = parseInt(v); page.applyFilters() } }
                    ThemeSelect { preferredWidth: 150; options: page.seasonOptions; value: page.season
                        onSelected: (v) => { page.season = v; page.applyFilters() } }
                    ThemeSelect { preferredWidth: 150; options: page.yearOptions; value: String(page.seasonYear)
                        onSelected: (v) => { page.seasonYear = parseInt(v); page.applyFilters() } }
                    Item { Layout.fillWidth: true }
                }

                Item {
                    Layout.fillWidth: true
                    Layout.preferredHeight: disclosureRow.implicitHeight + 8

                    RowLayout {
                        id: disclosureRow
                        anchors.fill: parent
                        spacing: 8

                        Text {
                            text: (page.showCatalogue ? "▾  " : "▸  ") + "Advanced Genre & Tag Filters"
                            color: disclosureMouse.containsMouse ? Theme.accent : Theme.inkMuted
                            font.pixelSize: 12
                            font.bold: true
                        }
                        Rectangle {
                            visible: (page.includedTags().length + page.excludedTags().length) > 0
                            width: 20; height: 20; radius: 10
                            color: Theme.accent
                            Text {
                                anchors.centerIn: parent
                                text: page.includedTags().length + page.excludedTags().length
                                color: "white"; font.pixelSize: 10; font.bold: true
                            }
                        }
                        Item { Layout.fillWidth: true }
                    }

                    MouseArea {
                        id: disclosureMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.showCatalogue = !page.showCatalogue
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: page.showCatalogue
                    spacing: 12

                    TextField {
                        Layout.preferredWidth: 300
                        placeholderText: "Type to narrow the list"
                        text: page.tagFilter
                        color: Theme.ink
                        placeholderTextColor: Theme.inkFaint
                        background: Rectangle {
                            radius: 18; color: Theme.surfaceHover; border.width: 1
                            border.color: Theme.borderSubtle
                        }
                        onTextChanged: page.tagFilter = text
                    }

                    Flickable {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.min(catalogueColumn.implicitHeight + 8, 360)
                        contentWidth: width
                        contentHeight: catalogueColumn.implicitHeight + 8
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds

                        ColumnLayout {
                            id: catalogueColumn
                            width: parent.width
                            spacing: 12

                            ColumnLayout {
                                Layout.fillWidth: true
                                visible: page.genreOptions.length > 0
                                spacing: 6
                                Text { text: "Genres"; color: Theme.inkMuted; font.pixelSize: 12; font.bold: true }
                                Flow {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: childrenRect.height
                                    spacing: 6
                                    Repeater {
                                        model: page.genreOptions
                                        delegate: Chip {
                                            required property string modelData
                                            text: modelData
                                            state: page.genres.indexOf(modelData) >= 0 ? "include" : "off"
                                            onClicked: {
                                                if (page.genres.indexOf(modelData) >= 0)
                                                    page.genres = page.genres.filter((g) => g !== modelData)
                                                else
                                                    page.genres = page.genres.concat([modelData])
                                                page.applyFilters()
                                            }
                                        }
                                    }
                                }
                            }

                            Repeater {
                                model: page.tagGroups
                                delegate: ColumnLayout {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: page.categoryLabel(modelData.category)
                                        color: Theme.inkMuted
                                        font.pixelSize: 12
                                        font.bold: true
                                    }
                                    Flow {
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: childrenRect.height
                                        spacing: 6
                                        Repeater {
                                            model: {
                                                const needle = page.tagFilter.trim().toLowerCase()
                                                if (needle === "") return modelData.tags
                                                return modelData.tags.filter((t) => t.name.toLowerCase().includes(needle))
                                            }
                                            delegate: Chip {
                                                required property var modelData
                                                text: modelData.name
                                                state: page.tagStates[modelData.name] || "off"
                                                onClicked: page.cycleTag(modelData.name)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}