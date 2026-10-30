// The add-to-list stack on a trending card's poster.
//
// One circular button in the card's bottom-right corner expands upward into
// the list choices (Watching / Completed / Plan to watch) and, when the work is
// already listed, shows WHERE it is saved -- play for Watching, a check for
// Completed, a calendar for Plan to watch, pause for On hold, a minus for
// Dropped. Hovering a listed work turns it into a red "x" that removes it.
//
// This is the control the trending rail used to hand-roll inline; it lives here
// so every card overlay agrees about the glyphs, the expansion animation and --
// most importantly -- which store the write goes through. All reads and writes
// go to the shared `ListStatusStore`, so a change made on one card is visible
// on the next repaint of any other.
import QtQuick
import QtQuick.Controls
import com.kitsune.app

Item {
    id: control

    // The work this control adds.
    property int animeId: 0
    // The id's namespace. `animeId` alone is not an identity: the Kitsu
    // fallback hands back Kitsu ids, and the store keys on the pair.
    property string animeProvider: "anilist"
    // The work's airing status ("RELEASING", "FINISHED", ...), or "" when
    // unknown. A still-airing work cannot be marked Completed, so that row is
    // dropped for it.
    property string animeStatus: ""

    // A work still airing cannot sensibly be marked Completed.
    readonly property bool canComplete: animeStatus !== "RELEASING"

    // The current status wire value ("current" / "completed" / ...) or "".
    readonly property string status: ListStatusStore.statusFor(animeId, animeProvider)
    readonly property bool listed: status !== ""

    // Set by hovering the trigger; the stack animates open from it.
    property bool expanded: false

    implicitWidth: 40
    implicitHeight: stackCol.implicitHeight + mainBtn.height + 12

    // Resolution-independent SVG icons as data URIs, so the glyph stays sharp
    // at any scale and the two states (unlisted / listed) never depend on a
    // font shipping a particular character.
    function getSvgIcon(name, color) {
        var fill = color === undefined ? "#ffffff" : Theme.hex(color)
        var path = ""
        if (name === "play")
            path = "M8 5v14l11-7z"
        else if (name === "check")
            path = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
        else if (name === "calendar")
            path = "M19 4h-1V2h-2v2H8V2H6v2H5c-1.11 0-1.99.9-1.99 2L3 20c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 16H5V10h14v10zm0-12H5V6h14v2z"
        else if (name === "pause")
            path = "M6 19h4V5H6v14zm8-14v14h4V5h-4z"
        else if (name === "minus")
            path = "M19 13H5v-2h14v2z"
        else // "plus"
            path = "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"

        var xml = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='" + fill + "'><path d='" + path + "'/></svg>"
        return "data:image/svg+xml," + encodeURIComponent(xml)
    }

    // The glyph for a list status, so a listed work shows *where* it is saved
    // (play / check / calendar / pause / minus) instead of a generic pencil.
    function statusIconName(value) {
        if (value === "current") return "play"
        if (value === "completed") return "check"
        if (value === "planning") return "calendar"
        if (value === "paused") return "pause"
        if (value === "dropped") return "minus"
        return "plus"
    }

    // Write a list status for this work.
    //
    // Deliberately NOT gated on being signed in: the list is a local file, so
    // an offline reader can organise their library and have it pushed on the
    // next sync. Refusing the click is what used to send every add straight to
    // a sign-in prompt.
    function applyStatus(value) {
        if (animeId <= 0 || value === "") return
        ListStatusStore.setStatus(animeId, value, animeProvider)
        expanded = false
    }

    function removeFromList() {
        if (animeId <= 0) return
        ListStatusStore.remove(animeId, animeProvider)
        expanded = false
    }

    HoverHandler {
        id: stackHover
        onHoveredChanged: {
            if (!hovered) control.expanded = false
        }
    }

    // The list choices, stacked upward from the trigger.
    Column {
        id: stackCol
        anchors.bottom: mainBtn.top
        anchors.bottomMargin: 6
        anchors.horizontalCenter: mainBtn.horizontalCenter
        spacing: 6
        opacity: control.expanded ? 1.0 : 0.0
        scale: control.expanded ? 1.0 : 0.75
        visible: opacity > 0
        transformOrigin: Item.Bottom

        Behavior on opacity { NumberAnimation { duration: 150 } }
        Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutBack } }

        ActionButton {
            iconName: "play"
            tipText: "Watching"
            isActive: control.status === "current"
            onClicked: control.applyStatus("current")
        }

        ActionButton {
            visible: control.canComplete
            iconName: "check"
            tipText: "Completed"
            isActive: control.status === "completed"
            onClicked: control.applyStatus("completed")
        }

        ActionButton {
            iconName: "calendar"
            tipText: "Plan to Watch"
            isActive: control.status === "planning"
            onClicked: control.applyStatus("planning")
        }
    }

    // The trigger: "+" when unlisted, the glyph of the work's own list when
    // listed, and a red "x" while hovered so removal is discoverable.
    Rectangle {
        id: mainBtn
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        width: 36
        height: 36
        radius: 18

        property bool isHovered: mainHover.hovered
        property bool showRemoveX: control.listed && (control.expanded || isHovered)

        color: showRemoveX
            ? "#ef4444"
            : (control.expanded || control.listed
                ? Theme.accent
                : Qt.rgba(0.12, 0.14, 0.20, 0.92))
        border.width: 1
        border.color: showRemoveX ? "#f87171" : Theme.glassBorder

        Behavior on color { ColorAnimation { duration: 150 } }

        HoverHandler {
            id: mainHover
            onHoveredChanged: {
                if (hovered) control.expanded = true
            }
        }

        MouseArea {
            id: mainClickArea
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                if (mainBtn.showRemoveX) control.removeFromList()
            }
        }

        Image {
            anchors.centerIn: parent
            width: 14
            height: 14
            source: control.getSvgIcon(mainBtn.showRemoveX
                ? "close"
                : (control.listed ? control.statusIconName(control.status) : "plus"))
            sourceSize.width: 28
            sourceSize.height: 28
            smooth: true
        }

        ToolTip.visible: mainClickArea.containsMouse && mainBtn.showRemoveX
        ToolTip.text: "Remove from list"
        ToolTip.delay: 200
    }

    // One circular choice in the stack. An inline component so the stack above
    // reads as a list of intents rather than button plumbing.
    component ActionButton: Item {
        id: actBtn
        property string iconName: ""
        property string tipText: ""
        property bool isActive: false
        signal clicked()

        width: 34
        height: 34

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

        Rectangle {
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
                source: control.getSvgIcon(actBtn.iconName)
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

            ToolTip.visible: actMouse.containsMouse && actBtn.tipText.length > 0
            ToolTip.text: actBtn.tipText
            ToolTip.delay: 200
        }
    }
}
