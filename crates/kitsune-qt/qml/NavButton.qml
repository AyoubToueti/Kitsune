// A small pill button used in the top bar. Hover turns it accent-coloured.
import QtQuick

Rectangle {
    id: button

    property string label: ""

    signal clicked()

    radius: 8
    color: hover.hovered ? Theme.accent : Theme.surfaceRaised
    implicitWidth: labelText.implicitWidth + 24
    implicitHeight: 36

    Text {
        id: labelText
        anchors.centerIn: parent
        text: button.label
        color: Theme.ink
        font.pixelSize: 14
        font.bold: true
    }

    HoverHandler { id: hover }
    MouseArea {
        anchors.fill: parent
        onClicked: button.clicked()
    }
}