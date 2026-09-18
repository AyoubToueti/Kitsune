//! Canonicalise a release name before parsing.
//!
//! Ported from Sonarr's `PreSubstitution`/`CleanReleaseGroup` steps, trimmed
//! to what anime indexers actually emit. The goal is not to rewrite the name
//! the user sees -- that is preserved verbatim on [`crate::types::Release`] --
//! but to give the parser a single spelling to match against, so a name using
//! an en dash or an underscore parses the same as one using a hyphen.
//!
//! Every function here is pure and total: no input panics, and the output is
//! always a valid `String`.

/// Rewrite a release name into the canonical form the parser expects.
///
/// The transformations are deliberately conservative. Anything that could
/// change which release a name refers to (dropping a group tag, reordering
/// words) is left alone; only spelling differences are collapsed.
pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());

    for ch in name.chars() {
        match ch {
            // Every dash-like glyph becomes an ASCII hyphen, so the episode
            // patterns only have to know about one separator.
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => out.push('-'),
            // Underscores and dots are word separators in release names
            // ("Show_01", "Show.01"), not part of a word.
            '_' | '.' => out.push(' '),
            // Full-width brackets and parens, common in CJK releases.
            '\u{FF08}' => out.push('('),
            '\u{FF09}' => out.push(')'),
            '\u{FF3B}' => out.push('['),
            '\u{FF3D}' => out.push(']'),
            other => out.push(other),
        }
    }

    collapse_whitespace(&out)
}

/// Collapse runs of whitespace to a single space and trim the ends.
///
/// Separate from [`normalize`] so the whitespace rule can be tested on its
/// own, and so callers that only need the collapse can reuse it.
pub fn collapse_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_space = false;

    for ch in input.chars() {
        if ch.is_whitespace() {
            // Defer the space: it is only emitted if a non-space follows, so
            // trailing whitespace never reaches the output.
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(ch);
    }

    out
}

/// The leading `[Group]` tag, if the name starts with one.
///
/// Returns the group name without its brackets. A name with no leading tag
/// yields `None` rather than an empty string, so callers can tell "no group"
/// from "a group whose name is blank".
pub fn leading_subgroup(name: &str) -> Option<&str> {
    let trimmed = name.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let end = rest.find(']')?;
    let group = rest[..end].trim();
    (!group.is_empty()).then_some(group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_dashes_become_hyphens() {
        assert_eq!(normalize("Show \u{2013} 01"), "Show - 01");
        assert_eq!(normalize("Show \u{2014} 01"), "Show - 01");
    }

    #[test]
    fn underscores_and_dots_become_spaces() {
        assert_eq!(normalize("Show_01"), "Show 01");
        assert_eq!(normalize("Show.01"), "Show 01");
    }

    #[test]
    fn full_width_brackets_are_folded() {
        assert_eq!(normalize("\u{FF3B}Group\u{FF3D} Show"), "[Group] Show");
    }

    #[test]
    fn whitespace_runs_collapse() {
        assert_eq!(normalize("Show    -    01"), "Show - 01");
    }

    #[test]
    fn leading_and_trailing_whitespace_is_trimmed() {
        assert_eq!(normalize("   Show - 01   "), "Show - 01");
    }

    #[test]
    fn collapse_whitespace_handles_tabs_and_newlines() {
        assert_eq!(collapse_whitespace("a\t\tb\nc"), "a b c");
    }

    #[test]
    fn collapse_whitespace_of_only_spaces_is_empty() {
        assert_eq!(collapse_whitespace("   \t\n "), "");
    }

    #[test]
    fn normalize_is_idempotent() {
        let once = normalize("[SubsPlease] Show \u{2013} 01 (1080p)");
        assert_eq!(normalize(&once), once);
    }

    #[test]
    fn leading_subgroup_reads_the_first_bracket() {
        assert_eq!(
            leading_subgroup("[SubsPlease] Show - 01"),
            Some("SubsPlease")
        );
    }

    #[test]
    fn leading_subgroup_is_none_without_a_tag() {
        assert_eq!(leading_subgroup("Show - 01"), None);
    }

    #[test]
    fn leading_subgroup_is_none_for_an_empty_tag() {
        assert_eq!(leading_subgroup("[] Show - 01"), None);
    }

    #[test]
    fn leading_subgroup_ignores_a_later_bracket() {
        // Only a *leading* tag counts; a bracket mid-name is not a group.
        assert_eq!(leading_subgroup("Show [1080p] - 01"), None);
    }
}
