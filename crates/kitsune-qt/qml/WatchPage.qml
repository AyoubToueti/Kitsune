// The watch page: pick a release, pick a file, play it in the embedded libmpv.
//
// Three stages, mirroring the web app's flow:
//   1. releases -- search the indexers for this work + episode.
//   2. files    -- add the chosen magnet and pick which file is the episode.
//   3. play     -- resolve the loopback stream URL and feed it to mpv.
//
// The player is a real QQuickFramebufferObject (MpvObject, C++), so libass
// draws ASS/SSA subtitles natively -- the reason playback moved out of the
// webview.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: page

    // What to play. The numbers come from the detail page's episode grid.
    property int animeId: 0
    property int malId: 0
    property int episodeNumber: 0
    // The work-wide episode number, when it differs from `episodeNumber`.
    property int absoluteEpisode: 0
    // The shared Backend, passed in by the window.
    property var backend: null

    // Debug fallback: play a bare URL with no torrent behind it.
    property string streamUrl: ""

    signal back()

    // --- state ---------------------------------------------------------------

    // "releases", "files" or "play".
    property string stage: "releases"
    property var releases: []
    property bool searching: false
    property string error: ""

    // The chosen release's magnet, for reference in the header.
    property string releaseTitle: ""
    property int torrentId: -1
    property var files: []
    property int chosenIdx: -1
    property string playingUrl: ""

    property var progress: null
    property bool paused: false

    // The title forms to search, derived from the work once it loads.
    property var titles: []

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    Connections {
        target: page.backend

        function onAnimeReady(json) {
            const parsed = JSON.parse(json)
            if (parsed === null || parsed.id !== page.animeId) return
            const t = parsed.title || {}
            page.titles = JSON.parse(page.backend.titleForms(
                t.userPreferred || "", t.english || "", t.romaji || ""))
            page.search()
        }

        function onReleasesReady(json) {
            page.searching = false
            page.releases = JSON.parse(json) || []
            if (page.releases.length === 0) {
                page.error = "No releases found for this episode."
            }
        }

        function onReleasesFailed(message) {
            page.searching = false
            page.error = message
        }

        function onTorrentAdded(json) {
            const handle = JSON.parse(json)
            if (handle === null) return
            page.torrentId = handle.id
            page.files = handle.files || []
            page.stage = "files"
            // Auto-pick: the episode match, or the sole video. `-1` waits.
            const picked = page.backend.pickFile(
                JSON.stringify(page.files), page.episodeNumber, page.offset())
            if (picked >= 0) {
                page.play(picked)
            }
        }

        function onTorrentAddFailed(message) {
            page.error = message
            page.searching = false
        }

        function onStreamUrlReady(torrentId, fileIdx, url) {
            if (torrentId !== page.torrentId || fileIdx !== page.chosenIdx) return
            page.playingUrl = url
            page.stage = "play"
            // Hold BEFORE loading, so leaving the page cannot remove the torrent
            // out from under the player that is reading it.
            page.backend.holdTorrent(page.torrentId)
            player.command(["loadfile", url, "replace"])
        }

        function onStreamUrlFailed(message) {
            page.error = message
        }

        function onTorrentStatsReady(json) {
            page.progress = JSON.parse(json)
        }
    }

    // The offset from an episode's own number to the work's absolute number.
    // Mirrors `absoluteOffset` in the core; here it is only non-zero for a
    // later cour, which the detail page already folded into absoluteEpisode.
    function offset() {
        return Math.max(0, page.absoluteEpisode - page.episodeNumber)
    }

    function search() {
        if (!backend || titles.length === 0) {
            page.error = "No title to search with."
            return
        }
        page.searching = true
        page.error = ""
        backend.searchReleases(
            JSON.stringify(titles),
            page.episodeNumber,
            page.absoluteEpisode,
            "episodes")
    }

    function pickRelease(release) {
        page.releaseTitle = release.title || ""
        page.error = ""
        page.searching = true
        backend.addMagnet(release.magnetUri)
    }

    function play(idx) {
        if (torrentId < 0) return
        page.chosenIdx = idx
        // Narrow the download to this file, then resolve its stream URL.
        backend.setOnlyFiles(torrentId, idx)
        backend.getStreamUrl(torrentId, idx)
    }

    function togglePause() {
        if (torrentId < 0) return
        if (paused) {
            backend.resumeTorrent(torrentId)
            paused = false
        } else {
            backend.pauseTorrent(torrentId)
            paused = true
        }
    }

    // Release the torrent on the way out. The hold taken at play time means a
    // still-playing torrent is not removed until playback stops.
    function teardown() {
        if (torrentId >= 0) {
            backend.releaseTorrent(torrentId)
            backend.removeTorrent(torrentId)
            torrentId = -1
        }
    }

    // Poll the download while a torrent is active, for the status strip.
    Timer {
        interval: 500
        running: page.torrentId >= 0
        repeat: true
        onTriggered: page.backend.getTorrentStats(page.torrentId)
    }

    // Load the work (for its titles) or, in debug mode, play the bare URL.
    Component.onCompleted: {
        if (streamUrl.length > 0) {
            page.playingUrl = streamUrl
            page.stage = "play"
            player.command(["loadfile", streamUrl, "replace"])
        } else if (animeId > 0) {
            backend.getAnime(animeId)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.pad
                anchors.rightMargin: Theme.pad
                spacing: 12

                NavButton {
                    label: "Back"
                    onClicked: {
                        page.teardown()
                        page.back()
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: {
                        if (page.stage === "play")
                            return "Episode " + page.episodeNumber
                        if (page.releaseTitle.length > 0)
                            return page.releaseTitle
                        return "Episode " + page.episodeNumber
                    }
                    color: Theme.inkMuted
                    font.pixelSize: 13
                    elide: Text.ElideMiddle
                }

                // Pause / resume, only meaningful while a torrent is active.
                NavButton {
                    visible: page.torrentId >= 0
                    label: page.paused ? "Resume" : "Pause"
                    onClicked: page.togglePause()
                }
            }
        }

        // The mpv surface. Hidden until playback starts so the picking UI has
        // the whole window; kept alive (not destroyed) so the render context
        // survives the stage changes.
        MpvObject {
            id: player
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: page.stage === "play"
        }

        // The picking UI: releases, then files.
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: page.stage !== "play"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Theme.pad
                spacing: 12

                Text {
                    text: page.searching
                        ? "Searching…"
                        : (page.stage === "files" ? "Files in torrent" : "Releases")
                    color: Theme.ink
                    font.pixelSize: 18
                    font.bold: true
                }

                Text {
                    visible: page.error.length > 0
                    Layout.fillWidth: true
                    text: page.error
                    color: Theme.danger
                    font.pixelSize: 13
                    wrapMode: Text.WordWrap
                }

                // Release list.
                ListView {
                    visible: page.stage === "releases"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 6
                    model: page.releases

                    delegate: Rectangle {
                        required property var modelData

                        width: ListView.view.width
                        height: 56
                        radius: Theme.radius
                        color: releaseMouse.containsMouse ? Theme.surfaceRaised : Theme.surface
                        border.color: releaseMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            spacing: 12

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.title
                                    color: Theme.ink
                                    font.pixelSize: 13
                                    elide: Text.ElideRight
                                }
                                Text {
                                    text: {
                                        const parts = []
                                        if (modelData.resolution && modelData.resolution !== "unknown")
                                            parts.push(modelData.resolution)
                                        if (modelData.sizeBytes)
                                            parts.push((modelData.sizeBytes / 1073741824).toFixed(2) + " GB")
                                        return parts.join("  ·  ")
                                    }
                                    color: Theme.inkMuted
                                    font.pixelSize: 11
                                }
                            }
                            Text {
                                text: modelData.seeders !== undefined
                                    ? ("▲ " + modelData.seeders)
                                    : ""
                                color: Theme.inkMuted
                                font.pixelSize: 12
                            }
                        }

                        MouseArea {
                            id: releaseMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.pickRelease(modelData)
                        }
                    }
                }

                // File list.
                ListView {
                    visible: page.stage === "files"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 6
                    model: page.files

                    delegate: Rectangle {
                        required property var modelData
                        required property int index

                        width: ListView.view.width
                        height: 48
                        radius: Theme.radius
                        color: fileMouse.containsMouse ? Theme.surfaceRaised : Theme.surface
                        border.color: (index === page.chosenIdx)
                            ? Theme.accent
                            : (fileMouse.containsMouse ? Theme.accent : Theme.borderSubtle)

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            spacing: 12
                            Text {
                                Layout.fillWidth: true
                                text: modelData.name
                                color: Theme.ink
                                font.pixelSize: 13
                                elide: Text.ElideRight
                            }
                            Text {
                                text: (modelData.lengthBytes / 1048576).toFixed(0) + " MB"
                                color: Theme.inkMuted
                                font.pixelSize: 11
                            }
                        }

                        MouseArea {
                            id: fileMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.play(modelData.idx)
                        }
                    }
                }

                // Status strip while a torrent is downloading.
                Text {
                    visible: page.progress !== null && page.torrentId >= 0
                    Layout.fillWidth: true
                    color: Theme.inkMuted
                    font.pixelSize: 12
                    text: {
                        if (page.progress === null) return ""
                        const total = page.progress.totalBytes || 0
                        const done = page.progress.progressBytes || 0
                        const pct = total > 0 ? Math.floor((done / total) * 100) : 0
                        return "Downloading: " + pct + "%  ·  " +
                            (page.progress.downloadMbps || 0).toFixed(1) + " MiB/s"
                    }
                }
            }
        }
    }
}