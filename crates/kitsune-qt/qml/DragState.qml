// The state of a card being dragged into a watch list.
//
// A singleton because the drag spans three unrelated components: the card
// starts it, the full-screen overlay renders it, and the drop writes it. None
// of them can pass a reference to the others -- the overlay is a sibling of
// the page stack, not a child of the card.
//
// An `Item` rather than a `QtObject`: the toast timer is a child, and
// `QtObject` has no default property to hold it (same reason `RetryState` is
// an Item).
pragma Singleton
import QtQuick

Item {
    id: state

    // Whether a drag is in progress.
    property bool active: false

    // The work being dragged. `animeId` is what the write needs; the other two
    // only dress the ghost.
    property int animeId: 0
    // The id's namespace. The drag only carries it so the write can name the
    // right work: `animeId` alone collides across sources.
    property string animeProvider: "anilist"
    property string title: ""
    property string cover: ""

    // Cursor position in WINDOW coordinates, so the ghost and the overlay can
    // both place themselves without knowing which item started the drag.
    property real cursorX: 0
    property real cursorY: 0

    // The wire status currently under the cursor, or "" when over nothing.
    // The overlay sets this; `finish` reads it.
    property string hoverStatus: ""

    // The confirmation shown after a successful drop.
    property string toastMessage: ""
    property bool toastVisible: false

    // Label for a status, for the toast and the targets. `ListStatusStore`
    // owns the vocabulary so the two cannot drift.
    function labelFor(status) {
        return ListStatusStore.labelFor(status)
    }

    // Start a drag from a card. Coordinates are window-space: the caller
    // passes `mapToItem(null, ...)` because a card lives inside a GridView
    // delegate, so its own `mouse.x` is relative to the delegate.
    function begin(id, workTitle, coverUrl, x, y, provider) {
        state.animeId = id
        state.animeProvider = provider || "anilist"
        state.title = workTitle
        state.cover = coverUrl
        state.cursorX = x
        state.cursorY = y
        state.hoverStatus = ""
        state.active = true
    }

    function move(x, y) {
        if (!state.active) return
        state.cursorX = x
        state.cursorY = y
    }

    function setHover(status) {
        if (!state.active) return
        // Guarded so the overlay can push on every cursor change without
        // re-triggering its own highlight bindings.
        if (state.hoverStatus !== status) state.hoverStatus = status
    }

    // Drop. No target under the cursor means "changed my mind": reset, and do
    // NOT write.
    function finish() {
        if (!state.active) return

        const status = state.hoverStatus
        const provider = state.animeProvider
        const id = state.animeId
        const label = labelFor(status)

        reset()

        if (status === "" || id <= 0) return

        // No sign-in gate. The list is local, so the drop is written and
        // pushed on the next sync; the old check here is what made dragging
        // silently do nothing for a reader without an AniList account.

        // Dropping on the list the work is already on is a no-op: re-writing
        // the same status only costs a round trip.
        if (ListStatusStore.statusFor(id, provider) === status) {
            showToast(state.title + " is already in " + label)
            return
        }

        ListStatusStore.setStatus(id, status, provider)
        showToast(state.title + " moved to " + label)
    }

    // Abandon the drag without acting.
    function cancel() {
        reset()
    }

    function reset() {
        state.active = false
        state.animeProvider = "anilist"
        state.animeId = 0
        state.title = ""
        state.cover = ""
        state.hoverStatus = ""
    }

    function showToast(message) {
        state.toastMessage = message
        state.toastVisible = true
        toastTimer.restart()
    }

    Timer {
        id: toastTimer
        interval: 3000
        onTriggered: state.toastVisible = false
    }
}