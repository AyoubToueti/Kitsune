// A single title's detail page: hero backdrop, poster, metadata, genres,
// synopsis, recommendations. Mirrors the Svelte `/anime/[id]` page's top half.
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

    // The episode list to render, from Jikan's catalogue (via `buildEpisodes`)
    // or synthesised from `episodeCount` when Jikan has nothing.
    property var episodes: []
    // How much to add to an episode's own number to get the work's absolute
    // number, for a later cour whose releases carry the running total.
    property int episodeOffset: 0
    property bool episodesLoading: false

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
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
            }
        }
        function onEpisodesReady(json) {
            // Remember the catalogue, then ask the core to combine it with the
            // work into the exact list to render (synthesis fallback included).
            page.episodeInfo = JSON.parse(json) || []
            page.rebuildEpisodes()
        }
        function onEpisodesFailed(message) {
            // A failed catalogue is not fatal: fall back to a synthesised list.
            page.episodeInfo = []
            page.rebuildEpisodes()
        }
    }

    // Jikan's per-episode catalogue, or empty when there is none.
    property var episodeInfo: []

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

    // The lowest number minus one, but only when it exceeds the list length
    // (a later cour). Mirrors `absoluteOffset` in the core.
    function computeOffset(list) {
        const numbers = list.map((e) => e.number).filter((n) => n !== undefined)
        if (numbers.length === 0) return 0
        const lowest = Math.min.apply(null, numbers)
        return lowest > list.length ? lowest - 1 : 0
    }

    function loadEpisodes() {
        // Build from the count first, so the grid is usable immediately even
        // while Jikan is in flight (and if it never answers).
        page.episodeInfo = []
        page.rebuildEpisodes()

        if (!anime || !anime.idMal) {
            return
        }
        // Jikan refines the list with real titles and air dates when it lands.
        page.episodesLoading = true
        backend.getEpisodes(anime.idMal)
    }

    // Start the fetch once BOTH `animeId` and `backend` are set. StackView
    // applies the initial properties in an unspecified order, so keying the
    // fetch off `animeId` alone would silently miss when `animeId` lands first
    // and `backend` is still null.
    function maybeLoad() {
        if (animeId > 0 && backend) {
            page.anime = null
            backend.getAnime(animeId)
        }
    }

    onAnimeIdChanged: maybeLoad()
    onBackendChanged: maybeLoad()

    // Whole-page scroll: the content is taller than the window.
    Flickable {
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
                height: 340
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
                        Layout.preferredWidth: 180
                        Layout.preferredHeight: 270
                        Layout.alignment: Qt.AlignVCenter
                        radius: Theme.radius
                        color: Theme.surfaceRaised
                        clip: true

                        Image {
                            anchors.fill: parent
                            source: page.anime ? (page.anime.coverImage || "") : ""
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            onStatusChanged: {
                                if (status === Image.Error) {
                                    console.log("poster image failed:", source)
                                }
                            }
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
                            font.pixelSize: 32
                            font.bold: true
                            wrapMode: Text.WordWrap
                        }

                        // Meta facts line
                        Text {
                            Layout.fillWidth: true
                            visible: page.anime !== null
                            color: Theme.inkMuted
                            font.pixelSize: 14
                            text: {
                                if (!page.anime) return ""
                                const parts = []
                                if (page.anime.format) parts.push(page.anime.format)
                                if (page.anime.seasonYear) {
                                    parts.push((page.anime.season || "") + " " + page.anime.seasonYear)
                                }
                                if (page.anime.episodeCount) parts.push(page.anime.episodeCount + " eps")
                                if (page.anime.durationMinutes) parts.push(page.anime.durationMinutes + " min")
                                if (page.anime.averageScore) parts.push("★ " + page.anime.averageScore)
                                return parts.join("  ·  ")
                            }
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
                                    color: Theme.surfaceRaised
                                    border.color: Theme.borderSubtle
                                    implicitWidth: genreLabel.implicitWidth + 18
                                    implicitHeight: 28
                                    Text {
                                        id: genreLabel
                                        anchors.centerIn: parent
                                        text: modelData
                                        color: Theme.inkMuted
                                        font.pixelSize: 12
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // --- Episodes ---------------------------------------------------
            //
            // The gateway to playback: picking an episode pushes the watch page
            // with the numbers it needs to search a release and match a file.
            Column {
                width: parent.width
                visible: page.episodes.length > 0
                spacing: 12
                topPadding: Theme.pad
                leftPadding: Theme.pad
                rightPadding: Theme.pad

                Text {
                    text: "Episodes"
                    color: Theme.ink
                    font.pixelSize: 18
                    font.bold: true
                }

                GridView {
                    width: parent.width - Theme.pad * 2
                    height: 220
                    cellWidth: 200
                    cellHeight: 64
                    clip: true
                    model: page.episodes

                    delegate: Rectangle {
                        required property var modelData
                        required property int index

                        width: 188
                        height: 56
                        radius: Theme.radius
                        color: episodeMouse.containsMouse ? Theme.surfaceRaised : Theme.surface
                        border.color: episodeMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.leftMargin: 12
                            anchors.right: parent.right
                            anchors.rightMargin: 12
                            spacing: 2

                            Text {
                                text: "Episode " + (modelData.number !== undefined
                                    ? modelData.number
                                    : (index + 1))
                                color: Theme.ink
                                font.pixelSize: 14
                                font.bold: true
                            }
                            Text {
                                width: parent.width
                                // A synthesised title is just "Episode N", which
                                // the line above already shows; only the real
                                // catalogue titles add anything.
                                text: {
                                    const t = modelData.title || ""
                                    const number = modelData.number !== undefined
                                        ? modelData.number
                                        : (index + 1)
                                    return t === ("Episode " + number) ? "" : t
                                }
                                color: Theme.inkMuted
                                font.pixelSize: 12
                                elide: Text.ElideRight
                            }
                        }

                        MouseArea {
                            id: episodeMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                const number = (modelData.number !== undefined
                                    ? modelData.number
                                    : (index + 1))
                                const malId = page.anime && page.anime.idMal ? page.anime.idMal : 0
                                page.openWatch(page.animeId, malId, number, number + page.episodeOffset)
                            }
                        }
                    }
                }
            }

            // --- Synopsis ---------------------------------------------------
            Column {
                width: parent.width
                visible: page.anime && page.anime.description
                spacing: 8
                topPadding: Theme.pad
                leftPadding: Theme.pad
                rightPadding: Theme.pad

                Text {
                    text: "Synopsis"
                    color: Theme.ink
                    font.pixelSize: 18
                    font.bold: true
                }
                Text {
                    width: parent.width - Theme.pad * 2
                    // AniList descriptions are HTML; strip the tags for a plain
                    // render rather than shipping a full HTML view.
                    text: page.anime
                        ? String(page.anime.description).replace(/<[^>]*>/g, "")
                        : ""
                    color: Theme.inkMuted
                    font.pixelSize: 14
                    wrapMode: Text.WordWrap
                    lineHeight: 1.4
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

                Text {
                    x: Theme.pad
                    text: "Recommendations"
                    color: Theme.ink
                    font.pixelSize: 18
                    font.bold: true
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

                    // A recommendation wraps the work under `anime`; the card
                    // shows the WORK's cover/score, not the vote tally.
                    delegate: PosterCard {
                        width: 180
                        readonly property var work: modelData.anime || ({})
                        title: work.title
                            ? (work.title.userPreferred
                                || work.title.english
                                || work.title.romaji
                                || "(untitled)")
                            : "(untitled)"
                        score: work.averageScore || 0
                        cover: work.coverImage || ""
                        format: work.format || ""
                        onClicked: page.openAnime(work.id)
                    }
                }
            }
        }
    }
}