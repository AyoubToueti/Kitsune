// A single anime poster: cover art, a score badge, and a clamped title.
// Mirrors the Svelte `AnimeCard` / `TrendingCard`.
import QtQuick
import QtQuick.Controls
import com.kitsune.app

Item {
    id: card

    property int animeId: 0
    property string title
    property real score
    property url cover
    property string format
    // The work's airing status in the provider's wording ("RELEASING",
    // "FINISHED", ...), or "" when unknown.
    property string animeStatus: ""

    signal clicked()

    // A work still airing cannot sensibly be marked Completed, so its
    // Completed button is hidden. Anything else (finished, cancelled, or an
    // unknown status) keeps the button.
    readonly property bool canComplete: animeStatus !== "RELEASING"

    implicitWidth: 180
    implicitHeight: width * 1.5 + 52

    // Direct access to the current anime status string
    readonly property string currentStatus: (typeof ListStatusStore !== "undefined" && ListStatusStore && ListStatusStore.getStatus)
                                             ? ListStatusStore.getStatus(card.animeId)
                                             : ""
    readonly property bool isOnList: currentStatus !== ""

    // Pure ASCII Base64 Data URI generator for resolution-independent SVG icons
    function getSvgIcon(name) {
        var path = ""
        if (name === "play")
            path = "M8 5v14l11-7z"
        else if (name === "check")
            path = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
        else if (name === "calendar")
            path = "M19 4h-1V2h-2v2H8V2H6v2H5c-1.11 0-1.99.9-1.99 2L3 20c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 16H5V10h14v10zm0-12H5V6h14v2z"
        else if (name === "edit")
            path = "M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"
        else // "plus"
            path = "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"

        var xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='white'><path d='" + path + "'/></svg>"
        // URL-encoded, NOT base64: `Qt.btoa` is deprecated in Qt 6 (its
        // output differs from the standard Web API), and an SVG data URI
        // does not need base64 to load.
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    HoverHandler { id: posterHover }

    Column {
        anchors.fill: parent
        spacing: 8

        // --- Poster Frame ---------------------------------------------
        Rectangle {
            id: poster
            width: parent.width
            height: parent.width * 1.5
            radius: 12
            color: Theme.surfaceRaised
            clip: true
            border.width: 1
            border.color: posterHover.hovered ? Theme.accent : Theme.glassBorder

            Behavior on border.color { ColorAnimation { duration: 180 } }

            // Cover image with smooth scale zoom on hover
            Image {
                anchors.fill: parent
                source: card.cover
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 400
                scale: posterHover.hovered ? 1.06 : 1.0

                Behavior on scale { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
            }

            // Dark Scrim Gradient Overlay
            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.35) }
                    GradientStop { position: 0.5; color: "transparent" }
                    GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.88) }
                }
            }

            // Score Badge (Top-Right)
            Rectangle {
                visible: card.score > 0
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                radius: 6
                color: Qt.rgba(0.07, 0.08, 0.12, 0.82)
                border.width: 1
                border.color: Theme.glassBorder
                implicitWidth: scoreLabel.implicitWidth + 12
                implicitHeight: 22

                Text {
                    id: scoreLabel
                    anchors.centerIn: parent
                    text: "\u2605 " + card.score
                    color: Theme.score
                    font.pixelSize: 11
                    font.bold: true
                }
            }

            // Format Badge (Bottom-Left)
            Rectangle {
                visible: card.format.length > 0
                anchors.left: parent.left
                anchors.bottom: parent.bottom
                anchors.margins: 8
                radius: 6
                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.88)
                implicitWidth: formatLabel.implicitWidth + 12
                implicitHeight: 20

                Text {
                    id: formatLabel
                    anchors.centerIn: parent
                    text: card.format
                    color: "white"
                    font.pixelSize: 10
                    font.bold: true
                    font.letterSpacing: 0.5
                }
            }

            // Click Poster Frame -> Open Details
            MouseArea {
                id: cardMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: card.clicked()
            }

            // --- Expandable Quick Action Stack (Bottom-Right) -------------
            Item {
                id: quickStack
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.margins: 8
                width: 40
                height: stackCol.implicitHeight + mainBtn.height + 12
                z: 30

                property bool expanded: false

                // Collapse when mouse leaves stack area completely
                HoverHandler {
                    id: stackHover
                    onHoveredChanged: {
                        if (!hovered) {
                            quickStack.expanded = false
                        }
                    }
                }

                // Continuous hit-box area to prevent flickering
                Rectangle {
                    anchors.fill: parent
                    color: "transparent"
                }

                // Expanded Vertical Action Stack
                Column {
                    id: stackCol
                    anchors.bottom: mainBtn.top
                    anchors.bottomMargin: 6
                    anchors.horizontalCenter: mainBtn.horizontalCenter
                    spacing: 6
                    opacity: quickStack.expanded ? 1.0 : 0.0
                    scale: quickStack.expanded ? 1.0 : 0.75
                    visible: opacity > 0
                    transformOrigin: Item.Bottom

                    Behavior on opacity { NumberAnimation { duration: 150 } }
                    Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutBack } }

                    // 1. Watching (Highlights & glows if currentStatus === "current")
                    ActionButton {
                        iconName: "play"
                        tipText: "Watching"
                        isActive: card.currentStatus === "current"
                        // Through the store so the glow updates at once, not
                        // only after the AniList round-trip.
                        onClicked: ListStatusStore.setStatus(card.animeId, "current")
                    }

                    // 2. Completed (Highlights & glows if currentStatus === "completed").
                    // Hidden while the work is still airing: you cannot have
                    // finished something that is not out yet.
                    ActionButton {
                        visible: card.canComplete
                        iconName: "check"
                        tipText: "Completed"
                        isActive: card.currentStatus === "completed"
                        onClicked: ListStatusStore.setStatus(card.animeId, "completed")
                    }

                    // 3. Plan to Watch (Highlights & glows if currentStatus === "planning")
                    ActionButton {
                        iconName: "calendar"
                        tipText: "Plan to Watch"
                        isActive: card.currentStatus === "planning"
                        onClicked: ListStatusStore.setStatus(card.animeId, "planning")
                    }
                }

                // Main Bottom-Right Trigger Button (+ / Edit SVG)
                Rectangle {
                    id: mainBtn
                    anchors.bottom: parent.bottom
                    anchors.right: parent.right
                    width: 36
                    height: 36
                    radius: 18
                    color: quickStack.expanded 
                        ? Theme.accent 
                        : (card.isOnList ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.85) : Qt.rgba(0.12, 0.14, 0.20, 0.92))
                    border.width: 1
                    border.color: Theme.glassBorder

                    // Expands stack strictly when hovering directly over the button
                    HoverHandler {
                        id: mainHover
                        onHoveredChanged: {
                            if (hovered) {
                                quickStack.expanded = true
                            }
                        }
                    }

                    Behavior on color { ColorAnimation { duration: 150 } }

                    Image {
                        anchors.centerIn: parent
                        width: 14
                        height: 14
                        source: card.getSvgIcon(card.isOnList ? "edit" : "plus")
                        sourceSize.width: 28
                        sourceSize.height: 28
                        smooth: true
                    }
                }
            }
        }

        // --- Title Label ----------------------------------------------
        Text {
            width: parent.width
            text: card.title
            color: posterHover.hovered ? Theme.accent : Theme.ink
            font.pixelSize: 13
            font.bold: true
            elide: Text.ElideRight
            maximumLineCount: 2
            wrapMode: Text.WordWrap

            Behavior on color { ColorAnimation { duration: 150 } }
        }
    }

    // --- Reusable SVG Action Button Component with Active Halo Glow ---
    component ActionButton: Item {
        id: actBtn
        property string iconName: ""
        property string tipText: ""
        property bool isActive: false
        signal clicked()

        width: 34
        height: 34

        // Outer Glowing Halo Ring (Visible when isActive === true)
        Rectangle {
            anchors.fill: parent
            anchors.margins: -3
            radius: (parent.width / 2) + 3
            color: "transparent"
            border.width: 2
            border.color: Theme.accent
            opacity: actBtn.isActive ? 0.95 : 0.0

            Behavior on opacity { NumberAnimation { duration: 180 } }
        }

        // Inner Action Button Body
        Rectangle {
            id: btnBody
            anchors.fill: parent
            radius: width / 2
            color: actBtn.isActive 
                ? Theme.accent 
                : (actMouse.containsMouse ? Theme.accentHover : Qt.rgba(0.12, 0.14, 0.20, 0.92))
            border.width: 1
            border.color: actBtn.isActive ? "#ffffff" : Theme.glassBorder

            Behavior on color { ColorAnimation { duration: 120 } }

            Image {
                anchors.centerIn: parent
                width: 14
                height: 14
                source: card.getSvgIcon(actBtn.iconName)
                sourceSize.width: 28
                sourceSize.height: 28
                smooth: true
            }

            MouseArea {
                id: actMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: actBtn.clicked()
            }

            ToolTip.visible: actMouse.containsMouse && tipText.length > 0
            ToolTip.text: tipText
            ToolTip.delay: 200
        }
    }
}