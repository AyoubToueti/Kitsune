//! Building the AniList authorise URL and reading the token back out of the
//! redirect.
//!
//! Both halves are pure, so the whole flow is asserted without a browser, a
//! deep link, or a network. That matters because the redirect is the part most
//! likely to differ from the documentation on a real machine.

/// The scheme the OS routes back to this app.
///
/// Must match `plugins.deep-link.desktop.schemes` in `tauri.conf.json` and the
/// redirect URL registered on the AniList application, or the browser has
/// nowhere to send the token.
pub const REDIRECT_SCHEME: &str = "kitsune";

/// The host part of the redirect URL.
///
/// AniList requires a full URL as the redirect target, and a custom scheme
/// needs *something* after it. The value is arbitrary as long as it matches
/// what was registered.
pub const REDIRECT_HOST: &str = "auth";

/// The client id of the Kitsune application on AniList.
///
/// NOT a secret: it is visible in the authorise URL the reader's browser opens,
/// and it only identifies the application rather than any user. Every install
/// of Kitsune ships the same value, which is how "sign in with X" normally
/// works -- the user authorises, and AniList hands back a token for THEIR
/// account. The token, not this, is the credential.
pub const CLIENT_ID: &str = "51778";

/// The URL a reader opens to authorise Kitsune.
///
/// Implicit grant, so `response_type=token`: AniList returns the access token
/// directly in the redirect fragment rather than an intermediate code. That
/// suits a desktop app, which has nowhere safe to keep a client secret and so
/// cannot use the authorization-code exchange.
pub fn authorize_url() -> String {
    format!(
        "https://anilist.co/api/v2/oauth/authorize?client_id={CLIENT_ID}&response_type=token"
    )
}

/// Pull the access token out of the URL AniList redirects to.
///
/// The token arrives in the FRAGMENT (`#access_token=...`), not the query
/// string, because the implicit grant is designed so the token never reaches a
/// server. AniList is also documented to append `token_type` and `expires_in`
/// alongside it, so the fragment is parsed as a whole rather than assumed to be
/// the token alone.
///
/// Deliberately tolerant of the path between scheme and fragment: the exact
/// shape the OS delivers (`kitsune://auth#...` versus `kitsune://auth/#...`)
/// is not something we control, and rejecting a good token over a slash would
/// be a poor trade. What is NOT tolerated is a URL that is not ours at all --
/// accepting one would let any deep link set the token.
pub fn parse_token(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme != REDIRECT_SCHEME {
        return None;
    }

    // Everything after the first `#`, which is where the implicit grant puts
    // its parameters.
    let fragment = rest.split_once('#')?.1;

    for pair in fragment.split('&') {
        let (key, value) = pair.split_once('=')?;
        if key != "access_token" {
            continue;
        }

        let token = percent_decode(value).trim().to_string();
        // An empty parameter is not a credential; returning it would store a
        // blank token and turn every request into a 401.
        return if token.is_empty() { None } else { Some(token) };
    }

    None
}

