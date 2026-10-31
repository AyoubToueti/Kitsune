// ErrorState.qml
// A reusable loading / empty / error / success block with a red-and-gold kitsune. One
// component, twelve foxes ("kind"), sensible default texts, optional buttons and a retry
// countdown for rate limits.
//
//   ErrorState { kind: "loading"; visible: page.anime === null }   // waiting for data
//   ErrorState { kind: "sleeping" }                          // nothing here yet
//   ErrorState { kind: "notFound";    onPrimaryClicked: page.back() }
//   ErrorState { kind: "rateLimited"; retryAfter: 30; onPrimaryClicked: page.reload() }
//   ErrorState { kind: "offline";     onPrimaryClicked: page.reload() }
//   ErrorState { kind: "serverError"; detail: message; onPrimaryClicked: page.reload() }
//
//   // working and success states:
//   ErrorState { kind: "running" }                          // a task is in progress
//   ErrorState { kind: "victory" }                          // big win, confetti
//   ErrorState { kind: "delivered"; primaryText: "Open"; onPrimaryClicked: page.openFile() }
//   ErrorState { kind: "foxfire" }                          // calm "all set"
//   ErrorState { kind: "checkmark"; artWidth: 96; showText: false }   // small "done"
//   ErrorState { kind: "wiggle" }                           // "saved!"
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
    // Working and success foxes: "running" (task in progress) | "victory" (big win)
    // | "delivered" (download ready) | "foxfire" (all set) | "checkmark" (done)
    // | "wiggle" (saved)
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
        },
        running: {
            title: "On it, hang tight",
            message: "This is working right now. It shouldn't take long.",
            primary: ""
        },
        victory: {
            title: "You did it!",
            message: "Everything went through. Nicely done.",
            primary: ""
        },
        delivered: {
            title: "Your download is ready",
            message: "The file has finished downloading and is ready to open.",
            primary: ""
        },
        foxfire: {
            title: "All set",
            message: "Your changes have been applied.",
            primary: ""
        },
        checkmark: {
            title: "Done",
            message: "Everything is complete.",
            primary: ""
        },
        wiggle: {
            title: "Saved!",
            message: "Your changes were saved.",
            primary: ""
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
        },
        running: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='rn-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient></defs><path d='M 48 100 C 48 82, 74 76, 100 78 C 124 80, 142 86, 142 102 C 142 116, 124 120, 98 120 C 70 120, 48 116, 48 100 Z' fill='url(%23rn-bodyG)'/><path d='M 64 85 C 82 78, 106 78, 126 83' stroke='%23FFD700' stroke-opacity='0.35' stroke-width='4' stroke-linecap='round' fill='none'/><path d='M 56 98 C 60 88, 78 86, 84 96 C 88 108, 70 118, 58 112' stroke='%238E0000' stroke-opacity='0.3' stroke-width='2.2' stroke-linecap='round' fill='none'/><path d='M 112 120 C 128 119, 140 112, 142 102 C 134 108, 124 112, 108 113 Z' fill='%23FFF8E1'/></svg>",
            prop: "",
            parts: [
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='34 135 124 12'><ellipse cx='96' cy='141' rx='62' ry='6' fill='%23000' opacity='0.35'/></svg>",
              box: [34, 135, 124, 12], pivot: [96, 141], layer: "under", z: 0,
              anim: "shadowPulse", dur: 720, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='76 102 16 40'><g><path d='M 78 104 C 78 116, 79.5 126, 79.5 132 C 79.5 138, 90 138, 90 132 C 90 126, 90 116, 90 104 Z' fill='%23A61E1E'/><path d='M 79.5 126 H 90 V 132 C 90 139, 79.5 139, 79.5 132 Z' fill='%233A0808'/></g></svg>",
              box: [76, 102, 16, 40], pivot: [84, 104], layer: "base", z: -3,
              anim: "stride", dur: 720, t: 0.5833, m: 1, rest: 1, restRot: -28, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='102 102 16 40'><g><path d='M 104 104 C 104 116, 105.5 126, 105.5 132 C 105.5 138, 116 138, 116 132 C 116 126, 116 116, 116 104 Z' fill='%23A61E1E'/><path d='M 105.5 126 H 116 V 132 C 116 139, 105.5 139, 105.5 132 Z' fill='%233A0808'/></g></svg>",
              box: [102, 102, 16, 40], pivot: [110, 104], layer: "base", z: -3,
              anim: "stride", dur: 720, t: 0.0833, m: 1, rest: 1, restRot: 26, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 58 58 56'><defs><linearGradient id='rn-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='rn-tipG' gradientUnits='userSpaceOnUse' x1='34' y1='98' x2='6' y2='70'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.5' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.5' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g><path d='M 54 88 C 38 66, 12 60, 2 72 C 6 96, 30 112, 56 108 Z' fill='url(%23rn-tailG)'/><path d='M 54 88 C 38 66, 12 60, 2 72 C 6 96, 30 112, 56 108 Z' fill='url(%23rn-tipG)'/></g></svg>",
              box: [0, 58, 58, 56], pivot: [54, 98], layer: "base", z: -2,
              anim: "tailwave", dur: 720, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='62 102 16 40'><g><path d='M 64 104 C 64 116, 65.5 126, 65.5 132 C 65.5 138, 76 138, 76 132 C 76 126, 76 116, 76 104 Z' fill='%23D32F2F'/><path d='M 65.5 126 H 76 V 132 C 76 139, 65.5 139, 65.5 132 Z' fill='%233A0808'/></g></svg>",
              box: [62, 102, 16, 40], pivot: [70, 104], layer: "base", z: 1,
              anim: "stride", dur: 720, t: 0.5, m: 1, rest: 1, restRot: 26, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='116 102 16 40'><g><path d='M 118 104 C 118 116, 119.5 126, 119.5 132 C 119.5 138, 130 138, 130 132 C 130 126, 130 116, 130 104 Z' fill='%23D32F2F'/><path d='M 119.5 126 H 130 V 132 C 130 139, 119.5 139, 119.5 132 Z' fill='%233A0808'/></g></svg>",
              box: [116, 102, 16, 40], pivot: [124, 104], layer: "base", z: 1,
              anim: "stride", dur: 720, t: 0, m: 1, rest: 1, restRot: -28, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='rn-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='rn-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient></defs><ellipse cx='71' cy='101' rx='18' ry='15' fill='%23DD3B3B'/><ellipse cx='124' cy='98' rx='15' ry='14' fill='%23E23C3C'/><g transform='translate(150 78) rotate(8) scale(0.85)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(-22 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23rn-earG)' stroke='url(%23rn-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(-22 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23rn-earG)' stroke='url(%23rn-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23rn-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-15.0' cy='-1.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><ellipse cx='19.0' cy='-1.0' rx='6.2' ry='7.2' fill='%232B0A0A'/><circle cx='-13.0' cy='-3.6' r='2.3' fill='%23fff'/><circle cx='21.0' cy='-3.6' r='2.3' fill='%23fff'/><path d='M -26 -11 Q -18 -15 -10 -11' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -11 Q 18 -15 26 -11' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -6 29.5 Q 0 33.5 6 29.5' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M -3.8 31 C -4.8 41, 4.8 41, 3.8 31 Z' fill='%23FF7B7B'/><path d='M 0 33 V 38' stroke='%23E04848' stroke-width='1.1' stroke-linecap='round'/></g></svg>",
              box: [0, 0, 200, 150], pivot: [100, 75], layer: "base", z: 2,
              anim: "none", dur: 0, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='37.4 33.4 41.2 5.2'><path d='M 40 36 H 76' stroke='%23FFC107' stroke-width='4' stroke-linecap='round' fill='none'/></svg>",
              box: [37.4, 33.4, 41.2, 5.2], pivot: [58, 36], layer: "over", z: 0,
              anim: "streak", dur: 800, t: 0, m: 1, rest: 0.6, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='9.4 45.4 39.2 5.2'><path d='M 12 48 H 46' stroke='%23FFC107' stroke-width='4' stroke-linecap='round' fill='none'/></svg>",
              box: [9.4, 45.4, 39.2, 5.2], pivot: [29, 48], layer: "over", z: 0,
              anim: "streak", dur: 800, t: 0.3375, m: 1, rest: 0.6, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='55.4 55.4 33.2 5.2'><path d='M 58 58 H 86' stroke='%23FFC107' stroke-width='4' stroke-linecap='round' fill='none'/></svg>",
              box: [55.4, 55.4, 33.2, 5.2], pivot: [72, 58], layer: "over", z: 0,
              anim: "streak", dur: 800, t: 0.6625, m: 1, rest: 0.6, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='27.9 127.9 12.2 12.2'><circle cx='34' cy='134' r='5.5' fill='%23FFF8E1'/></svg>",
              box: [27.9, 127.9, 12.2, 12.2], pivot: [34, 134], layer: "over", z: 0,
              anim: "dust", dur: 720, t: 0, m: 1, rest: 0.5, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='17.8 125.8 8.4 8.4'><circle cx='22' cy='130' r='3.6' fill='%23FFF8E1'/></svg>",
              box: [17.8, 125.8, 8.4, 8.4], pivot: [22, 130], layer: "over", z: 0,
              anim: "dust", dur: 720, t: 0.3333, m: 1, rest: 0.5, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='42.4 133.4 7.2 7.2'><circle cx='46' cy='137' r='3' fill='%23FFF8E1'/></svg>",
              box: [42.4, 133.4, 7.2, 7.2], pivot: [46, 137], layer: "over", z: 0,
              anim: "dust", dur: 720, t: 0.6667, m: 1, rest: 0.5, restRot: 0, floats: false }
            ]
        },
        victory: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='vc-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='vc-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='vc-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='vc-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='vc-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g transform='translate(0 -8)'><g><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23vc-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23vc-tipG)'/></g></g><g transform='translate(84 116) rotate(32)'><path d='M -6 0 C -6 10.8, -4.5 19.2, -4.5 18 C -4.5 24, 6 24, 6 18 C 6 19.2, 6 10.8, 6 0 Z' fill='url(%23vc-bodyG)'/><path d='M -4.5 12 H 6 V 18 C 6 25, -4.5 25, -4.5 18 Z' fill='%233A0808'/></g><g transform='translate(116 116) rotate(-32)'><path d='M -6 0 C -6 10.8, -4.5 19.2, -4.5 18 C -4.5 24, 6 24, 6 18 C 6 19.2, 6 10.8, 6 0 Z' fill='url(%23vc-bodyG)'/><path d='M -4.5 12 H 6 V 18 C 6 25, -4.5 25, -4.5 18 Z' fill='%233A0808'/></g><path d='M 66 124 C 58 102, 78 86, 100 86 C 122 86, 142 102, 134 124 C 126 129, 74 129, 66 124 Z' fill='url(%23vc-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 112, 110 122, 100 126 C 90 122, 82 112, 86 98 Z' fill='%23FFF8E1'/><g transform='translate(70 100) rotate(150)'><path d='M -6 0 C -6 11.7, -4.5 20.8, -4.5 20 C -4.5 26, 6 26, 6 20 C 6 20.8, 6 11.7, 6 0 Z' fill='url(%23vc-bodyG)'/><path d='M -4.5 14 H 6 V 20 C 6 27, -4.5 27, -4.5 20 Z' fill='%233A0808'/></g><g transform='translate(130 100) rotate(-150)'><path d='M -6 0 C -6 11.7, -4.5 20.8, -4.5 20 C -4.5 26, 6 26, 6 20 C 6 20.8, 6 11.7, 6 0 Z' fill='url(%23vc-bodyG)'/><path d='M -4.5 14 H 6 V 20 C 6 27, -4.5 27, -4.5 20 Z' fill='%233A0808'/></g><g transform='translate(100 62) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23vc-earG)' stroke='url(%23vc-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23vc-earG)' stroke='url(%23vc-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23vc-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -26 2 Q -18 -7 -10 2' stroke='%232B0A0A' stroke-width='2.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 2 Q 18 -7 26 2' stroke='%232B0A0A' stroke-width='2.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M -8 27.5 Q 0 38 8 27.5 Z' fill='%232B0A0A'/><ellipse cx='0' cy='30.6' rx='3.5' ry='2.2' fill='%23FF7B7B'/><ellipse cx='-28' cy='11' rx='6.5' ry='4' fill='%23FF8A80' opacity='0.35'/><ellipse cx='28' cy='11' rx='6.5' ry='4' fill='%23FF8A80' opacity='0.35'/></g></svg>",
            prop: "",
            parts: [
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='36 134 128 14'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/></svg>",
              box: [36, 134, 128, 14], pivot: [100, 141], layer: "under", z: 0,
              anim: "shadowJump", dur: 1500, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='45.4 43.4 6.2 9.2'><rect x='46' y='44' width='5' height='8' rx='1.2' fill='%23FFC107'/></svg>",
              box: [45.4, 43.4, 6.2, 9.2], pivot: [48.5, 48], layer: "over", z: 0,
              anim: "burstL", dur: 1500, t: 0, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='153.4 39.4 6.2 9.2'><rect x='154' y='40' width='5' height='8' rx='1.2' fill='%23FFF8E1'/></svg>",
              box: [153.4, 39.4, 6.2, 9.2], pivot: [156.5, 44], layer: "over", z: 0,
              anim: "burstR", dur: 1500, t: 0, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='27.4 73.4 7.2 7.2'><rect x='28' y='74' width='6' height='6' rx='1.2' fill='%23E53935'/></svg>",
              box: [27.4, 73.4, 7.2, 7.2], pivot: [31, 77], layer: "over", z: 0,
              anim: "burstL", dur: 1500, t: 0.3333, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='171.4 71.4 7.2 7.2'><rect x='172' y='72' width='6' height='6' rx='1.2' fill='%23FFC107'/></svg>",
              box: [171.4, 71.4, 7.2, 7.2], pivot: [175, 75], layer: "over", z: 0,
              anim: "burstR", dur: 1500, t: 0.3333, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='77.4 15.4 6.2 9.2'><rect x='78' y='16' width='5' height='8' rx='1.2' fill='%23FFE082'/></svg>",
              box: [77.4, 15.4, 6.2, 9.2], pivot: [80.5, 20], layer: "over", z: 0,
              anim: "burstU", dur: 1500, t: 0.6667, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='125.4 13.4 6.2 9.2'><rect x='126' y='14' width='5' height='8' rx='1.2' fill='%23E53935'/></svg>",
              box: [125.4, 13.4, 6.2, 9.2], pivot: [128.5, 18], layer: "over", z: 0,
              anim: "burstU", dur: 1500, t: 0, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='99.4 7.4 7.2 7.2'><rect x='100' y='8' width='6' height='6' rx='1.2' fill='%23FFC107'/></svg>",
              box: [99.4, 7.4, 7.2, 7.2], pivot: [103, 11], layer: "over", z: 0,
              anim: "burstU", dur: 1500, t: 0.3333, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='59.4 55.4 6.2 9.2'><rect x='60' y='56' width='5' height='8' rx='1.2' fill='%23FFF8E1'/></svg>",
              box: [59.4, 55.4, 6.2, 9.2], pivot: [62.5, 60], layer: "over", z: 0,
              anim: "burstL", dur: 1500, t: 0.6667, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='141.4 57.4 6.2 9.2'><rect x='142' y='58' width='5' height='8' rx='1.2' fill='%23FFE082'/></svg>",
              box: [141.4, 57.4, 6.2, 9.2], pivot: [144.5, 62], layer: "over", z: 0,
              anim: "burstR", dur: 1500, t: 0.6667, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='25.9 21.9 16.2 16.2'><path d='M 34 23 L 36.94 27.06 L 41 30 L 36.94 32.94 L 34 37 L 31.06 32.94 L 27 30 L 31.06 27.06 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/></svg>",
              box: [25.9, 21.9, 16.2, 16.2], pivot: [34, 30], layer: "over", z: 0,
              anim: "twinkle", dur: 1800, t: 0, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='158.9 14.9 18.2 18.2'><path d='M 168 16 L 171.36 20.64 L 176 24 L 171.36 27.36 L 168 32 L 164.64 27.36 L 160 24 L 164.64 20.64 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/></svg>",
              box: [158.9, 14.9, 18.2, 18.2], pivot: [168, 24], layer: "over", z: 0,
              anim: "twinkle", dur: 1800, t: 0.3333, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='93.9 -4.1 12.2 12.2'><path d='M 100 -3 L 102.10 -0.10 L 105 2 L 102.10 4.10 L 100 7 L 97.90 4.10 L 95 2 L 97.90 -0.10 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/></svg>",
              box: [93.9, -4.1, 12.2, 12.2], pivot: [100, 2], layer: "over", z: 0,
              anim: "twinkle", dur: 1800, t: 0.6667, m: 1, rest: 0.9, restRot: 0, floats: false }
            ]
        },
        delivered: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='dl-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='dl-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='dl-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='dl-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='dl-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g transform='translate(200 0) scale(-1 1)'><g><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23dl-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23dl-tipG)'/></g></g><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23dl-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23dl-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><g transform='translate(26 0)'><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23dl-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/></g><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><g transform='translate(98 66) rotate(8) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23dl-earG)' stroke='url(%23dl-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23dl-earG)' stroke='url(%23dl-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23dl-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><ellipse cx='-14' cy='-1' rx='6.2' ry='7.2' fill='%232B0A0A'/><ellipse cx='20' cy='-1' rx='6.2' ry='7.2' fill='%232B0A0A'/><circle cx='-12' cy='-3.6' r='2.3' fill='%23fff'/><circle cx='22' cy='-3.6' r='2.3' fill='%23fff'/><path d='M -26 -11 Q -18 -14 -10 -11' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -11 Q 18 -14 26 -11' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M -6 29.5 Q 0 34 6 29.5' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/></g></svg>",
            prop: "",
            parts: [
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='36 134 128 14'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/></svg>",
              box: [36, 134, 128, 14], pivot: [100, 141], layer: "under", z: 0,
              anim: "none", dur: 0, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='134 94 50 48'><g><rect x='141' y='114' width='36' height='26' rx='4' fill='%23FFE082' stroke='%23FFB300' stroke-width='1.4'/><rect x='137' y='107' width='44' height='10' rx='3.5' fill='%23FFC107' stroke='%23FFB300' stroke-width='1.2'/><rect x='156' y='107' width='6' height='33' fill='%23E53935'/><path d='M 159 106 C 150 96, 142 102, 149 107 Z M 159 106 C 168 96, 176 102, 169 107 Z' fill='%23E53935'/><circle cx='159' cy='106' r='3' fill='%23C62828'/></g></svg>",
              box: [134, 94, 50, 48], pivot: [159, 118], layer: "over", z: 0,
              anim: "giftHop", dur: 2400, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='147.29 62.8 21.42 20.49'><path d='M 158.00 64.00 L 160.53 70.52 L 167.51 70.91 L 162.09 75.33 L 163.88 82.09 L 158.00 78.30 L 152.12 82.09 L 153.91 75.33 L 148.49 70.91 L 155.47 70.52 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1.2' stroke-linejoin='round'/></svg>",
              box: [147.29, 62.8, 21.42, 20.49], pivot: [158, 73.05], layer: "over", z: 0,
              anim: "starBob", dur: 1800, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='132.4 82.4 11.2 11.2'><path d='M 138 83.5 L 139.89 86.11 L 142.5 88 L 139.89 89.89 L 138 92.5 L 136.11 89.89 L 133.5 88 L 136.11 86.11 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/></svg>",
              box: [132.4, 82.4, 11.2, 11.2], pivot: [138, 88], layer: "over", z: 0,
              anim: "twinkle", dur: 1800, t: 0, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='174.9 84.9 10.2 10.2'><path d='M 180 86 L 181.68 88.32 L 184 90 L 181.68 91.68 L 180 94 L 178.32 91.68 L 176 90 L 178.32 88.32 Z' fill='%23FFC107' stroke='%23FFE082' stroke-width='1' stroke-linejoin='round'/></svg>",
              box: [174.9, 84.9, 10.2, 10.2], pivot: [180, 90], layer: "over", z: 0,
              anim: "twinkle", dur: 1800, t: 0.3333, m: 1, rest: 0.9, restRot: 0, floats: false }
            ]
        },
        foxfire: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='ff-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='ff-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='ff-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='ff-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='ff-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g transform='translate(200 0) scale(-1 1)'><g><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23ff-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23ff-tipG)'/></g></g><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23ff-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23ff-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><g transform='translate(124 108) rotate(-115)'><path d='M -6 0 C -6 17.1, -4.5 30.4, -4.5 32 C -4.5 38, 6 38, 6 32 C 6 30.4, 6 17.1, 6 0 Z' fill='url(%23ff-bodyG)'/><path d='M -4.5 26 H 6 V 32 C 6 39, -4.5 39, -4.5 32 Z' fill='%233A0808'/></g><g transform='translate(98 66) rotate(8) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23ff-earG)' stroke='url(%23ff-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23ff-earG)' stroke='url(%23ff-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23ff-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><ellipse cx='-12' cy='-4' rx='6.2' ry='7.2' fill='%232B0A0A'/><ellipse cx='22' cy='-4' rx='6.2' ry='7.2' fill='%232B0A0A'/><circle cx='-10' cy='-6.6' r='2.3' fill='%23fff'/><circle cx='24' cy='-6.6' r='2.3' fill='%23fff'/><path d='M -26 -13 Q -18 -17 -10 -13' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -13 Q 18 -17 26 -13' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M -4 30 Q 1 33 6 30' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/></g></svg>",
            prop: "",
            parts: [
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='36 134 128 14'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/></svg>",
              box: [36, 134, 128, 14], pivot: [100, 141], layer: "under", z: 0,
              anim: "none", dur: 0, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='138 44 48 48'><g><circle cx='162' cy='68' r='24' fill='%23FF9A3C' opacity='0.16'/><circle cx='162' cy='68' r='17' fill='%23FFD54F' opacity='0.3'/></g></svg>",
              box: [138, 44, 48, 48], pivot: [162, 68], layer: "over", z: 0,
              anim: "orbPulse", dur: 1800, t: 0, m: 1, rest: 1, restRot: 0, floats: true },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='150 50 24 32'><path d='M 162 52 C 172 62, 174 71, 162 81 C 151 71, 153 62, 162 52 Z' fill='%23FFC107'/><path d='M 162 62 C 167 67, 168 72, 162 77 C 156 72, 157 67, 162 62 Z' fill='%23FFF8E1'/></svg>",
              box: [150, 50, 24, 32], pivot: [162, 66], layer: "over", z: 0,
              anim: "none", dur: 0, t: 0, m: 1, rest: 1, restRot: 0, floats: true },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='147.4 57.4 5.2 5.2'><circle cx='150' cy='60' r='2' fill='%23FFE082'/></svg>",
              box: [147.4, 57.4, 5.2, 5.2], pivot: [150, 60], layer: "over", z: 0,
              anim: "riseUp", dur: 2200, t: 0, m: 1, rest: 0.9, restRot: 0, floats: true },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='173.6 55.6 4.8 4.8'><circle cx='176' cy='58' r='1.8' fill='%23FFE082'/></svg>",
              box: [173.6, 55.6, 4.8, 4.8], pivot: [176, 58], layer: "over", z: 0,
              anim: "riseUp", dur: 2200, t: 0.3182, m: 1, rest: 0.9, restRot: 0, floats: true },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='165.9 47.9 4.2 4.2'><circle cx='168' cy='50' r='1.5' fill='%23FFF8E1'/></svg>",
              box: [165.9, 47.9, 4.2, 4.2], pivot: [168, 50], layer: "over", z: 0,
              anim: "riseUp", dur: 2200, t: 0.6364, m: 1, rest: 0.9, restRot: 0, floats: true }
            ]
        },
        checkmark: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='ck-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='ck-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='ck-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient></defs><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23ck-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23ck-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><g transform='translate(26 0)'><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23ck-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/></g><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><g transform='translate(100 66) rotate(6) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23ck-earG)' stroke='url(%23ck-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23ck-earG)' stroke='url(%23ck-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23ck-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -26 -11 Q -18 -14 -10 -11' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 -11 Q 18 -14 26 -11' stroke='%235A1010' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M -6 29.5 Q 0 34 6 29.5' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/></g></svg>",
            prop: "",
            parts: [
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='36 134 128 14'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/></svg>",
              box: [36, 134, 128, 14], pivot: [100, 141], layer: "under", z: 0,
              anim: "none", dur: 0, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='130 64 62 80'><defs><linearGradient id='ck-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='ck-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23ck-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23ck-tipG)'/></g></svg>",
              box: [130, 64, 62, 80], pivot: [138, 122], layer: "base", z: -1,
              anim: "flick", dur: 2600, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='76 52 48 28'><g transform='translate(100 66) rotate(6) scale(0.9)'><g><ellipse cx='-16' cy='0' rx='6.2' ry='7.2' fill='%232B0A0A'/><ellipse cx='16' cy='0' rx='6.2' ry='7.2' fill='%232B0A0A'/><circle cx='-14' cy='-2.6' r='2.3' fill='%23fff'/><circle cx='18' cy='-2.6' r='2.3' fill='%23fff'/></g></g></svg>",
              box: [76, 52, 48, 28], pivot: [100, 66], layer: "base", z: 1,
              anim: "blink", dur: 4000, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='146 26 40 40'><circle cx='166' cy='46' r='19' fill='%23FFC107'/><circle cx='166' cy='46' r='15' fill='%23FFF8E1'/></svg>",
              box: [146, 26, 40, 40], pivot: [166, 46], layer: "over", z: 0,
              anim: "popIn", dur: 3200, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='153 34 26 24'><path d='M 157 47 L 164 54 L 176 38' stroke='%23E53935' stroke-width='5.5' stroke-linecap='round' stroke-linejoin='round' fill='none'/></svg>",
              box: [153, 34, 26, 24], pivot: [166, 46], layer: "over", z: 0,
              anim: "tickPop", dur: 3200, t: 0, m: 1, rest: 1, restRot: 0, floats: false }
            ]
        },
        wiggle: {
            base: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'><defs><linearGradient id='wg-bodyG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23C62828'/></linearGradient><linearGradient id='wg-headG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF5252'/><stop offset='1' stop-color='%23E53935'/></linearGradient><linearGradient id='wg-earG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%233A0808'/><stop offset='0.3' stop-color='%233A0808'/><stop offset='0.3' stop-color='%23E53935'/><stop offset='1' stop-color='%23E53935'/></linearGradient></defs><path d='M 62 138 C 54 106, 76 86, 100 86 C 124 86, 146 106, 138 138 Z' fill='url(%23wg-bodyG)'/><path d='M 86 98 C 92 112, 108 112, 114 98 C 118 114, 110 132, 100 136 C 90 132, 82 114, 86 98 Z' fill='%23FFF8E1'/><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23wg-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/><g transform='translate(26 0)'><path d='M 80 120 C 80 114, 94 114, 94 120 L 94 134 C 94 141, 80 141, 80 134 Z' fill='url(%23wg-bodyG)'/><path d='M 80 128 L 94 128 L 94 134 C 94 141, 80 141, 80 134 Z' fill='%233A0808'/></g><ellipse cx='68' cy='138' rx='11' ry='6' fill='%233A0808'/><ellipse cx='132' cy='138' rx='11' ry='6' fill='%233A0808'/><g transform='translate(100 66) scale(0.9)'><path d='M -36 0 L -51 11 L -33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><path d='M 36 0 L 51 11 L 33 17 Z' fill='%23FFF8E1' stroke='%23FFF8E1' stroke-width='3' stroke-linejoin='round'/><g transform='rotate(0 -27 -18)'><path d='M -36 -12 L -42 -54 L -10 -30 Z' fill='url(%23wg-earG)' stroke='url(%23wg-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M -32 -22 L -35 -42 L -19 -30 Z' fill='%23FFC107'/></g><g transform='rotate(0 27 -18)'><path d='M 36 -12 L 42 -54 L 10 -30 Z' fill='url(%23wg-earG)' stroke='url(%23wg-earG)' stroke-width='5' stroke-linejoin='round'/><path d='M 32 -22 L 35 -42 L 19 -30 Z' fill='%23FFC107'/></g><path d='M -40 -8 C -40 -34, 40 -34, 40 -8 C 46 2, 42 10, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -42 10, -46 2, -40 -8 Z' fill='url(%23wg-headG)'/><path d='M -42.72 -0.16 C -29.34 8.10, -12.80 8.84, -4.00 15.00 L 0 16 L 4 15 C 12.80 8.84, 29.34 8.10, 42.72 -0.16 C 43.18 6.34, 38.69 11.93, 30 17 C 18 27, 8 31, 0 31 C -8 31, -18 27, -30 17 C -38.69 11.93, -43.18 6.34, -42.72 -0.16 Z' fill='%23FFF8E1'/><ellipse cx='-28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><ellipse cx='28' cy='11' rx='5' ry='3' fill='%23FFB300' opacity='0.4'/><path d='M -5.5 20 Q 0 17 5.5 20 Q 3.5 26.5 0 27.5 Q -3.5 26.5 -5.5 20 Z' fill='%23200505'/><path d='M -26 2 Q -18 -7 -10 2' stroke='%232B0A0A' stroke-width='2.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><path d='M 10 2 Q 18 -7 26 2' stroke='%232B0A0A' stroke-width='2.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/><ellipse cx='-28' cy='11' rx='6.5' ry='4' fill='%23FF8A80' opacity='0.45'/><ellipse cx='28' cy='11' rx='6.5' ry='4' fill='%23FF8A80' opacity='0.45'/><path d='M -8 29 Q -4 33.5 0 29 Q 4 33.5 8 29' stroke='%232B0A0A' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round' fill='none'/></g></svg>",
            prop: "",
            parts: [
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='36 134 128 14'><ellipse cx='100' cy='141' rx='64' ry='7' fill='%23000' opacity='0.35'/></svg>",
              box: [36, 134, 128, 14], pivot: [100, 141], layer: "under", z: 0,
              anim: "none", dur: 0, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='130 64 62 80'><defs><linearGradient id='wg-tailG' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='%23FF6B6B'/><stop offset='1' stop-color='%23D32F2F'/></linearGradient><linearGradient id='wg-tipG' gradientUnits='userSpaceOnUse' x1='176' y1='108' x2='164' y2='72'><stop offset='0' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='0'/><stop offset='0.55' stop-color='%23FFB300' stop-opacity='1'/><stop offset='1' stop-color='%23FFE082' stop-opacity='1'/></linearGradient></defs><g><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23wg-tailG)'/><path d='M 132 134 C 164 142, 190 122, 184 92 C 181 76, 168 66, 160 70 C 172 86, 168 108, 138 118 Z' fill='url(%23wg-tipG)'/></g></svg>",
              box: [130, 64, 62, 80], pivot: [138, 122], layer: "base", z: -1,
              anim: "wag", dur: 360, t: 0, m: 1, rest: 1, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='138.6 32.6 22.8 19.2'><g transform='translate(150 44) scale(1.2)'><path d='M 0 6 C -9 -1, -6 -9, 0 -4 C 6 -9, 9 -1, 0 6 Z' fill='%23FFC107'/></g></svg>",
              box: [138.6, 32.6, 22.8, 19.2], pivot: [150, 42.2], layer: "over", z: 0,
              anim: "heartUp", dur: 2000, t: 0, m: 1.2, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='42.4 42.4 19.2 16.2'><g transform='translate(52 52) scale(1)'><path d='M 0 6 C -9 -1, -6 -9, 0 -4 C 6 -9, 9 -1, 0 6 Z' fill='%23E53935'/></g></svg>",
              box: [42.4, 42.4, 19.2, 16.2], pivot: [52, 50.5], layer: "over", z: 0,
              anim: "heartUp", dur: 2000, t: 0.33, m: 1, rest: 0.9, restRot: 0, floats: false },
            { svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='111.3 11.3 17.4 14.7'><g transform='translate(120 20) scale(0.9)'><path d='M 0 6 C -9 -1, -6 -9, 0 -4 C 6 -9, 9 -1, 0 6 Z' fill='%23FF8A80'/></g></svg>",
              box: [111.3, 11.3, 17.4, 14.7], pivot: [120, 18.65], layer: "over", z: 0,
              anim: "heartUp", dur: 2000, t: 0.665, m: 0.9, rest: 0.9, restRot: 0, floats: false }
            ]
        }
    })

    readonly property string _zSvg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'><path d='M 2 2.5 H 10 L 2 9.5 H 10' stroke='%23FFC107' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round' fill='none'/></svg>"

    function _uri(svg) {
        return svg === "" ? "" : "data:image/svg+xml;utf8," + svg
    }

    // ---- Keyframe helpers for the animated parts ---------------------------

    function _ease(u) { return u * u * (3 - 2 * u) }
    function _easeOut(u) { return 1 - (1 - u) * (1 - u) }

    // Piecewise lookup over [[t, value], ...] keyframes. ease: 0 linear,
    // 1 ease-in-out, 2 ease-out (applied inside each segment, like CSS).
    function _kf(p, keys, ease) {
        for (let i = 1; i < keys.length; ++i) {
            if (p <= keys[i][0]) {
                const a = keys[i - 1]
                const b = keys[i]
                let u = (p - a[0]) / (b[0] - a[0])
                if (ease === 1) u = _ease(u)
                else if (ease === 2) u = _easeOut(u)
                return a[1] + (b[1] - a[1]) * u
            }
        }
        return keys[keys.length - 1][1]
    }

    readonly property var _restPose: ({ dx: 0, dy: 0, rot: 0, sx: 1, sy: 1, op: 1, rv: 1 })

    // Pose of one animated part at cycle position p (0..1). Distances are in
    // SVG units (the art is 200 wide), angles in degrees. `m` scales distances.
    function _pose(anim, p, m) {
        const r = { dx: 0, dy: 0, rot: 0, sx: 1, sy: 1, op: 1, rv: 1 }
        const c = Math.cos(_tau * p)
        const u = 0.5 - 0.5 * c
        let q = 0
        switch (anim) {
        case "stride":
            r.rot = _kf(p, [[0, -34], [0.5, 34], [0.75, 14], [1, -34]], 0)
            r.sy = _kf(p, [[0, 1], [0.5, 1], [0.75, 0.68], [1, 1]], 0)
            break
        case "tailwave":
            r.rot = 0.5 - 5.5 * c
            break
        case "shadowPulse":
            r.sx = r.sy = 1 - 0.12 * u
            break
        case "shadowJump":
            r.sx = r.sy = _kf(p, [[0, 1], [0.5, 0.62], [1, 1]], 1)
            r.op = _kf(p, [[0, 1], [0.5, 0.6], [1, 1]], 1)
            break
        case "streak":
            r.dx = 14 - 38 * p
            r.op = _kf(p, [[0, 0], [0.3, 0.9], [1, 0]], 0)
            break
        case "dust":
            q = _easeOut(p)
            r.dx = -16 * q
            r.dy = -7 * q
            r.sx = r.sy = 0.4 + q
            r.op = _kf(p, [[0, 0], [0.2, 0.55], [1, 0]], 2)
            break
        case "burstU":
        case "burstL":
        case "burstR":
            q = _easeOut(p)
            r.dx = anim === "burstL" ? -16 * q : anim === "burstR" ? 16 * q : 0
            r.dy = 10 - (anim === "burstU" ? 34 : 32) * q
            r.rot = (anim === "burstL" ? -240 : anim === "burstR" ? 240 : 260) * q
            r.sx = r.sy = 0.4 + 0.6 * q
            r.op = _kf(p, [[0, 0], [0.2, 1], [1, 0]], 2)
            break
        case "twinkle":
            r.sx = r.sy = _kf(p, [[0, 0.4], [0.5, 1.1], [1, 0.4]], 1)
            r.op = _kf(p, [[0, 0], [0.5, 1], [1, 0]], 1)
            break
        case "giftHop":
            r.dy = _kf(p, [[0, 0], [0.6, 0], [0.7, -5], [0.8, 0], [0.88, -2], [0.95, 0], [1, 0]], 1)
            break
        case "starBob":
            r.dy = -5 * u
            r.rot = -8 + 16 * u
            r.sx = r.sy = 1 + 0.1 * u
            break
        case "orbPulse":
            r.sx = r.sy = 0.92 + 0.2 * u
            break
        case "riseUp":
            q = _easeOut(p)
            r.dx = 4 * q
            r.dy = 6 - 28 * q
            r.sx = r.sy = 0.5 + 0.3 * q
            r.op = _kf(p, [[0, 0], [0.3, 1], [1, 0]], 2)
            break
        case "popIn":
        case "tickPop":
            r.sx = r.sy = _kf(p, [[0, 0.2], [0.1, 1.15], [0.18, 1], [0.88, 1], [1, 0.9]], 2)
            r.op = _kf(p, [[0, 0], [0.1, 1], [0.88, 1], [1, 0]], 2)
            if (anim === "tickPop") r.rv = _kf(p, [[0, 0], [0.1, 0], [0.3, 1], [1, 1]], 2)
            break
        case "blink":
            r.sy = _kf(p, [[0, 1], [0.92, 1], [0.95, 0.1], [1, 1]], 1)
            break
        case "flick":
            r.rot = -6 * u
            break
        case "wag":
            r.rot = -14 * c
            break
        case "heartUp":
            r.dy = _kf(p, [[0, 8], [0.25, 0], [1, -26]], 2)
            r.sx = r.sy = _kf(p, [[0, 0.4], [0.25, 1], [1, 1.1]], 2)
            r.op = _kf(p, [[0, 0], [0.25, 1], [1, 0]], 2)
            break
        }
        r.dx *= m
        r.dy *= m
        return r
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
            : root.effectiveKind === "delivered" ? 3400
            : root.effectiveKind === "rateLimited" ? 650
            : root.effectiveKind === "offline" ? 170
            : root.effectiveKind === "running" ? 720
            : root.effectiveKind === "victory" ? 1500
            : root.effectiveKind === "wiggle" ? 700
            : (root.effectiveKind === "foxfire" || root.effectiveKind === "checkmark") ? 4200
            : 2600
        running: root._animating
    }

    NumberAnimation on _propPhase {
        from: 0; to: 1
        loops: Animation.Infinite
        duration: root.effectiveKind === "notFound" ? 3000
            : (root.effectiveKind === "loading" || root.effectiveKind === "foxfire") ? 2400
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
        : (effectiveKind === "notFound" || effectiveKind === "delivered") ? 2.5 * Math.sin(_tau * _basePhase)
        : effectiveKind === "loading" ? 0.8 * Math.sin(_tau * _basePhase)
        : effectiveKind === "serverError" ? 2 * Math.sin(_tau * _basePhase)
        : effectiveKind === "running" ? -1.2 * Math.cos(_tau * _basePhase)
        : effectiveKind === "wiggle" ? -2 * Math.cos(_tau * _basePhase)
        : 0
    readonly property real _baseShiftX: !_animating ? 0
        : effectiveKind === "offline" ? artWidth * 0.004 * Math.sin(_tau * _basePhase)
        : 0

    // One SVG unit in pixels, and a 0 -> 1 -> 0 swell over the base cycle.
    readonly property real _unit: artWidth / 200
    readonly property real _swell: 0.5 - 0.5 * Math.cos(_tau * _basePhase)

    // Vertical bob and squash-and-stretch for the working / success foxes.
    readonly property real _baseShiftY: !_animating ? 0
        : effectiveKind === "running" ? -5 * _unit * _swell
        : effectiveKind === "wiggle" ? -2 * _unit * _swell
        : effectiveKind === "victory"
            ? _kf(_basePhase, [[0, 0], [0.3, -16], [0.5, -22], [0.7, -16], [1, 0]], 1) * _unit
        : 0
    readonly property real _baseScaleX: !_animating ? 1
        : effectiveKind === "victory"
            ? _kf(_basePhase, [[0, 1.05], [0.3, 0.97], [0.5, 1], [0.7, 0.98], [1, 1.05]], 1)
        : (effectiveKind === "foxfire" || effectiveKind === "checkmark") ? 1 + 0.012 * _swell
        : 1
    readonly property real _baseScaleY: !_animating ? 1
        : effectiveKind === "victory"
            ? _kf(_basePhase, [[0, 0.94], [0.3, 1.04], [0.5, 1], [0.7, 1.03], [1, 0.94]], 1)
        : (effectiveKind === "foxfire" || effectiveKind === "checkmark") ? 1 + 0.04 * _swell
        : 1

    // Extra animated pieces of the current fox, by where they sit.
    readonly property var _parts: _art[effectiveKind].parts !== undefined ? _art[effectiveKind].parts : []
    readonly property var _partsUnder: _parts.filter(function (p) { return p.layer === "under" })
    readonly property var _partsBase: _parts.filter(function (p) { return p.layer === "base" })
    readonly property var _partsOver: _parts.filter(function (p) { return p.layer === "over" })

    readonly property real _propAngle: !_animating ? 0
        : effectiveKind === "serverError" ? 360 * _propPhase
        : effectiveKind === "notFound" ? 6 * Math.sin(_tau * _propPhase)
        : 0
    readonly property real _propShiftY: !_animating ? 0
        : effectiveKind === "notFound" ? -_artHeight * 0.03 * Math.sin(_tau * _propPhase)
        : effectiveKind === "loading" ? -_artHeight * 0.02 * Math.sin(_tau * _propPhase)
        : effectiveKind === "foxfire" ? -_artHeight * 0.02 * (0.5 - 0.5 * Math.cos(_tau * _propPhase))
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

            // One animated piece of a fox (a leg, the tail, the eyes, a confetti
            // chip, a heart...). Each is a tiny SVG of its own, posed every frame by
            // root._pose(), so it can move independently of the body image.
            Component {
                id: partDelegate

                Item {
                    id: part
                    required property var modelData

                    readonly property var spec: modelData
                    readonly property real unit: art.width / 200
                    readonly property bool live: root._animating && spec.dur > 0
                    property real phase: 0
                    readonly property var pose: live
                        ? root._pose(spec.anim, (phase + spec.t) % 1, spec.m)
                        : root._restPose
                    readonly property real pivotX: (spec.pivot[0] - spec.box[0]) * unit
                    readonly property real pivotY: (spec.pivot[1] - spec.box[1]) * unit

                    NumberAnimation on phase {
                        from: 0; to: 1
                        loops: Animation.Infinite
                        duration: Math.max(1, part.spec.dur)
                        running: part.live
                    }

                    x: (spec.box[0] + pose.dx) * unit
                    y: (spec.box[1] + pose.dy) * unit + (spec.floats ? root._propShiftY : 0)
                    width: spec.box[2] * unit
                    height: spec.box[3] * unit
                    z: spec.z
                    opacity: live ? pose.op : spec.rest
                    transform: Rotation {
                        origin.x: part.pivotX
                        origin.y: part.pivotY
                        angle: part.live ? part.pose.rot : part.spec.restRot
                    }

                    // The squash sits one level below the rotation, so a leg
                    // shortens along its own axis before it swings.
                    Item {
                        anchors.fill: parent
                        transform: Scale {
                            origin.x: part.pivotX
                            origin.y: part.pivotY
                            xScale: part.pose.sx
                            yScale: part.pose.sy
                        }

                        // `rv` reveals the piece left to right (the drawn-on tick).
                        Item {
                            width: parent.width * part.pose.rv
                            height: parent.height
                            clip: part.spec.anim === "tickPop"

                            Image {
                                width: part.width
                                height: part.height
                                source: root._uri(part.spec.svg)
                                sourceSize.width: Math.max(1, Math.round(part.width * 2))
                                sourceSize.height: Math.max(1, Math.round(part.height * 2))
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                            }
                        }
                    }
                }
            }

            // Behind the body (shadows)
            Repeater {
                model: root._partsUnder
                delegate: partDelegate
            }

            // The body, plus the parts that follow its motion
            Item {
                id: baseGroup
                x: root._baseShiftX
                y: root._baseShiftY
                width: art.width
                height: art.height
                transformOrigin: Item.Bottom
                scale: root._baseScale
                rotation: root._baseRotation
                transform: Scale {
                    origin.x: baseGroup.width / 2
                    origin.y: baseGroup.height
                    xScale: root._baseScaleX
                    yScale: root._baseScaleY
                }

                Image {
                    id: baseImg
                    anchors.fill: parent
                    source: root._uri(root._art[root.effectiveKind].base)
                    sourceSize.width: Math.round(art.width * 2)
                    sourceSize.height: Math.round(art.height * 2)
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                }

                Repeater {
                    model: root._partsBase
                    delegate: partDelegate
                }
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

            // In front of the body (props, confetti, sparks, hearts)
            Repeater {
                model: root._partsOver
                delegate: partDelegate
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
