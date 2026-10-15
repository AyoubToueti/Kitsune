import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import QtQuick.Window
import com.kitsune.app

Popup {
    id: modal

    property int animeId: 0
    property int malId: 0
    property int episodeNumber: 0
    property int absoluteEpisode: 0
    property var titles: []
    property var backend: null
    property string episodeThumbnail: ""
    property bool autoPlay: false

    signal closed()

    property string stage: "releases"
    property var releases: []
    property bool searching: false
    property string error: ""

    property string releaseMode: "episodes"
    property string releaseQuery: ""
    property var resolutionFilter: []
    property var badges: ({})
    property bool probing: false

    property string releaseTitle: ""
    property int torrentId: -1
    property var files: []
    property int chosenIdx: -1
    property string playingUrl: ""

    property var progress: null
    property bool paused: false
    property bool settingsOpen: false
    property var speedHistory: []
    property real lastBytes: 0
    property int staleSeconds: 0

    TextEdit {
        id: clipboard
        visible: false
        width: 0
        height: 0
    }

    Component {
        id: playerComponent
        MpvObject {}
    }

    property bool fullscreen: false
    property bool chromeVisible: true
    signal requestFullscreen(bool on)

    parent: Overlay.overlay
    x: modal.fullscreen ? 0 : Math.round((parent.width - width) / 2)
    y: modal.fullscreen ? 0 : Math.round((parent.height - height) / 2)
    
    width: modal.fullscreen ? parent.width : Math.min(860, parent ? parent.width - 40 : 860)
    height: modal.fullscreen ? parent.height : Math.min(800, parent ? parent.height - 40 : 800)
    
    modal: true
    focus: true
    padding: 0
    closePolicy: modal.fullscreen
        ? Popup.NoAutoClose
        : (Popup.CloseOnEscape | Popup.CloseOnPressOutside)

    function toggleFullscreen() {
        modal.fullscreen = !modal.fullscreen
        if (Window.window) {
            Window.window.visibility = modal.fullscreen ? Window.FullScreen : Window.Windowed
        }
        wakeChrome()
        modal.requestFullscreen(modal.fullscreen)
    }

    function wakeChrome() {
        modal.chromeVisible = true
        hideChrome.restart()
    }

    function doSkipIntro() {
        const p = playerLoader.item
        if (!p) return
        
        // Skip the OPENING MUSIC: jump PAST the OP chapter to the chapter that
        // follows it (e.g. "Main EP"), NOT to the start of the OP. Never skip
        // the cold-open intro -- if there is no OP chapter, do nothing.
        // `chapterList` is array-LIKE (has .length, indexable) but
        // `Array.isArray()` is false for a QVariantList-backed property.
        const chapters = p.chapterList
        if (chapters && chapters.length !== undefined) {
            for (var i = 0; i < chapters.length; i++) {
                const ch = chapters[i]
                if (ch && ch.title && /op|opening/i.test(ch.title)) {
                    if (i + 1 < chapters.length) {
                        p.setProperty("chapter", i + 1)
                        osdToast.trigger("forward", "Skipped " + ch.title)
                    } else {
                        // OP is the last chapter: nothing follows but the end.
                        p.command(["seek", String(p.duration), "absolute"])
                        osdToast.trigger("forward", "Skipped " + ch.title)
                    }
                    modal.wakeChrome()
                    return
                }
            }
        }
        // No OP chapter: nothing to skip, and definitely not the intro.
        modal.wakeChrome()
    }

    function doSkipEnding() {
        const p = playerLoader.item
        if (!p) return
        
        // Skip the ENDING MUSIC: jump PAST the ED chapter to whatever follows
        // (e.g. a preview), or to the end if the ED is the last chapter.
        // NOTE: array-LIKE, so test .length, not Array.isArray().
        const chapters = p.chapterList
        if (chapters && chapters.length !== undefined) {
            for (var i = 0; i < chapters.length; i++) {
                const ch = chapters[i]
                if (ch && ch.title && /ed|ending|outro/i.test(ch.title)) {
                    if (i + 1 < chapters.length) {
                        p.setProperty("chapter", i + 1)
                        osdToast.trigger("forward", "Skipped " + ch.title)
                    } else {
                        p.command(["seek", String(p.duration), "absolute"])
                        osdToast.trigger("forward", "Skipped " + ch.title)
                    }
                    modal.wakeChrome()
                    return
                }
            }
        }
        // No ED chapter: nothing to skip.
        modal.wakeChrome()
    }

    // The title of the chapter the playhead is currently inside, or "".
    // Uses the same array-LIKE `chapterList` as the skip functions.
    function currentChapterTitle() {
        const p = playerLoader.item
        if (!p) return ""
        const chapters = p.chapterList
        if (!chapters || chapters.length === undefined || chapters.length === 0) return ""
        const pos = p.position || 0
        var title = ""
        for (var i = 0; i < chapters.length; i++) {
            const ch = chapters[i]
            if (ch && ch.time !== undefined && ch.time <= pos) title = ch.title || ""
            else break
        }
        return title
    }

    // Context-aware skip for the S key / overlay: skip the ENDING if the ED is
    // playing, otherwise the OPENING. Does nothing during the cold-open intro.
    function doSkipSection() {
        if (/ed|ending|outro/i.test(modal.currentChapterTitle())) modal.doSkipEnding()
        else modal.doSkipIntro()
    }

    // --- Keybindings -----------------------------------------------------
    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "Space"
        onActivated: {
            if (playerLoader.item) playerLoader.item.command(["cycle", "pause"])
            modal.wakeChrome()
        }
    }

    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "F"
        onActivated: modal.toggleFullscreen()
    }

    Shortcut {
        enabled: modal.fullscreen && modal.opened
        sequence: "Escape"
        onActivated: modal.toggleFullscreen()
    }

    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "S"
        onActivated: modal.doSkipSection()
    }

    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "Left"
        onActivated: {
            if (playerLoader.item) {
                playerLoader.item.command(["seek", "-5", "relative"])
                osdToast.trigger("rewind", "-5s")
            }
            modal.wakeChrome()
        }
    }

    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "Right"
        onActivated: {
            if (playerLoader.item) {
                playerLoader.item.command(["seek", "5", "relative"])
                osdToast.trigger("forward", "+5s")
            }
            modal.wakeChrome()
        }
    }

    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "M"
        onActivated: {
            if (playerLoader.item) {
                const nextMute = !playerLoader.item.muted
                playerLoader.item.setProperty("mute", nextMute)
                const currentVol = Math.round(playerLoader.item.volume || 100)
                osdToast.trigger(nextMute ? "mute" : "volume", nextMute ? "Muted" : currentVol + "%")
            }
            modal.wakeChrome()
        }
    }

    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "Up"
        onActivated: {
            if (playerLoader.item) {
                const vol = Math.min(130, Math.round((playerLoader.item.volume || 100) + 5))
                playerLoader.item.setProperty("volume", vol)
                osdToast.trigger("volume", vol + "%")
            }
            modal.wakeChrome()
        }
    }

    Shortcut {
        enabled: modal.stage === "play" && modal.opened
        sequence: "Down"
        onActivated: {
            if (playerLoader.item) {
                const vol = Math.max(0, Math.round((playerLoader.item.volume || 0) - 5))
                playerLoader.item.setProperty("volume", vol)
                osdToast.trigger(vol === 0 ? "mute" : "volume", vol + "%")
            }
            modal.wakeChrome()
        }
    }

    // Auto-hide controls timer
    Timer {
        id: hideChrome
        interval: 2500
        onTriggered: {
            // Keep the chrome up while the pointer is on it, the settings card
            // is open, or the chapter menu is open (the menu is anchored to the
            // bar, so hiding the bar would take it away mid-selection).
            if ((modal.fullscreen && topBarHover.hovered)
                    || bottomControlsHover.hovered
                    || modal.settingsOpen
                    || bottomControls.chapterMenuOpen) {
                hideChrome.restart()
            } else {
                modal.chromeVisible = false
            }
        }
    }

    onStageChanged: {
        if (modal.stage === "play") modal.wakeChrome()
    }

    // The anime whose title forms and releases are currently cached, so a
    // re-open for the SAME anime reuses them (no refetch) while a different
    // anime resets and fetches.
    property int loadedAnimeId: 0

    onOpened: {
        if (!backend || animeId <= 0) return
        if (loadedAnimeId !== animeId) {
            // Different work: drop any cached results and fetch.
            loadedAnimeId = animeId
            titles = []
            releases = []
            badges = ({})
            error = ""
            searching = false
            backend.getAnime(animeId)
        }
        // Same work: keep the cached titles/releases as-is.
    }
    onClosed: {
        modal.teardown()
        modal.closed()
    }

    function displayTitle(item) {
        const t = item.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "(untitled)"
    }

    function offset() {
        return Math.max(0, modal.absoluteEpisode - modal.episodeNumber)
    }

    function playerBuffering() {
        const p = playerLoader.item
        return p !== null && p.buffering === true
    }

    function loadStream() {
        const p = playerLoader.item
        if (p !== null && modal.playingUrl.length > 0) {
            p.command(["loadfile", modal.playingUrl, "replace"])
        }
    }

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

    function badgeColor(index) {
        const b = modal.badges[index]
        if (b === "green") return "#4ade80"
        if (b === "yellow") return "#facc15"
        if (b === "red") return "#e5484d"
        return Theme.inkFaint
    }

    function resolutionOptions() {
        const order = ["2160p", "1080p", "720p", "480p", "360p"]
        const seen = {}
        for (const r of modal.releases) {
            if (r.resolution && r.resolution !== "unknown") seen[r.resolution] = true
        }
        return order.filter(function (r) { return seen[r] })
    }

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
        if (playerLoader.item) playerLoader.item.command(["cycle", "pause"])
    }

    function stopPlayer() {
        const p = playerLoader.item
        if (p !== null) {
            p.command(["stop"])
            p.command(["quit"])
        }
    }

    function backToReleases() {
        stopPlayer()
        if (torrentId >= 0) {
            backend.releaseTorrent(torrentId)
            backend.removeTorrent(torrentId)
            torrentId = -1
        }
        chosenIdx = -1
        files = []
        playingUrl = ""
        progress = null
        speedHistory = []
        // Do NOT clear `releases` or call search(): the list is cached, so
        // returning to it is instant. Just make sure we are not stuck in the
        // searching state from the previous play.
        searching = false
        error = ""
        stage = "releases"
    }

    function teardown() {
        stopPlayer()
        if (torrentId >= 0) {
            backend.releaseTorrent(torrentId)
            backend.removeTorrent(torrentId)
            torrentId = -1
        }
        playerLoader.active = false
        playingUrl = ""
        // Return to the releases stage and clear any in-flight "searching"
        // state, so re-opening the modal starts from a clean, cached list
        // instead of a stuck spinner or a dead player stage.
        chosenIdx = -1
        files = []
        progress = null
        speedHistory = []
        searching = false
        stage = "releases"
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
            modal.probing = true
            modal.backend.probeReleases(JSON.stringify(modal.releases))
            if (modal.autoPlay && modal.releases.length > 0) {
                modal.pickRelease(modal.releases[0])
            }
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
        function onProbeDone() { modal.probing = false }

        function onTorrentAdded(json) {
            // The search is over the moment the magnet resolved: clear the
            // "searching" flag here too, or it stays true forever and the
            // releases stage shows a permanent spinner after a play.
            modal.searching = false
            const handle = JSON.parse(json)
            if (modal.autoPlay) {
                let best = -1
                let bestSize = -1
                for (const f of modal.files) {
                    if (/\.(mkv|mp4|avi|webm|mov|m4v)$/i.test(f.name)
                        && f.lengthBytes > bestSize) {
                        best = f.idx
                        bestSize = f.lengthBytes
                    }
                }
                if (best >= 0) { modal.play(best); return }
            }
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
            modal.backend.holdTorrent(modal.torrentId)
            modal.loadStream()
        }
        function onStreamUrlFailed(message) { modal.error = message }

        function onTorrentStatsReady(json) {
            const next = JSON.parse(json)
            if (next === null) return

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
        }

        function onDownloadDone() { modal.downloadStatus = "Saved." }
        function onDownloadFailed(message) { modal.downloadStatus = message }
    }

    property string downloadStatus: ""
    function readyFraction() { return 0.05 }

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

    background: GlassPanel {
        radius: modal.fullscreen ? 0 : 16
        strong: true
    }

    contentItem: Item {
        anchors.fill: parent

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // --- Window Header (Hidden in Fullscreen) -------------------
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: modal.fullscreen ? 0 : 84
                visible: !modal.fullscreen

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 20
                    anchors.rightMargin: 16
                    spacing: 16

                    Rectangle {
                        Layout.preferredWidth: 110
                        Layout.preferredHeight: 60
                        Layout.alignment: Qt.AlignVCenter
                        radius: 8
                        color: Theme.glassFillSubtle
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
                        spacing: 2

                        Text {
                            text: "EPISODE " + modal.episodeNumber
                            color: Theme.accent
                            font.pixelSize: 11
                            font.bold: true
                            font.letterSpacing: 1.2
                        }
                        Text {
                            Layout.fillWidth: true
                            text: modal.releaseTitle !== "" ? modal.releaseTitle : ("Episode " + modal.episodeNumber)
                            color: Theme.ink
                            font.pixelSize: 18
                            font.bold: true
                            elide: Text.ElideRight
                        }
                    }

                    Rectangle {
                        Layout.alignment: Qt.AlignTop | Qt.AlignRight
                        Layout.topMargin: 14
                        width: 30
                        height: 30
                        radius: 8
                        color: closeMouse.containsMouse ? Theme.glassFillHover : "transparent"
                        border.width: 1
                        border.color: Theme.glassBorder

                        PlayerIcon {
                            anchors.centerIn: parent
                            kind: "close"
                            color: Theme.inkMuted
                            width: 14; height: 14
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

            // --- Body Area -----------------------------------------------
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                // ===== Releases Stage ====================================
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
                        Rectangle {
                            width: 150
                            height: 30
                            radius: 15
                            color: Theme.glassFillSubtle
                            border.width: 1
                            border.color: Theme.glassBorder

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
                                radius: 16; color: Theme.glassFillSubtle; border.width: 1
                                border.color: releaseFilter.activeFocus ? Theme.accent : Theme.glassBorder
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
                                        ? Theme.accent : Theme.glassFillSubtle
                                    border.width: 1
                                    border.color: modal.resolutionFilter.indexOf(modelData) >= 0
                                        ? Theme.accent : Theme.glassBorder
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

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        visible: !modal.searching && modal.releases.length > 0
                        clip: true
                        spacing: 8
                        model: modal.visibleReleases()

                        delegate: GlassPanel {
                            required property var modelData
                            readonly property int realIndex: modal.releases.indexOf(modelData)

                            width: ListView.view.width
                            height: 52
                            interactive: true

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 12
                                anchors.rightMargin: 10
                                spacing: 10

                                Rectangle {
                                    Layout.alignment: Qt.AlignVCenter
                                    width: 8; height: 8; radius: 4
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
                                                color: Theme.glassFillHover
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

                                Rectangle {
                                    Layout.alignment: Qt.AlignVCenter
                                    width: 28; height: 28; radius: 7
                                    color: copyMouse.containsMouse ? Theme.glassFillHover : "transparent"
                                    border.width: 1
                                    border.color: Theme.glassBorder
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

                                Rectangle {
                                    Layout.alignment: Qt.AlignVCenter
                                    width: 28; height: 28; radius: 7
                                    color: saveMouse.containsMouse ? Theme.glassFillHover : "transparent"
                                    border.width: 1
                                    border.color: Theme.glassBorder
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

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: modal.pickRelease(modelData)
                            }
                        }
                    }
                }

                // ===== Files & Playback Stage ============================
                ColumnLayout {
                    anchors.fill: parent
                    anchors.leftMargin: modal.fullscreen ? 0 : 20
                    anchors.rightMargin: modal.fullscreen ? 0 : 20
                    visible: modal.stage === "files" || modal.stage === "play"
                    spacing: 12

                    // Viewport Container
                    Item {
                        id: playerViewport
                        Layout.fillWidth: true
                        Layout.fillHeight: modal.stage === "play" && modal.fullscreen
                        Layout.preferredHeight: (modal.stage === "play" && !modal.fullscreen) ? 420 : 0
                        visible: modal.stage === "play"
                        clip: true

                        // 1. BASE LAYER: Video Renderer
                        Loader {
                            id: playerLoader
                            anchors.fill: parent
                            active: modal.stage === "play"
                            sourceComponent: playerComponent
                            onLoaded: modal.loadStream()
                        }

                        // Pointer wake handler, click-to-pause & synchronized blank cursor hiding
                        MouseArea {
                            id: viewportMouseArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton
                            cursorShape: (modal.stage === "play" && !modal.chromeVisible) ? Qt.BlankCursor : Qt.ArrowCursor
                            onPositionChanged: modal.wakeChrome()
                            onClicked: {
                                modal.wakeChrome()
                                // A click on the video dismisses the chapter
                                // dropdown (it is otherwise Escape / re-tap only).
                                bottomControls.closeChapterMenu()
                                if (playerLoader.item) playerLoader.item.command(["cycle", "pause"])
                            }
                        }

                        // 2. OVERLAY LAYER: Top Floating HUD (Fullscreen Only)
                        GlassPanel {
                            id: topBar
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.margins: 14
                            height: 48
                            z: 10
                            visible: opacity > 0.0
                            opacity: (modal.stage === "play" && modal.chromeVisible && modal.fullscreen) ? 1.0 : 0.0

                            Behavior on opacity { NumberAnimation { duration: 200 } }

                            HoverHandler {
                                id: topBarHover
                            }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 16
                                anchors.rightMargin: 16

                                Text {
                                    text: "Episode " + modal.episodeNumber + " — " + modal.releaseTitle
                                    color: Theme.ink
                                    font.pixelSize: 13
                                    font.bold: true
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        // 3. OVERLAY LAYER: Bottom Floating OSD Controls Bar
                        PlayerControls {
                            id: bottomControls
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.margins: 14
                            z: 10
                            visible: opacity > 0.0
                            opacity: (modal.stage === "play" && modal.chromeVisible) ? 1.0 : 0.0

                            Behavior on opacity { NumberAnimation { duration: 200 } }

                            HoverHandler {
                                id: bottomControlsHover
                            }

                            player: playerLoader.item
                            playingUrl: modal.playingUrl
                            bufferedFraction: modal.fileFraction()
                            fullscreen: modal.fullscreen
                            onOpenSettings: modal.settingsOpen = !modal.settingsOpen
                            onToggleFullscreen: modal.toggleFullscreen()
                            onTriggerOSD: (icon, label) => osdToast.trigger(icon, label)
                            onSkipIntro: modal.doSkipIntro()
                            onSkipEnding: modal.doSkipEnding()
                        }

                        // 4. OVERLAY LAYER: Floating "Skip Intro / ED" Badge
                        GlassPanel {
                            id: skipOverlay
                            anchors.bottom: parent.bottom
                            anchors.right: parent.right
                            anchors.bottomMargin: 104
                            anchors.rightMargin: 24
                            z: 12
                            radius: 12
                            implicitWidth: skipOverlayRow.implicitWidth + 24
                            implicitHeight: 38
                            
                            readonly property real pos: playerLoader.item ? (playerLoader.item.position || 0) : 0
                            readonly property real dur: playerLoader.item ? (playerLoader.item.duration || 0) : 0
                            
                            // Find the current chapter title from the player's
                            // observed chapter list (same source the OSD uses).
                            readonly property string chap: {
                                if (!playerLoader.item) return ""
                                const chapters = playerLoader.item.chapterList
                                if (chapters && chapters.length > 0) {
                                    // Find which chapter we're currently in
                                    for (var i = chapters.length - 1; i >= 0; i--) {
                                        const ch = chapters[i]
                                        if (ch && ch.time !== undefined && ch.time <= pos) {
                                            return ch.title || ""
                                        }
                                    }
                                }
                                return ""
                            }
                            
                            // Chapter-based ONLY. The old time heuristic fired
                            // during the cold-open intro (pos 10-210s) and
                            // wrongly offered "SKIP INTRO"; chapters are now
                            // reliable, so match on the current chapter title.
                            readonly property bool isOpTime: /op|opening/i.test(chap)
                            readonly property bool isEdTime: /ed|ending|outro/i.test(chap)
                            
                            visible: modal.stage === "play" && modal.chromeVisible && (isOpTime || isEdTime)

                            RowLayout {
                                id: skipOverlayRow
                                anchors.centerIn: parent
                                spacing: 8

                                PlayerIcon {
                                    kind: "forward"
                                    color: Theme.accent
                                    width: 14; height: 14
                                }

                                Text {
                                    text: skipOverlay.isEdTime ? "SKIP ENDING [S]" : "SKIP OPENING [S]"
                                    color: Theme.ink
                                    font.pixelSize: 11
                                    font.bold: true
                                    font.letterSpacing: 0.8
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: modal.doSkipSection()
                            }
                        }

                        // 5. OVERLAY LAYER: Dynamic Center Action OSD Toast
                        Rectangle {
                            id: osdToast
                            anchors.centerIn: parent
                            z: 20
                            height: 64
                            radius: 32
                            implicitWidth: Math.max(64, osdRow.implicitWidth + 32)
                            color: Qt.rgba(0.07, 0.07, 0.11, 0.82)
                            border.width: 1.5
                            border.color: Theme.accent
                            opacity: 0.0
                            scale: 0.85
                            visible: opacity > 0.0

                            property string iconKind: "play"
                            property string labelText: ""
                            property var lastPauseState: undefined

                            function trigger(kind, label) {
                                iconKind = kind
                                labelText = label || ""
                                osdAnim.restart()
                            }

                            RowLayout {
                                id: osdRow
                                anchors.centerIn: parent
                                spacing: 8

                                PlayerIcon {
                                    kind: osdToast.iconKind
                                    color: Theme.accent
                                    width: 24
                                    height: 24
                                }

                                Text {
                                    visible: osdToast.labelText.length > 0
                                    text: osdToast.labelText
                                    color: Theme.ink
                                    font.pixelSize: 13
                                    font.bold: true
                                }
                            }

                            ParallelAnimation {
                                id: osdAnim
                                NumberAnimation { target: osdToast; property: "opacity"; from: 1.0; to: 0.0; duration: 650; easing.type: Easing.OutQuad }
                                NumberAnimation { target: osdToast; property: "scale"; from: 0.85; to: 1.15; duration: 650; easing.type: Easing.OutQuad }
                            }
                        }

                        Connections {
                            target: playerLoader.item
                            function onPausedChanged() {
                                if (!playerLoader.item || modal.stage !== "play") return
                                const currentPaused = playerLoader.item.paused
                                if (osdToast.lastPauseState === undefined) {
                                    osdToast.lastPauseState = currentPaused
                                    return
                                }
                                if (osdToast.lastPauseState !== currentPaused) {
                                    osdToast.lastPauseState = currentPaused
                                    osdToast.trigger(currentPaused ? "pause" : "play", "")
                                }
                            }
                        }

                        // 6. OVERLAY LAYER: Buffering Indicator Toast
                        GlassPanel {
                            anchors.centerIn: parent
                            z: 15
                            visible: modal.stage === "play" && modal.playerBuffering()
                            radius: 14
                            implicitWidth: bufferRow.implicitWidth + 32
                            implicitHeight: 40

                            Row {
                                id: bufferRow
                                anchors.centerIn: parent
                                spacing: 10

                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: "◌"
                                    color: Theme.accent
                                    font.pixelSize: 18
                                    NumberAnimation on rotation {
                                        from: 0; to: 360
                                        duration: 900
                                        loops: Animation.Infinite
                                        running: modal.playerBuffering()
                                    }
                                }
                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: "Buffering… " + modal.percent(modal.fileFraction()) + "%"
                                    color: Theme.ink
                                    font.pixelSize: 13
                                    font.bold: true
                                }
                            }
                        }
                    }

                    // --- Download & Peer Stats Panel (Windowed Mode Only) ---
                    GlassPanel {
                        Layout.fillWidth: true
                        visible: !modal.fullscreen
                        implicitHeight: torrentStatsCol.implicitHeight + 20
                        strong: true

                        ColumnLayout {
                            id: torrentStatsCol
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 8

                            RowLayout {
                                Layout.fillWidth: true

                                Rectangle {
                                    width: 8; height: 8; radius: 4
                                    color: modal.paused ? Theme.inkFaint
                                        : (modal.fileFraction() >= 1 ? "#4ade80" : Theme.accent)
                                }
                                Text {
                                    text: modal.paused ? "Paused"
                                        : (modal.stage === "play"
                                            ? (modal.playerBuffering() ? "Buffering…" : "Playing")
                                            : (modal.fileFraction() >= 1 ? "Fully buffered" : "Buffering…"))
                                    color: Theme.ink
                                    font.pixelSize: 12
                                    font.bold: true
                                }
                                Item { Layout.fillWidth: true }
                                Text {
                                    text: modal.percent(modal.fileFraction()) + "%"
                                    color: Theme.accent
                                    font.pixelSize: 12
                                    font.bold: true
                                }
                                Rectangle {
                                    width: 24; height: 24; radius: 6
                                    color: pauseMouse.containsMouse ? Theme.glassFillHover : "transparent"
                                    border.width: 1; border.color: Theme.glassBorder
                                    PlayerIcon {
                                        anchors.centerIn: parent
                                        kind: modal.paused ? "play" : "pause"
                                        color: Theme.inkMuted
                                        width: 12; height: 12
                                    }
                                    MouseArea {
                                        id: pauseMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: modal.togglePause()
                                    }
                                }
                                Rectangle {
                                    width: 24; height: 24; radius: 6
                                    color: cancelMouse.containsMouse ? Theme.glassFillHover : "transparent"
                                    border.width: 1; border.color: Theme.glassBorder
                                    PlayerIcon {
                                        anchors.centerIn: parent
                                        kind: "stop"
                                        color: Theme.inkMuted
                                        width: 11; height: 11
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

                            Rectangle {
                                Layout.fillWidth: true
                                height: 4
                                radius: 2
                                color: Theme.glassBorder
                                Rectangle {
                                    width: parent.width * modal.fileFraction()
                                    height: parent.height
                                    radius: 2
                                    color: Theme.accent
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                Text {
                                    text: modal.formatSize(modal.chosenBytes()) + " / " + modal.formatSize(modal.chosenTotal())
                                    color: Theme.inkFaint
                                    font.pixelSize: 10
                                }
                                Item { Layout.fillWidth: true }
                                Text {
                                    text: "whole torrent " + modal.percent(modal.torrentFraction()) + "%"
                                    color: Theme.inkFaint
                                    font.pixelSize: 10
                                }
                            }

                            Row {
                                spacing: 14
                                Text {
                                    text: "Peers " + modal.connectionLabel()
                                    color: Theme.inkMuted
                                    font.pixelSize: 10
                                }
                                Text {
                                    text: "Down " + modal.formatSpeed(modal.progress ? modal.progress.downloadMbps : 0)
                                    color: Theme.inkMuted
                                    font.pixelSize: 10
                                }
                                Text {
                                    text: "ETA " + modal.formatEta(modal.progress ? modal.progress.etaSeconds : 0)
                                    color: Theme.inkMuted
                                    font.pixelSize: 10
                                }
                                Text {
                                    text: "Health " + modal.healthLabel()
                                    color: modal.healthLabel() === "Healthy" ? "#4ade80" : Theme.inkMuted
                                    font.pixelSize: 10
                                    font.bold: true
                                }
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        visible: !modal.fullscreen
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

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        visible: !modal.fullscreen
                        clip: true
                        spacing: 6
                        model: modal.files

                        delegate: GlassPanel {
                            required property var modelData
                            required property int index
                            readonly property bool chosen: modelData.idx === modal.chosenIdx

                            width: ListView.view.width
                            height: 44
                            interactive: true

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
                                    radius: 6
                                    color: Theme.accent
                                    implicitWidth: playingText.implicitWidth + 14
                                    implicitHeight: 22
                                    Text {
                                        id: playingText
                                        anchors.centerIn: parent
                                        text: modal.stage === "play" ? "Playing" : "Loading…"
                                        color: "white"
                                        font.pixelSize: 10
                                        font.bold: true
                                    }
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: modal.play(modelData.idx)
                            }
                        }
                    }
                }
            }

            // --- Window Footer (Hidden in Fullscreen) -------------------
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: modal.fullscreen ? 0 : 48
                visible: !modal.fullscreen

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 1
                    color: Theme.glassBorder
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

        // Overlay Dismiss Backdrop
        MouseArea {
            anchors.fill: parent
            visible: modal.settingsOpen && modal.stage === "play"
            z: 99
            onClicked: modal.settingsOpen = false
        }

        // Overlay Settings Card
        PlayerSettings {
            id: settingsCard
            player: playerLoader.item
            open: modal.settingsOpen
            visible: modal.settingsOpen && modal.stage === "play"
            z: 100
            anchors.centerIn: parent
            onCloseRequested: modal.settingsOpen = false
        }
    }
}