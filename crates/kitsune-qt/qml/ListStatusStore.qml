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

    function refresh() {
        if (backend && signedIn) backend.userList()
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
    function remove(animeId) {
        const entryId = entryIdFor(animeId)
        if (entryId === 0) return
        const next = Object.assign({}, statuses)
        delete next[String(animeId)]
        statuses = next
        if (backend) backend.deleteListEntry(entryId)
    }

    // The statuses a quick-add offers, in the order shown. Labels are the
    // reader's words; values are AniList's own wire strings.
    readonly property var menuStatuses: [
        { "value": "current", "label": "Watching" },
        { "value": "completed", "label": "Completed" },
        { "value": "planning", "label": "Plan to watch" }
    ]

    function labelFor(status) {
        for (let i = 0; i < menuStatuses.length; i++) {
            if (menuStatuses[i].value === status) return menuStatuses[i].label
        }
        return "In list"
    }

    Connections {
        target: store.backend
        function onAuthStatusReady(signedInNow) {
            store.signedIn = signedInNow
            if (signedInNow) store.refresh()
            else store.statuses = ({})
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