// A small integer stepper that follows `Theme`.
//
// The Basic style's `SpinBox` paints its own background and arrow buttons from
// colours `palette` does not reach, so it stayed charcoal on a white card in
// light mode. Same reasoning as `ThemedSwitch` and `ThemedSlider`.
import QtQuick

Item {
    id: root

    property int from: 0
    property int to: 100
    property int value: 0

    /// Emitted when the reader changes the value, not when a binding sets it.
    /// Auto-saving settings listen to this rather than `value`, so populating
    /// the control from disk cannot start a save/read loop.
    signal edited()

    implicitWidth: 108
    implicitHeight: 32

    function clamp(v) {
        return Math.max(root.from, Math.min(root.to, v))
    }

    Rectangle {
        anchors.fill: parent
        radius: 8
        color: Theme.surface
        border.width: 1
        border.color: Theme.borderSubtle

        Row {
            anchors.fill: parent

            component StepButton: Rectangle {
                id: step
                property string glyph: ""
                signal stepped()

                width: 30
                height: parent ? parent.height : 32
                color: stepMouse.containsMouse ? Theme.surfaceHover : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: step.glyph
                    color: Theme.ink
                    font.pixelSize: 12
                    font.bold: true
                }

                MouseArea {
                    id: stepMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: step.stepped()
                }
            }

            StepButton {
                glyph: "\u2212"
                onStepped: {
                    root.value = root.clamp(root.value - 1)
                    root.edited()
                }
            }

            Text {
                width: parent.width - 60
                height: parent.height
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                text: String(root.value)
                color: Theme.ink
                font.pixelSize: 12
                font.bold: true
            }

            StepButton {
                glyph: "+"
                onStepped: {
                    root.value = root.clamp(root.value + 1)
                    root.edited()
                }
            }
        }
    }
}