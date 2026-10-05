//! The native "Open With" app chooser.
//!
//! The app used to auto-launch the stored player the moment enough of a file
//! was buffered. That is convenient but wrong when the reader wants a
//! different player for one episode, so the launch now asks the desktop
//! environment which application to use.
//!
//! Why a content type and not the stream URL: the obvious "open this URL"
//! helpers (`xdg-open`, the OpenURI portal) choose by URI SCHEME, so an
//! `http://127.0.0.1/...` stream is offered to browsers, not video players.
//! [`gtk::AppChooserDialog::for_content_type`] instead lists the applications
//! registered for a MIME type, which is what puts mpv/vlc/celluloid/totem in
//! the dialog. The URL is never inspected; it is passed to the chosen player
//! as a positional argument afterwards.
//!
//! The GTK dialog is `!Send` and must be built on the main thread, so
//! [`choose_player_blocking`] is called from the Tauri main thread by the
//! command layer, never from a worker. The argv mapping is split out as a pure
//! function so the tricky part -- turning a `.desktop` command line into the
//! program plus extra args the launcher expects -- is unit tested without a
//! display server.

use serde::Serialize;

/// The MIME type the chooser lists applications for.
///
/// Anime releases are overwhelmingly Matroska, and every desktop video player
/// registers `video/x-matroska`. A player that only registers `video/mp4`
/// would not appear, but such a player would be useless for these files
/// anyway. Chosen over `video/*` because the dialog takes a concrete type, not
/// a wildcard.
pub const VIDEO_CONTENT_TYPE: &str = "video/x-matroska";

/// A chosen player, resolved from a desktop application's command line.
///
/// `program` is the executable and `extra_args` are its own arguments with the
/// URL placeholder removed. Together they fit
/// [`super::launch::spawn_player`] directly: the program is spawned, the extra
/// args come first, and the stream URL is appended last as a positional
/// argument -- which mpv, vlc, celluloid and totem all accept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerChoice {
    /// The executable to spawn.
    pub program: String,
    /// The application's own arguments, in order, minus the URL placeholder.
    pub extra_args: Vec<String>,
}

/// A player offered in the in-app picker, before anything is launched.
///
/// Carries enough to render a row AND to launch it later, so the picker does
/// not have to re-resolve anything. `id` is the executable path, which is what
/// de-duplicates a player found both by GIO and by the PATH scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerOption {
    /// Stable identity: the resolved executable path.
    pub id: String,
    /// Human name for the row, e.g. "VLC media player" or "mpv".
    pub name: String,
    /// The program to spawn.
    pub program: String,
    /// The program's own arguments, in order, minus any URL placeholder.
    pub extra_args: Vec<String>,
    /// True for the reader's stored preference, so the picker can mark it.
    pub is_default: bool,
    /// The player's icon as a `data:` URL, or `None` when none was found.
    ///
    /// A `data:` URL rather than a path: the app's CSP allows `data:` but not
    /// `file:`, so the bytes have to travel inline. `None` is not a failure --
    /// the picker draws its own fallback glyph.
    pub icon: Option<String>,
}

/// An icon `data:` URL for a player's executable, if one can be found.
///
/// The icon name is the executable's file stem: a `.desktop` file names its
/// icon after the app (`vlc`), and the known-player scan matches the binary
/// the same way. Failure is silent -- a missing icon is cosmetic.
pub fn icon_for_program(program: &str) -> Option<String> {
    let stem = std::path::Path::new(program)
        .file_stem()
        .and_then(|s| s.to_str())?;
    let roots = super::icons::icon_theme_roots();
    let path = super::icons::resolve_icon_path(stem, &roots)?;
    super::icons::icon_data_url(&path)
}

