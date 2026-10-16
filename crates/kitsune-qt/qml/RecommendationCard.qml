// One recommendation: the recommended work's poster, its score, the community
// vote tally with vector up/down arrows, and the bottom-right quick action stack.
// Deliberately does NOT show who suggested it -- the reader only needs the
// work and how the community rates the suggestion. The tally is optimistic:
// it moves on click and is corrected by `recommendationRated`, or reverted on
// `recommendationRateFailed`.
import QtQuick
import QtQuick.Controls
import com.kitsune.app

Item {
    id: card

    // A `RecommendedAnime`: { anime, rating, userRating? }.
    required property var recommendation
    property int mediaId: 0
    property bool votable: false
    property var backend: null

    signal openAnime(int id)

    readonly property var anime: recommendation ? recommendation.anime : null
    readonly property int animeId: anime ? (anime.id || 0) : 0
    readonly property string title: {
        if (!anime) return "Untitled"
        const t = anime.title || {}
        return t.userPreferred || t.english || t.romaji || t.native || "Untitled"
    }

    // Real-time watch status tracking
    readonly property string currentStatus: (typeof ListStatusStore !== "undefined" && ListStatusStore && ListStatusStore.getStatus && card.animeId > 0)
                                             ? ListStatusStore.getStatus(card.animeId)
                                             : ""
    readonly property bool isOnList: currentStatus !== ""

    // A work still airing cannot sensibly be marked Completed, so its
    // Completed button is hidden.
    readonly property bool canComplete: !anime || anime.status !== "RELEASING"

    // Local copies for optimistic vote tally updates
    property int tally: recommendation ? (recommendation.rating || 0) : 0
    property string userVote: (recommendation && recommendation.userRating)
        ? recommendation.userRating : ""

    onRecommendationChanged: {
        tally = recommendation ? (recommendation.rating || 0) : 0
        userVote = (recommendation && recommendation.userRating)
            ? recommendation.userRating : ""
    }

    implicitWidth: 180
    implicitHeight: width * 1.5 + 84

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
        else if (name === "arrowUp")
            path = "M12 8l-6 6h12z"
        else if (name === "arrowDown")
            path = "M12 16l6-6H6z"
        else // "plus"
            path = "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"

        var xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='white'><path d='" + path + "'/></svg>"
        // URL-encoded, NOT base64: `Qt.btoa` is deprecated in Qt 6 (its
        // output differs from the standard Web API).
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    Connections {
        target: card.backend
        function onRecommendationRated(mediaId, recommendedId, rating) {
            if (mediaId !== card.mediaId) return
            if (!card.anime || recommendedId !== card.anime.id) return
            card.tally = rating
        }
        function onRecommendationRateFailed(message) {
            card.tally = card.recommendation ? (card.recommendation.rating || 0) : 0
            card.userVote = (card.recommendation && card.recommendation.userRating)
                ? card.recommendation.userRating : ""
        }
    }

    function voteLiteral(direction) {
        // camelCase, matching `RecommendationRating`'s serde form (the backend
        // parses this string into that enum). The GraphQL spellings
        // ("RATE_UP") are produced by the enum's own `literal()` on the way
        // out, so they must NOT be sent in.
        if (card.userVote === direction) return "noRating"
        return direction === "rateUp" ? "rateUp" : "rateDown"
    }

    function delta(from, to) {
        const value = (r) => r === "rateUp" ? 1 : (r === "rateDown" ? -1 : 0)
        return value(to) - value(from)
    }

    function vote(direction) {
        if (!card.votable || card.mediaId === 0 || !card.anime) return
        if (typeof ListStatusStore !== "undefined" && !ListStatusStore.signedIn) {
            if (card.backend) {
                const url = card.backend.beginLogin()
                if (url && url.length > 0) Qt.openUrlExternally(url)
            }
            return
        }
        const next = card.voteLiteral(direction)
        card.tally = card.tally + card.delta(card.userVote, next === "noRating" ? "" : next)
        card.userVote = next === "noRating" ? "" : next
        if (card.backend) card.backend.rateRecommendation(card.mediaId, card.anime.id, next)
    }

    HoverHandler { id: hover }

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
            border.color: hover.hovered ? Theme.accent : Theme.glassBorder

            Behavior on border.color { ColorAnimation { duration: 180 } }

            Image {
                anchors.fill: parent
                source: card.anime ? (card.anime.coverImage || "") : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 400
                scale: hover.hovered ? 1.06 : 1.0

                Behavior on scale { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
            }

            // Dark Scrim Overlay
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
                visible: card.anime && card.anime.averageScore != null
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
                    text: "\u2605 " + (card.anime ? card.anime.averageScore : "")
                    color: Theme.score
                    font.pixelSize: 11
                    font.bold: true
                }
            }

            // Click Poster Frame -> Open Anime Details
            MouseArea {
                id: cardMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: if (card.anime) card.openAnime(card.anime.id)
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

                // Collapse when mouse leaves stack bounds completely
                HoverHandler {
                    id: stackHover
                    onHoveredChanged: {
                        if (!hovered) {
                            quickStack.expanded = false
                        }
                    }
                }

                // Continuous hit-box backing area
                Rectangle {
                    anchors.fill: parent
                    color: "transparent"
                }

                // Expanded Action Buttons
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
                        onClicked: if (card.animeId > 0) ListStatusStore.setStatus(card.animeId, "current")
                    }

                    // 2. Completed (Highlights & glows if currentStatus === "completed").
                    // Hidden while the work is still airing.
                    ActionButton {
                        visible: card.canComplete
                        iconName: "check"
                        tipText: "Completed"
                        isActive: card.currentStatus === "completed"
                        onClicked: if (card.animeId > 0) ListStatusStore.setStatus(card.animeId, "completed")
                    }

                    // 3. Plan to Watch (Highlights & glows if currentStatus === "planning")
                    ActionButton {
                        iconName: "calendar"
                        tipText: "Plan to Watch"
                        isActive: card.currentStatus === "planning"
                        onClicked: if (card.animeId > 0) ListStatusStore.setStatus(card.animeId, "planning")
                    }
                }

                // Main Bottom-Right Trigger Button (+ / Edit)
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

                    // Opens menu only when hovering directly on the button
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
            color: hover.hovered ? Theme.accent : Theme.ink
            font.pixelSize: 12
            font.bold: true
            elide: Text.ElideRight
            maximumLineCount: 2
            wrapMode: Text.WordWrap

            Behavior on color { ColorAnimation { duration: 150 } }
        }

        // --- Voting Controls Row --------------------------------------
        Row {
            spacing: 6

            // Up Arrow Vote Button
            Rectangle {
                visible: card.votable
                width: 26; height: 24; radius: 6
                color: card.userVote === "rateUp"
                    ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.25)
                    : (upMouse.containsMouse ? Theme.surfaceHover : "transparent")
                border.width: 1
                border.color: card.userVote === "rateUp" ? Theme.accent : Theme.borderSubtle

                Behavior on color { ColorAnimation { duration: 120 } }

                Image {
                    anchors.centerIn: parent
                    width: 12
                    height: 12
                    source: card.getSvgIcon("arrowUp")
                    sourceSize.width: 24
                    sourceSize.height: 24
                    smooth: true
                }

                MouseArea {
                    id: upMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: card.vote("rateUp")
                }
            }

            // Recommendation Tally Badge
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                radius: 6
                color: Qt.rgba(0.07, 0.08, 0.12, 0.6)
                border.width: 1
                border.color: Theme.glassBorder
                implicitWidth: tallyRow.implicitWidth + 12
                implicitHeight: 22

                Row {
                    id: tallyRow
                    anchors.centerIn: parent
                    spacing: 4

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "\u2605"
                        color: Theme.score
                        font.pixelSize: 10
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: (card.tally > 0 ? "+" : "") + card.tally
                        color: card.tally > 0 ? Theme.accent : (card.tally < 0 ? Theme.danger : Theme.inkMuted)
                        font.pixelSize: 11
                        font.bold: true
                    }
                }
            }

            // Down Arrow Vote Button
            Rectangle {
                visible: card.votable
                width: 26; height: 24; radius: 6
                color: card.userVote === "rateDown"
                    ? Qt.rgba(Theme.danger.r, Theme.danger.g, Theme.danger.b, 0.25)
                    : (downMouse.containsMouse ? Theme.surfaceHover : "transparent")
                border.width: 1
                border.color: card.userVote === "rateDown" ? Theme.danger : Theme.borderSubtle

                Behavior on color { ColorAnimation { duration: 120 } }

                Image {
                    anchors.centerIn: parent
                    width: 12
                    height: 12
                    source: card.getSvgIcon("arrowDown")
                    sourceSize.width: 24
                    sourceSize.height: 24
                    smooth: true
                }

                MouseArea {
                    id: downMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: card.vote("rateDown")
                }
            }
        }
    }

    // --- Action Button Component with Active Halo Glow --------------------
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