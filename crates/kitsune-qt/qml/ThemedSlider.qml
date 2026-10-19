// A horizontal slider that follows `Theme`.
//
// Same reason as `ThemedSwitch`: the Basic style's `Slider` draws its groove
// from colours `palette` does not reach, so it stayed charcoal on a white card
// in light mode.
//
// The API mirrors the parts the app uses -- `from`, `to`, `stepSize`, `value`,
// `onMoved` and `onValueChanged` -- so a call site only changes its type name.
import QtQuick

Item {
    id: root

    property real from: 0
    property real to: 1
    property real stepSize: 0
    property real value: 0

    /// Emitted while the reader drags, matching `Slider.moved`.
    signal moved()

    implicitHeight: 22

    // Clamp into range and snap to the step, so a caller cannot put the knob
    // outside the groove.
    function normalized(v) {
        let clamped = Math.max(root.from, Math.min(root.to, v))
        if (root.stepSize > 0) {
            const steps = Math.round((clamped - root.from) / root.stepSize)
            clamped = root.from + steps * root.stepSize
        }
        return clamped
    }

    readonly property real ratio: root.to > root.from
        ? (normalized(root.value) - root.from) / (root.to - root.from)
        : 0

    function valueAt(x) {
        const t = Math.max(0, Math.min(1, x / groove.width))
        return normalized(root.from + t * (root.to - root.from))
    }

    Rectangle {
        id: groove
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width
        height: 4
        radius: 2
        color: Theme.separator

        Rectangle {
            width: Math.max(0, parent.width * root.ratio)
            height: parent.height
            radius: parent.radius
            color: Theme.accent
        }
    }

    Rectangle {
        id: knob
        width: 18
        height: 18
        radius: 9
        anchors.verticalCenter: parent.verticalCenter
        x: Math.max(0, Math.min(parent.width - width, parent.width * root.ratio - width / 2))
        color: Theme.accent
        border.width: 2
        border.color: Theme.surfaceRaised
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        preventStealing: true

        function apply(x) {
            root.value = root.valueAt(x)
            root.moved()
        }

        onPressed: (mouse) => apply(mouse.x)
        onPositionChanged: (mouse) => { if (pressed) apply(mouse.x) }
    }
}