/// Known players to add when they are installed but not registered.
///
/// GIO lists only what a `.desktop` file declares, so a player with no desktop
/// entry is invisible to it -- mpv on a bare NixOS profile is exactly that
/// case, and it silently vanished from the chooser. These names are probed on
/// `PATH` and added when present. A HINT list, not the whole list: discovery
/// still supplies everything the desktop knows about, and this only fills the
/// gap for the players this app is most likely to be paired with.
const KNOWN_PLAYERS: &[&str] = &["mpv", "vlc", "celluloid", "totem", "haruna", "smplayer"];

/// Build the player list: everything registered for the content type, plus any
/// known player that is installed but not registered.
///
/// `registered` is what the platform discovered (GIO on Linux, empty
/// elsewhere); `on_path` answers "is this program installed, and where". The
/// two seams are injected so the merge -- the part with the de-duplication and
/// the preference marking -- is tested without GIO or a real filesystem.
pub fn merge_players(
    registered: Vec<PlayerChoice>,
    known: &[&str],
    on_path: &dyn Fn(&str) -> Option<String>,
    default_program: Option<&str>,
) -> Vec<PlayerOption> {
    let mut options: Vec<PlayerOption> = Vec::new();

    // Registered players first: the desktop's own opinion of what opens this
    // type, which is the most trustworthy source.
    for choice in registered {
        // Name computed before `program` is moved into `push_unique`.
        let name = name_for(&choice.program);
        push_unique(&mut options, choice.program, name, choice.extra_args);
    }

    // Then the known names, but only those actually installed. A name that is
    // not on PATH is skipped rather than listed as a broken row.
    for name in known {
        if let Some(path) = on_path(name) {
            push_unique(&mut options, path, (*name).to_string(), Vec::new());
        }
    }

    // Mark the reader's stored choice, comparing on the resolved executable so
    // "mpv" and "/usr/bin/mpv" are recognised as the same player.
    if let Some(default) = default_program {
        let resolved = on_path(default).unwrap_or_else(|| default.to_string());
        for option in &mut options {
            if option.program == resolved || option.program == default {
                option.is_default = true;
            }
        }
    }

    options
}

/// Add `program` if it is not already listed, keyed on the executable path.
///
/// A player can arrive from both GIO and the PATH scan; without this it would
/// appear twice with different names.
fn push_unique(
    options: &mut Vec<PlayerOption>,
    program: String,
    name: String,
    extra_args: Vec<String>,
) {
    if options.iter().any(|existing| existing.program == program) {
        return;
    }
    // The icon is resolved from the executable's own name; a player whose
    // theme has no matching icon simply gets `None` and a fallback glyph.
    let icon = icon_for_program(&program);
    options.push(PlayerOption {
        id: program.clone(),
        name,
        program,
        extra_args,
        is_default: false,
        icon,
    });
}

/// A readable name for a registered player's executable.
///
/// The desktop name is not available from the executable alone, so the file
/// stem is title-cased: `/usr/bin/vlc` -> "Vlc". Good enough for a row; the
/// known-player path supplies the nicer names for the players we care about.
fn name_for(program: &str) -> String {
    std::path::Path::new(program)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            let mut chars = stem.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => program.to_string(),
            }
        })
        .unwrap_or_else(|| program.to_string())
}

/// Find a program on `PATH`, returning its absolute path.
///
/// The same lookup a shell does, without spawning one. `None` means the
/// program is not installed, which is what keeps an absent known player out of
/// the list.
pub fn find_on_path(name: &str) -> Option<String> {
    // An absolute or relative path is used as-is: the reader may have typed a
    // full path in settings, and `PATH` lookup would miss it.
    if name.contains('/') {
        let path = std::path::Path::new(name);
        return if path.is_file() {
            Some(name.to_string())
        } else {
            None
        };
    }

    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
}

