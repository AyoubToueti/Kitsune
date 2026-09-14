//! Launching an external video player.
//!
//! Kept separate from [`super::state`] so the argument shape and the
//! preference resolution can be tested without a running session or a real
//! player binary.

use std::process::{Child, Command, Stdio};

/// The player launched when the user has not chosen one.
///
/// mpv is the app's own preference: it renders styled ASS subtitles, which
/// the in-app `<video>` element cannot, so it is the right fallback when a
/// release ships softsubs.
pub const DEFAULT_PLAYER: &str = "mpv";

/// Players offered in the UI, in preference order.
///
/// Not a whitelist: [`resolve_player`] accepts any name, so a user with a
/// player this list has never heard of can still use it.
pub const SUGGESTED_PLAYERS: &[&str] = &["mpv", "vlc", "celluloid", "totem"];

/// Resolve the player to launch from a stored preference.
///
/// A blank or missing preference falls back to [`DEFAULT_PLAYER`]. The name
/// is deliberately NOT validated against [`SUGGESTED_PLAYERS`]: refusing to
/// launch a player the user typed would be worse than letting the spawn fail
/// with a message that names it.
pub fn resolve_player(preference: Option<&str>) -> String {
    match preference.map(str::trim) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => DEFAULT_PLAYER.to_string(),
    }
}

/// The arguments a player is launched with.
///
/// Just the URL: mpv, vlc, celluloid and totem all accept a stream URL as
/// their first positional argument. Split out so the shape is asserted
/// without spawning a process.
pub fn player_args(url: &str) -> Vec<String> {
    vec![url.to_string()]
}

/// Build the command to launch a player, without spawning it.
///
/// Stdio is detached so a player with its own terminal UI (mpv) does not
/// fight the app's output.
pub fn player_command(player: &str, url: &str) -> Command {
    let mut cmd = Command::new(player);
    cmd.args(player_args(url))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

/// Spawn a player against a URL.
pub fn spawn_player(player: &str, url: &str) -> std::io::Result<Child> {
    player_command(player, url).spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_player_is_mpv() {
        assert_eq!(DEFAULT_PLAYER, "mpv");
    }

    #[test]
    fn resolve_player_falls_back_when_absent() {
        assert_eq!(resolve_player(None), DEFAULT_PLAYER);
    }

    #[test]
    fn resolve_player_falls_back_when_blank() {
        // A stored empty string must not become "launch nothing".
        assert_eq!(resolve_player(Some("")), DEFAULT_PLAYER);
        assert_eq!(resolve_player(Some("   ")), DEFAULT_PLAYER);
    }

    #[test]
    fn resolve_player_keeps_a_chosen_name() {
        assert_eq!(resolve_player(Some("vlc")), "vlc");
    }

    #[test]
    fn resolve_player_trims_whitespace() {
        assert_eq!(resolve_player(Some("  vlc  ")), "vlc");
    }

    #[test]
    fn resolve_player_accepts_a_name_not_in_the_suggestion_list() {
        // The list is a convenience, not a gate.
        assert_eq!(resolve_player(Some("my-player")), "my-player");
    }

    #[test]
    fn player_args_is_just_the_url() {
        assert_eq!(
            player_args("http://127.0.0.1:3030/x"),
            vec!["http://127.0.0.1:3030/x"]
        );
    }

    #[test]
    fn player_command_program_is_the_player() {
        let cmd = player_command("mpv", "http://x");
        assert_eq!(cmd.get_program().to_string_lossy(), "mpv");
    }

    #[test]
    fn player_command_passes_the_url_as_an_argument() {
        let cmd = player_command("mpv", "http://127.0.0.1:3030/stream");
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, vec!["http://127.0.0.1:3030/stream"]);
    }

    #[test]
    fn suggested_players_lead_with_the_default() {
        assert_eq!(SUGGESTED_PLAYERS.first().copied(), Some(DEFAULT_PLAYER));
    }
}
