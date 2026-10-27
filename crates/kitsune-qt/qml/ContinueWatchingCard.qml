import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: card

    // A `ContinueWatchingItem`: { anime, progress }
    required property var entry
    // Where the reader stopped, from the local watch-position store, or null
    property var position: null

    // `provider` is the id's namespace, forwarded from the entry's own `Anime`.
    signal openAnime(int id, string provider)
    signal openWatch(int animeId, int episode)

    readonly property var anime: entry ? entry.anime : null
    readonly property string title: TitlePreference.title(anime)
    readonly property int episode: Math.max(1, entry ? entry.progress : 1)
    readonly property real percent: {
        if (!anime || !anime.episodeCount) return 0
        return Math.min(100, Math.round((episode / anime.episodeCount) * 100))
    }

    function fmtTime(seconds) {
        if (!seconds || seconds <= 0 || !isFinite(seconds)) return "00:00"
        const s = Math.floor(seconds % 60)
        const m = Math.floor((seconds / 60) % 60)
        const h = Math.floor(seconds / 3600)
        const ss = s < 10 ? "0" + s : "" + s
        const mm = m < 10 ? "0" + m : "" + m
        return (h > 0 ? h + ":" + mm : mm) + ":" + ss
    }

    width: 145
    height: 240

    HoverHandler { id: hover }

    Column {
        anchors.fill: parent
        spacing: 8

        // --- Poster Frame ---------------------------------------------
        Rectangle {
            id: poster
            width: parent.width
            height: 180
            //radius: 12
            color: Theme.surfaceRaised
            clip: true
            // no border on this rectangle

            // Image Cover with smooth scale zoom on hover
            Image {
                anchors.fill: parent
                source: card.anime ? (card.anime.coverImage || "") : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 320
                scale: hover.hovered ? 1.06 : 1.0

                Behavior on scale { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
            }

            // Dark Scrim Gradient Overlay for contrast
            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.45) }
                    GradientStop { position: 0.4; color: "transparent" }
                    GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.85) }
                }
            }

            // Remove-from-list button (Top-Left)
            Rectangle {
                id: removeBtn
                visible: card.anime && ListStatusStore.isListed(card.anime.id)
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: 8
                width: 20
                height: 20
                radius: 10
                z: 40
                color: removeMouse.containsMouse
                    ? Theme.danger
                    : Qt.rgba(0.07, 0.08, 0.12, 0.82)
                border.width: 1
                border.color: removeMouse.containsMouse ? Theme.danger : Theme.glassBorder
                scale: removeMouse.containsMouse ? 1.1 : 1.0

                Behavior on color { ColorAnimation { duration: 140 } }
                Behavior on scale { NumberAnimation { duration: 120 } }

                Text {
                    anchors.centerIn: parent
                    text: "\u2715"
                    color: removeMouse.containsMouse ? "white" : Theme.inkMuted
                    font.pixelSize: 10
                    font.bold: true
                }

                MouseArea {
                    id: removeMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: ListStatusStore.remove(card.anime.id)
                }
            }

            // Episode Number Badge (Top-Right)
            Rectangle {
                id: epBadge
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                radius: 6
                color: Qt.rgba(0.07, 0.08, 0.12, 0.8)
                border.width: 1
                border.color: Theme.glassBorder
                implicitWidth: epLabel.implicitWidth + 12
                implicitHeight: 20

                Text {
                    id: epLabel
                    anchors.centerIn: parent
                    text: "EP " + card.episode
                    color: Theme.accent
                    font.pixelSize: 10
                    font.bold: true
                    font.letterSpacing: 0.6
                }
            }

            // Click poster background opens the anime's details page.
            MouseArea {
                id: posterMouse
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton
                cursorShape: Qt.PointingHandCursor
                onClicked: card.openAnime(
                    card.anime ? card.anime.id : 0,
                    (card.anime && card.anime.provider) ? card.anime.provider : "anilist")
            }

            // Prominent Floating Glass Pill Overlay (Resume Watching)
            Rectangle {
                id: resumePill
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.margins: 10
                height: 34
                radius: 17
                z: 30
                color: pillMouse.containsMouse ? Theme.accent : Qt.rgba(0.07, 0.09, 0.15, 0.88)
                border.width: 1
                border.color: pillMouse.containsMouse ? Theme.accent : Qt.rgba(1, 1, 1, 0.25)
                opacity: hover.hovered ? 1.0 : 0.0
                transform: Translate { y: hover.hovered ? 0 : 8 }

                Behavior on opacity { NumberAnimation { duration: 180 } }
                Behavior on transform { YAnimator { duration: 200; easing.type: Easing.OutCubic } }

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 6

                    PlayerIcon {
                        kind: "play"
                        color: "white"
                        width: 11
                        height: 11
                    }

                    Text {
                        text: "Resume EP " + card.episode
                        color: "white"
                        font.pixelSize: 11
                        font.bold: true
                    }
                }

                MouseArea {
                    id: pillMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: card.openWatch(card.anime ? card.anime.id : 0, card.episode)
                }
            }

            // ── Border overlay — on top of everything ─────────────────────
            Rectangle {
                anchors.fill: parent
                color: "transparent"
                border.width: 1
                border.color: hover.hovered ? Theme.accent : Theme.glassBorder
                z: 1000
                Behavior on border.color { ColorAnimation { duration: 180 } }
            }
        }

        // --- Card Details Column --------------------------------------
        ColumnLayout {
            width: parent.width
            spacing: 3

            // Title
            Text {
                Layout.fillWidth: true
                text: card.title
                color: hover.hovered ? Theme.accent : Theme.ink
                font.pixelSize: 12
                font.bold: true
                elide: Text.ElideRight
                maximumLineCount: 1

                Behavior on color { ColorAnimation { duration: 150 } }
            }

            // EP and Time Progress Row
            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "EP " + card.episode
                    color: Theme.inkMuted
                    font.pixelSize: 11
                    font.bold: true
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: card.position && card.position.durationSeconds
                        ? (card.fmtTime(card.position.positionSeconds) + " / " + card.fmtTime(card.position.durationSeconds))
                        : (card.fmtTime(0) + " / " + card.fmtTime(0))
                    color: Theme.accent
                    font.pixelSize: 11
                    font.bold: true
                    font.family: "monospace"
                }
            }

            // Full-Width Progress Bar Track
            Rectangle {
                Layout.fillWidth: true
                Layout.topMargin: 2
                height: 2
                radius: 1
                color: Qt.rgba(1, 1, 1, 0.15)

                readonly property real progressRatio: {
                    if (card.position && card.position.durationSeconds > 0) {
                        return Math.min(1.0, Math.max(0.0, card.position.positionSeconds / card.position.durationSeconds))
                    }
                    return Math.min(1.0, Math.max(0.0, card.percent / 100))
                }

                Rectangle {
                    width: Math.max(0, parent.width * parent.progressRatio)
                    height: parent.height
                    radius: parent.radius
                    color: Theme.accent
                }
            }
        }
    }
}