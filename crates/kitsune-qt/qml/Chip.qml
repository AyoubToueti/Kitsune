// A small pill chip for genres and tags. Genres are on/off; tags cycle through
// off -> include -> exclude, matching `FilterPanel.svelte`'s tag chips.
import QtQuick

Rectangle {
    id: chip

    property string text: ""
    // "off" | "include" | "exclude"
    property string state: "off"

    signal clicked()

    radius: 14
    implicitWidth: label.implicitWidth + 22
    implicitHeight: 28

    color: state === "include"
        ? Theme.accent
        : (state === "exclude" ? Theme.danger : "transparent")
    border.width: 1
    border.color: state === "off"
        ? (chipMouse.containsMouse ? Theme.accent : Theme.borderSubtle)
        : color

    Text {
        id: label
        anchors.centerIn: parent
        text: chip.text
        color: chip.state === "off"
            ? (chipMouse.containsMouse ? Theme.accent : Theme.inkMuted)
            : "white"
        font.pixelSize: 12
    }

    MouseArea {
        id: chipMouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: chip.clicked()
    }
}