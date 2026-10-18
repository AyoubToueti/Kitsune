// The app-wide "request failed, retry?" state.
//
// A page that fails to load reports the failure here; the root window shows a
// centred retry overlay bound to this singleton, and re-runs the current page
// when the reader presses Retry. This replaces the old behaviour of leaving a
// screen silently empty (or asking the reader to restart the app).
//
// Rate limits get special treatment: AniList answers 429 with a `Retry-After`,
// and hammering it again only deepens the limit. So a 429 LOCKS the Retry
// button behind a countdown, and each consecutive failed retry makes the wait
// longer (exponential backoff, capped). A non-rate-limit failure shows the
// overlay unlocked, so the reader can retry at once.
//
// An `Item`, not a `QtObject`: the `Timer` below is a child, and `QtObject` has
// no default property to hold it (the same reason `ListStatusStore` is an Item).
pragma Singleton
import QtQuick

Item {
    id: state

    // Whether the retry overlay is showing at all.
    property bool active: false
    // The failure message, shown under the heading.
    property string message: ""
    // Which kind of failure this is: "rateLimit", "connection" or "other".
    //
    // Pages report the backend's own error string, which is accurate but
    // unreadable ("provider returned HTTP 429: rate limited by AniList; try
    // again in about 20s"). Classifying once here lets the overlay say
    // something a reader understands, and lets the rate-limit wording read the
    // LIVE countdown rather than a number baked into that string.
    property string kind: ""

    // The overlay's heading, per failure kind.
    readonly property string headline: state.kind === "rateLimit"
        ? "AniList is rate limiting us"
        : (state.kind === "connection"
            ? "Can't reach AniList"
            : "Couldn't load this page")

    // The line under the heading. The rate-limit case reads `remaining`, so the
    // seconds tick down with the button instead of freezing at whatever delay
    // the backend happened to mention. Once the lock lapses the count is gone,
    // so the wording changes rather than saying "in 0s".
    readonly property string detail: state.kind === "rateLimit"
        ? (state.locked
            ? ("Too many requests. You can retry in " + state.remaining + "s.")
            : "Too many requests. You can retry now.")
        : (state.kind === "connection"
            ? "That looks like a connection problem. Check your internet and try again."
            : state.message)
    // Whether the Retry button is currently locked behind a countdown.
    property bool locked: false
    // Whole seconds left until the button unlocks.
    property int remaining: 0
    // Consecutive rate-limit failures, which drive the backoff growth. Reset on
    // any success.
    property int attempts: 0

    // The first 429 lock, in seconds.
    readonly property int baseSeconds: 5
    // The longest a lock ever grows to.
    readonly property int maxSeconds: 120

    // Emitted when the reader presses Retry. The window connects this and
    // re-runs the current page.
    signal retryRequested()

    // A page reports a load failure. A rate limit locks the button; anything
    // else leaves it immediately pressable.
    function reportFailure(message) {
        state.message = message
        state.active = true
        state.kind = classify(message)

        if (state.kind === "rateLimit") {
            state.attempts += 1
            // 5, 10, 20, 40, 80, 120 (capped).
            let secs = state.baseSeconds * Math.pow(2, state.attempts - 1)
            if (secs > state.maxSeconds) secs = state.maxSeconds
            state.remaining = secs
            state.locked = true
            countdown.restart()
        } else {
            state.locked = false
            state.remaining = 0
        }
    }

    // Which of the three failure shapes `message` is.
    //
    // Rate limit and connection failures get their own wording; anything else
    // keeps the backend's text, which is the only case where it is specific
    // enough to be worth showing. Matching is tolerant of wording because the
    // transport strings have changed before and either spelling is enough.
    function classify(message) {
        const m = message ? String(message).toLowerCase() : ""
        if (m === "") return "other"
        if (m.indexOf("429") !== -1 || m.indexOf("rate limit") !== -1) {
            return "rateLimit"
        }
        const connectionWords = [
            "transport", "timed out", "timeout", "connection", "dns",
            "network", "request failed", "unreachable"
        ]
        for (let i = 0; i < connectionWords.length; i++) {
            if (m.indexOf(connectionWords[i]) !== -1) return "connection"
        }
        return "other"
    }

    // Kept as the one place the rate-limit rule is spelled, so a caller that
    // only needs the yes/no answer does not duplicate the match.
    function isRateLimit(message) {
        return classify(message) === "rateLimit"
    }

    // Whether the failure is one worth waiting out, rather than a page that
    // genuinely has nothing to show.
    //
    // A page keeps its loading skeleton up for these, so the reader sees the
    // screen still trying behind the overlay instead of a blank page that
    // looks like the app gave up. A content failure (a 404, a bad query) is
    // NOT transient: its skeleton would pulse forever for data that will
    // never arrive.
    function isTransient(message) {
        const kind = classify(message)
        return kind === "rateLimit" || kind === "connection"
    }

    // Called when a page loads successfully: the overlay goes away and the
    // backoff ladder resets, so the NEXT burst of failures starts small again.
    function reset() {
        state.active = false
        state.locked = false
        state.remaining = 0
        state.attempts = 0
        state.kind = ""
        countdown.stop()
    }

    // The reader pressed Retry. Refuse while locked; otherwise ask the window
    // to re-run the current page. The overlay stays up until the page either
    // succeeds (`reset`) or fails again (`reportFailure`, which re-locks on a
    // fresh 429).
    function requestRetry() {
        if (state.locked) return
        state.retryRequested()
    }

    Timer {
        id: countdown
        interval: 1000
        repeat: true
        onTriggered: {
            state.remaining -= 1
            if (state.remaining <= 0) {
                state.remaining = 0
                state.locked = false
                countdown.stop()
            }
        }
    }
}