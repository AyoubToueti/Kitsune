// A single pulsing placeholder block.
//
// The one primitive every skeleton is composed from, mirroring
// `Skeleton.svelte`. Sizing and shaping are left to the caller (width, height,
// radius, anchors), so the same component covers a poster, a text line, or a
// chip without a prop per shape.
//
// The pulse is an opacity fade, the QML equivalent of Tailwind's
// `animate-pulse`. It is gated on `Theme.animationsEnabled` so a future
// reduced-motion setting can freeze every placeholder at once -- see
// Theme.qml. When animations are off the block holds a steady mid-opacity.
import QtQuick

Rectangle {
    id: skeleton

    // The placeholder fill: a touch lighter than the surface it sits on, so an
    // empty card still reads as "content coming" rather than "broken".
    color: Theme.surfaceHover

    // Rounded corners by default; a caller may pass a different radius (e.g. a
    // full circle for an avatar dot).
    //radius: 6

    // Frozen at a steady 0.6 when animations are off, so the block is still
    // visible as a placeholder rather than vanishing.
    opacity: Theme.animationsEnabled ? 1.0 : 0.6

    // The fade itself. `running` follows the theme flag, so toggling the
    // setting stops every instance without touching each one.
    NumberAnimation on opacity {
        running: Theme.animationsEnabled
        from: 0.5
        to: 1.0
        duration: 1100
        loops: Animation.Infinite
        easing.type: Easing.InOutQuad
    }
}