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

    // The loaded recommendations, as a plain JS array of `RecommendedAnime`.
    //
    // Deliberately NOT a `ListModel`: a ListModel coerces a role holding a
    // nested object to a string, so `modelData.rec.anime` came back undefined
    // and every card rendered "Untitled". A plain array keeps the objects
    // intact and the delegate receives each one whole.
    property var items: []

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
            // A first page replaces; a later page appends. Reassigning the
            // array (rather than mutating it) is what makes the GridView see
            // the change.
            const next = page.pageNumber === 1 ? [] : page.items.slice()
            for (let i = 0; i < raw.length; i++) next.push(raw[i])
            page.items = next
            const info = pageObj ? pageObj.pageInfo : null
            page.hasMore = info ? info.hasNextPage !== false : false
        }
        function onRecommendationsFailed(id, message) {
            if (id !== page.animeId) return
            page.loading = false
            page.error = message
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
            items = []
            hasMore = true
            pageNumber = 1
            loadPage(1)
        }
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
                            : (page.items.length + " suggestions found")
                        color: page.error.length > 0 ? Theme.danger : Theme.inkMuted
                        font.pixelSize: 12
                    }
                }
            }
        }

        // Results Grid
        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: page.items
            cellWidth: 200
            cellHeight: 356
            leftMargin: Theme.pad
            rightMargin: Theme.pad
            topMargin: Theme.gap
            bottomMargin: Theme.pad

            delegate: RecommendationCard {
                required property var modelData
                width: 184
                mediaId: page.animeId
                votable: true
                // `modelData` IS the `RecommendedAnime` here (a plain array
                // model), not a role wrapper.
                recommendation: modelData
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
            visible: !page.loading && page.items.length === 0 && page.error.length === 0

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