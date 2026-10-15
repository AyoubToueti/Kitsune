import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Item {
    id: card

    // A `ContinueWatchingItem`: { anime, progress }[cite: 7].
    required property var entry
    // Where the reader stopped, from the local watch-position store, or null[cite: 7].
    property var position: null

    signal openAnime(int id)
    signal openWatch(int animeId, int episode)

    readonly property var anime: entry ? entry.anime : null
    readonly property string title: {
        if (!anime) return "Untitled"
        const t = anime.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "Untitled"
    }
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
    height: 235

    HoverHandler { id: hover }

    Column {
        anchors.fill: parent
        spacing: 8

        // --- Poster Frame ---------------------------------------------
        Rectangle {
            id: poster
            width: parent.width
            height: 185
            radius: 12
            color: Theme.surfaceRaised
            clip: true
            border.width: 1
            border.color: hover.hovered ? Theme.accent : Theme.glassBorder

            Behavior on border.color { ColorAnimation { duration: 180 } }

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

            // Episode Number Badge (Top-Left)
            Rectangle {
                anchors.top: parent.top
                anchors.left: parent.left
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

            // Score Badge (Top-Right)
            Rectangle {
                visible: card.anime && card.anime.averageScore != null
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                radius: 6
                color: Qt.rgba(0.07, 0.08, 0.12, 0.8)
                border.width: 1
                border.color: Theme.glassBorder
                implicitWidth: scoreLabel.implicitWidth + 12
                implicitHeight: 20

                Text {
                    id: scoreLabel
                    anchors.centerIn: parent
                    text: "★ " + (card.anime ? card.anime.averageScore : "")
                    color: Theme.score
                    font.pixelSize: 10
                    font.bold: true
                }
            }

            // Hover Central Play Button Overlay
            Rectangle {
                anchors.centerIn: parent
                width: 44
                height: 44
                radius: 22
                color: playBtnMouse.containsMouse ? Theme.accentHover : Theme.accent
                opacity: hover.hovered ? 1.0 : 0.0
                scale: hover.hovered ? 1.0 : 0.85

                Behavior on opacity { NumberAnimation { duration: 180 } }
                Behavior on scale { NumberAnimation { duration: 200; easing.type: Easing.OutBack } }

                PlayerIcon {
                    anchors.centerIn: parent
                    anchors.horizontalCenterOffset: 1
                    kind: "play"
                    color: "white"
                    width: 18
                    height: 18
                }

                MouseArea {
                    id: playBtnMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: card.openWatch(card.anime.id, card.episode)
                }
            }

            // Bottom Quick Action Bar
            Rectangle {
                id: quickBar
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: progressTrack.top
                anchors.margins: 6
                height: 28
                radius: 8
                color: Qt.rgba(0.07, 0.08, 0.12, 0.9)
                border.width: 1
                border.color: Theme.glassBorder
                opacity: hover.hovered ? 1.0 : 0.0
                y: hover.hovered ? 0 : 4

                Behavior on opacity { NumberAnimation { duration: 160 } }
                Behavior on y { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 4

                    Text {
                        text: "Resume"
                        color: Theme.ink
                        font.pixelSize: 11
                        font.bold: true
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        width: 20
                        height: 20
                        radius: 5
                        color: detailsMouse.containsMouse ? Theme.glassFillHover : Theme.glassFillSubtle

                        PlayerIcon {
                            anchors.centerIn: parent
                            kind: "gear"
                            color: Theme.inkMuted
                            width: 11
                            height: 11
                        }

                        MouseArea {
                            id: detailsMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: card.openAnime(card.anime.id)
                        }
                    }
                }
            }

            // Progress Bar Track (Bottom Edge)
            Rectangle {
                id: progressTrack
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 4
                color: Qt.rgba(0, 0, 0, 0.6)

                Rectangle {
                    width: parent.width * (card.percent / 100)
                    height: parent.height
                    color: Theme.accent
                }
            }

            // Click poster frame opens anime details page
            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton
                cursorShape: Qt.PointingHandCursor
                onClicked: card.openAnime(card.anime.id)
            }
        }

        // --- Card Details Label ---------------------------------------
        Column {
            width: parent.width
            spacing: 2

            Text {
                width: parent.width
                text: card.title
                color: hover.hovered ? Theme.accent : Theme.ink
                font.pixelSize: 12
                font.bold: true
                elide: Text.ElideRight
                maximumLineCount: 1

                Behavior on color { ColorAnimation { duration: 150 } }
            }

            Text {
                width: parent.width
                text: card.position
                    ? (card.fmtTime(card.position.positionSeconds) + " / " + card.fmtTime(card.position.durationSeconds))
                    : ("Episode " + card.episode + " • " + card.percent + "%")
                color: Theme.inkMuted
                font.pixelSize: 10
                font.family: "monospace"
                elide: Text.ElideRight
            }
        }
    }
}