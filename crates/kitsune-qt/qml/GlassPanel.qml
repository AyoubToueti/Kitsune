import QtQuick

Rectangle {
    id: panel

    property bool strong: false
    property bool interactive: false

    radius: Theme.radius
    color: panel.strong ? Theme.glassFillStrong : Theme.glassFill
    border.width: 1
    border.color: (panel.interactive && hover.hovered) ? Theme.accent : Theme.glassBorder

    Behavior on border.color { ColorAnimation { duration: 150 } }

    HoverHandler { id: hover }

    // Top-edge glass highlight sheen
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 1
        height: Math.min(32, Math.max(8, parent.height * 0.45))
        topLeftRadius: Math.max(0, panel.radius - 1)
        topRightRadius: Math.max(0, panel.radius - 1)
        bottomLeftRadius: 0
        bottomRightRadius: 0
        gradient: Gradient {
            GradientStop { position: 0.0; color: Theme.glassHighlight }
            GradientStop { position: 1.0; color: "transparent" }
        }
    }
}