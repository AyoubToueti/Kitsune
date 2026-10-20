// ErrorState.qml
// A reusable loading / empty / error block with a red-and-gold kitsune. One
// component, six foxes ("kind"), sensible default texts, optional buttons and a retry
// countdown for rate limits.
//
//   ErrorState { kind: "loading"; visible: page.anime === null }   // waiting for data
//   ErrorState { kind: "sleeping" }                          // nothing here yet
//   ErrorState { kind: "notFound";    onPrimaryClicked: page.back() }
//   ErrorState { kind: "rateLimited"; retryAfter: 30; onPrimaryClicked: page.reload() }
//   ErrorState { kind: "offline";     onPrimaryClicked: page.reload() }
//   ErrorState { kind: "serverError"; detail: message; onPrimaryClicked: page.reload() }
//
//   // or let an HTTP status pick the fox (0 means "no connection"):
//   ErrorState { httpStatus: 429; retryAfter: 12; onPrimaryClicked: page.reload() }
//
//   // art only, e.g. inside a small card:
//   ErrorState { kind: "sleeping"; artWidth: 96; showText: false }
//
// Place it with anchors.centerIn / anchors.fill / a Layout. It draws no
// background of its own. Colours for text and buttons come from Theme; the
// fox art itself is fixed red and gold.
pragma ComponentBehavior: Bound

import QtQuick
import com.kitsune.app

