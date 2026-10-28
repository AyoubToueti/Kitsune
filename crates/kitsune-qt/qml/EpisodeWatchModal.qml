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
    /// The id's NAMESPACE ("anilist" / "kitsu"). Passed in by `openFor`, so a
    /// detail lookup asks the source that issued this id rather than assuming
    /// AniList.
    property string animeProvider: "anilist"
    property var titles: []
    property var backend: null
    property string episodeThumbnail: ""
    property bool autoPlay: false

    property string stage: "releases"
    property var releases: []
    property bool searching: false
    // Whether a search has COMPLETED for this open.
    //
    // The empty state is deliberately not shown until it is true. Before the
    // first search `releases` is empty and `searching` is briefly false, which
    // painted the sleeping fox and a Retry button before anything had been
    // asked. "Nothing yet" and "nothing found" are different states.
    property bool searched: false
    property string error: ""

    property string releaseMode: "episodes"
    property string releaseQuery: ""
    property var resolutionFilter: []
    property var badges: ({})

    // --- Release sorting --------------------------------------------------
    property string releaseSortKey: "match"
    property bool releaseSortDesc: true
    property bool probing: false

    // Which open (and which search within that open) a reply belongs to.
    // Bumped in `openFor` and at the top of `search`; the backend echoes it on
    // every release/probe signal, and a mismatch means the answer is stale --
    // the previous episode's list must never paint into this open.
    property int searchGeneration: 0
    readonly property string searchTag: String(searchGeneration)

    property string releaseTitle: ""
    property string releaseMagnet: ""

    // --- Resume -----------------------------------------------------------
    readonly property string resumeBehavior: modal.appSettings.resumeBehavior || "ask"
    property var pendingResume: null
    property bool resumePromptOpen: false
    property real resumeSeekTarget: -1
    property bool resumeHandled: false
    property bool resumeLast: false
    property int resumeFileIdx: -1

    function fmtTime(seconds) {
        if (!seconds || seconds <= 0 || !isFinite(seconds)) return "00:00"
        const s = Math.floor(seconds % 60)
        const m = Math.floor((seconds / 60) % 60)
        const h = Math.floor(seconds / 3600)
        const ss = s < 10 ? "0" + s : "" + s
        const mm = m < 10 ? "0" + m : "" + m
        return (h > 0 ? h + ":" + mm : mm) + ":" + ss
    }

    function requestResume() {
        if (modal.backend) modal.backend.getWatchPositions()
    }

    readonly property int resumeRewind: {
        const v = modal.appSettings.resumeRewindSeconds
        return (v === undefined || v === null) ? 20 : Math.max(0, Math.round(v))
    }

    function resumeTarget(pos) {
        const at = (pos && pos.positionSeconds) ? pos.positionSeconds : 0
        return Math.max(0, at - modal.resumeRewind)
    }

    function applyResume(pos) {
        if (!pos || !pos.positionSeconds || pos.positionSeconds < 5) {
            modal.pendingResume = null
            return
        }
        modal.pendingResume = pos
        const behavior = modal.resumeBehavior
        if (behavior === "fresh") {
            modal.pendingResume = null
        } else if (behavior === "seek") {
            modal.resumeSeekTarget = modal.resumeTarget(pos)
        } else {
            modal.resumePromptOpen = true
        }
    }

    Timer {
        id: resumeApply
        interval: 400
        repeat: true
        running: modal.resumeSeekTarget >= 0
        onTriggered: {
            const p = playerLoader.item
            if (!p) return
            if ((p.duration || 0) > 0) {
                p.command(["seek", String(modal.resumeSeekTarget), "absolute"])
                modal.resumeSeekTarget = -1
                resumeApply.stop()
            }
        }
    }

    property int torrentId: -1
    // Local playback reuses the torrent file list and player stage, but every
    // row is a completed download and no torrent/peer UI is shown.
    property bool localPlayback: false
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
    property bool infoOpen: false
    property int infoTick: 0
    signal requestFullscreen(bool on)

    Timer {
        interval: 1000
        repeat: true
        running: modal.infoOpen && modal.stage === "play"
        onTriggered: modal.infoTick++
    }

    function prop(name) {
        const p = playerLoader.item
        if (!p || !p.getProperty) return "\u2014"
        try {
            const v = p.getProperty(name)
            if (v === undefined || v === null || v === "") return "\u2014"
            if (typeof v === "number") return (Math.round(v * 100) / 100).toString()
            return String(v)
        } catch (e) { return "\u2014" }
    }

    function videoSize() {
        const w = prop("width")
        const h = prop("height")
        if (w === "\u2014" || h === "\u2014") return "\u2014"
        return w + " \u00d7 " + h
    }

    // --- Dynamic Window Positioning & Aspect Ratio Tuning ----------------
    parent: Overlay.overlay
    x: modal.fullscreen ? 0 : Math.round((parent.width - width) / 2)
    y: modal.fullscreen ? 0 : Math.round((parent.height - height) / 2)
    
    width: modal.fullscreen 
        ? parent.width 
        : (parent ? Math.max(800, Math.min(Math.round(parent.width * 0.72), 980)) : 860)
        
    height: modal.fullscreen 
        ? parent.height 
        : (parent ? Math.max(520, Math.min(Math.round(parent.height * 0.88), 900)) : 760)
    
    modal: true
    focus: true
    padding: 0
    closePolicy: modal.fullscreen
        ? Popup.NoAutoClose
        : (Popup.CloseOnEscape | Popup.CloseOnPressOutside)

    function toggleFullscreen() {
        modal.fullscreen = !modal.fullscreen
        wakeChrome()
        modal.requestFullscreen(modal.fullscreen)
    }

    // Full UI (header + bottom bar). ONLY real pointer movement, fullscreen
    // toggles and skip actions call this.
    function wakeChrome() {
        modal.chromeVisible = true
        modal.seekOnlyVisible = false
        hideChrome.restart()
    }

    // A seek while the full UI is hidden peeks ONLY the timeline for 1.5 s.
    // If the full UI is already showing there is nothing to do.
    property bool seekOnlyVisible: false
    // Stays true through the fade-out after a peek, so the bar does not snap
    // back to its full layout while it is still visibly disappearing. Cleared
    // once the bar is fully hidden (see bottomControls.onVisibleChanged).
    property bool peekLatched: false
    function seekPeek() {
        if (modal.chromeVisible) return
        modal.peekLatched = true
        modal.seekOnlyVisible = true
        seekOnlyTimer.restart()
    }
    Timer {
        id: seekOnlyTimer
        interval: 1500
        onTriggered: modal.seekOnlyVisible = false
    }

    // --- Subtitles ride above the control bar -------------------------------
    //
    // mpv's `sub-margin-y` is measured as if the window were 720 px tall
    // (sub-scale-by-window), so the bar's height in Qt pixels is rescaled by
    // 720 / viewportHeight. Screen density cancels out because both numbers
    // are in the same logical units, and fullscreen vs windowed is handled by
    // reading the viewport's real height.
    readonly property real subBarGap: 10          // breathing room above the bar
    readonly property bool subBarShown:
        modal.stage === "play" && (modal.chromeVisible || modal.seekOnlyVisible)
    // How far to lift, as a PERCENT of the viewport height (what `sub-pos`
    // takes). Reading the viewport's real height makes this correct in both
    // windowed and fullscreen.
    readonly property real subLiftTarget: {
        const h = playerViewport.height
        if (!modal.subBarShown || h <= 0) return 0
        // 14 = the bar's bottom anchor margin.
        const reserved = bottomControls.height + 14 + modal.subBarGap
        return reserved / h * 100
    }
    property real subLift: subLiftTarget
    Behavior on subLift { NumberAnimation { duration: 200; easing.type: Easing.OutQuad } }
    onSubLiftChanged: modal.applySubMargin()

    function applySubMargin() {
        const p = playerLoader.item
        if (!p) return
        // `sub-pos` 100 = default bottom; lower values move subtitles up. Unlike
        // `sub-margin-y`, mpv applies it to styled ASS subtitles as well.
        p.setProperty("sub-pos", String(Math.round(100 - modal.subLift)))
    }

    function doSkipIntro() {
        const p = playerLoader.item
        if (!p) return
        const chapters = p.chapterList
        if (chapters && chapters.length !== undefined) {
            for (var i = 0; i < chapters.length; i++) {
                const ch = chapters[i]
                if (ch && ch.title && /op|opening/i.test(ch.title)) {
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
        modal.wakeChrome()
    }

    function doSkipEnding() {
        const p = playerLoader.item
        if (!p) return
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
        modal.wakeChrome()
    }

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

    function doSkipSection() {
        if (/ed|ending|outro/i.test(modal.currentChapterTitle())) modal.doSkipEnding()
        else modal.doSkipIntro()
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("playPause", "Space")
        onActivated: {
            // Pause: OSD only (via onPausedChanged). Never wakes the chrome.
            if (playerLoader.item) playerLoader.item.command(["cycle", "pause"])
        }
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("fullscreen", "F")
        onActivated: modal.toggleFullscreen()
    }

    Shortcut {
        enabled: modal.fullscreen && modal.opened
        sequence: "Escape"
        onActivated: modal.toggleFullscreen()
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("skip", "S")
        onActivated: modal.doSkipSection()
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("seekBack", "Left")
        onActivated: {
            if (playerLoader.item) {
                playerLoader.item.command(["seek", "-5", "relative"])
                osdToast.trigger("rewind", "-5s")
            }
            modal.seekPeek()
        }
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("seekForward", "Right")
        onActivated: {
            if (playerLoader.item) {
                playerLoader.item.command(["seek", "5", "relative"])
                osdToast.trigger("forward", "+5s")
            }
            modal.seekPeek()
        }
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("mute", "M")
        onActivated: {
            if (playerLoader.item) {
                const nextMute = !playerLoader.item.muted
                playerLoader.item.setProperty("mute", nextMute)
                const currentVol = Math.round(playerLoader.item.volume || 100)
                osdToast.trigger(nextMute ? "mute" : "volume", nextMute ? "Muted" : currentVol + "%")
            }
        }
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("volumeUp", "Up")
        onActivated: {
            if (playerLoader.item) {
                const vol = Math.min(200, Math.round((playerLoader.item.volume || 100) + 5))
                playerLoader.item.setProperty("volume", vol)
                osdToast.trigger("volume", vol + "%")
            }
        }
    }

    Shortcut {
        enabled: !modal.settingsOpen && modal.stage === "play" && modal.opened
        sequence: modal.sc("volumeDown", "Down")
        onActivated: {
            if (playerLoader.item) {
                const vol = Math.max(0, Math.round((playerLoader.item.volume || 0) - 5))
                playerLoader.item.setProperty("volume", vol)
                osdToast.trigger(vol === 0 ? "mute" : "volume", vol + "%")
            }
        }
    }

    Timer {
        id: hideChrome
        interval: 2500
        onTriggered: {
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

    // Whether the Continue record has been written for this playback.
    //
    // Deliberately separate from `recordWatchPosition`, which fires every five
    // seconds: this must happen ONCE, and only after the player has reported a
    // real duration.
    property bool lastPlayedRecorded: false

    Timer {
        interval: 5000
        repeat: true
        running: modal.stage === "play" && modal.opened
        onTriggered: {
            const p = playerLoader.item
            if (!p || !modal.backend || modal.animeId <= 0) return
            const pos = p.position || 0
            const dur = p.duration || 0
            if (pos > 0) {
                modal.backend.recordWatchPosition(modal.animeId, modal.episodeNumber, pos, dur)
            }

            // The Continue record waits for a REAL duration.
            //
            // It used to be written when the stream URL resolved, which is
            // before playback starts -- so the disc moved to the top with no
            // duration to show, and a playback that never actually began still
            // claimed to have been watched. `duration` is only known once the
            // container is open, so this is the first honest moment.
            if (!modal.lastPlayedRecorded && dur > 0) {
                modal.lastPlayedRecorded = true
                modal.backend.recordLastPlayed(
                    modal.animeId, modal.episodeNumber, modal.animeProvider)
            }
        }
    }

    property int loadedAnimeId: 0
    // The episode the release list currently belongs to. Tracked beside the
    // anime id because a different episode of the SAME work is a different
    // search: without it, Ep1 -> Ep2 skipped the fetch entirely and the old
    // list stayed on screen.
    property int loadedEpisode: 0
    property var shortcutMap: ({})
    property var appSettings: ({})

    function sc(action, fallback) {
        const v = shortcutMap[action]
        return (v && v.length > 0) ? v : fallback
    }

    function setShortcut(action, sequence) {
        const next = Object.assign({}, shortcutMap)
        if (sequence && sequence.length > 0) next[action] = sequence
        else delete next[action]
        modal.shortcutMap = next

        const s = Object.assign({}, modal.appSettings)
        s.shortcuts = next
        modal.appSettings = s
        if (modal.backend) modal.backend.setSettings(JSON.stringify(s))
    }

    function setResume(behavior, rewind) {
        const s = Object.assign({}, modal.appSettings)
        s.resumeBehavior = behavior
        s.resumeRewindSeconds = Math.max(0, Math.round(rewind))
        modal.appSettings = s
        if (modal.backend) modal.backend.setSettings(JSON.stringify(s))
    }

    Connections {
        target: modal.backend
        function onSettingsReady(json) {
            const s = JSON.parse(json)
            if (!s) return
            modal.appSettings = s
            modal.shortcutMap = s.shortcuts || ({})
        }
    }

    // Open the modal for a specific search, clearing everything the PREVIOUS
    // open left behind.
    //
    // Every caller goes through here rather than setting the fields by hand.
    // The hand-rolled version left `resumeLast` sticky (so a detail-page open
    // could replay the last release from a different work) and kept the old
    // release list when only the episode changed.
    //
    // `mode` is "episodes" or "packs"; `resumeLast` asks the backend for the
    // remembered release of this work instead of searching.
    // `provider` is the id's namespace, defaulting to AniList for callers that
    // predate the parameter. Without it the modal's own `getAnime` would ask
    // AniList for a Kitsu id and fetch a different work.
    function openFor(anime, mal, episode, absolute, mode, resumeLast, provider) {
        // Per-open search state. The generation bump orphans any reply still
        // in flight from the previous open.
        modal.searchGeneration += 1
        modal.localPlayback = false
        modal.animeProvider = provider || "anilist"
        modal.loadedAnimeId = 0
        modal.loadedEpisode = 0
        modal.titles = []
        modal.releases = []
        modal.badges = ({})
        modal.error = ""
        modal.searching = false
        modal.searched = false
        modal.probing = false
        modal.releaseTitle = ""
        modal.releaseMagnet = ""
        modal.releaseQuery = ""
        modal.resolutionFilter = []
        modal.releaseSortKey = "match"
        modal.releaseSortDesc = true
        modal.stage = "releases"

        // Resume state, so a previous "continue" cannot leak into this open.
        modal.resumeLast = resumeLast === true
        modal.resumeFileIdx = -1
        modal.resumeHandled = false
        modal.resumePromptOpen = false
        modal.resumeSeekTarget = -1
        modal.pendingResume = null
        modal.lastPlayedRecorded = false

        // Identity.
        modal.animeId = anime
        modal.malId = mal
        modal.episodeNumber = episode
        modal.absoluteEpisode = absolute
        modal.releaseMode = mode

        modal.open()
    }

    // Opened from the Downloads page. The payload is a JSON object containing
    // the anime identity plus every completed file for that anime. Rows use the
    // same shape as torrent files (idx/name/lengthBytes) so the existing player
    // stage and list can be reused unchanged.
    function openDownloads(payload) {
        let data = null
        try { data = JSON.parse(payload) } catch (e) { data = null }
        if (!data || !data.files || data.files.length === 0) return

        modal.teardown()
        modal.searchGeneration += 1
        modal.localPlayback = true
        modal.animeProvider = data.provider || "anilist"
        modal.animeId = data.animeId || 0
        modal.malId = 0
        modal.titles = []
        modal.releases = []
        modal.badges = ({})
        modal.error = ""
        modal.searching = false
        modal.searched = false
        modal.probing = false
        modal.releaseTitle = data.title || "Downloads"
        modal.releaseMagnet = ""
        modal.releaseQuery = ""
        modal.resolutionFilter = []
        modal.releaseMode = "episodes"
        modal.episodeThumbnail = data.coverImage || ""
        modal.files = data.files
        modal.loadedAnimeId = 0
        modal.loadedEpisode = 0
        modal.lastPlayedRecorded = false
        modal.downloadStatus = ""

        let selected = -1
        for (let i = 0; i < modal.files.length; i++) {
            if (String(modal.files[i].recordId) === String(data.selectedId)) {
                selected = i
                break
            }
        }
        if (selected < 0) selected = 0
        modal.playLocal(selected)
        if (!modal.opened) modal.open()
    }

    onOpened: {
        if (!backend || animeId <= 0) return
        backend.getSettings()
        // The downloaded list is already complete; there is nothing to search.
        if (modal.localPlayback) return
        if (modal.resumeLast) {
            modal.searching = true
            modal.stage = "releases"
            modal.backend.getAnime(animeId, modal.animeProvider)
            modal.backend.getLastRelease(animeId)
            return
        }
        // Guard on BOTH ids: a different episode of the same work needs its own
        // search. `openFor` clears these, so this is a safety net for any
        // future path that forgets to.
        if (loadedAnimeId !== animeId || loadedEpisode !== episodeNumber) {
            loadedAnimeId = animeId
            loadedEpisode = episodeNumber
            titles = []
            releases = []
            badges = ({})
            error = ""
            searching = false
            backend.getAnime(animeId, modal.animeProvider)
        }
    }

    onClosed: {
        // Safety net for a short watch: the five-second timer is what normally
        // records the Continue entry, so closing before the first tick would
        // otherwise leave nothing behind even though playback did start.
        //
        // Guarded on a REAL duration, exactly like the timer. That is what
        // separates "watched for a moment" from "opened the modal and backed
        // out": the former has an open container and reports one, the latter
        // never started a player at all.
        //
        // This used to fire unconditionally, which is why clicking an episode
        // and closing the popup moved the disc without anything having played.
        modal.savePositionNow()
        const p = playerLoader.item
        if (!modal.lastPlayedRecorded && p && (p.duration || 0) > 0
                && modal.backend && modal.animeId > 0) {
            modal.lastPlayedRecorded = true
            modal.backend.recordLastPlayed(
                modal.animeId, modal.episodeNumber, modal.animeProvider)
        }
        modal.teardown()
    }

    function savePositionNow() {
        const p = playerLoader.item
        if (!p || !modal.backend || modal.animeId <= 0) return
        const pos = p.position || 0
        const dur = p.duration || 0
        if (pos > 0) {
            modal.backend.recordWatchPosition(
                modal.animeId, modal.episodeNumber, pos, dur)
        }
    }

    function displayTitle(item) {
        return TitlePreference.title(item)
    }

    function offset() {
        return Math.max(0, modal.absoluteEpisode - modal.episodeNumber)
    }

    function fileNameFor(idx) {
        for (let i = 0; i < modal.files.length; i++) {
            if (modal.files[i].idx === idx) return modal.files[i].name || ""
        }
        return ""
    }

    // `bufferingDelayed`, not `buffering`: the raw flag flickers on
    // sub-second demuxer hiccups and made the panel flash over healthy
    // video. The delayed flag only reports a stall that persisted.
    function playerBuffering() {
        const p = playerLoader.item
        return p !== null && p.bufferingDelayed === true
    }

    function loadStream() {
        const p = playerLoader.item
        if (p === null || modal.playingUrl.length === 0) return
        p.command(["loadfile", modal.playingUrl, "replace"])
    }

    function formatSize(bytes) {
        if (!bytes || bytes <= 0) return ""
        const units = ["B", "KB", "MB", "GB", "TB"]
        let v = bytes, u = 0
        while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
        return v.toFixed(v < 10 && u > 0 ? 1 : 0) + " " + units[u]
    }

    function formatSpeed(mbps) {
        if (!mbps || mbps <= 0) return "\u2014"
        return mbps.toFixed(1) + " MiB/s"
    }

    function formatEta(seconds) {
        if (seconds === null || seconds === undefined || seconds <= 0) return "\u2014"
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
        if (modal.progress === null) return "Waiting for stats\u2026"
        if (modal.progress.state === "error") return "Error"
        if (modal.paused || modal.progress.state === "paused") return "Paused"
        if (modal.progress.peersLive > 0) return "Connected (" + modal.progress.peersLive + " peers)"
        if (modal.progress.peersConnecting > 0 || modal.progress.peersQueued > 0) return "Connecting\u2026"
        return "Searching for peers"
    }

    function healthLabel() {
        if (modal.progress === null) return "\u2014"
        if (modal.progress.state === "error") return "Error"
        if (modal.paused) return "Paused"
        if (modal.staleSeconds >= 10) return "Stalled"
        if (modal.progress.peersLive >= 3) return "Healthy"
        if (modal.progress.peersLive > 0) return "Weak"
        return "Searching"
    }

    function badgeColor(index) {
        const b = modal.badges[index]
        if (b === "green") return Theme.healthGreen
        if (b === "yellow") return Theme.healthYellow
        if (b === "red") return Theme.healthRed
        return Theme.inkFaint
    }

    // A broadcast/radar glyph: this control asks the trackers and swarm
    // whether the release is actually reachable, rather than just opening it.
    function availabilityIcon(fill) {
        const path = "M12 11c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm6 2c0-3.31-2.69-6-6-6s-6 2.69-6 6c0 2.22 1.21 4.15 3 5.19l1-1.74c-1.19-.7-2-1.97-2-3.45 0-2.21 1.79-4 4-4s4 1.79 4 4c0 1.48-.81 2.75-2 3.45l1 1.74c1.79-1.04 3-2.97 3-5.19zm-6-10C6.48 3 2 7.48 2 13c0 3.7 2.01 6.92 4.99 8.65l1-1.73C5.61 18.53 4 15.96 4 13c0-4.42 3.58-8 8-8s8 3.58 8 8c0 2.96-1.61 5.53-4 6.92l1 1.73C19.99 19.92 22 16.7 22 13c0-5.52-4.48-10-10-10z"
        const xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='" + fill + "'><path d='" + path + "'/></svg>"
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    function copyIcon(fill) {
        const path = "M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"
        const xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='" + fill + "'><path d='" + path + "'/></svg>"
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    function downloadIcon(fill) {
        const path = "M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"
        const xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='" + fill + "'><path d='" + path + "'/></svg>"
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    function availabilityLabel(index) {
        const b = modal.badges[index]
        if (b === "green") return "Availability: healthy \u2014 click to re-check"
        if (b === "yellow") return "Availability: uncertain \u2014 click to re-check"
        if (b === "red") return "Availability: no confirmation \u2014 click to re-check"
        return "Check availability"
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
        const filtered = modal.releases.filter(function (r) {
            if (needle !== "" && r.title.toLowerCase().indexOf(needle) < 0) return false
            if (modal.resolutionFilter.length > 0
                && modal.resolutionFilter.indexOf(r.resolution) < 0) return false
            return true
        })
        return modal.sortReleases(filtered)
    }

    function sortReleases(items) {
        const key = modal.releaseSortKey
        if (key === "match") return items

        const desc = modal.releaseSortDesc
        function valueOf(r) {
            if (key === "date") return r.publishedAt
            if (key === "name") return (r.title || "").toLowerCase()
            if (key === "size") return r.sizeBytes
            if (key === "seeders") return r.seeders
            if (key === "leechers") return r.leechers
            return undefined
        }

        return items.slice().sort(function (a, b) {
            const va = valueOf(a)
            const vb = valueOf(b)
            const aMissing = va === undefined || va === null
            const bMissing = vb === undefined || vb === null
            if (aMissing && bMissing) return 0
            if (aMissing) return 1
            if (bMissing) return -1

            let cmp
            if (typeof va === "string" || typeof vb === "string") {
                cmp = String(va).localeCompare(String(vb))
            } else {
                cmp = va - vb
            }
            return desc ? -cmp : cmp
        })
    }

    readonly property var sortColumns: [
        { "key": "match", "label": "Match" },
        { "key": "date", "label": "Date" },
        { "key": "name", "label": "Name" },
        { "key": "size", "label": "Size" },
        { "key": "seeders", "label": "Seeders" },
        { "key": "leechers", "label": "Leechers" }
    ]

    function setSort(key) {
        if (modal.releaseSortKey === key) {
            modal.releaseSortDesc = !modal.releaseSortDesc
            return
        }
        modal.releaseSortKey = key
        modal.releaseSortDesc = key !== "name"
    }

    function sortArrow(key) {
        if (modal.releaseSortKey !== key) return ""
        if (key === "match") return ""
        return modal.releaseSortDesc ? " \u2193" : " \u2191"
    }

    function toggleResolution(r) {
        const next = modal.resolutionFilter.slice()
        const at = next.indexOf(r)
        if (at >= 0) next.splice(at, 1)
        else next.push(r)
        modal.resolutionFilter = next
    }

    function releaseMatchesDownload(release, record) {
        if (record === null || record === undefined) return false
        const releaseTitle = String(release.title || "").trim().toLowerCase()
        const recordTitle = String(record.releaseTitle || "").trim().toLowerCase()
        if (releaseTitle.length > 0 && recordTitle.length > 0
                && releaseTitle === recordTitle) return true
        const magnet = String(record.magnet || "")
        if (magnet.length > 0 && magnet === String(release.magnetUri || "")) return true
        const torrent = String(record.torrentUrl || "")
        if (torrent.length > 0 && torrent === String(release.torrentUrl || "")) return true
        return false
    }

    // `force` skips the backend's cache; the Retry button passes it.
    function search(force) {
        if (!modal.backend || titles.length === 0) return
        // A new search orphans the previous one within this open (the Packs
        // toggle, or Retry).
        modal.searchGeneration += 1
        searching = true
        searched = false
        error = ""
        releases = []
        badges = ({})
        modal.backend.searchReleases(
            modal.searchTag,
            JSON.stringify(titles),
            episodeNumber,
            absoluteEpisode,
            releaseMode,
            force === true)
    }

    function pickRelease(release) {
        releaseTitle = release.title || ""
        releaseMagnet = release.magnetUri || ""
        error = ""
        searching = true
        backend.addMagnet(release.magnetUri)
    }

    function play(idx) {
        if (modal.localPlayback) {
            modal.playLocal(idx)
            return
        }
        if (torrentId < 0) return
        chosenIdx = idx
        // ONE call: select the file, wait for the engine to accept it, warm
        // both ends, then resolve the URL. See Backend::prepareStream.
        backend.prepareStream(torrentId, idx)
    }

    function playLocal(idx) {
        const file = modal.files[idx]
        if (!file) return false
        let path = file.path || ""
        if (path.length === 0 && modal.backend
                && file.recordId !== undefined && file.recordId !== null)
            path = modal.backend.downloadPath(String(file.recordId)) || ""
        if (path.length === 0) {
            modal.chosenIdx = -1
            modal.playingUrl = ""
            modal.stage = "files"
            modal.downloadStatus = "That downloaded file is no longer on disk."
            return false
        }
        modal.localPlayback = true
        modal.chosenIdx = file.idx !== undefined ? file.idx : idx
        modal.episodeNumber = file.episode || 0
        modal.absoluteEpisode = file.episode || 0
        modal.lastPlayedRecorded = false
        modal.playingUrl = path
        modal.stage = "play"
        if (playerLoader.item) modal.loadStream()
        return true
    }

    function fileSubtitle(file) {
        const size = modal.formatSize(file.lengthBytes)
        if (!modal.localPlayback) return size
        const name = file.name || ""
        if (name.length === 0) return size
        return size.length > 0 ? name + " \u00b7 " + size : name
    }

    function togglePause() {
        if (modal.localPlayback || torrentId < 0) return
        if (paused) { backend.resumeTorrent(torrentId); paused = false }
        else { backend.pauseTorrent(torrentId); paused = true }
    }

    function stopPlayer() {
        const p = playerLoader.item
        if (p !== null) {
            p.command(["stop"])
            p.command(["quit"])
        }
    }

    function backToDownloads() {
        stopPlayer()
        chosenIdx = -1
        playingUrl = ""
        stage = "files"
    }

    function backToReleases() {
        if (modal.localPlayback) {
            modal.backToDownloads()
            return
        }
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
        playingUrl = ""
        chosenIdx = -1
        files = []
        progress = null
        speedHistory = []
        searching = false
        stage = "releases"
        resumeLast = false
        resumeFileIdx = -1
        resumeHandled = false
        resumePromptOpen = false
        resumeSeekTarget = -1
        pendingResume = null
        localPlayback = false
    }

    function copyMagnet(release) {
        clipboard.text = release.magnetUri
        clipboard.selectAll()
        clipboard.copy()
        copiedFlash.restart()
    }
    property bool copied: false
    Timer { id: copiedFlash; interval: 1200; onTriggered: modal.copied = false }

    // --- Permanent download ------------------------------------------------
    //
    // The download deliberately does NOT take `modal.torrentId`: that id is what
    // `teardown` removes when the modal closes, and a download must survive the
    // close. The backend adds its own torrent and owns it from then on.

    // --- Include/exclude picker --------------------------------------------
    //
    // A release with several files is a pack, and guessing which episode the
    // reader wants is how they end up with seven. The picker resolves the file
    // list first, then starts the download for exactly what was ticked.
    property var downloadPicker: null
    property var availabilityPopup: null

    function checkAvailability(release) {
        if (!release) return
        if (availabilityPopup === null) {
            availabilityPopup = availabilityPopupComponent.createObject(modal, {
                "backend": modal.backend
            })
        }
        availabilityPopup.check(release)
    }

    Component {
        id: availabilityPopupComponent
        AvailabilityPopup {}
    }

    function openDownloadPicker(release) {
        if (!release || !release.magnetUri) return
        if (downloadPicker === null) {
            downloadPicker = filePickerComponent.createObject(modal, {
                "backend": modal.backend
            })
            // Connected HERE, not with an inline `onAcceptedDownload` handler.
            // `DownloadFilePicker` declares `modal: true` (the Popup property),
            // which shadows this popup's `modal` id for anything inside that
            // component's scope -- so an inline handler's `modal.startDownloadFor`
            // resolved against the boolean `true` and threw "Property
            // 'startDownloadFor' of object true is not a function". A closure
            // created in this function resolves `modal` in THIS scope, where it
            // is still the popup.
            downloadPicker.acceptedDownload.connect(function (pickedRelease, fileIndices) {
                modal.startDownloadFor(pickedRelease, fileIndices)
            })
            // A single-file release never opens the picker, so these are what
            // report its progress. Both are needed: resolving can take a few
            // seconds on a cold swarm, and silence would look like a dead click.
            downloadPicker.resolving.connect(function () {
                modal.downloadStatus = "Checking the release\u2026"
            })
            downloadPicker.failure.connect(function (message) {
                modal.downloadStatus = message
            })
        }
        downloadPicker.openFor(release, modal.episodeNumber, modal.offset())
    }

    function startDownloadFor(release, fileIndices) {
        if (!release || !fileIndices || fileIndices.length === 0) return
        downloadStatus = fileIndices.length > 1
            ? ("Starting " + fileIndices.length + " episodes\u2026")
            : "Starting\u2026"
        backend.startEpisodeDownload(
            modal.animeId,
            modal.animeProvider,
            modal.episodeNumber,
            0,
            modal.downloadTitle(),
            "",
            modal.episodeThumbnail,
            JSON.stringify(release),
            JSON.stringify(fileIndices))
    }

    Component {
        id: filePickerComponent
        DownloadFilePicker {}
    }

    // The title to file the download under. `titles[0]` is the preferred form
    // the backend already resolved for this open; the fallback keeps a
    // download from landing in a folder called "undefined".
    function downloadTitle() {
        for (let i = 0; i < modal.titles.length; i++) {
            if (modal.titles[i]) return modal.titles[i]
        }
        return "Unknown anime"
    }

    Connections {
        target: modal.backend

        function onAnimeReady(json) {
            const parsed = JSON.parse(json)
            if (parsed === null || parsed.id !== modal.animeId) return
            const t = parsed.title || {}
            modal.titles = JSON.parse(modal.backend.titleForms(
                t.userPreferred || "", t.english || "", t.romaji || ""))
            modal.episodeThumbnail = parsed.coverImage || ""
            if (!modal.resumeLast) modal.search()
        }

                function onReleasesReady(tag, json) {
            // A reply for a previous open (or an abandoned search) must be
            // dropped, not painted: it would show the wrong episode's list.
            if (tag !== modal.searchTag) return
            modal.searching = false
            modal.searched = true
            modal.releases = JSON.parse(json) || []
            if (modal.releases.length === 0) {
                modal.error = modal.releaseMode === "packs"
                    ? "No season packs found for this title."
                    : "No episode releases found."
                return
            }
            // Episodes that already sit on disk are read from the store, not
            // the UI: the release the reader kept then comes back here marked,
            // even after a restart. The identity check is specific so an
            // unrelated re-upload is not falsely marked too.
            if (!modal.localPlayback && modal.backend && modal.animeId > 0
                    && modal.releaseMode === "episodes" && modal.episodeNumber > 0) {
                try {
                    const rows = JSON.parse(
                        modal.backend.downloadedForAnime(
                            modal.animeId, modal.animeProvider)) || []
                    const needle = String(modal.episodeNumber)
                    const chosen = rows.filter(function (r) {
                        return r.episode !== undefined && r.episode !== null
                            && String(r.episode) === needle
                    })
                    if (chosen.length === 0) return
                    for (const release of modal.releases) {
                        if (chosen.some(function (r) {
                            return modal.releaseMatchesDownload(release, r)
                        })) {
                            release.downloaded = true
                        }
                    }
                } catch (_) {
                    // A malformed payload must never block a normal search.
                }
            }
            // No automatic probing. It used to fire here for the whole list,
            // which delayed the release list by seconds and, worse, competed
            // with the download for peers and bandwidth. Availability is now
            // checked ON DEMAND: the dot on each release is a button.
            if (modal.autoPlay && modal.releases.length > 0) {
                modal.pickRelease(modal.releases[0])
            }
        }

        function onReleasesFailed(tag, message) {
            if (tag !== modal.searchTag) return
            modal.searching = false
            modal.searched = true
            modal.error = message
        }

        function onProbeResult(tag, index, badge, combinedScore) {
            if (tag !== modal.searchTag) return
            const next = Object.assign({}, modal.badges)
            next[index] = badge
            modal.badges = next
        }
        function onProbeDone(tag) {
            if (tag !== modal.searchTag) return
            modal.probing = false
        }

        function onTorrentAdded(json) {
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
            if (modal.resumeLast && modal.resumeFileIdx >= 0
                    && modal.resumeFileIdx < modal.files.length) {
                modal.resumeLast = false
                modal.play(modal.resumeFileIdx)
                return
            }
            // A pack has no episode to match on, and `pickFile(..., 0, ...)`
            // falls through to the first video file -- which would silently
            // play episode 1 of a batch. Leave the reader on the file list to
            // choose.
            if (modal.releaseMode === "packs") return
            const picked = modal.backend.pickFile(
                JSON.stringify(modal.files), modal.episodeNumber, modal.offset())
            if (picked >= 0) modal.play(picked)
        }

        function onLastReleaseReady(json) {
            if (!modal.resumeLast) return
            const release = JSON.parse(json)
            if (release && release.magnet) {
                modal.resumeFileIdx = (release.fileIdx === undefined
                    || release.fileIdx === null) ? -1 : release.fileIdx
                modal.releaseMagnet = release.magnet
                modal.releaseTitle = release.name || ""
                modal.searching = true
                modal.backend.addMagnet(release.magnet)
            } else {
                modal.resumeLast = false
                modal.searching = false
                if (loadedAnimeId !== modal.animeId) {
                    loadedAnimeId = modal.animeId
                    titles = []
                    releases = []
                    badges = ({})
                    modal.backend.getAnime(modal.animeId, modal.animeProvider)
                }
            }
        }
        function onTorrentAddFailed(message) {
            modal.searching = false
            // A download borrowed this add: report it in the toast rather than
            // replacing the release list with an error, which would suggest the
            // whole search failed.
            // A download started from the picker reports through
            // `episodeDownloadFailed`; this handler is only for the play path.
            modal.error = message
        }

        function onStreamUrlReady(torrentId, fileIdx, url) {
            if (torrentId !== modal.torrentId || fileIdx !== modal.chosenIdx) return
            modal.playingUrl = url
            modal.stage = "play"
            modal.backend.holdTorrent(modal.torrentId)
            modal.loadStream()
            if (modal.backend && modal.animeId > 0) {
                // NOT `recordLastPlayed` here: the stream has only just been
                // resolved, so there is no duration and playback may not start
                // at all. The five-second timer writes it once `duration` is
                // real. See `lastPlayedRecorded`.
                modal.backend.setListEntry(modal.animeId, "current", modal.episodeNumber)
                if (modal.releaseMagnet.length > 0) {
                    modal.backend.recordLastRelease(
                        modal.animeId, modal.releaseMagnet, modal.chosenIdx,
                        modal.fileNameFor(modal.chosenIdx))
                }
            }
            modal.resumeHandled = false
            modal.requestResume()
        }

        function onWatchPositionsReady(json) {
            const all = JSON.parse(json) || {}
            const saved = all[String(modal.animeId)]
            if (!saved) return
            if (saved.episode !== undefined && saved.episode !== null
                    && saved.episode !== modal.episodeNumber) return
            if (modal.resumeHandled) return
            modal.resumeHandled = true
            modal.applyResume(saved)
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

        // Permanent-download answers. The record is echoed back so the toast
        // can name the episode, not just say "started".
        function onDownloadStarted(json) {
            const record = JSON.parse(json)
            if (record === null) {
                modal.downloadStatus = "Could not start the download."
                return
            }
            modal.downloadStatus = "Downloading episode " + record.episode
                + "\u2026 (see Downloads)"
        }

        function onEpisodeDownloadFailed(id, message) {
            modal.downloadStatus = message
        }
    }

    // A one-line answer for the download button. Lives beside the stage, not in
    // the release list, so a reply still shows after the list has scrolled.
    // Cleared on a timer rather than on the next click, because the message
    // ("that release has several files...") is worth reading.
    property string downloadStatus: ""
    Timer {
        id: downloadStatusTimer
        interval: 6000
        onTriggered: modal.downloadStatus = ""
    }
    onDownloadStatusChanged: {
        if (modal.downloadStatus !== "") downloadStatusTimer.restart()
    }

    Timer {
        interval: 500
        running: modal.torrentId >= 0
        repeat: true
        onTriggered: modal.backend.getTorrentStats(modal.torrentId)
    }

    background: GlassPanel {
        radius: modal.fullscreen ? 0 : 16
        strong: true
    }

    contentItem: Item {
        anchors.fill: parent

        // Download feedback, floating over the release list's bottom edge so it
        // never reflows the list the reader is scanning.
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 16
            visible: modal.downloadStatus !== ""
            z: 40
            radius: 8
            color: Theme.glassFillHover
            border.width: 1
            border.color: Theme.glassBorder
            implicitWidth: statusText.implicitWidth + 28
            implicitHeight: statusText.implicitHeight + 16
            Text {
                id: statusText
                anchors.centerIn: parent
                width: Math.min(implicitWidth, 520)
                text: modal.downloadStatus
                color: Theme.ink
                font.pixelSize: 12
                wrapMode: Text.WordWrap
            }
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // --- Window Header -----------------------------------
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
                            // A pack search has no episode, so naming one
                            // would read as "EPISODE 0".
                            text: modal.localPlayback
                                ? "DOWNLOADED EPISODES"
                                : (modal.releaseMode === "packs"
                                    ? "SEASON PACKS"
                                    : ("EPISODE " + modal.episodeNumber))
                            color: Theme.accent
                            font.pixelSize: 11
                            font.bold: true
                            font.letterSpacing: 1.2
                        }
                        Text {
                            Layout.fillWidth: true
                            text: modal.releaseTitle !== ""
                                ? modal.releaseTitle
                                : (modal.releaseMode === "packs"
                                    ? "Season packs"
                                    : ("Episode " + modal.episodeNumber))
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
                                            // An episode search needs an episode; a
                                            // packs-only open has none, so the
                                            // Episodes tab stays inert.
                                            enabled: !(modelData.value === "episodes"
                                                && modal.episodeNumber <= 0)
                                            cursorShape: enabled
                                                ? Qt.PointingHandCursor : Qt.ArrowCursor
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

                    ErrorState {
                        Layout.fillWidth: true
                        Layout.preferredHeight: implicitHeight
                        visible: modal.searching
                        compact: true
                        kind: "loading"
                        showActions: false
                        title: "Searching indexers\u2026"
                        message: ""
                        artWidth: 96
                    }

                    // Only in the releases stage: `error` is shared with
                    // file/stream failures, and Retry means nothing there. An
                    // empty result set counts as a failed search, so either way
                    // the primary button forces a real search.
                    ErrorState {
                        // A search that found nothing is not a failure: `kind`
                        // is left to `failureMessage`, which maps "No ... found"
                        // onto the not-found fox rather than the error one.
                        readonly property bool noHits:
                            modal.error.toLowerCase().indexOf("no ") === 0

                        Layout.fillWidth: true
                        visible: modal.stage === "releases" && !modal.searching
                            && modal.searched
                            && (modal.error !== "" || modal.releases.length === 0)
                        compact: true
                        artWidth: 96
                        kind: modal.error !== "" ? "" : "sleeping"
                        failureMessage: modal.error
                        title: noHits
                            ? (modal.releaseMode === "packs"
                                ? "No season packs found" : "No releases found")
                            : ""
                        message: noHits
                            ? "Nothing on the indexers matched. Try another search."
                            : ""
                        primaryText: modal.probing ? "" : "Retry"
                        showActions: !modal.probing
                        onPrimaryClicked: modal.search(true)
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        visible: !modal.searching && modal.releases.length > 0
                        spacing: 8

                        TextField {
                            id: releaseFilter
                            Layout.fillWidth: true
                            placeholderText: "Filter releases\u2026"
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

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.topMargin: 2
                        spacing: 14
                        visible: !modal.searching && modal.releases.length > 0

                        Text {
                            text: "SORT"
                            color: Theme.inkFaint
                            font.pixelSize: 9
                            font.bold: true
                            font.letterSpacing: 1.1
                        }

                        Repeater {
                            model: modal.sortColumns

                            delegate: Text {
                                required property var modelData
                                readonly property bool active: modal.releaseSortKey === modelData.key

                                text: modelData.label + modal.sortArrow(modelData.key)
                                color: active ? Theme.accent : Theme.inkMuted
                                font.pixelSize: 10
                                font.bold: active

                                MouseArea {
                                    anchors.fill: parent
                                    anchors.margins: -4
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: modal.setSort(modelData.key)
                                }
                            }
                        }

                        Item { Layout.fillWidth: true }
                    }

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        visible: !modal.searching && modal.releases.length > 0
                        clip: true
                        spacing: 8
                        model: modal.visibleReleases()

                        delegate: GlassPanel {
                            id: releaseCard
                            required property var modelData
                            readonly property int realIndex: modal.releases.indexOf(modelData)
                            readonly property bool downloaded: modelData.downloaded === true

                            width: ListView.view.width
                            height: 52
                            interactive: true
                            strong: true

                            border.color: releaseCard.downloaded
                                ? Theme.healthGreen
                                : (interactive && releaseMouse.containsMouse
                                    ? Theme.accent
                                    : Theme.glassBorder)

                            // Card-level click trigger placed FIRST so child MouseAreas take precedence
                            MouseArea {
                                id: releaseMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: modal.pickRelease(modelData)
                            }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 12
                                anchors.rightMargin: 10
                                spacing: 10

                                Rectangle {
                                    id: downloadedMark
                                    Layout.alignment: Qt.AlignVCenter
                                    implicitWidth: downloadedMarkText.implicitWidth + 12
                                    implicitHeight: 18
                                    radius: 5
                                    visible: releaseCard.downloaded
                                    color: releaseCard.downloaded ? Theme.glassFillHover : "transparent" 
                                    border.color: Theme.healthGreen
                                    Text {
                                        id: downloadedMarkText
                                        anchors.centerIn: parent
                                        text: "Downloaded"
                                        color: Theme.healthGreen
                                        font.pixelSize: 9
                                        font.weight: Font.DemiBold
                                    }
                                }

                                Item {
                                    id: availabilityButton
                                    Layout.alignment: Qt.AlignVCenter
                                    implicitWidth: 28
                                    implicitHeight: 28

                                    readonly property color statusColor: modal.badgeColor(realIndex)
                                    readonly property bool hovered: availabilityMouse.containsMouse

                                    // Soft halo that blooms out on hover, so the
                                    // control reads as a button rather than a
                                    // status dot.
                                    Rectangle {
                                        anchors.centerIn: parent
                                        width: 28; height: 28; radius: 14
                                        color: "transparent"
                                        border.width: 1
                                        border.color: availabilityButton.statusColor
                                        opacity: availabilityButton.hovered ? 0.35 : 0
                                        scale: availabilityButton.hovered ? 1.35 : 1
                                        Behavior on opacity {
                                            NumberAnimation { duration: Theme.animationsEnabled ? 160 : 0 }
                                        }
                                        Behavior on scale {
                                            NumberAnimation { duration: Theme.animationsEnabled ? 220 : 0; easing.type: Easing.OutCubic }
                                        }
                                    }

                                    Rectangle {
                                        anchors.fill: parent
                                        radius: width / 2
                                        color: availabilityButton.hovered ? Theme.glassFillHover : "transparent"
                                        border.width: 1
                                        border.color: availabilityButton.hovered
                                            ? availabilityButton.statusColor
                                            : Theme.glassBorder
                                        scale: availabilityButton.hovered ? 1.08 : 1

                                        Behavior on color {
                                            ColorAnimation { duration: Theme.animationsEnabled ? 140 : 0 }
                                        }
                                        Behavior on border.color {
                                            ColorAnimation { duration: Theme.animationsEnabled ? 140 : 0 }
                                        }
                                        Behavior on scale {
                                            NumberAnimation { duration: Theme.animationsEnabled ? 160 : 0; easing.type: Easing.OutCubic }
                                        }

                                        Image {
                                            anchors.centerIn: parent
                                            width: 15
                                            height: 15
                                            fillMode: Image.PreserveAspectFit
                                            smooth: true
                                            source: modal.availabilityIcon(
                                                Theme.hex(availabilityButton.statusColor))
                                            opacity: availabilityButton.hovered ? 1 : 0.8
                                            Behavior on opacity {
                                                NumberAnimation { duration: Theme.animationsEnabled ? 140 : 0 }
                                            }
                                        }
                                    }

                                    // The release card still opens on its main
                                    // surface click. This separate child target
                                    // means checking availability does not also
                                    // select the release.
                                    MouseArea {
                                        id: availabilityMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            mouse.accepted = true
                                            modal.checkAvailability(modelData)
                                        }
                                        ToolTip.visible: containsMouse
                                        ToolTip.text: modal.availabilityLabel(realIndex)
                                        ToolTip.delay: 300
                                    }
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
                                                // Leechers sit beside seeders so the
                                                // row shows both numbers the SORT
                                                // row can order by.
                                                if (modelData.leechers !== undefined && modelData.leechers !== null)
                                                    out.push(modelData.leechers + " leechers")
                                                if (modelData.sizeBytes)
                                                    out.push(modal.formatSize(modelData.sizeBytes))
                                                return out
                                            }
                                            delegate: Rectangle {
                                                required property string modelData
                                                radius: 5
                                                color: Theme.glassFillHover
                                                border.width: 1
                                                border.color: Theme.glassBorder
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

                                // --- Copy Magnet Button ---
                                Rectangle {
                                    Layout.alignment: Qt.AlignVCenter
                                    width: 28; height: 28; radius: 7
                                    color: copyMouse.containsMouse ? Theme.glassFillHover : "transparent"
                                    border.width: 1
                                    border.color: Theme.glassBorder
                                    z: 5

                                    Image {
                                        anchors.centerIn: parent
                                        width: 13; height: 13
                                        fillMode: Image.PreserveAspectFit
                                        smooth: true
                                        source: modal.copyIcon(
                                            Theme.hex(copyMouse.containsMouse ? Theme.accent : Theme.inkMuted))
                                        opacity: copyMouse.containsMouse ? 1 : 0.8
                                    }

                                    MouseArea {
                                        id: copyMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: modal.copyMagnet(modelData)
                                        ToolTip.visible: containsMouse
                                        ToolTip.text: "Copy magnet link"
                                        ToolTip.delay: 300
                                    }
                                }

                                // --- Download Button ---
                                Rectangle {
                                    Layout.alignment: Qt.AlignVCenter
                                    width: 28; height: 28; radius: 7
                                    color: saveMouse.containsMouse ? Theme.glassFillHover : "transparent"
                                    border.width: 1
                                    border.color: Theme.glassBorder
                                    z: 5

                                    Image {
                                        anchors.centerIn: parent
                                        width: 14; height: 14
                                        fillMode: Image.PreserveAspectFit
                                        smooth: true
                                        source: modal.downloadIcon(
                                            Theme.hex(saveMouse.containsMouse ? Theme.accent : Theme.inkMuted))
                                        opacity: saveMouse.containsMouse ? 1 : 0.8
                                    }

                                    MouseArea {
                                        id: saveMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: modal.openDownloadPicker(modelData)
                                        ToolTip.visible: containsMouse
                                        ToolTip.text: "Download"
                                        ToolTip.delay: 300
                                    }
                                }
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

                    Item {
                        id: playerViewport
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        visible: modal.stage === "play"
                        clip: true

                        Loader {
                            id: playerLoader
                            anchors.fill: parent
                            active: modal.stage === "play"
                            sourceComponent: playerComponent
                            onLoaded: {
                                modal.loadStream()
                                modal.applySubMargin()
                            }
                        }

                        MouseArea {
                            id: viewportMouseArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton
                            cursorShape: (modal.stage === "play" && !modal.chromeVisible) ? Qt.BlankCursor : Qt.ArrowCursor
                            // Qt re-sends hover events with the SAME coordinates
                            // whenever an item (OSD, bars) appears or vanishes
                            // under the cursor. Only a real move wakes the UI.
                            property real lastX: -1
                            property real lastY: -1
                            onPositionChanged: function (mouse) {
                                if (Math.abs(mouse.x - lastX) < 3 && Math.abs(mouse.y - lastY) < 3) return
                                lastX = mouse.x
                                lastY = mouse.y
                                modal.wakeChrome()
                            }
                            onClicked: {
                                // Click-to-pause must not reveal the UI.
                                bottomControls.closeChapterMenu()
                                if (playerLoader.item) playerLoader.item.command(["cycle", "pause"])
                            }

                            // Wheel over the video adjusts the volume. It is a
                            // plain mpv property, so it survives track and file
                            // changes and stays in step with the slider in
                            // PlayerControls.
                            onWheel: function (wheel) {
                                if (!playerLoader.item || modal.stage !== "play") return
                                const notches = wheel.angleDelta.y / 120
                                if (notches === 0) return

                                const cur = playerLoader.item.volume
                                const next = Math.max(0, Math.min(200, cur + notches * 5))
                                playerLoader.item.setProperty("volume", String(Math.round(next)))
                                // Wheeling up a muted player should give sound
                                // back, not silently stay muted.
                                if (next > 0 && playerLoader.item.muted)
                                    playerLoader.item.setProperty("mute", false)
                                osdToast.trigger("volume", Math.round(next) + "%")

                                wheel.accepted = true
                            }
                        }

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
                                    text: "Episode " + modal.episodeNumber + " \u2014 " + modal.releaseTitle
                                    color: Theme.ink
                                    font.pixelSize: 13
                                    font.bold: true
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        PlayerControls {
                            id: bottomControls
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.margins: 14
                            z: 10
                            visible: opacity > 0.0
                            opacity: (modal.stage === "play"
                                && (modal.chromeVisible || modal.seekOnlyVisible)) ? 1.0 : 0.0
                            timelineOnly: modal.peekLatched && !modal.chromeVisible
                            onVisibleChanged: if (!visible) modal.peekLatched = false

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
                            onOpenInfo: modal.infoOpen = !modal.infoOpen
                        }

                        GlassPanel {
                            id: infoCard
                            visible: modal.infoOpen
                            anchors.top: parent.top
                            anchors.right: parent.right
                            anchors.margins: 14
                            z: 15
                            radius: 12
                            width: 300
                            implicitHeight: infoCol.implicitHeight + 24

                            ColumnLayout {
                                id: infoCol
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 4

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        Layout.fillWidth: true
                                        text: "Video details"
                                        color: Theme.ink
                                        font.pixelSize: 12
                                        font.bold: true
                                    }
                                    Text {
                                        text: "\u2715"
                                        color: infoCloseMouse.containsMouse ? Theme.accent : Theme.inkMuted
                                        font.pixelSize: 13
                                        MouseArea {
                                            id: infoCloseMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: modal.infoOpen = false
                                        }
                                    }
                                }

                                Repeater {
                                    model: {
                                        const _ = modal.infoTick
                                        return [
                                            { k: "Resolution", v: modal.videoSize() },
                                            { k: "Container FPS", v: modal.prop("container-fps") },
                                            { k: "Video codec", v: modal.prop("video-codec") },
                                            { k: "Pixel format", v: modal.prop("video-format") },
                                            { k: "Audio codec", v: modal.prop("audio-codec") },
                                            { k: "Audio channels", v: modal.prop("audio-params/channel-count") },
                                            { k: "Sample rate", v: modal.prop("audio-params/samplerate") },
                                            { k: "HW decoding", v: modal.prop("hwdec-current") },
                                            { k: "Video bitrate", v: modal.prop("video-bitrate") },
                                            { k: "Audio bitrate", v: modal.prop("audio-bitrate") }
                                        ]
                                    }
                                    delegate: RowLayout {
                                        required property var modelData
                                        Layout.fillWidth: true
                                        spacing: 8
                                        Text {
                                            text: modelData.k
                                            color: Theme.inkFaint
                                            font.pixelSize: 11
                                            Layout.preferredWidth: 110
                                        }
                                        Text {
                                            text: modelData.v
                                            color: Theme.ink
                                            font.pixelSize: 11
                                            font.family: "monospace"
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                        }
                                    }
                                }
                            }
                        }

                        GlassPanel {
                            id: resumePrompt
                            visible: modal.resumePromptOpen && modal.pendingResume !== null
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            anchors.topMargin: 24
                            z: 25
                            radius: 12
                            implicitWidth: resumeRow.implicitWidth + 32
                            implicitHeight: 56
                            strong: true

                            RowLayout {
                                id: resumeRow
                                anchors.centerIn: parent
                                spacing: 12

                                Text {
                                    text: "Continue from "
                                        + modal.fmtTime(modal.pendingResume ? modal.pendingResume.positionSeconds : 0)
                                        + "?"
                                    color: Theme.ink
                                    font.pixelSize: 12
                                    font.bold: true
                                }

                                Rectangle {
                                    height: 30
                                    radius: 9
                                    color: contMouse.containsMouse ? Theme.accentHover : Theme.accent
                                    implicitWidth: contLabel.implicitWidth + 20
                                    Text {
                                        id: contLabel
                                        anchors.centerIn: parent
                                        text: "Continue"
                                        color: "white"
                                        font.pixelSize: 11
                                        font.bold: true
                                    }
                                    MouseArea {
                                        id: contMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            modal.resumeSeekTarget =
                                                modal.pendingResume ? modal.resumeTarget(modal.pendingResume) : -1
                                            modal.resumePromptOpen = false
                                        }
                                    }
                                }

                                Rectangle {
                                    height: 30
                                    radius: 9
                                    color: startMouse.containsMouse ? Theme.glassFillHover : Theme.glassFillSubtle
                                    border.width: 1
                                    border.color: Theme.glassBorder
                                    implicitWidth: startLabel.implicitWidth + 20
                                    Text {
                                        id: startLabel
                                        anchors.centerIn: parent
                                        text: "Dismiss"
                                        color: Theme.ink
                                        font.pixelSize: 11
                                        font.bold: true
                                    }
                                    MouseArea {
                                        id: startMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            modal.resumePromptOpen = false
                                            modal.pendingResume = null
                                        }
                                    }
                                }
                            }
                        }

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
                            
                            readonly property string chap: {
                                if (!playerLoader.item) return ""
                                const chapters = playerLoader.item.chapterList
                                if (chapters && chapters.length > 0) {
                                    for (var i = chapters.length - 1; i >= 0; i--) {
                                        const ch = chapters[i]
                                        if (ch && ch.time !== undefined && ch.time <= pos) {
                                            return ch.title || ""
                                        }
                                    }
                                }
                                return ""
                            }
                            
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

                        // The same thinking fox as every other wait in the app.
                        // It sits over the video, so it carries its own scrim
                        // rather than a solid card.
                        Item {
                            anchors.centerIn: parent
                            z: 15
                            width: Math.min(340, parent.width - 64)
                            height: bufferFox.implicitHeight + 24

                            visible: modal.stage === "play" && modal.playerBuffering()

                            Rectangle {
                                anchors.fill: parent
                                radius: 16
                                color: Qt.rgba(0, 0, 0, 0.55)
                                border.width: 1
                                border.color: Theme.glassBorder
                            }

                            ErrorState {
                                id: bufferFox
                                anchors.centerIn: parent
                                width: parent.width - 24
                                compact: true
                                kind: "loading"
                                showActions: false
                                title: "Buffering\u2026"
                                message: modal.localPlayback
                                    ? ""
                                    : (modal.percent(modal.fileFraction()) + "%")
                                artWidth: 88
                            }
                        }
                    }

                    GlassPanel {
                        Layout.fillWidth: true
                        visible: !modal.fullscreen && !modal.localPlayback
                        implicitHeight: torrentStatsCol.implicitHeight + 16
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
                                        : (modal.fileFraction() >= 1 ? Theme.healthGreen : Theme.accent)
                                }
                                Text {
                                    text: modal.paused ? "Paused"
                                        : (modal.stage === "play"
                                            ? (modal.playerBuffering() ? "Buffering\u2026" : "Playing")
                                            : (modal.fileFraction() >= 1 ? "Fully buffered" : "Buffering\u2026"))
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
                                    color: modal.healthLabel() === "Healthy" ? Theme.healthGreen : Theme.inkMuted
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
                            text: modal.localPlayback
                                ? "\u2190 Back to downloads"
                                : "\u2190 Back to releases"
                            color: backMouse.containsMouse ? Theme.accent : Theme.inkMuted
                            font.pixelSize: 11
                            MouseArea {
                                id: backMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: modal.localPlayback
                                    ? modal.backToDownloads()
                                    : modal.backToReleases()
                            }
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: modal.localPlayback
                                ? modal.files.length
                                    + (modal.files.length === 1
                                        ? " downloaded file" : " downloaded files")
                                : modal.files.length
                                    + (modal.files.length === 1 ? " file" : " files")
                            color: Theme.inkFaint
                            font.pixelSize: 11
                        }
                    }

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: modal.stage !== "play"
                        Layout.preferredHeight: modal.stage === "play" ? 120 : 220
                        Layout.maximumHeight: modal.stage === "play" ? 150 : -1
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

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: modal.play(modelData.idx)
                            }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 12
                                anchors.rightMargin: 10
                                spacing: 10

                                Text {
                                    text: "\u25B8"
                                    color: Theme.accent
                                    font.pixelSize: 12
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 1
                                    Text {
                                        Layout.fillWidth: true
                                        text: modal.localPlayback
                                            ? (modelData.episodeLabel || modelData.name)
                                            : modelData.name
                                        color: Theme.ink
                                        font.pixelSize: 11
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        text: modal.fileSubtitle(modelData)
                                        color: Theme.inkFaint
                                        font.pixelSize: 10
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
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
                                        text: modal.stage === "play" ? "Playing" : "Loading\u2026"
                                        color: "white"
                                        font.pixelSize: 10
                                        font.bold: true
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // --- Window Footer -----------------------------------
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

                    Rectangle {
                        Layout.alignment: Qt.AlignVCenter
                        implicitWidth: speedFooterRow.implicitWidth + 20
                        height: 26
                        radius: 13
                        color: speedFooterMouse.containsMouse
                            ? Theme.glassFillHover : Theme.glassFillSubtle
                        border.width: 1
                        border.color: SpeedTestStore.running
                            ? Theme.accent : Theme.glassBorder

                        Row {
                            id: speedFooterRow
                            anchors.centerIn: parent
                            spacing: 6

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: {
                                    if (SpeedTestStore.running) {
                                        const pct = SpeedTestStore.progress
                                            ? SpeedTestStore.progress.percent : 0
                                        return "Testing\u2026 " + pct + "%"
                                    }
                                    if (SpeedTestStore.result !== null) {
                                        return "down " + SpeedTestStore.downloadText()
                                            + " \u00b7 " + SpeedTestStore.latencyText()
                                    }
                                    return "Test speed"
                                }
                                color: SpeedTestStore.result !== null
                                    ? Theme.accent : Theme.inkMuted
                                font.pixelSize: 11
                            }
                        }

                        MouseArea {
                            id: speedFooterMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: !SpeedTestStore.running
                            cursorShape: SpeedTestStore.running
                                ? Qt.ArrowCursor : Qt.PointingHandCursor
                            onClicked: SpeedTestStore.run()
                        }
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

        MouseArea {
            anchors.fill: parent
            visible: modal.settingsOpen && modal.stage === "play"
            z: 99
            onClicked: modal.settingsOpen = false
        }

        PlayerSettings {
            id: settingsCard
            player: playerLoader.item
            shortcuts: modal.shortcutMap
            resumeBehavior: modal.resumeBehavior
            resumeRewind: modal.resumeRewind
            open: modal.settingsOpen
            visible: modal.settingsOpen && modal.stage === "play"
            z: 100
            anchors.centerIn: parent
            onCloseRequested: modal.settingsOpen = false
            onShortcutChanged: (action, sequence) => modal.setShortcut(action, sequence)
            onResumeChanged: (behavior, rewind) => modal.setResume(behavior, rewind)
        }
    }
}