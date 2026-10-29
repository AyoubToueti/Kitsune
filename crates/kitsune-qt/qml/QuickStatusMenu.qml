import QtQuick
import QtQuick.Controls
import com.kitsune.app

Item {
    id: menu

    property int animeId: 0
    property string animeStatus: ""
    readonly property bool canComplete: animeStatus !== "RELEASING"
    property real targetRadius: 70

    // Direct status check via ListStatusStore
    readonly property string currentStatus: (typeof ListStatusStore !== "undefined" && ListStatusStore)
                                             ? ListStatusStore.getStatus(menu.animeId)
                                             : ""
    readonly property bool isOnList: currentStatus !== ""

    implicitWidth: 140
    implicitHeight: 140

    property bool expanded: false

    function getSvgIcon(name, color) {
        var fill = color === undefined ? "#ffffff" : Theme.hex(color)
        var path = ""
        if (name === "play")
            path = "M8 5v14l11-7z"
        else if (name === "check")
            path = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
        else if (name === "calendar")
            path = "M19 4h-1V2h-2v2H8V2H6v2H5c-1.11 0-1.99.9-1.99 2L3 20c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 16H5V10h14v10zm0-12H5V6h14v2z"
        else if (name === "edit")
            path = "M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"
        else if (name === "close" || name === "x")
            path = "M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
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

    // Write a status and collapse the stack.
    //
    // A ROOT function, not inline handler code: the delegates call it as
    // `menu.applyStatus(...)`, so nothing inside a delegate or an inline
    // component has to touch the enclosing object's properties directly.
    //
    // Not gated on `signedIn`: the list is local, so an offline reader can
    // organise it and have the change pushed on the next sync.
    function applyStatus(value) {
        if (value !== "" && animeId > 0) {
            ListStatusStore.setStatus(animeId, value)
        }
        expanded = false
    }

    function removeFromList() {
        if (animeId > 0) {
            ListStatusStore.remove(animeId)
        }
        expanded = false
    }

    HoverHandler {
        id: stackHover
        onHoveredChanged: {
            if (!hovered) {
                menu.expanded = false
            }
        }
    }

    readonly property real centerX: menu.width - 26
    readonly property real centerY: menu.height - 26

    Repeater {
        model: [
            { status: "current",   icon: "play",     tip: "Watching",       angle5: 270.0, angle4: 270.0, showAlways: true },
            { status: "planning",  icon: "calendar", tip: "Plan to Watch",  angle5: 247.5, angle4: 240.0, showAlways: true },
            { status: "paused",    icon: "pause",    tip: "On Hold",        angle5: 225.0, angle4: 210.0, showAlways: true },
            { status: "dropped",   icon: "minus",    tip: "Dropped",        angle5: 202.5, angle4: 180.0, showAlways: true },
            { status: "completed", icon: "check",    tip: "Completed",      angle5: 180.0, angle4: 180.0, showAlways: false }
        ]

        delegate: ActionPillButton {
            required property var modelData

            itemStatus: modelData ? modelData.status : ""
            iconName: modelData ? modelData.icon : ""
            tipText: modelData ? modelData.tip : ""

            visible: modelData ? (modelData.showAlways || menu.canComplete) : false
            
            property real itemAngle: (modelData && menu.canComplete) ? modelData.angle5 : (modelData ? modelData.angle4 : 0)
            property real rad: itemAngle * Math.PI / 180.0

            property real hoverPopOffset: isHovered ? 8.0 : 0.0
            Behavior on hoverPopOffset { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

            property real activeRadius: (menu.expanded ? menu.targetRadius : 0.0) + hoverPopOffset

            Behavior on activeRadius {
                NumberAnimation {
                    duration: 220
                    easing.type: Easing.OutBack
                    easing.overshoot: 1.15
                }
            }

            x: menu.centerX + activeRadius * Math.cos(rad) - width / 2
            y: menu.centerY + activeRadius * Math.sin(rad) - height / 2

            rotation: itemAngle - 270.0
            iconRotation: 270.0 - itemAngle

            // The glyph is built HERE, in the delegate, where `menu` resolves.
            // The inline component below used to call `menu.getSvgIcon(...)`
            // from inside itself, and a type defined with `component` does not
            // reliably see the enclosing id at run time -- that is what raised
            // "Value is undefined and could not be converted to an object".
            iconSource: menu.getSvgIcon(iconName)

            isActive: menu.currentStatus === itemStatus
            menuExpanded: menu.expanded

            // The write goes through a ROOT function rather than touching
            // `menu.expanded` from the delegate's handler.
            onClicked: menu.applyStatus(itemStatus)
        }
    }

    Rectangle {
        id: mainBtn
        x: menu.centerX - width / 2
        y: menu.centerY - height / 2
        width: 36
        height: 36
        radius: 18
        z: 10

        property bool isHovered: mainHover.hovered
        property bool showRemoveX: menu.isOnList && (menu.expanded || isHovered)

        color: showRemoveX 
            ? "#ef4444" 
            : (menu.expanded || menu.isOnList
                ? Theme.accent 
                : Qt.rgba(0.12, 0.14, 0.20, 0.92))
        border.width: 1
        border.color: showRemoveX ? "#f87171" : Theme.glassBorder

        Behavior on color { ColorAnimation { duration: 150 } }

        HoverHandler {
            id: mainHover
            onHoveredChanged: {
                if (hovered) {
                    menu.expanded = true
                }
            }
        }

        MouseArea {
            id: mainClickArea
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                if (mainBtn.showRemoveX) menu.removeFromList()
            }
        }

        Image {
            anchors.centerIn: parent
            width: 14
            height: 14
            // Listed: the glyph of the list the work is saved to, so the
            // button says WHERE it is rather than merely that it is somewhere.
            // Unlisted: "+". Hovering/expanded on a listed work: red "x".
            source: menu.getSvgIcon(mainBtn.showRemoveX
                ? "close"
                : (menu.isOnList ? menu.statusIconName(menu.currentStatus) : "plus"))
            sourceSize.width: 28
            sourceSize.height: 28
            smooth: true
        }

        ToolTip.visible: mainClickArea.containsMouse && mainBtn.showRemoveX
        ToolTip.text: "Remove from list"
        ToolTip.delay: 200
    }

    component ActionPillButton: Item {
        id: actPill
        property string itemStatus: ""
        property string iconName: ""
        property string tipText: ""
        // The ready-made image URL, supplied by the delegate. The component
        // must not reach for `menu` itself: an inline `component` cannot
        // reliably see the enclosing id at run time, and the resulting
        // `undefined` base is what made setting a property throw.
        property url iconSource: ""
        property bool isActive: false
        property bool menuExpanded: false
        property real iconRotation: 0.0
        signal clicked()

        readonly property bool isHovered: pillMouse.containsMouse

        width: 28
        height: 44
        opacity: menuExpanded ? 1.0 : 0.0
        scale: menuExpanded ? (isHovered ? 1.18 : 1.0) : 0.3
        z: isHovered ? 20 : 1

        Behavior on opacity { NumberAnimation { duration: 150 } }
        Behavior on scale { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            radius: (parent.width / 2) + 2
            color: "transparent"
            border.width: 2
            border.color: actPill.isHovered ? "#ffffff" : Theme.accent
            opacity: (actPill.isActive || actPill.isHovered) ? 0.95 : 0.0

            Behavior on opacity { NumberAnimation { duration: 180 } }
        }

        Rectangle {
            id: pillBody
            anchors.fill: parent
            radius: width / 2
            color: actPill.isActive 
                ? Theme.accent 
                : (actPill.isHovered ? Theme.accentHover : Qt.rgba(0.10, 0.12, 0.18, 0.92))
            border.width: 1
            border.color: actPill.isHovered ? "#ffffff" : (actPill.isActive ? "#ffffff" : Theme.glassBorder)

            Behavior on color { ColorAnimation { duration: 120 } }

            Image {
                anchors.centerIn: parent
                width: 13
                height: 13
                rotation: actPill.iconRotation
                source: actPill.iconSource
                sourceSize.width: 26
                sourceSize.height: 26
                smooth: true
            }

            MouseArea {
                id: pillMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: actPill.clicked()
            }

            ToolTip.visible: pillMouse.containsMouse && tipText.length > 0
            ToolTip.text: tipText
            ToolTip.delay: 200
        }
    }
}