/// Decode the percent-escapes a URL fragment may carry.
///
/// AniList's tokens are JWT-shaped -- base64url plus dots -- so they need no
/// escaping at all, and this exists only so a token that does arrive escaped is
/// read correctly rather than stored with `%2F` in it.
///
/// Only escapes that decode to a single ASCII byte are applied. Anything else
/// is kept verbatim, which matters more than it looks: `%de` is a syntactically
/// valid escape, and decoding it produces a lone `0xDE` that is not valid UTF-8.
/// A lossy conversion would then silently CORRUPT the token into something with
/// a replacement character in it, and the reader would see inexplicable 401s
/// rather than a sign-in prompt. Non-ASCII bytes cannot appear in a real token,
/// so treating them as literal `%` is the safe reading.
///
/// Written out rather than pulled in as a dependency for the dozen lines it
/// takes, and because `percent-encoding` is not otherwise needed here.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            // A plus in a URL fragment means a literal plus, not a space: the
            // form-encoding rule does not apply here.
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) if byte.is_ascii() => {
                        out.push(byte);
                        i += 3;
                    }
                    // Not an escape we should apply: either the two characters
                    // after `%` are not hex, or they decode to a byte that
                    // cannot belong to a token. Kept verbatim rather than
                    // dropped, so nothing is silently shortened or mangled.
                    _ => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_authorize_url_names_the_application_and_the_implicit_grant() {
        let url = authorize_url();

        assert!(url.starts_with("https://anilist.co/api/v2/oauth/authorize?"));
        assert!(url.contains(&format!("client_id={CLIENT_ID}")));
        // The implicit grant returns a token directly; `code` would mean the
        // authorization-code flow, which needs a client secret.
        assert!(url.contains("response_type=token"));
    }

    #[test]
    fn a_token_is_read_from_the_fragment() {
        let url = "kitsune://auth#access_token=secret-token";

        assert_eq!(parse_token(url).as_deref(), Some("secret-token"));
    }

    #[test]
    fn the_other_fragment_parameters_are_ignored() {
        // AniList documents sending these alongside the token.
        let url = "kitsune://auth#access_token=secret-token&token_type=Bearer&expires_in=31536000";

        assert_eq!(parse_token(url).as_deref(), Some("secret-token"));
    }

    /// The exact path the OS delivers is not ours to control, so a trailing
    /// slash before the fragment must not cost the reader their token.
    #[test]
    fn a_trailing_slash_before_the_fragment_is_tolerated() {
        let url = "kitsune://auth/#access_token=secret-token";

        assert_eq!(parse_token(url).as_deref(), Some("secret-token"));
    }

    #[test]
    fn the_token_is_found_whatever_its_position_in_the_fragment() {
        let url = "kitsune://auth#expires_in=31536000&access_token=secret-token";

        assert_eq!(parse_token(url).as_deref(), Some("secret-token"));
    }

    /// A deep link from anywhere else must never set the token, or any handler
    /// on the machine could hand Kitsune a credential of its choosing.
    #[test]
    fn a_url_for_another_scheme_is_rejected() {
        assert_eq!(parse_token("evil://auth#access_token=stolen"), None);
        assert_eq!(
            parse_token("https://example.com/#access_token=stolen"),
            None
        );
    }

    #[test]
    fn a_url_with_no_fragment_yields_nothing() {
        assert_eq!(parse_token("kitsune://auth"), None);
        assert_eq!(parse_token("kitsune://auth?"), None);
    }

    #[test]
    fn a_fragment_without_a_token_yields_nothing() {
        assert_eq!(parse_token("kitsune://auth#token_type=Bearer"), None);
    }

    /// An empty token is not a credential, and storing it would make every
    /// request 401 instead of leaving the reader signed out.
    #[test]
    fn an_empty_token_yields_nothing() {
        assert_eq!(parse_token("kitsune://auth#access_token="), None);
        assert_eq!(parse_token("kitsune://auth#access_token=%20"), None);
    }

    #[test]
    fn the_token_is_trimmed() {
        let url = "kitsune://auth#access_token=%20secret-token%20";

        assert_eq!(parse_token(url).as_deref(), Some("secret-token"));
    }

    #[test]
    fn percent_escapes_in_the_token_are_decoded() {
        // A `%` is legal in a fragment, so a naive read would corrupt it.
        let url = "kitsune://auth#access_token=abc%2Fdef";

        assert_eq!(parse_token(url).as_deref(), Some("abc/def"));
    }

    /// A `+` in a fragment is a literal plus, not a space; applying the
    /// form-encoding rule here would silently alter the token.
    #[test]
    fn a_plus_is_not_decoded_to_a_space() {
        let url = "kitsune://auth#access_token=abc+def";

        assert_eq!(parse_token(url).as_deref(), Some("abc+def"));
    }

    /// A `%` followed by two hex digits is syntactically a valid escape, but if
    /// it decodes to a non-ASCII byte it cannot be part of a real token.
    /// Decoding it would produce invalid UTF-8, and the lossy conversion would
    /// corrupt the token into something with a replacement character -- which
    /// would fail every request instead of prompting a sign-in.
    #[test]
    fn an_escape_that_is_not_ascii_is_left_alone() {
        let url = "kitsune://auth#access_token=abc%def";

        assert_eq!(parse_token(url).as_deref(), Some("abc%def"));
    }

    /// The ASCII case still decodes, which is the reason the decoder exists.
    #[test]
    fn an_ascii_escape_is_still_decoded() {
        let url = "kitsune://auth#access_token=abc%2Fdef";

        assert_eq!(parse_token(url).as_deref(), Some("abc/def"));
    }

    #[test]
    fn a_url_with_no_scheme_separator_is_rejected() {
        assert_eq!(parse_token("nonsense"), None);
    }
}