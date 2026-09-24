pub mod auth;
pub mod commands;
pub mod indexer;
pub mod player;
pub mod providers;
pub mod torrent;
pub mod types;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // One provider for the process: each `reqwest::Client` keeps a
    // connection pool, so building one per call would waste connections
    // and invite AniList's rate limiter.
    //
    // Built concrete first, then coerced, so the auth commands can hold
    // `Arc<AniListProvider>` while the metadata commands hold
    // `Arc<dyn AnimeProvider>`. Both point at the SAME instance -- a second
    // one would have its own token state and its own connection pool.
    let anilist = std::sync::Arc::new(providers::AniListProvider::new());
    let provider: commands::SharedProvider = anilist.clone();

        // Jikan is managed separately: it is not an `AnimeProvider`, it only fills
        // in episode lists for works AniList already described.
        let episodes: commands::SharedEpisodeProvider =
            std::sync::Arc::new(providers::JikanProvider::new());

        let mut builder = tauri::Builder::default();

        // MUST be first, before any other plugin: a deep link on Linux and
        // Windows arrives as a command-line argument to a NEW process, and
        // this is what forwards it to the running one instead of starting a
        // second app that never sees the token. Desktop only -- it has no
        // mobile equivalent.
        #[cfg(desktop)]
        {
            builder = builder.plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
                // The deep-link plugin handles the forwarded argument itself;
                // nothing to do here beyond having claimed the instance.
            }));
        }

        builder
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_deep_link::init())
            .manage(provider)
            .manage(anilist)
            .manage(episodes)
            .manage(player::PlayerState::new())
            // One registry for the process: each indexer holds its own pooled
            // `reqwest::Client`, so rebuilding one per search would waste
            // connections and re-resolve DNS.
            .manage(indexer::IndexerRegistry::new())
            // The AniList token, read from disk at startup so a signed-in
            // reader stays signed in across restarts.
            .manage(auth::TokenStore::new())
            .manage(auth::LastPlayedStore::new())
            .setup(|app| {
                use tauri::{Emitter, Manager};
                // Imported once at the top of the closure: the trait provides
                // `deep_link`, which both the runtime registration and the URL
                // listener below need. Scoping it to the `#[cfg]` block left
                // the listener without it.
                use tauri_plugin_deep_link::DeepLinkExt;

                // Put any stored token back into the provider. Without this a
                // signed-in reader would look signed out on every launch: the
                // credential is on disk but nothing has told the provider.
                {
                    let store = app.state::<auth::TokenStore>();
                    let anilist = app.state::<auth::SharedAniList>();
                    if auth::restore_token(&store, &anilist) {
                        tracing::info!("restored the stored AniList token");
                    }
                }

                // In development the scheme is not registered with the OS, so
                // `kitsune://` would open nothing. Registering at runtime means
                // the flow can be tested without installing the app. Not
                // possible on macOS, where it only works once bundled.
                #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
                {
                    if let Err(err) = app.deep_link().register_all() {
                        tracing::warn!("could not register the kitsune:// scheme: {err:#}");
                    }
                }

                // A redirect that arrives while the app is running.
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    let store = handle.state::<auth::TokenStore>();
                    let anilist = handle.state::<auth::SharedAniList>();

                    for url in event.urls() {
                        // The raw string, not the parsed `Url`: `parse_token`
                        // owns the fragment parsing and is tested against the
                        // exact shape the OS delivers.
                        if auth::apply_token(&store, &anilist, url.as_str()) {
                            let _ = handle.emit(auth::AUTH_CHANGED_EVENT, true);
                        }
                    }
                });

                // A redirect that LAUNCHED the app. The listener above only
                // fires for a URL arriving at an already-running process, so
                // without this a sign-in started while Kitsune was closed would
                // appear to do nothing.
                {
                    if let Ok(Some(urls)) = app.deep_link().get_current() {
                        let store = app.state::<auth::TokenStore>();
                        let anilist = app.state::<auth::SharedAniList>();

                        for url in urls {
                            if auth::apply_token(&store, &anilist, url.as_str()) {
                                let _ = app.emit(auth::AUTH_CHANGED_EVENT, true);
                            }
                        }
                    }
                }

                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                // Metadata.
                greet,
                commands::get_trending,
                commands::get_list,
                commands::get_browse,
                commands::get_genres,
                commands::get_tags,
                commands::get_schedule,
                commands::get_anime,
                commands::get_episodes,
                // Finding releases.
                indexer::commands::search_releases,
                indexer::commands::download_torrent,
                indexer::commands::probe_releases,
                // Playing one: the torrent session and the external player.
                player::commands::add_torrent,
                player::commands::add_magnet,
                player::commands::remove_torrent,
                player::commands::get_stream_url,
                player::commands::open_in_player,
                player::commands::get_player,
                player::commands::set_player,
                player::commands::suggested_players,
                // Signing in to AniList.
                auth::commands::begin_login,
                auth::commands::auth_status,
                auth::commands::logout,
                // The reader's own list.
                auth::list::get_list_entry,
                auth::list::set_list_entry,
                auth::list::continue_watching,
                auth::list::user_list,
                auth::list::delete_list_entry,
                // Where the reader last was, kept locally.
                auth::list::record_last_played,
                auth::list::last_played,
            ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_includes_the_name() {
        let message = greet("World");
        assert!(
            message.contains("World"),
            "greeting should include the name, got: {message}"
        );
    }

    #[test]
    fn greet_uses_expected_prefix() {
        assert!(greet("Ayoub").starts_with("Hello, Ayoub!"));
    }

    /// Proves the `tempfile` dev-dependency is wired up and usable.
    #[test]
    fn tempfile_harness_works() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let file = dir.path().join("probe.txt");
        std::fs::write(&file, b"ok").expect("failed to write probe file");
        assert_eq!(
            std::fs::read(&file).expect("failed to read probe file"),
            b"ok"
        );
    }

    /// Proves the async test runtime works via `tokio-test`.
    #[test]
    fn tokio_test_harness_works() {
        let value = tokio_test::block_on(async { 1 + 1 });
        assert_eq!(value, 2);
    }

    /// Proves the `wiremock` dev-dependency compiles and can start a server.
    /// Real request/response mocking is exercised in the provider tests.
    #[tokio::test]
    async fn wiremock_harness_starts_a_server() {
        let server = wiremock::MockServer::start().await;
        assert!(
            server.uri().starts_with("http://"),
            "unexpected mock server uri: {}",
            server.uri()
        );
    }
}
