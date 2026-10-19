// A toggle that follows `Theme`, for the same reason `ThemeSelect` exists:
// the Basic style's `Switch` draws its own track from a palette that does not
// pick up the app's colours reliably, so it stayed charcoal on a white card in
// light mode.
//
// Same API surface as the parts of `Switch` the app actually used -- `checked`
// and the `toggled` signal -- so a call site only changes its type name.
import QtQuick

Item {
    id: root

    property bool checked: false

    /// Emitted after `checked` changes, mirroring `Switch.toggled`.
    signal toggled()

    implicitWidth: 46
    implicitHeight: 26

    function toggle() {
        root.checked = !root.checked
        root.toggled()
    }

    Rectangle {
        id: track
        anchors.fill: parent
        radius: height / 2
        color: root.checked ? Theme.accent : Theme.separator
        border.width: 1
        border.color: root.checked ? Theme.accent : Theme.borderSubtle

        Behavior on color { ColorAnimation { duration: 140 } }
        Behavior on border.color { ColorAnimation { duration: 140 } }

        Rectangle {
            id: knob
            width: parent.height - 8
            height: width
            radius: width / 2
            y: 4
            x: root.checked ? parent.width - width - 4 : 4
            color: "#ffffff"

            Behavior on x { NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }
        }
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggle()
    }
}