/// Whether an argument is a desktop-entry field code we must not pass through.
///
/// The freedesktop spec lets a `.desktop` file write placeholders such as `%U`
/// or `%f` where the files/URLs go. The dialog's command line usually ends in
/// one of these. We drop them because the URL is passed SEPARATELY (as the
/// final argument by the launcher); leaving a literal `%U` in would make the
/// player try to open a file named `%U`.
fn is_placeholder(arg: &str) -> bool {
    matches!(
        arg,
        "%u" | "%U" | "%f" | "%F" | "%i" | "%c" | "%k" | "%d" | "%D" | "%n" | "%N"
            | "%v" | "%m"
    )
}

/// Turn a desktop application's argv into a launchable choice.
///
/// The first element is the program; the rest are its arguments with any
/// placeholder removed. `None` for an empty argv: there is nothing to launch,
/// and returning an empty program would fail later with a worse message.
pub fn choice_from_argv(argv: &[String]) -> Option<PlayerChoice> {
    let mut iter = argv.iter();
    let program = iter.next()?.trim().to_string();
    if program.is_empty() {
        return None;
    }

    let extra_args = iter
        .filter(|arg| !is_placeholder(arg))
        .map(|arg| arg.to_string())
        .collect();

    Some(PlayerChoice {
        program,
        extra_args,
    })
}

/// Split a `.desktop` command line into argv, the way the shell would.
///
/// The command line arrives as one string. A path with spaces is quoted in it,
/// so a naive `split(' ')` would cut a program name in half. This honours
/// single quotes, double quotes and backslash escapes, which covers every real
/// `.desktop` line.
///
/// Hand-rolled rather than `glib::shell_parse_argv` on purpose: glib sits
/// behind the Linux-only `gtk` dependency, and this module must compile and
/// test on Windows too, where that crate is absent. An all-whitespace line
/// yields `None` rather than an empty argv.
pub fn argv_from_commandline(commandline: &str) -> Option<Vec<String>> {
    let argv = split_command_line(commandline);
    if argv.is_empty() {
        None
    } else {
        Some(argv)
    }
}

/// Tokenise a command line into arguments, honouring quotes and escapes.
///
/// Kept separate from [`argv_from_commandline`] so the tokeniser can be tested
/// directly. It is deliberately small: a `.desktop` line is a program, some
/// flags, and a `%U`-style placeholder, not a shell script.
fn split_command_line(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    // Tracks whether `current` holds an argument, so `""` yields an EMPTY
    // argument rather than being dropped as if it were absent.
    let mut has_token = false;
    let mut chars = line.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                has_token = true;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                has_token = true;
            }
            // A backslash escapes the next character, except inside single
            // quotes where it is literal.
            '\\' if !in_single_quote => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                    has_token = true;
                }
            }
            c if c.is_whitespace() && !in_single_quote && !in_double_quote => {
                if has_token {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            c => {
                current.push(c);
                has_token = true;
            }
        }
    }

    if has_token {
        args.push(current);
    }

    args
}

/// Resolve a picked application into a launchable choice.
///
/// Tries the full `.desktop` command line first, because it carries the
/// arguments the application expects (`--started-from-file`, a flatpak
/// wrapper, and so on). Falls back to the bare executable when the command
/// line is missing or unparseable -- the choice is still usable, just without
/// those extras, and a bare `mpv <url>` works for every player we care about.
///
/// Split out and pure so both branches are unit tested without GTK.
pub fn resolve_choice(
    commandline: Option<&str>,
    executable: Option<&str>,
) -> Option<PlayerChoice> {
    if let Some(line) = commandline {
        if let Some(choice) = argv_from_commandline(line).and_then(|argv| choice_from_argv(&argv)) {
            return Some(choice);
        }
    }

    // Fallback: the executable with no extra args. Better than opening
    // nothing, which is what treating a missing command line as a cancel
    // would do.
    executable
        .map(str::trim)
        .filter(|exe| !exe.is_empty())
        .map(|exe| PlayerChoice {
            program: exe.to_string(),
            extra_args: Vec::new(),
        })
}

