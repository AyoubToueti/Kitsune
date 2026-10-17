// The full recommendations list for one work, paged as the reader scrolls.
// Pushed from the detail page's "View more". Mirrors the Svelte
// `/anime/[id]/recommendations` route: a heading with the work's title, a Back
// link, and a grid of RecommendationCards (score + votes + quick-add).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: page

    property var backend: null
    property int animeId: 0
    property string workTitle: ""

    signal back()
    signal openAnime(int id)

    readonly property int perPage: 24

    property int pageNumber: 1
    property bool loading: false
    property bool hasMore: true
    property string error: ""

    // The loaded recommendations, as a ListModel of `{ recJson }` rows.
    //
    // A ListModel (not a plain array) so appending a page emits an incremental
    // `rowsInserted` signal and the GridView KEEPS its scroll position.
    // Reassigning a plain array replaces the whole model, which resets the
    // view to the top -- the "scrolls me back up on the next page" bug.
    //
    // Each row stores the recommendation as a JSON STRING rather than the
    // object itself: a ListModel coerces a role holding a nested object to the
    // string "[object Object]", which made every card render "Untitled". A
    // string survives the round-trip; the delegate parses it back.
    ListModel { id: recModel }

    // An inline back chevron. URL-encoded, NOT base64: `Qt.btoa` is deprecated
    // in Qt 6 (its output differs from the standard Web API).
    function getBackArrowSvg() {
        var xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='white'><path d='M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z'/></svg>"
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    Connections {
        target: page.backend
        function onRecommendationsReady(id, json) {
            if (id !== page.animeId) return
            const pageObj = JSON.parse(json)
            const raw = (pageObj && pageObj.items) ? pageObj.items : []
            page.loading = false
            // A first page replaces; a later page appends. `append` (not a
            // model reassignment) is what keeps the scroll position.
            if (page.pageNumber === 1) recModel.clear()
            for (let i = 0; i < raw.length; i++) {
                recModel.append({ "recJson": JSON.stringify(raw[i]) })
            }
            const info = pageObj ? pageObj.pageInfo : null
            page.hasMore = info ? info.hasNextPage !== false : false
            RetryState.reset()
        }
        function onRecommendationsFailed(id, message) {
            if (id !== page.animeId) return
            page.loading = false
            page.error = message
            // Raise the retry overlay; the first-load skeleton stays up.
            RetryState.reportFailure(message)
        }
    }

    function loadPage(number) {
        if (!backend || loading || animeId <= 0) return
        if (number > 1 && !hasMore) return
        loading = true
        error = ""
        pageNumber = number
        backend.getRecommendations(animeId, number, perPage)
    }

    function maybeLoad() {
        if (animeId > 0 && backend) {
            recModel.clear()
            hasMore = true
            pageNumber = 1
            loadPage(1)
        }
    }

    // Re-run the first page. Called by the retry overlay.
    function reload() {
        recModel.clear()
        hasMore = true
        pageNumber = 1
        loading = false
        error = ""
        loadPage(1)
    }
    onAnimeIdChanged: maybeLoad()
    onBackendChanged: maybeLoad()

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Glassmorphic Header Bar
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 76

            Rectangle {
                anchors.fill: parent
                color: Qt.rgba(Theme.surface.r, Theme.surface.g, Theme.surface.b, 0.6)
                border.width: 1
                border.color: Theme.glassBorder
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad
                spacing: 16

                // Back Button Frame
                Rectangle {
                    Layout.preferredWidth: 38
                    Layout.preferredHeight: 38
                    radius: 12
                    color: backMouse.containsMouse ? Theme.surfaceHover : Qt.rgba(1, 1, 1, 0.04)
                    border.width: 1
                    border.color: backMouse.containsMouse ? Theme.accent : Theme.glassBorder
                    scale: backMouse.containsMouse ? 1.04 : 1.0

                    Behavior on scale { NumberAnimation { duration: 150 } }
                    Behavior on border.color { ColorAnimation { duration: 150 } }

                    Image {
                        anchors.centerIn: parent
                        width: 16
                        height: 16
                        source: page.getBackArrowSvg()
                        sourceSize.width: 32
                        sourceSize.height: 32
                        smooth: true
                    }

                    MouseArea {
                        id: backMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.back()
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        Layout.fillWidth: true
                        text: page.workTitle.length > 0
                            ? ("Recommended for " + page.workTitle)
                            : "Recommendations"
                        color: Theme.ink
                        font.pixelSize: 19
                        font.bold: true
                        elide: Text.ElideRight
                    }

                    Text {
                        text: page.error.length > 0
                            ? ("Failed: " + page.error)
                            : (recModel.count + " suggestions found")
                        color: page.error.length > 0 ? Theme.danger : Theme.inkMuted
                        font.pixelSize: 12
                    }
                }
            }
        }

        // First-load placeholder: shown until the first page of recommendations
        // arrives. Later pages append and keep the footer "Loading…", so a
        // skeleton would only jitter the scroll.
        PosterGridSkeleton {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Theme.pad
            Layout.rightMargin: Theme.pad
            Layout.topMargin: Theme.gap
            visible: page.loading && recModel.count === 0
            count: 8
            cellWidth: 200
            cellHeight: 356
        }

        // Results Grid
        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: recModel
            cellWidth: 200
            cellHeight: 356
            leftMargin: Theme.pad
            rightMargin: Theme.pad
            topMargin: Theme.gap
            bottomMargin: Theme.pad
            visible: !(page.loading && recModel.count === 0)

            delegate: RecommendationCard {
                required property string recJson
                width: 184
                mediaId: page.animeId
                votable: true
                // The row holds the recommendation as a JSON string; parse it
                // back into the object the card expects.
                recommendation: JSON.parse(recJson)
                backend: page.backend
                onOpenAnime: (id) => page.openAnime(id)
            }

            onAtYEndChanged: {
                if (atYEnd && page.hasMore) page.loadPage(page.pageNumber + 1)
            }

            footer: Item {
                width: GridView.view.width
                height: page.loading ? 52 : 0

                Row {
                    anchors.centerIn: parent
                    spacing: 8
                    visible: page.loading

                    Text {
                        text: "Loading recommendations\u2026"
                        color: Theme.inkMuted
                        font.pixelSize: 13
                        font.bold: true
                    }
                }
            }
        }

        // Empty State Banner
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 120
            visible: !page.loading && recModel.count === 0 && page.error.length === 0

            Rectangle {
                anchors.centerIn: parent
                implicitWidth: emptyText.implicitWidth + 32
                implicitHeight: 40
                radius: 20
                color: Qt.rgba(1, 1, 1, 0.04)
                border.width: 1
                border.color: Theme.glassBorder

                Text {
                    id: emptyText
                    anchors.centerIn: parent
                    text: "No recommendations found for this title yet."
                    color: Theme.inkMuted
                    font.pixelSize: 13
                }
            }
        }
    }
}