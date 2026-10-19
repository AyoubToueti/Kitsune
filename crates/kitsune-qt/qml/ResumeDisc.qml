import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Effects
import com.kitsune.app

Item {
    id: disc

    // The Backend, so the disc can read last-played + positions itself.
    property var backend: null
    // Whether the disc may show at all (the window decides based on route).
    property bool allowed: true
    // The anime id of the detail page on screen, or 0. The disc hides on its own work's page.
    property int currentDetailId: 0

    // The last-played record: { animeId, episode, at } or null.
    property var lastPlayed: null
    // The work's catalogue entry, fetched for cover + title.
    property var anime: null
    // The saved position for that work.
    property var position: null

    // Emitted when the disc (or its card) is clicked: open the work's page.
    signal openRequested(int animeId, int episode)

    readonly property bool visible_: allowed
        && lastPlayed !== null && anime !== null
        && (currentDetailId === 0 || currentDetailId !== lastPlayed.animeId)
    visible: visible_

    width: 64
    height: 64

    function load() {
        if (!disc.backend) return
        disc.backend.getLastPlayed()
        disc.backend.getWatchPositions()
    }

    Connections {
        target: disc.backend

        function onLastPlayedReady(json) {
            const record = JSON.parse(json)
            disc.lastPlayed = record
            if (record && record.animeId) disc.backend.getAnime(record.animeId)
            else disc.anime = null
        }
        function onAnimeReady(json) {
            const parsed = JSON.parse(json)
            if (!parsed) return
            if (disc.lastPlayed && parsed.id === disc.lastPlayed.animeId) disc.anime = parsed
        }
        function onWatchPositionsReady(json) {
            const all = JSON.parse(json) || {}
            disc.position = disc.lastPlayed ? (all[String(disc.lastPlayed.animeId)] || null) : null
        }
        function onLibraryChanged() { disc.load() }
    }

    onBackendChanged: if (backend) load()
    Component.onCompleted: load()
    onAllowedChanged: if (allowed) load()

    function fmtTime(seconds) {
        if (!seconds || seconds <= 0 || !isFinite(seconds)) return "00:00"
        const s = Math.floor(seconds % 60)
        const m = Math.floor((seconds / 60) % 60)
        const h = Math.floor(seconds / 3600)
        const ss = s < 10 ? "0" + s : "" + s
        const mm = m < 10 ? "0" + m : "" + m
        return (h > 0 ? h + ":" + mm : mm) + ":" + ss
    }

    function relativeTime(atSeconds) {
        if (!atSeconds) return "just now"
        const now = Math.floor(Date.now() / 1000)
        const diff = Math.max(0, now - atSeconds)
        if (diff < 60) return "just now"
        if (diff < 3600) return Math.floor(diff / 60) + "m ago"
        if (diff < 86400) return Math.floor(diff / 3600) + "h ago"
        return Math.floor(diff / 86400) + "d ago"
    }

    function title() {
        if (!disc.anime) return ""
        return TitlePreference.title(disc.anime)
    }

    HoverHandler { id: hover }

    // Smooth turntable spin inertia physics
    property real angle: 0
    property real currentDegPerSecond: 30
    readonly property real targetDegPerSecond: hover.hovered ? 320 : 30

    Timer {
        interval: 16
        repeat: true
        running: disc.visible_
        onTriggered: {
            disc.currentDegPerSecond += (disc.targetDegPerSecond - disc.currentDegPerSecond) * 0.1
            disc.angle = (disc.angle + disc.currentDegPerSecond * 0.016) % 360
        }
    }

    readonly property real progress: {
        const p = disc.position
        if (!p || !p.durationSeconds || p.durationSeconds <= 0) return 0
        const f = p.positionSeconds / p.durationSeconds
        return Math.max(0, Math.min(1, f))
    }

    // Outer Fixed Progress Arc Ring
    Canvas {
        id: progressRing
        anchors.centerIn: parent
        width: 70
        height: 70
        z: 2

        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            const w = width
            const cx = w / 2
            const cy = w / 2
            const lw = 3.5
            const r = cx - lw / 2 - 1

            ctx.beginPath()
            ctx.arc(cx, cy, r, 0, Math.PI * 2)
            ctx.strokeStyle = Qt.rgba(1, 1, 1, 0.12)
            ctx.lineWidth = lw
            ctx.stroke()

            const frac = disc.progress
            if (frac > 0) {
                ctx.beginPath()
                ctx.arc(cx, cy, r, -Math.PI / 2, -Math.PI / 2 + (Math.PI * 2 * frac))
                ctx.strokeStyle = Theme.accent
                ctx.lineWidth = lw
                ctx.lineCap = "round"
                ctx.stroke()
            }
        }

        Connections {
            target: disc
            function onProgressChanged() { progressRing.requestPaint() }
        }
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        Component.onCompleted: requestPaint()
    }

    // Interactive Spinning Vinyl Disc Body
    Item {
        id: face
        anchors.fill: parent
        rotation: disc.angle
        scale: hover.hovered ? 1.08 : 1.0

        Behavior on scale { NumberAnimation { duration: 200; easing.type: Easing.OutBack } }

        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: "#111319"
            border.width: 1
            border.color: hover.hovered ? Theme.accent : "#2a2e3d"
            Behavior on border.color { ColorAnimation { duration: 150 } }

            Rectangle {
                anchors.centerIn: parent
                width: parent.width * 0.88
                height: width
                radius: width / 2
                color: "transparent"
                border.width: 1
                border.color: Qt.rgba(1, 1, 1, 0.06)
            }
            Rectangle {
                anchors.centerIn: parent
                width: parent.width * 0.76
                height: width
                radius: width / 2
                color: "transparent"
                border.width: 1
                border.color: Qt.rgba(1, 1, 1, 0.08)
            }
        }

        Item {
            anchors.centerIn: parent
            width: parent.width * 0.64
            height: width

            Image {
                id: cover
                anchors.fill: parent
                source: disc.anime ? (disc.anime.coverImage || "") : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 128
                visible: false
                layer.enabled: true
            }

            Rectangle {
                id: circleMask
                anchors.fill: parent
                radius: width / 2
                color: "white"
                visible: false
                layer.enabled: true
            }

            MultiEffect {
                source: cover
                anchors.fill: parent
                maskEnabled: true
                maskSource: circleMask
            }
        }

        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: "transparent"
            gradient: Gradient {
                orientation: Gradient.Vertical
                GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0.18) }
                GradientStop { position: 0.4; color: "transparent" }
                GradientStop { position: 0.6; color: "transparent" }
                GradientStop { position: 1.0; color: Qt.rgba(1, 1, 1, 0.08) }
            }
        }

        Rectangle {
            anchors.centerIn: parent
            width: parent.width * 0.16
            height: width
            radius: width / 2
            color: "#0b0d12"
            border.width: 1.5
            border.color: "#ffffff"
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: disc.openRequested(disc.lastPlayed.animeId, disc.lastPlayed.episode || 1)
        }
    }

    Rectangle {
        id: hoverCard
        visible: opacity > 0
        opacity: hover.hovered ? 1.0 : 0.0
        scale: hover.hovered ? 1.0 : 0.95

        anchors.right: face.left
        anchors.rightMargin: 14
        anchors.verticalCenter: face.verticalCenter
        width: 240
        height: cardCol.implicitHeight + 24
        radius: 14
        // A floating panel over the page, not over artwork, so it follows the
        // page palette. The dark literal here made the card unreadable in
        // light mode once `Theme.ink` went near-black.
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.borderSubtle

        Behavior on opacity { NumberAnimation { duration: 180 } }
        Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }

        Column {
            id: cardCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: 12
            spacing: 6

            RowLayout {
                width: parent.width

                Text {
                    text: "CONTINUE WATCHING"
                    color: Theme.accent
                    font.pixelSize: 9
                    font.bold: true
                    font.letterSpacing: 1.0
                    Layout.fillWidth: true
                }

                Rectangle {
                    radius: 4
                    color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.15)
                    implicitWidth: percentText.implicitWidth + 8
                    implicitHeight: 16

                    Text {
                        id: percentText
                        anchors.centerIn: parent
                        text: Math.round(disc.progress * 100) + "%"
                        color: Theme.accent
                        font.pixelSize: 9
                        font.bold: true
                    }
                }
            }

            Text {
                width: parent.width
                text: disc.title()
                color: Theme.ink
                font.pixelSize: 13
                font.bold: true
                elide: Text.ElideRight
            }

            RowLayout {
                width: parent.width

                Text {
                    text: disc.lastPlayed && disc.lastPlayed.episode
                        ? "Episode " + disc.lastPlayed.episode : "Not started"
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    font.bold: true
                    Layout.fillWidth: true
                }

                Text {
                    text: disc.lastPlayed ? disc.relativeTime(disc.lastPlayed.at) : ""
                    color: Theme.inkFaint
                    font.pixelSize: 10
                }
            }

            Rectangle {
                width: parent.width
                height: 3
                radius: 1.5
                color: Qt.rgba(1, 1, 1, 0.1)

                Rectangle {
                    width: parent.width * disc.progress
                    height: parent.height
                    radius: parent.radius
                    color: Theme.accent
                }
            }

            Text {
                text: disc.position
                    ? (disc.fmtTime(disc.position.positionSeconds) + " / " + disc.fmtTime(disc.position.durationSeconds))
                    : "00:00 / 00:00"
                color: Theme.inkFaint
                font.pixelSize: 10
                font.family: "monospace"
            }

            Rectangle { width: parent.width; height: 1; color: Theme.glassBorder }

            RowLayout {
                width: parent.width
                spacing: 6

                Text {
                    text: "Resume Playback"
                    color: cardMouse.containsMouse ? Theme.accent : Theme.ink
                    font.pixelSize: 11
                    font.bold: true
                    Layout.fillWidth: true
                }

                PlayerIcon {
                    kind: "play"
                    color: Theme.accent
                    width: 10
                    height: 10
                }
            }
        }

        MouseArea {
            id: cardMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: disc.openRequested(disc.lastPlayed.animeId, disc.lastPlayed.episode || 1)
        }
    }
}