Item {
    id: root

    // ---- Public API -------------------------------------------------------

    // "loading" (thinking) | "sleeping" (empty) | "notFound" (404)
    // | "rateLimited" (429) | "offline" | "serverError" (500 / anything unexpected)
    property string kind: "serverError"

    // Optional. When >= 0 it overrides `kind`: 0 -> offline, 404/410 -> notFound,
    // 429 -> rateLimited, everything else -> serverError.
    property int httpStatus: -1

    // Optional. A raw backend failure string. When set it classifies the fox
    // through `RetryState`, so a page can pass its message as-is instead of
    // mapping the failure itself.
    property string failureMessage: ""

    // Leave empty to use the default text for the kind.
    property string title: ""
    property string message: ""
    // Small faint line under the message (error text, request id...).
    // Defaults to "HTTP <status>" when httpStatus is set.
    property string detail: ""
    // Button labels. Leave empty for the default; set showActions false to hide.
    property string primaryText: ""
    property string secondaryText: ""

    property bool showText: true
    property bool showActions: true
    property bool animated: true
    // Set false to drop the fox and keep only the text/actions, for a strip
    // too short to carry art.
    property bool showArt: true
    // Tighter type, spacing and buttons, for footers and inline status strips
    // where the full-size block would dominate the surrounding content.
    property bool compact: false
    property real artWidth: 220
    property real maxContentWidth: 420

    // Seconds until the primary button becomes available again (e.g. the
    // Retry-After header of a 429). Counts down by itself.
    property int retryAfter: 0

    signal primaryClicked()
    signal secondaryClicked()
    // Emitted when the retryAfter countdown reaches zero.
    signal retryReady()

    function kindForStatus(code) {
        if (code === 0) return "offline"
        if (code === 404 || code === 410) return "notFound"
        if (code === 429) return "rateLimited"
        return "serverError"
    }

    // Map a raw backend failure string onto a fox.
    //
    // Transport wording is delegated to `RetryState`, which is the one place
    // it is classified, so a page can pass its message through unchanged and
    // never has to learn the vocabulary. On top of that, a "nothing matched"
    // message -- an empty release search, no recommendations -- is not a
    // failure at all, so it gets the not-found fox rather than the error one.
    function kindForMessage(message) {
        if (!message) return "serverError"
        const m = String(message).toLowerCase()
        // "No releases found for this episode.", "No season packs found ...",
        // "No episode releases found." -- a successful search with no hits.
        if (m.indexOf("no ") === 0 && m.indexOf("found") !== -1) return "notFound"
        const c = RetryState.classify(message)
        if (c === "rateLimit") return "rateLimited"
        if (c === "connection") return "offline"
        return "serverError"
    }

    // ---- Defaults ---------------------------------------------------------

    readonly property var _copy: ({
        loading: {
            title: "Just a moment",
            message: "Fetching the latest data. This shouldn't take long.",
            primary: ""
        },
        sleeping: {
            title: "Nothing here yet",
            message: "There's nothing to show right now. Check back in a little while.",
            primary: ""
        },
        notFound: {
            title: "We couldn't find that",
            message: "The page or title you're looking for doesn't exist, or it has been moved.",
            primary: "Go back"
        },
        rateLimited: {
            title: "Slow down a little",
            message: "Too many requests were sent in a short time. Take a breather, then try again.",
            primary: "Try again"
        },
        offline: {
            title: "You're offline",
            message: "Check your internet connection and try again.",
            primary: "Retry"
        },
        serverError: {
            title: "Something went wrong",
            message: "An unexpected error happened. Please try again in a moment.",
            primary: "Try again"
        }
    })

    readonly property string effectiveKind: {
        const k = httpStatus >= 0
            ? kindForStatus(httpStatus)
            : (failureMessage !== "" ? kindForMessage(failureMessage) : kind)
        return _copy[k] !== undefined ? k : "serverError"
    }
    readonly property var _c: _copy[effectiveKind]
    readonly property string _title: title !== "" ? title : _c.title
    readonly property string _message: message !== "" ? message : _c.message
    readonly property string _detail: detail !== "" ? detail
        : (httpStatus > 0 ? "HTTP " + httpStatus : "")
    readonly property string _primary: primaryText !== "" ? primaryText : _c.primary
    readonly property string _primaryLabel: _remaining > 0
        ? _primary + " in " + _remaining + "s" : _primary

    // ---- Retry countdown --------------------------------------------------

    property int _remaining: 0
    onRetryAfterChanged: _remaining = retryAfter
    Component.onCompleted: _remaining = retryAfter

    Timer {
        interval: 1000
        repeat: true
        running: root._remaining > 0 && root.visible
        onTriggered: {
            root._remaining = root._remaining - 1
            if (root._remaining === 0) root.retryReady()
        }
    }

    // ---- Art + motion -----------------------------------------------------

    // The foxes as data-URI SVGs: a "base" layer and an optional "prop" layer
    // (question mark, sweat drop, wifi icon, stars) that move independently.
    readonly property var _art: ({
        sleeping: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='sl-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='sl-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='sl-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='sl-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='sl-tipG' gradientUnits='userSpaceOnUse' x1='66' y1='104' x2='30' y2='114'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.5' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.5' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g class='base'><ellipse cx='98' cy='131' rx='82' ry='8' fill='%23000' opacity='0.35'/><path d='M 22 112 C 14 76, 56 44, 108 46 C 156 48, 186 82, 172 112 C 150 126, 50 126, 22 112 Z' fill='url(%23sl-bodyG)'/><path d='M 66 62 C 84 51, 112 47, 136 52' stroke='%23FFD700' stroke-opacity='0.35' stroke-width='4' stroke-linecap='round' fill='none'/><path d='M 164 74 C 192 92, 182 128, 120 130 C 70 132, 30 128, 18 114 C 12 106, 24 98, 36 102 C 60 112, 100 114, 130 108 C 152 103, 160 92, 164 74 Z' fill='url(%23sl-tailG)'/><path d='M 164 74 C 192 92, 182 128, 120 130 C 70 132, 30 128, 18 114 C 12 106, 24 98, 36 102 C 60 112, 100 114, 130 108 C 152 103, 160 92, 164 74 Z' fill='url(%23sl-tipG)'/><path d='M 96 122 Q 108 126 122 121 M 138 118 Q 150 114 158 104' stroke='%238E0000' stroke-opacity='0.35' stroke-width='2' stroke-linecap='round' fill='none'/><path d='M 36 102 C 60 112, 100 114, 130 108 C 152 103, 160 92, 164 74' stroke='%234A0000' stroke-opacity='0.3' stroke-width='2.5' stroke-linecap='round' fill='none'/><g transform='translate(106 108) rotate(-8)'><ellipse rx='15' ry='8' fill='%233A0808'/><path d='M 7 -4 v 7 M 12 -2 v 5' stroke='%235A1515' stroke-width='1.3' stroke-linecap='round'/></g><g transform='translate(70 84) rotate(-6) scale(0.92)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23sl-earG)' stroke='url(%23sl-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23sl-earG)' stroke='url(%23sl-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23sl-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><path d='M -26 -2 Q -18 5 -10 -2' stroke='%232B0A0A' stroke-width='2.6' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -2 Q 18 5 26 -2' stroke='%232B0A0A' stroke-width='2.6' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/></g></g></svg>",
            prop: ""
        },
        notFound: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='nf-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='nf-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='nf-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='nf-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='nf-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g class='base'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23nf-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23nf-tipG)'/><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23nf-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23nf-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><g transform='translate(26 0)'><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23nf-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/></g><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><circle cx='38' cy='110' r='16' fill='%23FFF8E1' fill-opacity='0.18' stroke='%23FFC107' stroke-width='5'/><path d='M 31 104 Q 35 100 41 101' stroke='%23fff' stroke-opacity='0.55' stroke-width='2' stroke-linecap='round' fill='none'/><path d='M 49.5 121.5 L 61 133' stroke='%235A1515' stroke-width='7' stroke-linecap='round'/><path d='M 49.5 121.5 L 53 125' stroke='%23FFB300' stroke-width='7' stroke-linecap='round'/><g transform='translate(100 66) rotate(12) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23nf-earG)' stroke='url(%23nf-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23nf-earG)' stroke='url(%23nf-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23nf-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-14.0' cy='-1.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><ellipse cx='20.0' cy='-1.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><circle cx='-12.0' cy='-3.6' r='2.3' fill='%23fff'/><circle cx='22.0' cy='-3.6' r='2.3' fill='%23fff'/><path d='M -26 -11 Q -18 -14 -10 -11' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -17 Q 18 -22 26 -16' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -5 30 Q -2.5 28 0 30 Q 2.5 32 5 30' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/></g></g></svg>",
            prop: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><g class='prop'><path d='M 158 28 C 158 14, 180 14, 180 27 C 180 36, 168 37, 168 47' stroke='%23FFC107' stroke-width='6.5' stroke-linecap='round' fill='none'/><circle cx='168' cy='59' r='4' fill='%23FFC107'/></g></svg>"
        },
        rateLimited: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='rl-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='rl-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='rl-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='rl-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='rl-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g class='base'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23rl-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23rl-tipG)'/><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23rl-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23rl-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><g transform='translate(26 0)'><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23rl-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/></g><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><rect x='31' y='87' width='7' height='6' rx='2' fill='%23FFB300'/><circle cx='34.5' cy='111' r='19' fill='%23FFC107'/><circle cx='34.5' cy='111' r='14.5' fill='%23FFF8E1'/><path d='M 34.5 99.5 v 3 M 34.5 119.5 v 3 M 23 111 h 3 M 43 111 h 3' stroke='%238E4B00' stroke-width='1.6' stroke-linecap='round'/><path d='M 34.5 111 L 34.5 103' stroke='%233A0808' stroke-width='2.4' stroke-linecap='round'/><path d='M 34.5 111 L 41 115' stroke='%23E53935' stroke-width='2' stroke-linecap='round'/><circle cx='34.5' cy='111' r='2' fill='%233A0808'/><g transform='translate(100 66) rotate(-4) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23rl-earG)' stroke='url(%23rl-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23rl-earG)' stroke='url(%23rl-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23rl-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><path d='M -26 -6 L -12 0 L -26 6' stroke='%232B0A0A' stroke-width='3' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 26 -6 L 12 0 L 26 6' stroke='%232B0A0A' stroke-width='3' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -6 29.5 Q 0 33.5 6 29.5' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M -3.8 31 C -4.8 41, 4.8 41, 3.8 31 Z' fill='%23FF7B7B'/><path d='M 0 33 V 38' stroke='%23E04848' stroke-width='1.1' stroke-linecap='round'/></g></g></svg>",
            prop: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><g class='prop'><path d='M 149 33 C 155 42, 157 48, 149 53 C 141 48, 143 42, 149 33 Z' fill='%237FD1F7'/><ellipse cx='146.4' cy='45.5' rx='1.6' ry='3' fill='%23fff' opacity='0.7'/></g></svg>"
        },
        offline: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='of-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='of-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='of-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='of-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='of-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g class='base'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23of-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23of-tipG)'/><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23of-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23of-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><g transform='translate(26 0)'><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23of-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/></g><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><g transform='translate(100 66) rotate(-3) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(-36 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23of-earG)' stroke='url(%23of-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(36 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23of-earG)' stroke='url(%23of-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23of-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-17.0' cy='0.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><ellipse cx='17.0' cy='0.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><circle cx='-15.0' cy='-2.6' r='2.3' fill='%23fff'/><circle cx='19.0' cy='-2.6' r='2.3' fill='%23fff'/><path d='M -26 -9 L -10 -15' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -15 L 26 -9' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -5 31 Q 0 28 5 31' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M -24 7 C -28 14, -20 14, -24 7 Z' fill='%237FD1F7'/></g></g></svg>",
            prop: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><g class='prop'><path d='M 80.91 11.91 A 27 27 0 0 1 119.09 11.91' stroke='%23FFC107' stroke-width='4.6' stroke-linecap='round' fill='none'/><path d='M 87.27 18.27 A 18 18 0 0 1 112.73 18.27' stroke='%23FFC107' stroke-width='4.6' stroke-linecap='round' fill='none'/><path d='M 93.64 24.64 A 9 9 0 0 1 106.36 24.64' stroke='%23FFC107' stroke-width='4.6' stroke-linecap='round' fill='none'/><circle cx='100' cy='31' r='3.4' fill='%23FFC107'/><path d='M 79 7 L 121 35' stroke='%23200505' stroke-width='6.5' stroke-linecap='round'/><path d='M 79 7 L 121 35' stroke='%23FFF8E1' stroke-width='2.6' stroke-linecap='round'/></g></svg>"
        },
        serverError: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='se-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='se-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='se-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='se-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='se-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g class='base'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23se-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23se-tipG)'/><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23se-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23se-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><g transform='translate(26 0)'><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23se-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/></g><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><g transform='translate(100 66) rotate(7) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23se-earG)' stroke='url(%23se-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23se-earG)' stroke='url(%23se-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23se-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><path d='M -24 -6 L -10 6 M -10 -6 L -24 6' stroke='%232B0A0A' stroke-width='3' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -6 L 24 6 M 24 -6 L 10 6' stroke='%232B0A0A' stroke-width='3' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -8 30 Q -4 27 0 30 Q 4 33 8 30' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><g transform='rotate(-24 8 -22)'><rect x='-12' y='-30' width='40' height='13' rx='6.5' fill='%23FFE082' stroke='%23FFB300' stroke-width='1.2'/><rect x='-1' y='-30' width='18' height='13' fill='%23FFF8E1' fill-opacity='0.55'/><circle cx='3' cy='-23.5' r='1' fill='%23C98A00'/><circle cx='8' cy='-23.5' r='1' fill='%23C98A00'/><circle cx='13' cy='-23.5' r='1' fill='%23C98A00'/></g></g></g></svg>",
            prop: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><g class='prop'><path d='M 100 0 L 102.40 5.60 L 108 8 L 102.40 10.40 L 100 16 L 97.60 10.40 L 92 8 L 97.60 5.60 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/><path d='M 48 30 L 49.80 34.20 L 54 36 L 49.80 37.80 L 48 42 L 46.20 37.80 L 42 36 L 46.20 34.20 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/><path d='M 152 31.5 L 153.95 36.05 L 158.5 38 L 153.95 39.95 L 152 44.5 L 150.05 39.95 L 145.5 38 L 150.05 36.05 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/></g></svg>"
        },
        loading: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='ld-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='ld-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='ld-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='ld-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='ld-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g class='base'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23ld-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23ld-tipG)'/><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23ld-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23ld-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><g transform='translate(100 66) rotate(-8) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23ld-earG)' stroke='url(%23ld-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23ld-earG)' stroke='url(%23ld-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23ld-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-14.0' cy='-3.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><ellipse cx='20.0' cy='-3.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><circle cx='-12.0' cy='-5.6' r='2.3' fill='%23fff'/><circle cx='22.0' cy='-5.6' r='2.3' fill='%23fff'/><path d='M -26 -10 L -10 -9' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -16 Q 18 -22 26 -17' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -3 30 Q 2 28 7 30' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/></g><path d='M 124 121 C 136 113, 130 101, 118 98' stroke='%23E03A3A' stroke-width='12' stroke-linecap='round' fill='none'/><circle cx='117' cy='98' r='6.5' fill='%233A0808'/></g></svg>",
            prop: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><g class='prop'><circle cx='154' cy='24' r='10.8' fill='%23FFC107'/><circle cx='166' cy='17' r='12.8' fill='%23FFC107'/><circle cx='179' cy='24' r='10.8' fill='%23FFC107'/><circle cx='166' cy='29' r='11.8' fill='%23FFC107'/><circle cx='146' cy='50' r='4.4' fill='%23FFC107'/><circle cx='151' cy='43' r='5.6' fill='%23FFC107'/><circle cx='154' cy='24' r='9' fill='%23FFF8E1'/><circle cx='166' cy='17' r='11' fill='%23FFF8E1'/><circle cx='179' cy='24' r='9' fill='%23FFF8E1'/><circle cx='166' cy='29' r='10' fill='%23FFF8E1'/><circle cx='146' cy='50' r='2.6' fill='%23FFF8E1'/><circle cx='151' cy='43' r='3.8' fill='%23FFF8E1'/></g></svg>"
        }
    })

    readonly property string _zSvg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'><path d='M 2 2.5 H 10 L 2 9.5 H 10' stroke='%23FFC107' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round' fill='none'/></svg>"

    function _uri(svg) {
        return svg === "" ? "" : "data:image/svg+xml;utf8," + svg
    }

    readonly property real _tau: Math.PI * 2
    readonly property real _artHeight: artWidth * 0.75
    readonly property bool _animating: animated && visible

    // Type and button metrics, so `compact` can shrink the whole block in one
    // switch rather than at every call site.
    readonly property int _gap: compact ? 6 : 10
    readonly property int _titleSize: compact ? 13 : 20
    readonly property int _messageSize: compact ? 11 : 13
    readonly property int _detailSize: compact ? 10 : 11
    readonly property int _buttonHeight: compact ? 32 : 40
    readonly property int _buttonSize: compact ? 12 : 14
    readonly property int _buttonPad: compact ? 24 : 40

    property real _basePhase: 0
    property real _propPhase: 0

    NumberAnimation on _basePhase {
        from: 0; to: 1
        loops: Animation.Infinite
        duration: root.effectiveKind === "sleeping" ? 4200
            : root.effectiveKind === "loading" ? 3600
            : root.effectiveKind === "notFound" ? 3400
            : root.effectiveKind === "rateLimited" ? 650
            : root.effectiveKind === "offline" ? 170 : 2600
        running: root._animating
    }

    NumberAnimation on _propPhase {
        from: 0; to: 1
        loops: Animation.Infinite
        duration: root.effectiveKind === "notFound" ? 3000
            : root.effectiveKind === "loading" ? 2400
            : root.effectiveKind === "rateLimited" ? 1500
            : root.effectiveKind === "offline" ? 2400 : 6000
        running: root._animating
    }

    readonly property real _baseScale: !_animating ? 1
        : effectiveKind === "sleeping" ? 1 + 0.025 * Math.sin(_tau * _basePhase)
        : effectiveKind === "loading" ? 1 + 0.018 * Math.sin(_tau * _basePhase)
        : effectiveKind === "rateLimited" ? 1 + 0.03 * Math.sin(_tau * _basePhase)
        : 1
    readonly property real _baseRotation: !_animating ? 0
        : effectiveKind === "notFound" ? 2.5 * Math.sin(_tau * _basePhase)
        : effectiveKind === "loading" ? 0.8 * Math.sin(_tau * _basePhase)
        : effectiveKind === "serverError" ? 2 * Math.sin(_tau * _basePhase)
        : 0
    readonly property real _baseShiftX: !_animating ? 0
        : effectiveKind === "offline" ? artWidth * 0.004 * Math.sin(_tau * _basePhase)
        : 0

    readonly property real _propAngle: !_animating ? 0
        : effectiveKind === "serverError" ? 360 * _propPhase
        : effectiveKind === "notFound" ? 6 * Math.sin(_tau * _propPhase)
        : 0
    readonly property real _propShiftY: !_animating ? 0
        : effectiveKind === "notFound" ? -_artHeight * 0.03 * Math.sin(_tau * _propPhase)
        : effectiveKind === "loading" ? -_artHeight * 0.02 * Math.sin(_tau * _propPhase)
        : effectiveKind === "rateLimited" ? _artHeight * 0.11 * _propPhase
        : 0
    readonly property real _propOpacity: !_animating ? 1
        : effectiveKind === "rateLimited"
            ? (_propPhase < 0.15 ? _propPhase / 0.15 : 1 - (_propPhase - 0.15) / 0.85)
        : effectiveKind === "offline"
            ? (_propPhase < 0.4 ? 1 : _propPhase < 0.5 ? 0.25 : _propPhase < 0.6 ? 1
                : _propPhase < 0.68 ? 0.3 : 1)
        : 1

    // ---- Layout -----------------------------------------------------------

    implicitWidth: maxContentWidth
    implicitHeight: col.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: root._title + ". " + root._message

    Column {
        id: col
        width: Math.min(root.width, root.maxContentWidth)
        anchors.centerIn: parent
        // Column skips invisible children, so hiding the art collapses its row.
        spacing: root._gap

        // --- Fox ---
        Item {
            id: art
            visible: root.showArt
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.artWidth
            height: root._artHeight

            Image {
                id: baseImg
                x: root._baseShiftX
                width: art.width
                height: art.height
                source: root._uri(root._art[root.effectiveKind].base)
                sourceSize.width: Math.round(art.width * 2)
                sourceSize.height: Math.round(art.height * 2)
                fillMode: Image.PreserveAspectFit
                smooth: true
                transformOrigin: Item.Bottom
                scale: root._baseScale
                rotation: root._baseRotation
            }

            Image {
                id: propImg
                visible: source.toString() !== ""
                y: root._propShiftY
                width: art.width
                height: art.height
                source: root._uri(root._art[root.effectiveKind].prop)
                sourceSize.width: Math.round(art.width * 2)
                sourceSize.height: Math.round(art.height * 2)
                fillMode: Image.PreserveAspectFit
                smooth: true
                opacity: root._propOpacity
                transform: Rotation {
                    origin.x: art.width * (root.effectiveKind === "serverError" ? 0.5 : 0.84)
                    origin.y: art.height * (root.effectiveKind === "serverError" ? 0.413 : 0.27)
                    angle: root._propAngle
                }
            }

            // Floating "z z Z" for the sleeping fox
            Repeater {
                model: 3
                delegate: Image {
                    id: zMark
                    required property int index
                    property real t: 0
                    readonly property real eff: root._animating ? t : 0.3 + 0.2 * index

                    visible: root.effectiveKind === "sleeping"
                    source: root._uri(root._zSvg)
                    width: art.width * (0.07 + 0.022 * index)
                    height: width
                    sourceSize.width: Math.round(width * 3)
                    sourceSize.height: Math.round(height * 3)
                    x: art.width * (0.56 + 0.13 * eff) - width / 2
                    y: art.height * (0.30 - 0.27 * eff) - height / 2
                    opacity: eff < 0.2 ? (eff / 0.2) * 0.95 : 0.95 * (1.0 - (eff - 0.2) / 0.8)

                    SequentialAnimation {
                        running: root._animating && root.effectiveKind === "sleeping"
                        PauseAnimation { duration: zMark.index * 1100 }
                        SequentialAnimation {
                            loops: Animation.Infinite
                            NumberAnimation {
                                target: zMark; property: "t"
                                from: 0; to: 1; duration: 3300
                            }
                        }
                    }
                }
            }

            // Thinking dots inside the thought bubble of the "loading" fox
            Repeater {
                model: [{x: 157, y: 24}, {x: 166, y: 24}, {x: 175, y: 24}]
                delegate: Rectangle {
                    required property int index
                    required property var modelData
                    readonly property real pulse: root._animating
                        ? 0.5 + 0.5 * Math.sin(root._tau * (root._propPhase * 2 - index * 0.2))
                        : 1

                    visible: root.effectiveKind === "loading"
                    width: art.width * 6.4 / 200
                    height: width
                    radius: width / 2
                    color: "#E53935"
                    x: art.width * modelData.x / 200 - width / 2
                    y: art.height * modelData.y / 150 - height / 2 + root._propShiftY
                    scale: 0.6 + 0.55 * pulse
                    opacity: 0.35 + 0.65 * pulse
                }
            }
        }

        // --- Text ---
        Text {
            visible: root.showText
            width: col.width
            text: root._title
            color: Theme.ink
            font.pixelSize: root._titleSize
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
        }

        Text {
            visible: root.showText && text !== ""
            width: col.width
            text: root._message
            color: Theme.inkMuted
            font.pixelSize: root._messageSize
            lineHeight: 1.35
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
        }

        Text {
            visible: root.showText && text !== ""
            width: col.width
            text: root._detail
            color: Theme.inkFaint
            font.pixelSize: root._detailSize
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WrapAnywhere
            elide: Text.ElideRight
            maximumLineCount: 3
        }

        // --- Buttons ---
        Item {
            visible: root.showActions && (root._primary !== "" || root.secondaryText !== "")
            width: 1
            height: 6
        }

        Row {
            id: actions
            visible: root.showActions && (root._primary !== "" || root.secondaryText !== "")
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 10

            Rectangle {
                id: primaryBtn
                visible: root._primary !== ""
                width: primaryLabel.implicitWidth + root._buttonPad
                height: root._buttonHeight
                radius: root._buttonHeight / 2
                color: primaryMouse.containsMouse && primaryMouse.enabled
                    ? Theme.accentHover : Theme.accent
                opacity: root._remaining > 0 ? 0.55 : 1.0

                Behavior on color { ColorAnimation { duration: 150 } }

                Text {
                    id: primaryLabel
                    anchors.centerIn: parent
                    text: root._primaryLabel
                    color: "white"
                    font.pixelSize: root._buttonSize
                    font.bold: true
                }

                MouseArea {
                    id: primaryMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    enabled: root._remaining === 0
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.primaryClicked()
                }

                Accessible.role: Accessible.Button
                Accessible.name: primaryLabel.text
                Accessible.onPressAction: if (primaryMouse.enabled) root.primaryClicked()
            }

            Rectangle {
                id: secondaryBtn
                visible: root.secondaryText !== ""
                width: secondaryLabel.implicitWidth + root._buttonPad
                height: root._buttonHeight
                radius: root._buttonHeight / 2
                color: secondaryMouse.containsMouse ? Theme.surfaceHover : "transparent"
                border.width: 1
                border.color: secondaryMouse.containsMouse ? Theme.accent : Theme.borderSubtle

                Behavior on border.color { ColorAnimation { duration: 150 } }

                Text {
                    id: secondaryLabel
                    anchors.centerIn: parent
                    text: root.secondaryText
                    color: Theme.ink
                    font.pixelSize: root._buttonSize
                    font.bold: true
                }

                MouseArea {
                    id: secondaryMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.secondaryClicked()
                }

                Accessible.role: Accessible.Button
                Accessible.name: secondaryLabel.text
                Accessible.onPressAction: root.secondaryClicked()
            }
        }
    }
}
