// A short-lived cache for the two things a DetailPage refetches when it is
// rebuilt from scratch: the work itself and its episode catalogue.
//
// Why this exists: `goBack()` in main.qml keeps the popped page's state (the
// StackView item is never destroyed), but `goForward()` pushes a NEW instance
// with the same props. That fresh page runs `maybeLoad()` and re-requests
// everything, so Forward cost a skeleton flash plus (for a long runner) up to
// five Kitsu episode pages. Back was free; Forward was not.
//
// RAW JSON STRINGS, never parsed objects. `page.anime` is assigned the parsed
// object and shared by reference all over DetailPage; caching that same object
// would alias live UI state into the cache, so a later mutation would silently
// poison the entry. Re-parsing on read is cheap and always yields a fresh
// object.
//
// Session-only, in memory: a restart is a clean slate.
pragma Singleton
import QtQuick

QtObject {
    id: cache

    /// One hour, matching `ttl::CATALOGUE` in
    /// crates/kitsune-core/src/providers/cache.rs so the two layers agree about
    /// how stale an episode list may be.
    readonly property int ttlMs: 60 * 60 * 1000

    // Keys are "provider:id" -- ids live in per-source namespaces, so a bare id
    // would let an AniList work answer for a Kitsu one. Values are
    // { at: <epoch ms>, json: <string> }.
    property var animeEntries: ({})
    property var episodeEntries: ({})
    property var detailEntries: ({})

    /// The cache key for a work. Matches DetailPage.episodeKey().
    function key(id, provider) {
        return (provider || "anilist") + ":" + id
    }

    function isFresh(entry) {
        return entry !== undefined && entry !== null
            && typeof entry.json === "string" && entry.json !== ""
            && (Date.now() - entry.at) <= cache.ttlMs
    }

    // --- detail (partial paint) -------------------------------------------

    function detail(k) {
        const entry = cache.detailEntries[k]
        return cache.isFresh(entry) ? entry.json : ""
    }

    function putDetail(k, json) {
        if (!json || json === "null") return
        cache.detailEntries[k] = { at: Date.now(), json: json }
    }

    // --- the work ---------------------------------------------------------

    function anime(k) {
        const entry = cache.animeEntries[k]
        return cache.isFresh(entry) ? entry.json : ""
    }

    function putAnime(k, json) {
        // A "null" payload means the work does not exist; caching that would
        // pin a broken page for an hour after a transient provider gap.
        if (!json || json === "null") return
        cache.animeEntries[k] = { at: Date.now(), json: json }
    }

    // --- episodes ---------------------------------------------------------

    function episodes(k) {
        const entry = cache.episodeEntries[k]
        return cache.isFresh(entry) ? entry.json : ""
    }

    function putEpisodes(k, json) {
        if (!json || json === "null") return
        cache.episodeEntries[k] = { at: Date.now(), json: json }
    }

    // --- invalidation -----------------------------------------------------

    /// Drop one work. Used by DetailPage.reload(), the explicit Retry path,
    /// which must never be answered from cache.
    function invalidate(k) {
        delete cache.animeEntries[k]
        delete cache.episodeEntries[k]
        delete cache.detailEntries[k]
    }

    function clear() {
        cache.animeEntries = ({})
        cache.episodeEntries = ({})
        cache.detailEntries = ({})
    }
}