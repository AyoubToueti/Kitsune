// ActionButton.qml
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.kitsune.app

Rectangle {
    id: buttonRoot
    property string label: ""
    property string icon: ""
    property color tone: Theme.ink
    property bool prominent: false
    property bool quiet: false
    property bool hovered: buttonMouse.containsMouse
    signal activated()

    implicitWidth: buttonRow.implicitWidth + (prominent ? 24 : 20)
    implicitHeight: prominent ? 32 : 30
    radius: 8
    color: prominent
        ? (buttonMouse.containsMouse ? Qt.lighter(tone, 1.12) : tone)
        : (buttonMouse.containsMouse ? Theme.glassFillHover
            : (quiet ? "transparent" : Theme.surfaceSunken))
    border.width: prominent || quiet ? 0 : 1
    border.color: buttonMouse.containsMouse
        ? Qt.rgba(tone.r, tone.g, tone.b, 0.55)
        : Theme.glassBorder

    RowLayout {
        id: buttonRow
        anchors.centerIn: parent
        spacing: 6
        Text {
            visible: buttonRoot.icon.length > 0
            text: buttonRoot.icon
            color: buttonRoot.prominent ? "#17110d" : buttonRoot.tone
            font.pixelSize: 10
            font.bold: true
        }
        Text {
            text: buttonRoot.label
            color: buttonRoot.prominent ? "#17110d"
                : (buttonMouse.containsMouse ? buttonRoot.tone : Theme.ink)
            font.pixelSize: 11
            font.weight: buttonRoot.prominent ? Font.DemiBold : Font.Medium
        }
    }

    MouseArea {
        id: buttonMouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: buttonRoot.activated()
    }
}