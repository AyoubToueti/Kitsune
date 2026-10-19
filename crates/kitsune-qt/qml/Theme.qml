pragma Singleton
import QtQuick

QtObject {
    id: theme

    // Whether animated effects (the skeleton pulse, and any future motion) are
    // allowed to run.
    //
    // Qt exposes a platform reduced-motion preference only from 6.12 onward
    // (QAccessibilityHints::motionPreference); this app targets 6.11, so the
    // flag is a plain constant for now. Skeleton blocks read it, so a future
    // settings toggle or a Qt bump can honour the OS hint by changing this one
    // value -- no call site needs to change.
    readonly property bool animationsEnabled: true

    /// What the reader chose: `"light"`, `"dark"` or `"system"`.
    ///
    /// Kept unvalidated for the same reason as `TitlePreference.language`: the
    /// raw value round-trips through a save untouched, and anything that is not
    /// a known value is treated as "system" by `dark` below.
    property string choice: "system"

    /// Whether the OS is currently in dark mode.
    ///
    /// `Qt.styleHints.colorScheme` landed in Qt 6.5 and updates live, so
    /// "system" tracks a desktop theme flip without a restart. The guard keeps
    /// an older Qt from throwing; it reports dark, which is the palette the app
    /// shipped with.
    readonly property bool systemDark: {
        if (typeof Qt.styleHints !== "undefined"
                && Qt.styleHints.colorScheme !== undefined) {
            return Qt.styleHints.colorScheme !== Qt.ColorScheme.Light
        }
        return true
    }

    /// Whether the dark palette is in force.
    ///
    /// An explicit choice wins outright; anything else follows the OS. Written
    /// as "not light" so an unknown scheme value (Qt reports `Unknown` on some
    /// platforms) stays dark rather than flashing white at startup.
    readonly property bool dark: choice === "dark"
        || (choice !== "light" && systemDark)

    // Surfaces
    readonly property color surface: dark ? "#0b0d12" : "#f5f6f9"
    readonly property color surfaceRaised: dark ? "#141821" : "#ffffff"
    readonly property color surfaceHover: dark ? "#1c2130" : "#eceff4"
    // A recessed well: the record body on the resume disc, and any future
    // inset panel. Light mode keeps it a shade below `surface` rather than a
    // near-black slab.
    readonly property color surfaceSunken: dark ? "#111319" : "#e6e9f0"
    readonly property color borderSubtle: dark ? "#232838" : "#d9dee8"
    // A hairline inside a card rather than around it. Distinct from
    // `borderSubtle` because the vinyl disc needs a mid-grey that reads on
    // artwork.
    readonly property color separator: dark ? "#2a2e3d" : "#c9cfdc"
    // A quiet input well that sits on `surfaceRaised` without competing with
    // it: the navbar search box, and any future inline text field. A
    // translucent white reads as "slightly darker than raised" on the dark
    // palette and as "slightly grey" on the light one, so each palette gets
    // its own alpha rather than sharing one literal.
    readonly property color fieldFill: dark ? "#08ffffff" : "#0d000000"

    // Ink
    readonly property color ink: dark ? "#e8eaf0" : "#16181d"
    readonly property color inkMuted: dark ? "#9aa3b8" : "#545b6a"
    readonly property color inkFaint: dark ? "#6b7488" : "#8a92a3"

    // Accents
    readonly property color accent: dark ? "#7c5cff" : "#6f4bff"
    readonly property color accentHover: dark ? "#8f74ff" : "#5c3ae0"
    // Progress ring on the resume disc.
    readonly property color purple: dark ? "#a855f7" : "#7c3aed"
    readonly property color score: dark ? "#ffc857" : "#b7791f"
    readonly property color danger: dark ? "#e5484d" : "#d13438"

    // Health badges for a probed release. Green means a tracker confirmed peers
    // and the metadata was fetched; red means nothing answered. A red release
    // is still a legitimate choice, so these are deliberately not the danger
    // hue.
    readonly property color healthGreen: dark ? "#3fb950" : "#1a7f37"
    readonly property color healthYellow: dark ? "#d29922" : "#9a6700"
    readonly property color healthRed: dark ? "#6b7488" : "#8a92a3"

    // Glass surfaces. In light mode these are white with a dark hairline,
    // matching the Svelte app's `surface-raised`; the player and the watch
    // modal draw over video and stay dark on their own.
    readonly property color glassFill: dark ? "#cc0c101a" : "#e6ffffff"
    readonly property color glassFillStrong: dark ? "#f00b0e17" : "#f7ffffff"
    readonly property color glassFillSubtle: dark ? "#0fffffff" : "#0a000000"
    readonly property color glassFillHover: dark ? "#26ffffff" : "#14000000"
    readonly property color glassBorder: dark ? "#22ffffff" : "#1f000000"
    readonly property color glassBorderHover: dark ? "#44ffffff" : "#33000000"
    readonly property color glassHighlight: dark ? "#1affffff" : "#66ffffff"
    readonly property color glassShadow: dark ? "#80000000" : "#1a000000"

    // Over artwork. Scrims and badges sit on poster stills, so they stay dark
    // in both modes: white text over arbitrary artwork is only readable against
    // a dark wash. `onArtwork` is the matching foreground.
    readonly property color scrim: "#cc0b0d12"
    readonly property color scrimStrong: "#e6000000"
    readonly property color scrimSoft: "#80000000"
    readonly property color onArtwork: "#ffffff"
    readonly property color onArtworkMuted: "#c8cdd8"
    readonly property color onArtworkFaint: "#9aa3b8"

    // Spacing
    readonly property int radius: 12
    readonly property int gap: 16
    readonly property int pad: 24
}