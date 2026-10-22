pragma Singleton
import QtQuick

QtObject {
    id: theme

    // Whether animated effects (the skeleton pulse, and any future motion) are
    // allowed to run.
    readonly property bool animationsEnabled: true

    /// What the reader chose: `"light"`, `"dark"` or `"system"`.
    property string choice: "system"

    /// Whether the OS is currently in dark mode.
    readonly property bool systemDark: {
        if (typeof Qt.styleHints !== "undefined"
                && Qt.styleHints.colorScheme !== undefined) {
            return Qt.styleHints.colorScheme !== Qt.ColorScheme.Light
        }
        return true
    }

    /// Whether the dark palette is in force.
    readonly property bool dark: choice === "dark"
        || (choice !== "light" && systemDark)

    // Surfaces
    //
    // The dark mode matches your screenshot exactly: a deep, slightly cool
    // blue-black that makes the vibrant red pop. Light mode stays clean and
    // neutral.
    readonly property color surface: dark ? "#090a0f" : "#f5f6f9"
    readonly property color surfaceRaised: dark ? "#14161c" : "#ffffff"
    readonly property color surfaceHover: dark ? "#1c1f28" : "#eceff4"
    readonly property color surfaceSunken: dark ? "#0e1015" : "#e6e9f0"
    readonly property color borderSubtle: dark ? "#232630" : "#d9dee8"
    // A hairline inside a card rather than around it.
    readonly property color separator: dark ? "#2a2e38" : "#c9cfdc"
    // A quiet input well that sits on `surfaceRaised`.
    readonly property color fieldFill: dark ? "#08ffffff" : "#0d000000"

    // Ink
    readonly property color ink: dark ? "#ffffff" : "#16181d"
    readonly property color inkMuted: dark ? "#9aa3b8" : "#545b6a"
    readonly property color inkFaint: dark ? "#6b7488" : "#8a92a3"

    // Accents
    //
    // Pulled directly from your logo: a vivid vermilion red that matches 
    // the 'Filter' button and tags in the screenshot.
    readonly property color accent: dark ? "#f04a5e" : "#e13b4f"
    readonly property color accentHover: dark ? "#ff5c6c" : "#c92c3f"
    
    // Progress ring on the resume disc. Using a deeper shade of the brand red
    // rather than a separate hue so the UI feels unified.
    readonly property color purple: dark ? "#d63a4c" : "#b82536"
    
    // The screenshot uses a bright green for the score badges, so we'll swap 
    // the gold for a crisp green.
    readonly property color score: dark ? "#00c853" : "#1a7f37"
    
    // Distinct from the accent red so destructive actions are obvious.
    readonly property color danger: dark ? "#ff3b30" : "#d13438"

    // Health badges for a probed release.
    readonly property color healthGreen: dark ? "#00c853" : "#1a7f37"
    readonly property color healthYellow: dark ? "#ffc107" : "#9a6700"
    readonly property color healthRed: dark ? "#6b7488" : "#8a92a3"

    // Glass surfaces. In light mode these are white; the player and the watch 
    // modal draw over video and stay dark on their own.
    readonly property color glassFill: dark ? "#cc0c101a" : "#e6ffffff"
    readonly property color glassFillStrong: dark ? "#f00b0e17" : "#f7ffffff"
    readonly property color glassFillSubtle: dark ? "#0fffffff" : "#0a000000"
    readonly property color glassFillHover: dark ? "#26ffffff" : "#14000000"
    readonly property color glassBorder: dark ? "#22ffffff" : "#1f000000"
    readonly property color glassBorderHover: dark ? "#44ffffff" : "#33000000"
    readonly property color glassHighlight: dark ? "#1affffff" : "#66ffffff"
    readonly property color glassShadow: dark ? "#80000000" : "#1a000000"

    // Over artwork. Scrims and badges sit on poster stills.
    readonly property color scrim: "#cc0b0d12"
    readonly property color scrimStrong: "#e6000000"
    readonly property color scrimSoft: "#80000000"
    readonly property color onArtwork: "#ffffff"
    readonly property color onArtworkMuted: "#c8cdd8"
    readonly property color onArtworkFaint: "#9aa3b8"

    // The detail page's hero banner scrim.
    readonly property color heroScrim: dark ? "#cc090a0f" : "#d9f5f6f9"

    // The home carousel fades into the page.
    readonly property color heroFadeStart: dark ? "#ff090a0f" : "#fff5f6f9"
    readonly property color heroFadeMid: dark ? "#f2090a0f" : "#f2f5f6f9"
    readonly property color heroFadeEnd: dark ? "#00090a0f" : "#00f5f6f9"
    readonly property color heroArtEdge: dark ? "#00090a0f" : "#00f5f6f9"

    /// A colour as an SVG-safe `#rrggbb` string.
    function hex(c) {
        function two(v) {
            const n = Math.round(Math.max(0, Math.min(1, v)) * 255)
            return (n < 16 ? "0" : "") + n.toString(16)
        }
        return "#" + two(c.r) + two(c.g) + two(c.b)
    }

    // Spacing
    readonly property int radius: 12
    readonly property int gap: 16
    readonly property int pad: 24
}