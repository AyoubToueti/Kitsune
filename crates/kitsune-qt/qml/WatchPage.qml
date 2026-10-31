// The watch page: pick a release, pick a file, play it in the embedded libmpv.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: page

    property int animeId: 0
    property string animeProvider: "anilist"
    property int malId: 0
    property int episodeNumber: 0
    property int absoluteEpisode: 0
    property var backend: null
    property string streamUrl: ""

    signal back()

    // --- UI Visibility & Timers -----------------------------------------------
    property bool fullUiVisible: true
    property bool seekOnlyVisible: false
    property bool peekLatched: false

    Timer {
        id: fullUiTimer
        interval: 3000
        onTriggered: {
            if (playerBar.hovered || playerBar.chapterMenuOpen || headerHover.hovered) {
                fullUiTimer.restart()
                return
            }
            page.fullUiVisible = false
        }
    }

    Timer {
        id: seekOnlyTimer
        interval: 1500
        onTriggered: page.seekOnlyVisible = false
    }

    function wakeUi() {
        page.fullUiVisible = true
        page.seekOnlyVisible = false
        fullUiTimer.restart()
    }

    function seekRelative(seconds) {
        if (player && page.stage === "play") {
            player.command(["seek", String(seconds), "relative"])
            watchOsd.trigger(seconds > 0 ? "forward" : "rewind", (seconds > 0 ? "+" : "") + seconds + "s")
            
            if (!page.fullUiVisible) {
                page.peekLatched = true
                page.seekOnlyVisible = true
                seekOnlyTimer.restart()
            }
        }
    }

    // --- State ---------------------------------------------------------------

    property string stage: "releases"
    property var releases: []
    property bool searching: false
    property bool searched: false
    property string error: ""

    property int searchGeneration: 0
    readonly property string searchTag: String(searchGeneration)

    property string releaseTitle: ""
    property int torrentId: -1
    property var files: []
    property int chosenIdx: -1
    property string playingUrl: ""

    property var progress: null
    property bool paused: false
    property var titles: []

    function displayTitle(item) {
        return TitlePreference.title(item)
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

        function onReleasesReady(tag, json) {
            if (tag !== page.searchTag) return
            page.searching = false
            page.searched = true
            page.releases = JSON.parse(json) || []
            if (page.releases.length === 0) {
                page.error = "No releases found for this episode."
            }
        }

        function onReleasesFailed(tag, message) {
            if (tag !== page.searchTag) return
            page.searching = false
            page.searched = true
            page.error = message
        }

        function onTorrentAdded(json) {
            const handle = JSON.parse(json)
            if (handle === null) return
            page.torrentId = handle.id
            page.files = handle.files || []
            page.stage = "files"
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
            page.wakeUi()
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

    function offset() {
        return Math.max(0, page.absoluteEpisode - page.episodeNumber)
    }

    function search(force) {
        if (!backend || titles.length === 0) {
            page.error = "No title to search with."
            return
        }
        page.searchGeneration += 1
        page.searching = true
        page.searched = false
        page.error = ""
        backend.searchReleases(
            page.searchTag,
            JSON.stringify(titles),
            page.episodeNumber,
            page.absoluteEpisode,
            "episodes",
            force === true)
    }

    function pickRelease(release) {
        page.releaseTitle = release.title || ""
        page.error = ""
        page.searching = true
        backend.addMagnet(release.magnetUri)
    }

    function bufferDetail(p) {
        if (!p) return ""
        const parts = []
        const forward = p.cacheForwardBytes || 0
        parts.push(forward > 0
            ? (forward / 1048576).toFixed(1) + " MiB buffered ahead"
            : "waiting for the first pieces")
        if (page.progress !== null) {
            const total = page.progress.totalBytes || 0
            const done = page.progress.progressBytes || 0
            if (total > 0) parts.push("download " + Math.floor((done / total) * 100) + "%")
            const speed = page.progress.downloadMbps || 0
            if (speed > 0) parts.push(speed.toFixed(1) + " MiB/s")
        }
        return parts.join("  \u00b7  ")
    }

    function play(idx) {
        if (torrentId < 0) return
        page.chosenIdx = idx
        backend.prepareStream(torrentId, idx)
    }

    function togglePause() {
        if (torrentId < 0 && page.stage !== "play") return
        if (player) {
            const willPause = !player.paused
            player.command(["cycle", "pause"])
            watchOsd.trigger(willPause ? "pause" : "play", willPause ? "Paused" : "Playing")
        }
        keyScope.forceActiveFocus()
        if (torrentId >= 0) {
            if (paused) {
                backend.resumeTorrent(torrentId)
                paused = false
            } else {
                backend.pauseTorrent(torrentId)
                paused = true
            }
        }
    }

    function teardown() {
        if (torrentId >= 0) {
            backend.releaseTorrent(torrentId)
            backend.removeTorrent(torrentId)
            torrentId = -1
        }
    }

    Timer {
        interval: 500
        running: page.torrentId >= 0
        repeat: true
        onTriggered: page.backend.getTorrentStats(page.torrentId)
    }

    Component.onCompleted: {
        if (streamUrl.length > 0) {
            page.playingUrl = streamUrl
            page.stage = "play"
            page.wakeUi()
            player.command(["loadfile", streamUrl, "replace"])
        } else if (animeId > 0) {
            backend.getAnime(animeId, page.animeProvider)
        }
    }

    onStageChanged: if (page.stage === "play") keyScope.forceActiveFocus()

    FocusScope {
        id: keyScope
        anchors.fill: parent
        focus: page.stage === "play"

        Keys.onSpacePressed: page.togglePause()
        Keys.onLeftPressed: page.seekRelative(-5)
        Keys.onRightPressed: page.seekRelative(5)
        Keys.onUpPressed: {
            if (player) {
                const next = Math.min(200, player.volume + 5)
                player.setProperty("volume", String(Math.round(next)))
                watchOsd.trigger("volume", Math.round(next) + "%")
            }
        }
        Keys.onDownPressed: {
            if (player) {
                const next = Math.max(0, player.volume - 5)
                player.setProperty("volume", String(Math.round(next)))
                watchOsd.trigger("volume", Math.round(next) + "%")
            }
        }
    }

    Item {
        anchors.fill: parent

        // --- Video Playback View ---
        Item {
            anchors.fill: parent
            visible: page.stage === "play"

            MpvObject {
                id: player
                anchors.fill: parent
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
                property real lastX: -1
                property real lastY: -1
                onPositionChanged: function (mouse) {
                    if (page.stage !== "play") return
                    if (Math.abs(mouse.x - lastX) < 3 && Math.abs(mouse.y - lastY) < 3) return
                    lastX = mouse.x
                    lastY = mouse.y
                    page.wakeUi()
                }
                onWheel: function (wheel) {
                    if (page.stage !== "play") return
                    const notches = wheel.angleDelta.y / 120
                    if (notches === 0) return

                    const next = Math.max(0, Math.min(200, player.volume + notches * 5))
                    player.setProperty("volume", String(Math.round(next)))
                    if (next > 0 && player.muted)
                        player.setProperty("mute", false)
                    watchOsd.trigger("volume", Math.round(next) + "%")
                    wheel.accepted = true
                }
            }

            Rectangle {
                id: watchOsd
                anchors.centerIn: parent
                z: 20
                height: 56
                radius: 28
                implicitWidth: Math.max(56, watchOsdRow.implicitWidth + 28)
                color: Qt.rgba(0.07, 0.07, 0.11, 0.82)
                border.width: 1.5
                border.color: Theme.accent
                opacity: 0.0
                visible: opacity > 0.0

                property string iconKind: "volume"
                property string labelText: ""

                function trigger(kind, label) {
                    iconKind = kind
                    labelText = label || ""
                    watchOsdAnim.restart()
                }

                Row {
                    id: watchOsdRow
                    anchors.centerIn: parent
                    spacing: 8

                    PlayerIcon {
                        anchors.verticalCenter: parent.verticalCenter
                        kind: watchOsd.iconKind
                        color: Theme.accent
                        width: 22
                        height: 22
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: watchOsd.labelText.length > 0
                        text: watchOsd.labelText
                        color: Theme.ink
                        font.pixelSize: 13
                        font.bold: true
                    }
                }

                ParallelAnimation {
                    id: watchOsdAnim
                    NumberAnimation { target: watchOsd; property: "opacity"; from: 1.0; to: 0.0; duration: 650; easing.type: Easing.OutQuad }
                    NumberAnimation { target: watchOsd; property: "scale"; from: 0.9; to: 1.08; duration: 650; easing.type: Easing.OutQuad }
                }
            }

            Item {
                anchors.centerIn: parent
                z: 15
                width: Math.min(340, parent.width - 64)
                height: watchBufferFox.implicitHeight + 24
                visible: player.bufferingDelayed === true

                Rectangle {
                    anchors.fill: parent
                    radius: 16
                    color: Qt.rgba(0, 0, 0, 0.55)
                    border.width: 1
                    border.color: Theme.glassBorder
                }

                ErrorState {
                    id: watchBufferFox
                    anchors.centerIn: parent
                    width: parent.width - 24
                    compact: true
                    kind: "loading"
                    showActions: false
                    title: "Buffering\u2026"
                    message: page.bufferDetail(player)
                    artWidth: 88
                }
            }

            PlayerControls {
                id: playerBar
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.margins: 14
                z: 25
                player: player
                playingUrl: page.playingUrl
                fullscreen: false
                opacity: (page.stage === "play" && (page.fullUiVisible || page.seekOnlyVisible)) ? 1.0 : 0.0
                visible: opacity > 0.0
                timelineOnly: page.peekLatched && !page.fullUiVisible
                onVisibleChanged: if (!visible) page.peekLatched = false

                Behavior on opacity { NumberAnimation { duration: 150 } }

                onOpenSettings: {}
                onToggleFullscreen: {}
                onTriggerOSD: (icon, label) => watchOsd.trigger(icon, label)
                onSkipIntro: page.seekRelative(85)
                onSkipEnding: page.seekRelative(85)
            }
        }

        // --- Top Floating Header Bar (Overlay) ---
        Rectangle {
            id: topHeader
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 56
            z: 30
            color: page.stage === "play" ? Qt.rgba(0.07, 0.07, 0.11, 0.75) : Theme.surface
            opacity: (page.stage !== "play" || page.fullUiVisible) ? 1.0 : 0.0
            visible: opacity > 0.0

            Behavior on opacity { NumberAnimation { duration: 150 } }

            HoverHandler { id: headerHover }

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

                NavButton {
                    visible: page.torrentId >= 0
                    label: page.paused ? "Resume" : "Pause"
                    onClicked: page.togglePause()
                }
            }
        }

        // --- Release & File Selection Views ---
        Item {
            anchors.top: topHeader.bottom
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            visible: page.stage !== "play"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Theme.pad
                spacing: 12

                Text {
                    text: page.stage === "files" ? "Files in torrent" : "Releases"
                    color: Theme.ink
                    font.pixelSize: 18
                    font.bold: true
                }

                ErrorState {
                    Layout.fillWidth: true
                    Layout.preferredHeight: implicitHeight
                    visible: page.stage === "releases" && page.searching
                    compact: true
                    kind: "loading"
                    showActions: false
                    title: "Searching indexers\u2026"
                    message: ""
                    artWidth: 96
                }

                ErrorState {
                    readonly property bool noHits: page.error.toLowerCase().indexOf("no ") === 0

                    visible: page.stage === "releases" && !page.searching
                        && page.searched
                        && (page.error.length > 0 || page.releases.length === 0)
                    Layout.fillWidth: true
                    Layout.preferredHeight: implicitHeight
                    compact: true
                    artWidth: 96
                    kind: page.error.length > 0 ? "" : "sleeping"
                    failureMessage: page.error
                    title: noHits ? "No releases found" : ""
                    message: noHits
                        ? "Nothing on the indexers matched this episode. Try another search."
                        : ""
                    primaryText: "Retry"
                    onPrimaryClicked: page.search(true)
                }

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