import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: bar

    property var player: null
    property var backend: null
    property string playingUrl: ""
    // The streaming torrent's id (negative for local files). RAM streams
    // refuse the scrub preview: every preview seek downloads a 32 MiB window
    // of the shared RAM buffer at the hovered position, stealing bandwidth
    // from real playback and churning the buffer with data nobody watches.
    property int torrentId: -1
    property bool fullscreen: false
    property var chapterList: []
    property bool showRemainingTime: false
    property bool timelineOnly: false
    // Live transfer stats for the playing stream: the camelCase
    // TorrentProgress JSON the backend emits from `getTorrentStats`
    // (downloadMbps, peersLive, peersConnecting, peersQueued, state, ...).
    // Null for local files, which hides the transfer chips -- a local file
    // has no swarm, and its whole timeline is already on disk.
    property var torrentStats: null

    signal openSettings()
    signal toggleFullscreen()
    signal triggerOSD(string icon, string label)
    signal skipIntro()
    signal skipEnding()
    signal openInfo()

    // In timeline-only mode (a seek while the rest of the UI is hidden) just
    // the seek row shows, so the bar is shorter.
    implicitHeight: bar.timelineOnly ? 40 : 80
    property int tick: 0

    // True while the pointer is anywhere over the bar, so the page can keep
    // the UI awake instead of hiding it from under the cursor.
    readonly property bool hovered: barHover.hovered
    HoverHandler { id: barHover }

    readonly property bool chapterMenuOpen: chapterMenuPopup.visible

    function closeChapterMenu() { chapterMenuPopup.close() }

    Connections {
        target: bar.player
        function onStateChanged() {
            bar.tick++
        }
        function onChapterListChanged() {
            bar.syncChapterList()
        }
    }

    onPlayerChanged: if (bar.player) bar.syncChapterList()

    function syncChapterList() {
        if (!bar.player || bar.player.chapterList === undefined) {
            bar.chapterList = []
            return
        }
        var raw = bar.player.chapterList
        if (raw !== null && raw !== undefined && raw.length !== undefined) {
            var normalized = []
            for (var i = 0; i < raw.length; i++) {
                var ch = raw[i]
                if (ch && typeof ch === "object") {
                    normalized.push({
                        title: ch.title || ("Chapter " + (i + 1)),
                        time: ch.time || 0
                    })
                }
            }
            bar.chapterList = normalized
        } else {
            bar.chapterList = []
        }
    }

    function getChapterAtTime(t) {
        if (bar.chapterList && bar.chapterList.length > 0) {
            var currentTitle = ""
            for (var i = 0; i < bar.chapterList.length; i++) {
                var ch = bar.chapterList[i]
                if (ch && ch.time !== undefined && ch.time <= t) {
                    currentTitle = ch.title || ("Chapter " + (i + 1))
                } else if (ch && ch.time !== undefined && ch.time > t) {
                    break
                }
            }
            if (currentTitle.length > 0) return currentTitle
        }
        return ""
    }

    function pos() { return bar.player !== null ? bar.player.position : 0 }
    function dur() { return bar.player !== null ? bar.player.duration : 0 }
    function isPaused() { return bar.player !== null ? bar.player.paused : true }
    function rate() { return bar.player !== null ? bar.player.speed : 1 }
    function vol() { return bar.player !== null ? bar.player.volume : 100 }
    function isMuted() { return bar.player !== null ? bar.player.muted : false }

    function fmt(seconds) {
        if (!seconds || seconds <= 0 || !isFinite(seconds)) return "00:00"
        const s = Math.floor(seconds % 60)
        const m = Math.floor((seconds / 60) % 60)
        const h = Math.floor(seconds / 3600)
        const ss = s < 10 ? "0" + s : "" + s
        const mm = m < 10 ? "0" + m : "" + m
        if (h > 0) return h + ":" + mm + ":" + ss
        return mm + ":" + ss
    }

    // The mpv cache window, exactly as `seekable-ranges` reports it: the
    // spans the demuxer can serve right now, clamped into [0, duration] with
    // gaps of 2.5s or less bridged (the prototype's `coalesceSpans`
    // coalescing -- hairline gaps would render as flickering dashes).
    // Deliberately NOT a "cached so far" fill: this is a MOVING window around
    // the playhead. mpv keeps a ~30s read-ahead and a 16MiB back buffer, so
    // the back edge slides forward as old data is evicted, the leading edge
    // grows while the torrent feeds the demuxer, and the whole stripe travels
    // with playback -- the way mpv's own cache display moves. The backend
    // only re-emits `seekableRanges` when a range actually changes.
    function cachedSpans() {
        if (!bar.player || bar.player.seekableRanges === undefined) return []
        const raw = bar.player.seekableRanges
        if (!raw || raw.length === undefined) return []
        const dur = bar.dur()
        if (!dur || dur <= 0) return []
        const list = []
        for (let i = 0; i < raw.length; i++) {
            const a = Math.max(0, raw[i].start)
            const z = Math.min(dur, raw[i].end)
            if (z > a) list.push([a, z])
        }
        list.sort((x, y) => x[0] - y[0])
        const out = []
        for (let i = 0; i < list.length; i++) {
            const last = out.length > 0 ? out[out.length - 1] : null
            if (last && list[i][0] - last.end <= 2.5) {
                last.end = Math.max(last.end, list[i][1])
            } else {
                out.push({ start: list[i][0], end: list[i][1] })
            }
        }
        return out
    }

    // Remaining cache ahead of the playhead, in media seconds. This is mpv's
    // `cache-duration` (the `cacheAhead` property, polled at 200ms; the
    // backend reads it from `demuxer-cache-state` and falls back to
    // `demuxer-cache-time` minus the playhead, because that property alone is
    // the buffered-end TIMESTAMP, not a duration): it GROWS while the torrent
    // delivers data faster than the decoder consumes it, SHRINKS as the
    // playhead advances, and jumps back up on a backward seek inside a cached
    // span. Never a cumulative total -- exactly the "how long can I keep
    // watching before it stalls" number.
    function cacheAheadSeconds() {
        if (!bar.player || bar.player.cacheAhead === undefined) return 0
        return Math.max(0, bar.player.cacheAhead)
    }

    // "57s" / "2m 20s", the shape mpv's own OSC and the web player use.
    function fmtCacheAhead(s) {
        s = Math.max(0, Math.round(s))
        if (s < 60) return s + "s"
        const r = s % 60
        return Math.floor(s / 60) + "m " + (r < 10 ? "0" + r : "" + r) + "s"
    }

    // Health tint for the cache dot: how safely playback can continue if the
    // swarm dropped this instant. Green >= 45s, amber >= 12s, red below.
    function cacheHealthColor(s) {
        if (s < 12) return Theme.healthRed
        if (s < 45) return Theme.healthYellow
        return Theme.healthGreen
    }

    // Bytes as whole MiB, for the RAM buffer chip.
    function fmtMib(bytes) {
        return String(Math.round((bytes || 0) / (1024 * 1024)))
    }

    // Download rate in MiB/s (the engine's per-torrent rate; the backend
    // converts already). "\u2014" while nothing is arriving: paused, stalled,
    // or still handshaking with the swarm.
    function speedLabel() {
        if (bar.torrentStats === null) return ""
        const mbps = bar.torrentStats.downloadMbps || 0
        if (mbps <= 0) return "\u2014"
        return mbps.toFixed(1) + " MiB/s"
    }

    // Peers currently transferring pieces. "\u2026" while the swarm is still
    // connecting, "0" once the trackers have answered and found nobody.
    function peersLabel() {
        if (bar.torrentStats === null) return ""
        const live = bar.torrentStats.peersLive || 0
        if (live > 0) return String(live)
        const warming = (bar.torrentStats.peersConnecting || 0)
            + (bar.torrentStats.peersQueued || 0)
        return warming > 0 ? "\u2026" : "0"
    }

    // Track pieces grow while the pointer works the bar, so the played layer
    // is easier to judge before committing to a seek.
    readonly property real trackHeight: (seek.hovering || seek.dragging) ? 7 : 4
    // The cache stripe color: the prototype's `cacheStripe` token. A near
    // black line through the track -- the mpv look -- that stays readable on
    // both the dark glass track and the light one.
    readonly property color cacheStripe: Qt.rgba(0, 0, 0, 0.82)
    // The stripe runs thinner than the track it sits in: 3px inside the
    // grown (7px) hovered track, 2px inside the idle (4px) one, exactly like
    // the prototype's seek row.
    readonly property real stripeHeight: trackHeight > 4 ? 3 : 2

    function formattedTimeDisplay() {
        const currentPos = bar.pos()
        const totalDur = bar.dur()
        if (bar.showRemainingTime) {
            const remaining = Math.max(0, totalDur - currentPos)
            return "-" + bar.fmt(remaining)
        }
        return bar.fmt(currentPos)
    }

    function volReadout() {
        if (bar.isMuted()) return "Muted"
        return Math.round(bar.vol()) + "%"
    }

    GlassPanel {
        anchors.fill: parent
        radius: 14
        strong: true

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 8
            spacing: 6

            // --- Timeline Seek Bar & Floating Time Badges ------------------
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                // Current / Remaining Position Timestamp on Timeline
                Text {
                    text: bar.formattedTimeDisplay()
                    color: Theme.accent
                    font.pixelSize: 11
                    font.family: "monospace"
                    font.bold: true

                    ToolTip.visible: timeHover.containsMouse
                    ToolTip.text: bar.showRemainingTime ? "Click to show elapsed time" : "Click to show remaining time"
                    ToolTip.delay: 300

                    MouseArea {
                        id: timeHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: bar.showRemainingTime = !bar.showRemainingTime
                    }
                }

                // Seek Track Area
                Item {
                    id: seek
                    Layout.fillWidth: true
                    height: 18

                    property bool dragging: false
                    property bool hovering: false
                    property real hoverFraction: 0
                    property real dragFraction: 0

                    readonly property real hoverTime: (seek.dragging ? seek.dragFraction : seek.hoverFraction) * bar.dur()
                    readonly property real fraction: {
                        if (seek.dragging) return seek.dragFraction
                        const d = bar.dur()
                        if (d <= 0) return 0
                        return Math.min(Math.max(bar.pos() / d, 0), 1)
                    }

                    // Floating Video Preview & Section Title Tooltip Card
                    Rectangle {
                        id: hoverTooltip
                        visible: (seek.hovering || seek.dragging) && bar.dur() > 0
                        z: 30
                        x: Math.min(Math.max(0, (seek.width * (seek.dragging ? seek.dragFraction : seek.hoverFraction)) - width / 2), seek.width - width)
                        anchors.bottom: seek.top
                        anchors.bottomMargin: 8
                        width: 160
                        height: 118
                        radius: 10
                        color: Qt.rgba(0.07, 0.07, 0.11, 0.95)
                        border.width: 1
                        border.color: Theme.glassBorder
                        clip: true

                        readonly property string chapTitle: bar.getChapterAtTime(seek.hoverTime)

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 4
                            spacing: 4

                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 88
                                radius: 6
                                color: "black"
                                clip: true

                                MpvObject {
                                    id: previewPlayer
                                    anchors.fill: parent
                                    visible: (seek.hovering || seek.dragging) && bar.dur() > 0

                                    function setupPreview() {
                                        // RAM streams never feed the preview:
                                        // the frame candy is not worth 32 MiB
                                        // of buffer churn per hover position.
                                        if (bar.backend && bar.torrentId >= 0
                                                && bar.backend.isRamStreaming(bar.torrentId))
                                            return
                                        setProperty("pause", true)
                                        setProperty("mute", true)
                                        setProperty("aid", "no")
                                        setProperty("sid", "no")
                                        if (renderReady && bar.playingUrl.length > 0)
                                            command(["loadfile", bar.playingUrl, "replace"])
                                    }

                                    Component.onCompleted: setupPreview()
                                    onRenderReadyChanged: if (renderReady) setupPreview()
                                }

                                Connections {
                                    target: bar
                                    function onPlayingUrlChanged() {
                                        if (bar.backend && bar.torrentId >= 0
                                                && bar.backend.isRamStreaming(bar.torrentId))
                                            return
                                        if (previewPlayer && previewPlayer.renderReady
                                                && bar.playingUrl.length > 0)
                                            previewPlayer.command(["loadfile", bar.playingUrl, "replace"])
                                    }
                                }

                                // A REPEATING throttle, not a one-shot debounce.
                                // A debounce only fires once the pointer STOPS
                                // moving (every change restarts it), so the
                                // preview trailed the cursor while scrubbing and
                                // only caught up when the mouse settled. Seeking
                                // to the latest position each tick keeps it in
                                // step with the pointer.
                                Timer {
                                    id: previewSeekTimer
                                    interval: 120
                                    repeat: true
                                    running: (seek.hovering || seek.dragging) && bar.dur() > 0
                                    onTriggered: {
                                        if (previewPlayer && bar.dur() > 0)
                                            previewPlayer.command(["seek", String(seek.hoverTime), "absolute+keyframes"])
                                    }
                                }

                                // One immediate seek as the pointer enters, so
                                // the very first frame does not wait a whole
                                // tick to appear.
                                Connections {
                                    target: seek
                                    function onHoveringChanged() {
                                        if (seek.hovering && previewPlayer && bar.dur() > 0)
                                            previewPlayer.command(["seek", String(seek.hoverTime), "absolute+keyframes"])
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 18
                                Layout.leftMargin: 4
                                Layout.rightMargin: 4
                                spacing: 4

                                Text {
                                    Layout.fillWidth: true
                                    text: hoverTooltip.chapTitle.length > 0 ? hoverTooltip.chapTitle : "Episode"
                                    color: Theme.accent
                                    font.pixelSize: 10
                                    font.bold: true
                                    elide: Text.ElideRight
                                }

                                Text {
                                    text: bar.fmt(seek.hoverTime)
                                    color: Theme.ink
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.family: "monospace"
                                }
                            }
                        }
                    }

                    // Background Track
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width
                        height: bar.trackHeight
                        radius: height / 2
                        color: Theme.glassFillHover
                    }

                    // Played Progress
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width * seek.fraction
                        height: bar.trackHeight
                        radius: height / 2
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop { position: 0.0; color: Theme.accent }
                            GradientStop { position: 1.0; color: Theme.accentHover }
                        }
                    }

                    // Cached ranges -- the black cache bar from the prototype,
                    // the mpv look: a dark stripe through the track's center
                    // marking what mpv's demuxer holds right now. It is a
                    // MOVING window around the playhead, not a "cached so
                    // far" fill: the back edge slides forward as mpv evicts
                    // its 16MiB back buffer, the leading edge grows while the
                    // torrent feeds the ~30s read-ahead -- the whole stripe
                    // travels with playback. It rides ON TOP of the played
                    // layer so the moving back edge stays visible over the
                    // red fill instead of vanishing under it. The Cache chip
                    // still counts down what is ahead of the playhead.
                    Repeater {
                        model: bar.cachedSpans()

                        delegate: Rectangle {
                            required property var modelData

                            visible: bar.dur() > 0 && modelData.end > modelData.start
                            anchors.verticalCenter: parent.verticalCenter
                            x: (modelData.start / bar.dur()) * seek.width
                            width: Math.max(2,
                                ((Math.min(modelData.end, bar.dur())
                                  - Math.min(modelData.start, bar.dur()))
                                 / bar.dur()) * seek.width)
                            height: bar.stripeHeight
                            radius: height / 2
                            color: bar.cacheStripe
                        }
                    }

                    // Embedded Chapter Markers
                    Repeater {
                        model: bar.chapterList
                        delegate: Rectangle {
                            required property var modelData
                            readonly property real chTime: modelData.time || 0
                            visible: bar.dur() > 0 && chTime > 0 && chTime < bar.dur()
                            x: (chTime / bar.dur()) * seek.width - 1
                            anchors.verticalCenter: parent.verticalCenter
                            width: 2
                            height: 6
                            radius: 1
                            color: Theme.inkFaint
                            z: 5
                        }
                    }

                    // Scrubber Knob
                    Rectangle {
                        visible: bar.dur() > 0
                        x: Math.min(parent.width - width, Math.max(0, parent.width * seek.fraction - width / 2))
                        anchors.verticalCenter: parent.verticalCenter
                        width: 12
                        height: 12
                        radius: 6
                        color: Theme.accent
                        border.width: 2
                        border.color: Theme.surfaceRaised
                        z: 10
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onEntered: seek.hovering = true
                        onExited: seek.hovering = false
                        onPositionChanged: function (mouse) {
                            seek.hoverFraction = Math.min(Math.max(mouse.x / width, 0), 1)
                            if (seek.dragging) {
                                seek.dragFraction = seek.hoverFraction
                            }
                        }
                        onPressed: function (mouse) {
                            seek.dragging = true
                            seek.hoverFraction = Math.min(Math.max(mouse.x / width, 0), 1)
                            seek.dragFraction = seek.hoverFraction
                        }
                        onReleased: {
                            seek.dragging = false
                            const d = bar.dur()
                            if (bar.player !== null && d > 0) {
                                bar.player.command(["seek", String(seek.dragFraction * d), "absolute"])
                            }
                        }
                    }
                }

                // Total Duration Timestamp on Timeline
                Text {
                    text: bar.fmt(bar.dur())
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    font.family: "monospace"
                    font.bold: true
                }

                // ---- live transfer chips (torrent streaming only) ----------
                //
                // They replace the stats bar that used to sit under the video
                // (removed): the numbers that matter while streaming now live
                // in the player bar itself, the way mpv's OSC does it.
                // `torrentStats` is null for local files, which hides all
                // three -- a local file has no swarm and no cache pressure.

                // Remaining cache ahead of the playhead (mpv's
                // `cache-duration` via the backend's `cacheAhead` property):
                // grows while the torrent delivers data faster than playback
                // consumes it, shrinks as the playhead advances, jumps back up
                // on a backward seek inside a cached span. The dot colors by
                // how safely playback can continue: green >= 45s, amber >= 12s,
                // red below.
                Rectangle {
                    visible: bar.torrentStats !== null && bar.dur() > 0
                    implicitWidth: cacheChipRow.implicitWidth + 14
                    height: 22
                    radius: 11
                    color: Theme.glassFillHover
                    border.width: 1
                    border.color: Theme.glassBorder

                    Row {
                        id: cacheChipRow
                        anchors.centerIn: parent
                        spacing: 6

                        Rectangle {
                            width: 6; height: 6; radius: 3
                            anchors.verticalCenter: parent.verticalCenter
                            color: bar.cacheHealthColor(bar.cacheAheadSeconds())
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "Cache " + bar.fmtCacheAhead(bar.cacheAheadSeconds())
                            color: Theme.ink
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }
                }

                // RAM buffer occupancy, RAM streaming only. Shows how much
                // of the shared buffer the pool holds and, when a piece had
                // to be re-downloaded after eviction, an amber dot so the
                // user can tell a hiccup apart from a dying swarm.
                Rectangle {
                    visible: bar.torrentStats !== null && bar.torrentStats.ramBuffer !== undefined
                        && bar.torrentStats.ramBuffer !== null
                    implicitWidth: ramChipRow.implicitWidth + 14
                    height: 22
                    radius: 11
                    color: Theme.glassFillHover
                    border.width: 1
                    border.color: Theme.glassBorder

                    Row {
                        id: ramChipRow
                        anchors.centerIn: parent
                        spacing: 6

                        Rectangle {
                            width: 6; height: 6; radius: 3
                            anchors.verticalCenter: parent.verticalCenter
                            color: (bar.torrentStats !== null
                                    && bar.torrentStats.ramBuffer
                                    && bar.torrentStats.ramBuffer.ghostReads > 0)
                                   ? Theme.healthYellow
                                   : Theme.healthGreen
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: {
                                const rb = bar.torrentStats.ramBuffer
                                return "Buffer " + bar.fmtMib(rb.storedBytes) + " / "
                                    + bar.fmtMib(rb.limitBytes) + " MiB"
                            }
                            color: Theme.ink
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }
                }

                // Download rate. The engine reports one speed per torrent
                // (MiB/s); for a single-episode stream that is this file's
                // rate. "—" while nothing is arriving.
                Rectangle {
                    visible: bar.torrentStats !== null
                    implicitWidth: speedChipRow.implicitWidth + 14
                    height: 22
                    radius: 11
                    color: Theme.glassFillHover
                    border.width: 1
                    border.color: Theme.glassBorder

                    Row {
                        id: speedChipRow
                        anchors.centerIn: parent
                        spacing: 5

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "\u2193"
                            color: Theme.inkMuted
                            font.pixelSize: 11
                            font.bold: true
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: bar.speedLabel()
                            color: Theme.ink
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }
                }

                // Peers currently transferring pieces. "…" while the swarm is
                // still handshaking, "0" once the trackers found nobody.
                Rectangle {
                    visible: bar.torrentStats !== null
                    implicitWidth: peersChipRow.implicitWidth + 14
                    height: 22
                    radius: 11
                    color: Theme.glassFillHover
                    border.width: 1
                    border.color: Theme.glassBorder

                    Row {
                        id: peersChipRow
                        anchors.centerIn: parent
                        spacing: 5

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: bar.peersLabel()
                            color: Theme.ink
                            font.pixelSize: 10
                            font.bold: true
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "peers"
                            color: Theme.inkMuted
                            font.pixelSize: 10
                        }
                    }
                }
            }

            // --- Controls Action Row -------------------------------
            RowLayout {
                Layout.fillWidth: true
                spacing: 6
                visible: !bar.timelineOnly

                Rectangle {
                    width: 32
                    height: 32
                    radius: 10
                    color: playMouse.containsMouse ? Theme.accentHover : Theme.accent

                    PlayerIcon {
                        anchors.centerIn: parent
                        kind: bar.isPaused() ? "play" : "pause"
                        color: "white"
                        width: 14
                        height: 14
                    }

                    MouseArea {
                        id: playMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: if (bar.player) bar.player.command(["cycle", "pause"])
                    }
                }

                Rectangle {
                    height: 32
                    radius: 10
                    color: Theme.glassFillSubtle
                    border.width: 1
                    border.color: Theme.glassBorder
                    implicitWidth: pillRow.implicitWidth + 8

                    Row {
                        id: pillRow
                        anchors.centerIn: parent
                        spacing: 2

                        PillButton {
                            icon: "rewind"
                            tip: "Jump back 5s"
                            onClicked: {
                                if (bar.player) bar.player.command(["seek", "-5", "relative"])
                                bar.triggerOSD("rewind", "-5s")
                            }
                        }
                        PillButton {
                            icon: "prev"
                            tip: "Previous frame"
                            onClicked: {
                                if (bar.player) bar.player.command(["frame-back-step"])
                                bar.triggerOSD("prev", "Step Back")
                            }
                        }
                        PillButton {
                            icon: "next"
                            tip: "Next frame"
                            onClicked: {
                                if (bar.player) bar.player.command(["frame-step"])
                                bar.triggerOSD("next", "Step Forward")
                            }
                        }
                        PillButton {
                            icon: "forward"
                            tip: "Jump forward 5s"
                            onClicked: {
                                if (bar.player) bar.player.command(["seek", "5", "relative"])
                                bar.triggerOSD("forward", "+5s")
                            }
                        }
                    }
                }

                Rectangle {
                    visible: bar.chapterList.length > 0
                    height: 32
                    radius: 10
                    color: Theme.glassFillSubtle
                    border.width: 1
                    border.color: Theme.glassBorder
                    implicitWidth: chapNavRow.implicitWidth + 8

                    Row {
                        id: chapNavRow
                        anchors.centerIn: parent
                        spacing: 2

                        PillButton {
                            icon: "rewind"
                            tip: "Previous chapter"
                            onClicked: {
                                if (bar.player) {
                                    bar.player.command(["add", "chapter", "-1"])
                                    bar.triggerOSD("rewind", "Previous Chapter")
                                }
                            }
                        }
                        PillButton {
                            icon: "forward"
                            tip: "Next chapter"
                            onClicked: {
                                if (bar.player) {
                                    bar.player.command(["add", "chapter", "1"])
                                    bar.triggerOSD("forward", "Next Chapter")
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    height: 32
                    radius: 10
                    color: Theme.glassFillSubtle
                    border.width: 1
                    border.color: Theme.glassBorder
                    implicitWidth: skipRow.implicitWidth + 12

                    RowLayout {
                        id: skipRow
                        anchors.centerIn: parent
                        spacing: 4

                        TextPillButton {
                            label: "Skip OP"
                            tip: "Skip Opening"
                            onClicked: bar.skipIntro()
                        }

                        TextPillButton {
                            label: "Skip ED"
                            tip: "Skip Ending"
                            onClicked: bar.skipEnding()
                        }
                    }
                }

                // Layout Spacer (pushes right-hand buttons to the right side)
                Item {
                    Layout.fillWidth: true
                }

                Rectangle {
                    visible: bar.chapterList.length > 0
                    id: chapterMenuBtn
                    height: 32
                    radius: 8
                    color: chapterMenuMouse.containsMouse ? Theme.glassFillHover : Theme.glassFillSubtle
                    border.width: 1
                    border.color: chapterMenuPopup.visible ? Theme.accent : Theme.glassBorder
                    implicitWidth: Math.max(110, chapterMenuLabel.implicitWidth + 24)

                    readonly property string currentTitle: {
                        if (bar.chapterList.length === 0) return ""
                        const p = bar.pos()
                        var title = bar.chapterList[0].title
                        for (var i = 0; i < bar.chapterList.length; i++) {
                            if (bar.chapterList[i].time <= p) title = bar.chapterList[i].title
                            else break
                        }
                        return title
                    }

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 4
                        Text {
                            id: chapterMenuLabel
                            text: chapterMenuBtn.currentTitle || "Chapters"
                            color: Theme.ink
                            font.pixelSize: 10
                            font.bold: true
                            elide: Text.ElideRight
                            Layout.maximumWidth: 120
                        }
                        Text {
                            text: "▾"
                            color: Theme.inkMuted
                            font.pixelSize: 9
                        }
                    }

                    MouseArea {
                        id: chapterMenuMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (chapterMenuPopup.visible) chapterMenuPopup.close()
                            else chapterMenuPopup.open()
                        }
                    }

                    Popup {
                        id: chapterMenuPopup
                        y: -height - 6
                        readonly property real btnLeftInBar: chapterMenuBtn.mapToItem(bar, 0, 0).x
                        x: Math.min(0, (bar.width - 8) - btnLeftInBar - width)
                        width: Math.max(220, chapterMenuBtn.width)
                        height: Math.min(320, chapMenuColumn.implicitHeight + 16)
                        modal: false
                        focus: true
                        closePolicy: Popup.CloseOnEscape
                        background: Rectangle {
                            radius: 10
                            // A panel, not a scrim over artwork, so it follows
                            // the theme like the rest of the player chrome.
                            color: Theme.surfaceRaised
                            border.width: 1
                            border.color: Theme.borderSubtle
                        }
                        padding: 8

                        Flickable {
                            anchors.fill: parent
                            contentHeight: chapMenuColumn.implicitHeight
                            clip: true
                            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                            Column {
                                id: chapMenuColumn
                                width: parent.width
                                spacing: 2

                                Repeater {
                                    model: bar.chapterList
                                    delegate: Rectangle {
                                        required property var modelData
                                        required property int index
                                        width: chapMenuColumn.width
                                        height: 28
                                        radius: 6
                                        color: chapRowMouse.containsMouse ? Theme.glassFillHover : "transparent"

                                        RowLayout {
                                            anchors.fill: parent
                                            anchors.leftMargin: 8
                                            anchors.rightMargin: 8
                                            spacing: 8
                                            Text {
                                                text: bar.fmt(modelData.time)
                                                color: Theme.accent
                                                font.pixelSize: 10
                                                font.bold: true
                                                font.family: "monospace"
                                                Layout.preferredWidth: 48
                                            }
                                            Text {
                                                text: modelData.title
                                                color: Theme.ink
                                                font.pixelSize: 10
                                                elide: Text.ElideRight
                                                Layout.fillWidth: true
                                            }
                                        }

                                        MouseArea {
                                            id: chapRowMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (bar.player) {
                                                    bar.player.setProperty("chapter", index)
                                                    bar.triggerOSD("forward", modelData.title)
                                                }
                                                chapterMenuPopup.close()
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    height: 32
                    radius: 10
                    color: Theme.glassFillSubtle
                    border.width: 1
                    border.color: Theme.glassBorder
                    implicitWidth: volRow.implicitWidth + 14

                    RowLayout {
                        id: volRow
                        anchors.centerIn: parent
                        spacing: 4

                        PillButton {
                            icon: bar.isMuted() ? "mute" : "volume"
                            tip: bar.isMuted() ? "Unmute" : "Mute"
                            onClicked: {
                                if (bar.player) {
                                    const nextMute = !bar.isMuted()
                                    bar.player.setProperty("mute", nextMute)
                                    bar.triggerOSD(nextMute ? "mute" : "volume", nextMute ? "Muted" : Math.round(bar.vol()) + "%")
                                }
                            }
                        }

                        ThemedSlider {
                            id: volSlider
                            Layout.preferredWidth: 52
                            implicitHeight: 18
                            from: 0
                            // Matches mpv's `volume-max`, raised to 200 in
                            // mpvitem.cpp. Keep the two in step or the slider
                            // offers a range mpv silently clamps away.
                            to: 200
                            value: bar.vol()
                            onMoved: {
                                if (bar.player) {
                                    bar.player.setProperty("volume", value)
                                    bar.triggerOSD(value <= 0 ? "mute" : "volume", Math.round(value) + "%")
                                }
                            }
                        }

                        Text {
                            text: bar.volReadout()
                            color: Theme.inkFaint
                            font.pixelSize: 10
                            font.bold: true
                            Layout.preferredWidth: 30
                            horizontalAlignment: Text.AlignRight
                        }
                    }
                }

                SquareButton {
                    icon: "info"
                    tip: "Video details"
                    onClicked: bar.openInfo()
                }

                SquareButton {
                    icon: "gear"
                    tip: "Player settings"
                    onClicked: bar.openSettings()
                }

                SquareButton {
                    icon: bar.fullscreen ? "exit-fullscreen" : "fullscreen"
                    tip: bar.fullscreen ? "Exit Fullscreen" : "Fullscreen"
                    onClicked: bar.toggleFullscreen()
                }
            }
        }
    }

    component PillButton: Rectangle {
        property string icon: ""
        property string tip: ""
        signal clicked()

        width: 26
        height: 26
        radius: 6
        color: btnMouse.containsMouse ? Theme.glassFillHover : "transparent"

        PlayerIcon {
            anchors.centerIn: parent
            kind: parent.icon
            color: btnMouse.containsMouse ? Theme.accent : Theme.inkMuted
            width: 12
            height: 12
        }

        MouseArea {
            id: btnMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }

        ToolTip.visible: btnMouse.containsMouse && tip.length > 0
        ToolTip.text: tip
        ToolTip.delay: 500
    }

    component TextPillButton: Rectangle {
        property string label: ""
        property string tip: ""
        signal clicked()

        implicitWidth: btnText.implicitWidth + 10
        height: 24
        radius: 6
        color: textBtnMouse.containsMouse ? Theme.glassFillHover : "transparent"

        Text {
            id: btnText
            anchors.centerIn: parent
            text: parent.label
            color: textBtnMouse.containsMouse ? Theme.accent : Theme.inkMuted
            font.pixelSize: 10
            font.bold: true
        }

        MouseArea {
            id: textBtnMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }

        ToolTip.visible: textBtnMouse.containsMouse && tip.length > 0
        ToolTip.text: tip
        ToolTip.delay: 500
    }

    component SquareButton: Rectangle {
        id: sqBtn
        property string icon: ""
        property string tip: ""
        signal clicked()

        width: 32
        height: 32
        radius: 10
        color: sqMouse.containsMouse ? Theme.glassFillHover : Theme.glassFillSubtle
        border.width: 1
        border.color: sqMouse.containsMouse ? Theme.accent : Theme.glassBorder

        Behavior on color { ColorAnimation { duration: 120 } }
        Behavior on border.color { ColorAnimation { duration: 120 } }

        PlayerIcon {
            anchors.centerIn: parent
            kind: sqBtn.icon
            color: sqMouse.containsMouse ? Theme.accent : Theme.ink
            width: 14
            height: 14
        }

        MouseArea {
            id: sqMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: sqBtn.clicked()
        }

        ToolTip.visible: sqMouse.containsMouse && tip.length > 0
        ToolTip.text: tip
        ToolTip.delay: 500
    }
}