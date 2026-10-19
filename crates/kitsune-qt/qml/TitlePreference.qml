// Which title form the UI shows, in one place.
//
// Every list surface used to carry its own `displayTitle(item)` with the same
// hard-coded English-first order, duplicated across fourteen files. The order
// now lives here so a setting can change it once and have every surface follow.
//
// `QtObject` rather than `Item`: there is nothing to lay out and no `Timer`
// child, so it does not need a default property the way `RetryState` does.
pragma Singleton
import QtQuick

QtObject {
    id: pref

    /// `"english"` or `"romaji"`, as written by the settings store.
    ///
    /// The raw value is kept unvalidated so the reader's choice round-trips
    /// through a save untouched; `effectiveLanguage` decides what to actually
    /// use, the same way core's `Settings::title_language()` does.
    property string language: "english"

    /// The language actually in force. Anything unrecognised -- a value written
    /// by a newer build, a typo -- falls back to English rather than leaving
    /// every card untitled.
    readonly property string effectiveLanguage:
        language === "romaji" ? "romaji" : "english"

    /// The title to show for a work or a title object.
    ///
    /// Accepts either shape on purpose: most call sites pass the whole item
    /// (which nests `title`), while `topForms`-style callers may pass the title
    /// alone. One implementation means the fourteen delegations cannot drift.
    ///
    /// The chosen form wins, then the rest of the forms in order, and blank
    /// strings are skipped -- many works have no English title and some older
    /// entries have no romaji, so a strict "chosen form or nothing" would leave
    /// cards untitled.
    ///
    /// `userPreferred` is deliberately NOT consulted. The provider never
    /// populates it today, and an explicit app setting that a server field
    /// could silently override would look broken.
    function title(t) {
        if (!t) return "(untitled)"

        // An item nests its titles; a title object is the titles.
        const forms = t.title || t

        const candidates = effectiveLanguage === "romaji"
            ? [forms.romaji, forms.english, forms.native]
            : [forms.english, forms.romaji, forms.native]

        for (let i = 0; i < candidates.length; i++) {
            const candidate = candidates[i]
            if (typeof candidate === "string" && candidate.trim() !== "") {
                return candidate
            }
        }
        return "(untitled)"
    }
}