/// Show the chooser and resolve the picked application, on the GTK main thread.
///
/// BLOCKING: `run()` spins a nested main loop until the reader picks an
/// application or cancels. It must therefore be called from the main thread
/// (the command layer hops there), never from a tokio worker.
///
/// `Ok(None)` is a cancel: the reader changed their mind, which the caller
/// treats as "open nothing". `Err` means a pick was made but could not be
/// resolved into anything launchable -- surfaced rather than swallowed, or the
/// pick would look like a cancel and nothing would open with no explanation.
#[cfg(target_os = "linux")]
pub fn choose_player_blocking(content_type: &str) -> Result<Option<PlayerChoice>, String> {
    use gtk::prelude::*;

    // No parent window: the dialog stands alone. Threading the app's
    // webview window in would couple this to the Tauri GTK escape hatch for
    // no benefit -- a modal parent only affects stacking.
    tracing::info!("chooser: building AppChooserDialog for {content_type}");
    let dialog = gtk::AppChooserDialog::for_content_type(
        gtk::Window::NONE,
        gtk::DialogFlags::MODAL,
        content_type,
    );

    tracing::info!("chooser: showing dialog (run)");
    let response = dialog.run();
    tracing::info!("chooser: dialog returned {response:?}");

    let choice = if response == gtk::ResponseType::Ok {
        let resolved = dialog.app_info().and_then(|info| {
            let commandline = info.commandline().map(|p| p.to_string_lossy().into_owned());
            let executable = info.executable().to_string_lossy().into_owned();
            tracing::info!(
                "chooser: picked name={:?} commandline={:?} executable={:?}",
                info.name(),
                commandline,
                executable
            );
            resolve_choice(commandline.as_deref(), Some(executable.as_str()))
        });
        tracing::info!("chooser: resolved choice = {resolved:?}");
        resolved
    } else {
        None
    };

    // `run()` leaves the dialog hidden but alive; closing it frees the widget
    // and its signal handlers. Without this, every launch would leak one
    // dialog.
    dialog.close();

    match (response == gtk::ResponseType::Ok, choice) {
        (true, Some(choice)) => Ok(Some(choice)),
        (true, None) => Err("the picked application could not be resolved".to_string()),
        (false, _) => Ok(None),
    }
}

/// The non-Linux stub.
///
/// The chooser is a GTK feature, so on Windows and macOS there is no dialog to
/// show. Reporting `Ok(None)` here would be read as "the reader cancelled" and
/// the launch would silently open nothing -- a regression from the old
/// auto-launch. `Err` with this marker instead lets the command layer fall back
/// to the stored player, which is exactly the pre-chooser behaviour.
#[cfg(not(target_os = "linux"))]
pub fn choose_player_blocking(_content_type: &str) -> Result<Option<PlayerChoice>, String> {
    Err(CHOOSER_UNSUPPORTED.to_string())
}

/// Applications the desktop registers for the content type, as choices.
///
/// GIO, not GTK: this only READS the app registry, so it does not need a
/// display and does not have to run on the main thread. Empty on a failure --
/// an unreadable registry is not worth failing the whole picker over, since
/// the known-player scan still supplies mpv and vlc.
#[cfg(target_os = "linux")]
pub fn registered_players(content_type: &str) -> Vec<PlayerChoice> {
    // `gio` is re-exported through the `gtk` crate, which is already a Linux
    // dependency of this crate; no extra dependency is introduced. The
    // extension trait supplies `executable`/`commandline`, which are not
    // inherent methods.
    use gtk::gio;

    let mut players = Vec::new();
    for app in gio::AppInfo::all_for_type(content_type) {
        if let Some(choice) = choice_for_app(&app) {
            players.push(choice);
        }
    }
    players
}

/// The non-Linux stub: no app registry is consulted, so nothing is listed.
#[cfg(not(target_os = "linux"))]
pub fn registered_players(_content_type: &str) -> Vec<PlayerChoice> {
    Vec::new()
}

