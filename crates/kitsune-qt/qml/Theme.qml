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

    // Glass surfaces
    readonly property color glassFill: "#cc0c101a"
    readonly property color glassFillStrong: "#f00b0e17"
    readonly property color glassFillSubtle: "#0fffffff"
    readonly property color glassFillHover: "#26ffffff"
    readonly property color glassBorder: "#22ffffff"
    readonly property color glassBorderHover: "#44ffffff"
    readonly property color glassHighlight: "#1affffff"
    readonly property color glassShadow: "#80000000"

    // Spacing
    readonly property int radius: 12
    readonly property int gap: 16
    readonly property int pad: 24
}