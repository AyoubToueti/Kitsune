// Browse with filters: a search box, a sort menu, genre chips, and a results
// grid. Mirrors the Svelte `/filter` + `/search` routes.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    // The shared Backend, passed in by the window.
    property var backend: null
    // Optional genre to pre-select (from a genre chip on the detail page).
    property string initialGenre: ""
    // Optional search text to pre-fill.
    property string initialSearch: ""

    signal openAnime(int id)

    // Current query state.
    property string searchText: initialSearch
    property var selectedGenres: initialGenre.length > 0 ? [initialGenre] : []
    property string sort: "popularity"
    property int pageNumber: 1
    // Guards the infinite-scroll handler: without this, `atYEnd` is true the
    // moment the grid is shorter than the viewport, so it refetches forever,
    // clearing the model (and its in-flight cover images) on every pass.
    property bool loading: false

    ListModel { id: results }

    Connections {
        target: page.backend
        function onBrowseReady(json) {
            const pageObj = JSON.parse(json)
            const items = pageObj.items || []
            page.loading = false
            // Page 1 REPLACES (a new query); later pages APPEND.
            if (page.pageNumber === 1) {
                results.clear()
            }
            for (const item of items) {
                results.append({
                    "id": item.id,
                    "title": displayTitle(item),
                    "score": item.averageScore || 0,
                    "cover": item.coverImage || "",
                    "format": item.format || "",
                })
            }
            status.text = results.count + " results  ·  page " + page.pageNumber
        }
        function onBrowseFailed(message) {
            page.loading = false
            status.text = "Failed: " + message
        }
    }

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    // Rebuild the query and fetch. `q` is a JSON BrowseQuery matching the Rust
    // struct's camelCase wire shape.
    function runSearch(resetPage) {
        if (!backend || loading) return
        if (resetPage) pageNumber = 1
        loading = true
        const q = {
            "sort": sort,
            "genres": selectedGenres,
        }
        if (searchText.length > 0) q.search = searchText
        backend.getBrowse(JSON.stringify(q), pageNumber, 40)
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // --- Toolbar -------------------------------------------------------
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 64
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad
                spacing: 12

                Text {
                    text: "Browse"
                    color: Theme.accent
                    font.pixelSize: 24
                    font.bold: true
                }

                TextField {
                    id: searchField
                    Layout.preferredWidth: 320
                    placeholderText: "Search titles…"
                    text: page.searchText
                    color: Theme.ink
                    placeholderTextColor: Theme.inkFaint
                    background: Rectangle {
                        radius: 8
                        color: Theme.surfaceRaised
                        border.color: Theme.borderSubtle
                    }
                    onAccepted: {
                        page.searchText = text
                        page.runSearch(true)
                    }
                }

                ComboBox {
                    id: sortBox
                    Layout.preferredWidth: 160
                    model: [
                        { "label": "Popular", "value": "popularity" },
                        { "label": "Top rated", "value": "score" },
                        { "label": "Trending", "value": "trending" },
                        { "label": "Newest", "value": "newest" },
                        { "label": "A–Z", "value": "titleAz" },
                        { "label": "Favourites", "value": "favorites" },
                    ]
                    textRole: "label"
                    valueRole: "value"
                    onActivated: {
                        page.sort = currentValue
                        page.runSearch(true)
                    }
                }

                Text {
                    id: status
                    Layout.fillWidth: true
                    text: ""
                    color: Theme.inkFaint
                    font.pixelSize: 13
                    elide: Text.ElideRight
                }
            }
        }

        // --- Results grid --------------------------------------------------
        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: results
            cellWidth: 216
            cellHeight: 380
            leftMargin: Theme.pad
            topMargin: Theme.pad
            rightMargin: Theme.pad
            bottomMargin: Theme.pad

            delegate: PosterCard {
                width: 200
                title: model.title
                score: model.score
                cover: model.cover
                format: model.format
                onClicked: page.openAnime(model.id)
            }

            // Infinite scroll: only advance when there is real content to
            // scroll AND a page already rendered. An empty/short grid reports
            // `atYEnd` immediately, so the `count > 0` guard is what stops the
            // refetch loop that otherwise wipes the model on every pass.
            onAtYEndChanged: {
                if (atYEnd && !page.loading && results.count > 0) {
                    page.pageNumber += 1
                    page.runSearch(false)
                }
            }
        }
    }

    Component.onCompleted: runSearch(true)
}