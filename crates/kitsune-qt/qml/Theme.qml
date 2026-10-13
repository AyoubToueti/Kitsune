// Design tokens, mirroring `src/app.css` (@theme) so the Qt app matches the
// Svelte one. A singleton so every component shares one set of values.
pragma Singleton
import QtQuick

QtObject {
    // Surfaces
    readonly property color surface: "#0b0d12"
    readonly property color surfaceRaised: "#141821"
    readonly property color surfaceHover: "#1c2130"
    readonly property color borderSubtle: "#232838"

    // Ink
    readonly property color ink: "#e8eaf0"
    readonly property color inkMuted: "#9aa3b8"
    readonly property color inkFaint: "#6b7488"

    // Accents
    readonly property color accent: "#7c5cff"
    readonly property color accentHover: "#8f74ff"
    readonly property color score: "#ffc857"
    readonly property color danger: "#e5484d"

    // Spacing
    readonly property int radius: 12
    readonly property int gap: 16
    readonly property int pad: 24
}