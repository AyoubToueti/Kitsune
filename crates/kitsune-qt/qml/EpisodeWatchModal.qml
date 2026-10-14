// The episode watch flow, as a modal over the detail page: pick a release,
// pick a file, then watch it in the embedded player.
//
// Mirrors the web app's EpisodeWatchModal, with one difference that matters:
// this app has a REAL in-window player, so the buffering card leads with the
// live video instead of an external-player notice.
//
// Stages:
//   releases -- indexer results, with swarm-health dots from a real probe.
//   files    -- the chosen magnet's files, auto-picking the episode match.
//   play     -- the loopback stream URL fed to mpv, over the buffering panel.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import com.kitsune.app

Popup {
    id: modal

    // What to play. Set by the detail page when an episode is clicked.
    property int animeId: 0
    property int malId: 0
    property int episodeNumber: 0
    property int absoluteEpisode: 0
    // The work's titles, as a JSON array of strings, for the indexer search.
    property var titles: []
    // The shared Backend.
    property var backend: null
    // The episode thumbnail, shown in the header.
    property string episodeThumbnail: ""

    signal closed()

    // --- state -----------------------------------------------------------

    // "releases" | "files" | "play"
    property string stage: "releases"
    property var releases: []
    property bool searching: false
    property string error: ""

    // "episodes" | "packs"
    property string releaseMode: "episodes"
    property string releaseQuery: ""
    // Selected resolutions to keep (empty = all).
    property var resolutionFilter: []
    // Health badge per release index, from the probe: "green"|"yellow"|"red".
    property var badges: ({})
    property bool probing: false

    property string releaseTitle: ""
    property int torrentId: -1
    property var files: []
    property int chosenIdx: -1
    property string playingUrl: ""

    property var progress: null
    property bool paused: false
    // Recent download-speed samples (MiB/s) for the sparkline.
    property var speedHistory: []
    property real lastBytes: 0
    property int staleSeconds: 0

    // The clipboard helper, hidden: QML has no direct clipboard API, but a
    // read-only TextEdit can select-all + copy.
    TextEdit {
        id: clipboard
        visible: false
        width: 0
        height: 0
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: 680
    // An explicit height, NOT `implicitHeight`: the body uses
    // `Layout.fillHeight`, which contributes nothing to the content item's
    // implicit height, so an implicit-sized popup would collapse to just the
    // header and footer.
    height: Math.min(720, (parent ? parent.height : 800) - 60)
    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.CloseOnEscape

    onOpened: if (backend && animeId > 0 && titles.length === 0) backend.getAnime(animeId)
    onClosed: {
        modal.teardown()
        modal.closed()
    }

    // --- helpers ---------------------------------------------------------

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    function offset() {
        return Math.max(0, modal.absoluteEpisode - modal.episodeNumber)
    }

    // A human size, e.g. "1.4 GB".
    function formatSize(bytes) {
        if (!bytes || bytes <= 0) return ""
        const units = ["B", "KB", "MB", "GB", "TB"]
        let v = bytes, u = 0
        while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
        return v.toFixed(v < 10 && u > 0 ? 1 : 0) + " " + units[u]
    }

    function formatSpeed(mbps) {
        if (!mbps || mbps <= 0) return "—"
        return mbps.toFixed(1) + " MiB/s"
    }

    function formatEta(seconds) {
        if (seconds === null || seconds === undefined || seconds <= 0) return "—"
        const h = Math.floor(seconds / 3600)
        const m = Math.floor((seconds % 3600) / 60)
        const s = Math.floor(seconds % 60)
        if (h > 0) return h + "h " + m + "m"
        if (m > 0) return m + "m " + s + "s"
        return s + "s"
    }

    function percent(f) {
        return Math.round(Math.min(Math.max(f, 0), 1) * 100)
    }

    // The fraction of the CHOSEN file downloaded, which is what playback waits
    // on -- the whole-torrent figure includes every other episode in a pack.
    function fileFraction() {
        if (modal.progress === null || modal.chosenIdx < 0) return 0
        const per = modal.progress.fileProgress || []
        const done = per[modal.chosenIdx] || 0
        let total = 0
        for (const f of modal.files) {
            if (f.idx === modal.chosenIdx) total = f.lengthBytes
        }
        if (total <= 0) return 0
        return Math.min(done / total, 1)
    }

    function torrentFraction() {
        if (modal.progress === null) return 0
        const total = modal.progress.totalBytes || 0
        if (total <= 0) return 0
        return Math.min((modal.progress.progressBytes || 0) / total, 1)
    }

    // The chosen file's bytes, for the "21 MB / 544 MB" line.
    function chosenBytes() {
        if (modal.progress === null || modal.chosenIdx < 0) return 0
        const per = modal.progress.fileProgress || []
        return per[modal.chosenIdx] || 0
    }
    function chosenTotal() {
        for (const f of modal.files) if (f.idx === modal.chosenIdx) return f.lengthBytes
        return 0
    }

    function connectionLabel() {
        if (modal.progress === null) return "Waiting for stats…"
        if (modal.progress.state === "error") return "Error"
        if (modal.paused || modal.progress.state === "paused") return "Paused"
        if (modal.progress.peersLive > 0) return "Connected (" + modal.progress.peersLive + " peers)"
        if (modal.progress.peersConnecting > 0 || modal.progress.peersQueued > 0) return "Connecting…"
        return "Searching for peers"
    }

    function healthLabel() {
        if (modal.progress === null) return "—"
        if (modal.progress.state === "error") return "Error"
        if (modal.paused) return "Paused"
        if (modal.staleSeconds >= 10) return "Stalled"
        if (modal.progress.peersLive >= 3) return "Healthy"
        if (modal.progress.peersLive > 0) return "Weak"
        return "Searching"
    }

    // The badge colour for a release index.
    function badgeColor(index) {
        const b = modal.badges[index]
        if (b === "green") return "#4ade80"
        if (b === "yellow") return "#facc15"
        if (b === "red") return "#e5484d"
        return Theme.inkFaint
    }

    // The distinct resolutions present, highest first.
    function resolutionOptions() {
        const order = ["2160p", "1080p", "720p", "480p", "360p"]
        const seen = {}
        for (const r of modal.releases) {
            if (r.resolution && r.resolution !== "unknown") seen[r.resolution] = true
        }
        return order.filter(function (r) { return seen[r] })
    }

    // The releases the filter box and resolution chips leave.
    function visibleReleases() {
        const needle = modal.releaseQuery.trim().toLowerCase()
        return modal.releases.filter(function (r) {
            if (needle !== "" && r.title.toLowerCase().indexOf(needle) < 0) return false
            if (modal.resolutionFilter.length > 0
                && modal.resolutionFilter.indexOf(r.resolution) < 0) return false
            return true
        })
    }

    function toggleResolution(r) {
        const next = modal.resolutionFilter.slice()
        const at = next.indexOf(r)
        if (at >= 0) next.splice(at, 1)
        else next.push(r)
        modal.resolutionFilter = next
    }

    // --- actions ---------------------------------------------------------

    function search() {
        if (!backend || titles.length === 0) return
        searching = true
        error = ""
        releases = []
        badges = ({})
        backend.searchReleases(
            JSON.stringify(titles),
            episodeNumber,
            absoluteEpisode,
            releaseMode)
    }

    function pickRelease(release) {
        releaseTitle = release.title || ""
        error = ""
        searching = true
        backend.addMagnet(release.magnetUri)
    }

    function play(idx) {
        if (torrentId < 0) return
        chosenIdx = idx
        backend.setOnlyFiles(torrentId, idx)
        backend.getStreamUrl(torrentId, idx)
    }

    function togglePause() {
        if (torrentId < 0) return
        if (paused) { backend.resumeTorrent(torrentId); paused = false }
        else { backend.pauseTorrent(torrentId); paused = true }
    }

    function backToReleases() {
        if (torrentId >= 0) {
            backend.releaseTorrent(torrentId)
            backend.removeTorrent(torrentId)
            torrentId = -1
        }
        chosenIdx = -1
        files = []
        playingUrl = ""
        stage = "releases"
    }

    function teardown() {
        if (torrentId >= 0) {
            backend.releaseTorrent(torrentId)
            backend.removeTorrent(torrentId)
            torrentId = -1
        }
    }

    function copyMagnet(release) {
        clipboard.text = release.magnetUri
        clipboard.selectAll()
        clipboard.copy()
        copiedFlash.restart()
    }
    property bool copied: false
    Timer { id: copiedFlash; interval: 1200; onTriggered: modal.copied = false }

    function saveTorrent(release) {
        if (!release.torrentUrl) return
        pendingTorrentUrl = release.torrentUrl
        saveDialog.open()
    }
    property string pendingTorrentUrl: ""

    // --- backend wiring --------------------------------------------------

    Connections {
        target: modal.backend

        function onAnimeReady(json) {
            const parsed = JSON.parse(json)
            if (parsed === null || parsed.id !== modal.animeId) return
            const t = parsed.title || {}
            modal.titles = JSON.parse(modal.backend.titleForms(
                t.userPreferred || "", t.english || "", t.romaji || ""))
            modal.episodeThumbnail = parsed.coverImage || ""
            modal.search()
        }

        function onReleasesReady(json) {
            modal.searching = false
            modal.releases = JSON.parse(json) || []
            if (modal.releases.length === 0) {
                modal.error = modal.releaseMode === "packs"
                    ? "No season packs found for this title."
                    : "No episode releases found."
                return
            }
            // Probe for real swarm health; the dots fill in as answers land.
            modal.probing = true
            modal.backend.probeReleases(JSON.stringify(modal.releases))
        }

        function onReleasesFailed(message) {
            modal.searching = false
            modal.error = message
        }

        function onProbeResult(index, badge, combinedScore) {
            const next = Object.assign({}, modal.badges)
            next[index] = badge
            modal.badges = next
        }
        function onProbeDone() {
            modal.probing = false
        }

        function onTorrentAdded(json) {
            const handle = JSON.parse(json)
            if (handle === null) return
            modal.torrentId = handle.id
            modal.files = handle.files || []
            modal.stage = "files"
            const picked = modal.backend.pickFile(
                JSON.stringify(modal.files), modal.episodeNumber, modal.offset())
            if (picked >= 0) modal.play(picked)
        }
        function onTorrentAddFailed(message) {
            modal.error = message
            modal.searching = false
        }

        function onStreamUrlReady(torrentId, fileIdx, url) {
            if (torrentId !== modal.torrentId || fileIdx !== modal.chosenIdx) return
            modal.playingUrl = url
            modal.stage = "play"
            // Hold BEFORE loading, so leaving cannot remove the torrent out
            // from under the player reading it.
            modal.backend.holdTorrent(modal.torrentId)
            player.command(["loadfile", url, "replace"])
        }
        function onStreamUrlFailed(message) {
            modal.error = message
        }

        function onTorrentStatsReady(json) {
            const next = JSON.parse(json)
            if (next === null) return

            // Track staleness and speed history here rather than in the core:
            // both are facts about CHANGE over time, which a single snapshot
            // cannot express.
            const bytes = next.progressBytes || 0
            if (bytes === modal.lastBytes) {
                modal.staleSeconds += 0.5
            } else {
                modal.staleSeconds = 0
                modal.lastBytes = bytes
            }
            const hist = modal.speedHistory.slice()
            hist.push(next.downloadMbps || 0)
            while (hist.length > 30) hist.shift()
            modal.speedHistory = hist

            modal.progress = next

            // Launch as soon as enough of the file is present.
            if (modal.stage === "files" && !modal.paused
                && modal.fileFraction() >= readyFraction()) {
                // The stream URL was already resolved by `play`; loading is
                // driven by onStreamUrlReady.
            }
        }

        function onDownloadDone() {
            modal.downloadStatus = "Saved."
        }
        function onDownloadFailed(message) {
            modal.downloadStatus = message
        }
    }

    property string downloadStatus: ""

    // The fraction that must be present before playback is worth starting.
    function readyFraction() { return 0.05 }

    // Poll the download while a torrent is active.
    Timer {
        interval: 500
        running: modal.torrentId >= 0
        repeat: true
        onTriggered: modal.backend.getTorrentStats(modal.torrentId)
    }

    FileDialog {
        id: saveDialog
        title: "Save torrent file"
        fileMode: FileDialog.SaveFile
        defaultSuffix: "torrent"
        nameFilters: ["Torrent files (*.torrent)"]
        onAccepted: {
            let p = selectedFile.toString()
            if (p.startsWith("file://")) p = p.slice(7)
            modal.backend.downloadTorrent(modal.pendingTorrentUrl, decodeURIComponent(p))
        }
    }

    background: Rectangle {
        radius: 14
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.borderSubtle
    }

    contentItem: ColumnLayout {
        spacing: 0

        // --- Header ------------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 92

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 16
                spacing: 16

                // Episode thumbnail.
                Rectangle {
                    Layout.preferredWidth: 124
                    Layout.preferredHeight: 68
                    Layout.alignment: Qt.AlignVCenter
                    radius: 8
                    color: Theme.surfaceHover
                    clip: true

                    Image {
                        anchors.fill: parent
                        source: modal.episodeThumbnail
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    spacing: 3

                    Text {
                        text: "EPISODE " + modal.episodeNumber
                        color: Theme.accent
                        font.pixelSize: 11
                        font.bold: true
                        font.letterSpacing: 1.2
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "Episode " + modal.episodeNumber
                        color: Theme.ink
                        font.pixelSize: 20
                        font.bold: true
                        elide: Text.ElideRight
                    }
                }

                // Close X.
                Rectangle {
                    Layout.alignment: Qt.AlignTop | Qt.AlignRight
                    Layout.topMargin: 14
                    width: 30
                    height: 30
                    radius: 8
                    color: closeMouse.containsMouse ? Theme.surfaceHover : "transparent"
                    border.width: 1
                    border.color: Theme.borderSubtle

                    Text {
                        anchors.centerIn: parent
                        text: "✕"
                        color: Theme.inkMuted
                        font.pixelSize: 12
                    }
                    MouseArea {
                        id: closeMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: modal.close()
                    }
                }
            }
        }

        // --- Body --------------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // ===== Releases stage ========================================
            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 20
                visible: modal.stage === "releases"
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: "AVAILABLE RELEASES"
                        color: Theme.inkFaint
                        font.pixelSize: 10
                        font.bold: true
                        font.letterSpacing: 1.2
                    }
                    Item { Layout.fillWidth: true }
                    // Episodes / Packs segmented toggle.
                    Rectangle {
                        width: 150
                        height: 30
                        radius: 15
                        color: Theme.surface
                        border.width: 1
                        border.color: Theme.borderSubtle

                        Rectangle {
                            width: parent.width / 2 - 4
                            height: parent.height - 6
                            y: 3
                            x: modal.releaseMode === "packs" ? parent.width / 2 : 3
                            radius: 12
                            color: Theme.accent
                            Behavior on x { NumberAnimation { duration: 150 } }
                        }
                        Row {
                            anchors.fill: parent
                            Repeater {
                                model: [
                                    { "value": "episodes", "label": "Episodes" },
                                    { "value": "packs", "label": "Packs" }
                                ]
                                delegate: Item {
                                    required property var modelData
                                    width: 75
                                    height: 30
                                    Text {
                                        anchors.centerIn: parent
                                        text: modelData.label
                                        color: modal.releaseMode === modelData.value ? "white" : Theme.inkMuted
                                        font.pixelSize: 11
                                        font.bold: true
                                    }
                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            if (modal.releaseMode === modelData.value) return
                                            modal.releaseMode = modelData.value
                                            modal.search()
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Spinner / error.
                Column {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 8
                    visible: modal.searching
                    topPadding: 30
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "◌"
                        color: Theme.accent
                        font.pixelSize: 26
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Searching indexers…"
                        color: Theme.inkMuted
                        font.pixelSize: 12
                    }
                }

                Text {
                    Layout.fillWidth: true
                    visible: !modal.searching && modal.error !== ""
                    text: modal.error
                    color: Theme.danger
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }

                // Filter box + resolution chips.
                ColumnLayout {
                    Layout.fillWidth: true
                    visible: !modal.searching && modal.releases.length > 0
                    spacing: 8

                    TextField {
                        id: releaseFilter
                        Layout.fillWidth: true
                        placeholderText: "Filter releases…"
                        color: Theme.ink
                        placeholderTextColor: Theme.inkFaint
                        font.pixelSize: 12
                        background: Rectangle {
                            radius: 16; color: Theme.surface; border.width: 1
                            border.color: releaseFilter.activeFocus ? Theme.accent : Theme.borderSubtle
                        }
                        onTextChanged: modal.releaseQuery = text
                    }

                    Flow {
                        Layout.fillWidth: true
                        Layout.preferredHeight: childrenRect.height
                        spacing: 6
                        Repeater {
                            model: modal.resolutionOptions()
                            delegate: Rectangle {
                                required property string modelData
                                radius: 12
                                color: modal.resolutionFilter.indexOf(modelData) >= 0
                                    ? Theme.accent : Theme.surface
                                border.width: 1
                                border.color: modal.resolutionFilter.indexOf(modelData) >= 0
                                    ? Theme.accent : Theme.borderSubtle
                                implicitWidth: resLabel.implicitWidth + 20
                                implicitHeight: 24

                                Text {
                                    id: resLabel
                                    anchors.centerIn: parent
                                    text: modelData
                                    color: modal.resolutionFilter.indexOf(modelData) >= 0
                                        ? "white" : Theme.inkMuted
                                    font.pixelSize: 11
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: modal.toggleResolution(modelData)
                                }
                            }
                        }
                    }
                }

                // Release rows.
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    visible: !modal.searching && modal.releases.length > 0
                    clip: true
                    spacing: 8
                    model: modal.visibleReleases()

                    delegate: Rectangle {
                        required property var modelData
                        // The release's index in the FULL list, for the badge.
                        readonly property int realIndex: modal.releases.indexOf(modelData)

                        width: ListView.view.width
                        height: 52
                        radius: 10
                        color: Theme.surface
                        border.width: 1
                        border.color: relHover.hovered ? Theme.accent : Theme.borderSubtle

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 10
                            spacing: 10

                            // Health dot.
                            Rectangle {
                                Layout.alignment: Qt.AlignVCenter
                                width: 8
                                height: 8
                                radius: 4
                                color: modal.badgeColor(realIndex)
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 3
                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.title
                                    color: Theme.ink
                                    font.pixelSize: 12
                                    elide: Text.ElideRight
                                }
                                Row {
                                    spacing: 6
                                    // resolution / source / seeders / size chips.
                                    Repeater {
                                        model: {
                                            const out = []
                                            if (modelData.resolution && modelData.resolution !== "unknown")
                                                out.push(modelData.resolution)
                                            if (modelData.source && modelData.source !== "unknown")
                                                out.push(modelData.source)
                                            if (modelData.seeders !== undefined && modelData.seeders !== null)
                                                out.push(modelData.seeders + " seeders")
                                            if (modelData.sizeBytes)
                                                out.push(modal.formatSize(modelData.sizeBytes))
                                            return out
                                        }
                                        delegate: Rectangle {
                                            required property string modelData
                                            radius: 5
                                            color: Theme.surfaceHover
                                            implicitWidth: chipText.implicitWidth + 12
                                            implicitHeight: 18
                                            Text {
                                                id: chipText
                                                anchors.centerIn: parent
                                                text: modelData
                                                color: Theme.inkMuted
                                                font.pixelSize: 10
                                            }
                                        }
                                    }
                                }
                            }

                            // Copy magnet.
                            Rectangle {
                                Layout.alignment: Qt.AlignVCenter
                                width: 28; height: 28; radius: 7
                                color: copyMouse.containsMouse ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: Theme.borderSubtle
                                Text {
                                    anchors.centerIn: parent
                                    text: "⧉"
                                    color: copyMouse.containsMouse ? Theme.accent : Theme.inkMuted
                                    font.pixelSize: 12
                                }
                                MouseArea {
                                    id: copyMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: modal.copyMagnet(modelData)
                                }
                            }

                            // Save .torrent.
                            Rectangle {
                                Layout.alignment: Qt.AlignVCenter
                                width: 28; height: 28; radius: 7
                                color: saveMouse.containsMouse ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: Theme.borderSubtle
                                Text {
                                    anchors.centerIn: parent
                                    text: "⤓"
                                    color: saveMouse.containsMouse ? Theme.accent : Theme.inkMuted
                                    font.pixelSize: 13
                                }
                                MouseArea {
                                    id: saveMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: modal.saveTorrent(modelData)
                                }
                            }
                        }

                        HoverHandler { id: relHover }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            // Below the icons: only fires where they do not.
                            onClicked: modal.pickRelease(modelData)
                        }
                    }
                }
            }

            // ===== Files + play stage ====================================
            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 20
                visible: modal.stage === "files" || modal.stage === "play"
                spacing: 12

                // The buffering card, with the player at its top.
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: bufferingCol.implicitHeight + 24
                    radius: 12
                    color: Theme.surface
                    border.width: 1
                    border.color: Theme.borderSubtle

                    ColumnLayout {
                        id: bufferingCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 12
                        spacing: 10

                        // The embedded player. Kept alive across stages so its
                        // render context survives; only shown once playing.
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: modal.stage === "play" ? 300 : 0
                            visible: modal.stage === "play"
                            radius: 8
                            color: "black"
                            clip: true

                            MpvObject {
                                id: player
                                anchors.fill: parent
                            }
                        }

                        // Status headline + percent + controls.
                        RowLayout {
                            Layout.fillWidth: true

                            Rectangle {
                                width: 8; height: 8; radius: 4
                                color: modal.paused ? Theme.inkFaint
                                    : (modal.fileFraction() >= 1 ? "#4ade80" : Theme.accent)
                            }
                            Text {
                                text: modal.paused ? "Paused"
                                    : (modal.stage === "play" ? "Playing"
                                        : (modal.fileFraction() >= 1 ? "Fully buffered" : "Buffering…"))
                                color: Theme.ink
                                font.pixelSize: 13
                                font.bold: true
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: modal.percent(modal.fileFraction()) + "%"
                                color: Theme.accent
                                font.pixelSize: 13
                                font.bold: true
                            }
                            // Pause / resume.
                            Rectangle {
                                width: 26; height: 26; radius: 7
                                color: pauseMouse.containsMouse ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: Theme.borderSubtle
                                Text {
                                    anchors.centerIn: parent
                                    text: modal.paused ? "▶" : "⏸"
                                    color: Theme.inkMuted
                                    font.pixelSize: 11
                                }
                                MouseArea {
                                    id: pauseMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: modal.togglePause()
                                }
                            }
                            // Cancel.
                            Rectangle {
                                width: 26; height: 26; radius: 7
                                color: cancelMouse.containsMouse ? Theme.surfaceHover : "transparent"
                                border.width: 1
                                border.color: Theme.borderSubtle
                                Text {
                                    anchors.centerIn: parent
                                    text: "■"
                                    color: Theme.inkMuted
                                    font.pixelSize: 10
                                }
                                MouseArea {
                                    id: cancelMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: modal.backToReleases()
                                }
                            }
                        }

                        // Progress bar.
                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            radius: 3
                            color: Theme.borderSubtle
                            Rectangle {
                                width: parent.width * modal.fileFraction()
                                height: parent.height
                                radius: 3
                                color: Theme.accent
                            }
                        }

                        // Bytes + whole torrent.
                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: modal.formatSize(modal.chosenBytes()) + " / "
                                    + modal.formatSize(modal.chosenTotal())
                                color: Theme.inkFaint
                                font.pixelSize: 11
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: "whole torrent " + modal.percent(modal.torrentFraction()) + "%"
                                color: Theme.inkFaint
                                font.pixelSize: 11
                            }
                        }

                        // Peers / speed / ETA / health.
                        Row {
                            spacing: 18
                            Text {
                                text: "Peers  " + modal.connectionLabel()
                                color: Theme.inkMuted
                                font.pixelSize: 11
                            }
                            Text {
                                text: "Down  " + modal.formatSpeed(modal.progress ? modal.progress.downloadMbps : 0)
                                color: Theme.inkMuted
                                font.pixelSize: 11
                            }
                            Text {
                                text: "ETA  " + modal.formatEta(modal.progress ? modal.progress.etaSeconds : 0)
                                color: Theme.inkMuted
                                font.pixelSize: 11
                            }
                            Text {
                                text: "Health  " + modal.healthLabel()
                                color: modal.healthLabel() === "Healthy" ? "#4ade80" : Theme.inkMuted
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        // Speed sparkline.
                        Row {
                            spacing: 3
                            Repeater {
                                model: modal.speedHistory
                                delegate: Rectangle {
                                    required property real modelData
                                    width: 10
                                    height: Math.max(3, Math.min(30, 3 + modelData * 12))
                                    anchors.bottom: parent.bottom
                                    color: Theme.accent
                                    opacity: 0.7
                                }
                            }
                        }
                    }
                }

                // Back to releases.
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: "← Back to releases"
                        color: backMouse.containsMouse ? Theme.accent : Theme.inkMuted
                        font.pixelSize: 11
                        MouseArea {
                            id: backMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: modal.backToReleases()
                        }
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: modal.files.length + (modal.files.length === 1 ? " file" : " files")
                        color: Theme.inkFaint
                        font.pixelSize: 11
                    }
                }

                // File rows.
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 6
                    model: modal.files

                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool chosen: modelData.idx === modal.chosenIdx

                        width: ListView.view.width
                        height: 46
                        radius: 9
                        color: chosen ? Theme.surfaceHover : Theme.surface
                        border.width: chosen ? 2 : 1
                        border.color: chosen ? Theme.accent
                            : (fileHover.hovered ? Theme.accent : Theme.borderSubtle)

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 10
                            spacing: 10

                            Text {
                                text: "▸"
                                color: Theme.accent
                                font.pixelSize: 12
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.name
                                    color: Theme.ink
                                    font.pixelSize: 11
                                    elide: Text.ElideRight
                                }
                                Text {
                                    text: modal.formatSize(modelData.lengthBytes)
                                    color: Theme.inkFaint
                                    font.pixelSize: 10
                                }
                            }
                            Rectangle {
                                visible: chosen
                                radius: 8
                                color: Theme.accent
                                implicitWidth: playingText.implicitWidth + 18
                                implicitHeight: 24
                                Text {
                                    id: playingText
                                    anchors.centerIn: parent
                                    text: modal.stage === "play" ? "Playing" : "Loading…"
                                    color: "white"
                                    font.pixelSize: 11
                                    font.bold: true
                                }
                            }
                        }

                        HoverHandler { id: fileHover }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: modal.play(modelData.idx)
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    visible: modal.error !== ""
                    text: modal.error
                    color: Theme.danger
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }

                Text {
                    Layout.fillWidth: true
                    visible: modal.downloadStatus !== ""
                    text: modal.downloadStatus
                    color: Theme.inkMuted
                    font.pixelSize: 11
                }
            }
        }

        // --- Footer ------------------------------------------------------
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 54
            color: Theme.surfaceRaised

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 1
                color: Theme.borderSubtle
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 20
                spacing: 10

                Text {
                    visible: modal.copied
                    text: "Magnet copied"
                    color: Theme.accent
                    font.pixelSize: 11
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: "Close"
                    color: footerClose.containsMouse ? Theme.accent : Theme.inkMuted
                    font.pixelSize: 12
                    MouseArea {
                        id: footerClose
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: modal.close()
                    }
                }
            }
        }
    }
}