/// Turn a GIO application into a launchable choice.
///
/// Prefers the full command line, which carries the arguments the app expects.
/// Falls back to the executable alone when that line is missing or blank --
/// [`resolve_choice`] owns that policy, so it is not repeated here.
#[cfg(target_os = "linux")]
fn choice_for_app(app: &gtk::gio::AppInfo) -> Option<PlayerChoice> {
    // `commandline`/`executable` come from the extension trait, not inherent
    // methods, so it must be in scope at the call site.
    use gtk::gio::prelude::AppInfoExt;

    let commandline = app.commandline().map(|p| p.to_string_lossy().into_owned());
    let executable = app.executable().to_string_lossy().into_owned();
    resolve_choice(commandline.as_deref(), Some(executable.as_str()))
}

/// Every player the in-app picker should offer, in display order.
///
/// The merged list: the desktop's registered apps, plus known players that are
/// installed. `default` is the reader's stored preference, marked in the list
/// so the picker can show which one is already chosen.
pub fn list_players(
    content_type: &str,
    default: Option<&str>,
) -> Vec<PlayerOption> {
    merge_players(
        registered_players(content_type),
        KNOWN_PLAYERS,
        &find_on_path,
        default,
    )
}

/// Marker returned when the platform has no app chooser.
///
/// The command layer compares against this to decide between "the reader
/// cancelled" (open nothing) and "there is no chooser here" (use the stored
/// player).
pub const CHOOSER_UNSUPPORTED: &str = "app chooser is not supported on this platform";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_maps_first_token_to_program() {
        let argv = vec!["mpv".to_string(), "--fullscreen".to_string()];
        let choice = choice_from_argv(&argv).expect("a choice");
        assert_eq!(choice.program, "mpv");
        assert_eq!(choice.extra_args, vec!["--fullscreen".to_string()]);
    }

    #[test]
    fn argv_drops_the_url_placeholder() {
        // A typical vlc .desktop line.
        let argv = vec![
            "/usr/bin/vlc".to_string(),
            "--started-from-file".to_string(),
            "%U".to_string(),
        ];
        let choice = choice_from_argv(&argv).expect("a choice");
        assert_eq!(choice.program, "/usr/bin/vlc");
        assert_eq!(
            choice.extra_args,
            vec!["--started-from-file".to_string()]
        );
    }

    #[test]
    fn argv_drops_any_placeholder_spelling() {
        for placeholder in ["%u", "%f", "%F", "%i", "%c", "%k"] {
            let argv = vec!["mpv".to_string(), placeholder.to_string()];
            let choice = choice_from_argv(&argv).expect("a choice");
            assert!(
                choice.extra_args.is_empty(),
                "{placeholder} should be dropped"
            );
        }
    }

    #[test]
    fn argv_keeps_ordinary_arguments_that_merely_contain_a_percent() {
        // Only a bare placeholder is dropped; a real option is kept.
        let argv = vec!["mpv".to_string(), "--title=50%".to_string()];
        let choice = choice_from_argv(&argv).expect("a choice");
        assert_eq!(choice.extra_args, vec!["--title=50%".to_string()]);
    }

    #[test]
    fn empty_argv_has_no_choice() {
        assert_eq!(choice_from_argv(&[]), None);
    }

    #[test]
    fn merge_keeps_registered_players_first() {
        let registered = vec![PlayerChoice {
            program: "/usr/bin/vlc".to_string(),
            extra_args: vec!["--started-from-file".to_string()],
        }];
        let merged = merge_players(registered, &[], &|_| None, None);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].program, "/usr/bin/vlc");
        assert_eq!(merged[0].name, "Vlc");
        assert_eq!(merged[0].extra_args, vec!["--started-from-file".to_string()]);
    }

    #[test]
    fn merge_adds_known_players_that_are_installed() {
        let on_path = |name: &str| match name {
            "mpv" => Some("/usr/bin/mpv".to_string()),
            _ => None,
        };
        let merged = merge_players(vec![], &["mpv", "vlc"], &on_path, None);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].program, "/usr/bin/mpv");
        assert_eq!(merged[0].name, "mpv");
    }

    #[test]
    fn merge_does_not_duplicate_a_player_found_both_ways() {
        // VLC registered by GIO AND installed on PATH must appear once.
        let registered = vec![PlayerChoice {
            program: "/usr/bin/vlc".to_string(),
            extra_args: vec![],
        }];
        let on_path = |name: &str| {
            (name == "vlc").then(|| "/usr/bin/vlc".to_string())
        };
        let merged = merge_players(registered, &["vlc"], &on_path, None);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn merge_marks_the_stored_preference() {
        let on_path = |name: &str| {
            (name == "mpv").then(|| "/usr/bin/mpv".to_string())
        };
        let merged = merge_players(vec![], &["mpv"], &on_path, Some("mpv"));
        assert!(merged[0].is_default);
    }

    #[test]
    fn merge_marks_the_default_by_resolved_path_too() {
        // The stored preference is a bare name; the list holds an absolute
        // path. They must still be recognised as the same player.
        let registered = vec![PlayerChoice {
            program: "/nix/store/abc/bin/mpv".to_string(),
            extra_args: vec![],
        }];
        let on_path = |name: &str| {
            (name == "mpv").then(|| "/nix/store/abc/bin/mpv".to_string())
        };
        let merged = merge_players(registered, &["mpv"], &on_path, Some("mpv"));
        assert_eq!(merged.len(), 1);
        assert!(merged[0].is_default);
    }

    #[test]
    fn find_on_path_uses_an_explicit_path_as_is() {
        // A reader-typed absolute path is not looked up on PATH.
        let existing = std::env::current_exe().expect("current exe");
        let found = find_on_path(&existing.to_string_lossy());
        assert_eq!(found.as_deref(), Some(existing.to_string_lossy().as_ref()));
    }

    #[test]
    fn find_on_path_misses_a_nonexistent_absolute_path() {
        assert_eq!(find_on_path("/definitely/not/here/nope"), None);
    }

    #[test]
    fn resolve_prefers_the_full_commandline() {
        let choice = resolve_choice(
            Some("/usr/bin/vlc --started-from-file %U"),
            Some("/usr/bin/vlc"),
        )
        .expect("a choice");
        assert_eq!(choice.program, "/usr/bin/vlc");
        assert_eq!(
            choice.extra_args,
            vec!["--started-from-file".to_string()]
        );
    }

    #[test]
    fn resolve_falls_back_to_the_executable_without_a_commandline() {
        // Some .desktop entries expose no command line; the app is still
        // launchable, and opening it bare beats opening nothing.
        let choice = resolve_choice(None, Some("/usr/bin/mpv")).expect("a choice");
        assert_eq!(choice.program, "/usr/bin/mpv");
        assert!(choice.extra_args.is_empty());
    }

    #[test]
    fn resolve_falls_back_when_the_commandline_is_blank() {
        let choice =
            resolve_choice(Some("   "), Some("/usr/bin/mpv")).expect("a choice");
        assert_eq!(choice.program, "/usr/bin/mpv");
    }

    #[test]
    fn resolve_has_no_choice_without_either_source() {
        assert_eq!(resolve_choice(None, None), None);
        assert_eq!(resolve_choice(Some("  "), Some("  ")), None);
    }

    #[test]
    fn blank_program_has_no_choice() {
        let argv = vec!["   ".to_string()];
        assert_eq!(choice_from_argv(&argv), None);
    }

    #[test]
    fn commandline_splits_a_quoted_path_as_one_token() {
        let argv = argv_from_commandline("\"/opt/My Player/mpv\" --fullscreen")
            .expect("a parse");
        assert_eq!(
            argv,
            vec!["/opt/My Player/mpv".to_string(), "--fullscreen".to_string()]
        );
    }

    #[test]
    fn commandline_of_only_whitespace_has_no_argv() {
        assert_eq!(argv_from_commandline("   "), None);
    }
}