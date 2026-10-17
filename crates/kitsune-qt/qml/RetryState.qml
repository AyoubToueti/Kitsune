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

        if (isRateLimit(message)) {
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

    // Whether a failure message names a rate limit. Kept tolerant of wording:
    // the backend sends "429" and "rate limited ...", and either is enough.
    function isRateLimit(message) {
        if (!message) return false
        const m = String(message).toLowerCase()
        return m.indexOf("429") !== -1 || m.indexOf("rate limit") !== -1
    }

    // Called when a page loads successfully: the overlay goes away and the
    // backoff ladder resets, so the NEXT burst of failures starts small again.
    function reset() {
        state.active = false
        state.locked = false
        state.remaining = 0
        state.attempts = 0
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