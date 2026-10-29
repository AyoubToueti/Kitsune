// A shared cache of the reader's list statuses, so every poster card's
// quick-add control can tell whether a work is already on a list (and show a
// pencil rather than a "+") without each card issuing its own lookup.
//
// One `userList()` read fills the whole map; a write updates it optimistically
// and then re-reads on `listChanged`. Signed out, the map stays empty and the
// quick-add control simply offers to sign in.
pragma Singleton
import QtQuick

// An `Item` rather than a `QtObject` because the `Connections` block below is
// a child, and `QtObject` has no default property to hold it.
Item {
    id: store

    // The Backend, set once by the window. Null until then.
    property var backend: null

    // animeId (as a string key) -> { status, entryId }
    property var statuses: ({})

    // Whether a token is stored; the quick-add control hides its menu (and
    // offers sign-in) when this is false.
    property bool signedIn: false

    function setBackend(b) {
        backend = b
        if (b) {
            b.authStatus()
            refresh()
        }
    }

    // The reader's list is a LOCAL file, so it is read whether or not an
    // AniList account is connected. Gating this on `signedIn` was what made an
    // offline reader see an empty library and a "sign in to add" prompt: the
    // store never asked the backend, so every card said "not on your list".
    function refresh() {
        if (backend) backend.userList()
    }

    // The status wire value for a work, or "" when not on the list.
    //
    // Two names for one lookup: `statusFor` reads best in the store's own
    // code, while the poster cards ask for `getStatus`. Both exist so a card
    // never has to guess which spelling the store uses.
    function statusFor(animeId) {
        const entry = statuses[String(animeId)]
        return entry ? (entry.status || "") : ""
    }
    function getStatus(animeId) {
        return statusFor(animeId)
    }

    // The list entry's id for a work, or 0 when not on the list.
    function entryIdFor(animeId) {
        const entry = statuses[String(animeId)]
        return entry ? (entry.entryId || 0) : 0
    }

    // Whether the work is on any list.
    function isListed(animeId) {
        return statusFor(animeId) !== ""
    }

    // Optimistically set a work's status, then write it. `status` is the wire
    // value ("current" | "completed" | "planning" | ...). Progress is left
    // alone (0), so moving a work between lists does not reset how far the
    // reader got.
    function setStatus(animeId, status) {
        const next = Object.assign({}, statuses)
        const existing = next[String(animeId)] || {}
        next[String(animeId)] = {
            status: status,
            entryId: existing.entryId || 0
        }
        statuses = next
        if (backend) backend.setListEntry(animeId, status, 0)
    }

    // Remove a work from the reader's list.
    //
    // Keyed on the WORK, not the list entry: a work added while AniList was
    // unreachable has no entry id, and gating on one would make it impossible
    // to remove.
    function remove(animeId) {
        const next = Object.assign({}, statuses)
        delete next[String(animeId)]
        statuses = next
        if (backend) backend.deleteListEntry(animeId)
    }

    // The statuses a quick-add offers, in the order shown. Labels are the
    // reader's words; values are AniList's own wire strings.
    //
    // Deliberately only three: the popup menu is small, and these are the
    // moves a reader makes often. `allStatuses` below is the full set.
    readonly property var menuStatuses: [
        { "value": "current", "label": "Watching" },
        { "value": "completed", "label": "Completed" },
        { "value": "planning", "label": "Plan to watch" }
    ]

    // Every status the API accepts, for surfaces that can show them all --
    // the drag-to-list overlay, and the My List tabs. Same order as
    // `MyListPage.tabs`, so the two do not disagree about what "the lists"
    // are.
    readonly property var allStatuses: [
        { "value": "current", "label": "Watching" },
        { "value": "planning", "label": "Plan to watch" },
        { "value": "completed", "label": "Completed" },
        { "value": "paused", "label": "On hold" },
        { "value": "dropped", "label": "Dropped" }
    ]

    // The reader's word for a status. Reads `allStatuses`, not `menuStatuses`:
    // the drag overlay can drop on "dropped", which the menu never offers, and
    // a toast saying "moved to In list" would be a lie.
    function labelFor(status) {
        for (let i = 0; i < allStatuses.length; i++) {
            if (allStatuses[i].value === status) return allStatuses[i].label
        }
        return "In list"
    }

    Connections {
        target: store.backend
        function onAuthStatusReady(signedInNow) {
            store.signedIn = signedInNow
            // Read either way. The list is local, so a signed-out reader still
            // has one -- clearing it here made the whole library disappear the
            // moment AniList was unreachable or never connected.
            store.refresh()
        }
        function onUserListReady(json) {
            const entries = JSON.parse(json) || []
            const next = {}
            for (let i = 0; i < entries.length; i++) {
                const e = entries[i]
                if (e && e.anime) {
                    next[String(e.anime.id)] = {
                        status: e.status || "",
                        entryId: e.entryId || 0
                    }
                }
            }
            store.statuses = next
        }
        // Any successful write re-reads, so the cache cannot drift.
        function onListChanged() { store.refresh() }
    }
}