 

We are building **Project "Kitsune"** (placeholder name). 

---

### 📂 The Project Structure
When you initialize the project, your workspace should look like this. This separates your UI from your heavy Rust logic.

```text
kitsune/
├── src-tauri/          # Rust backend (Tauri core, Torrent engine, HTTP server)
│   ├── src/
│   │   ├── main.rs     # Tauri entry point
│   │   ├── commands.rs # IPC commands exposed to frontend
│   │   ├── torrent/    # Torrent engine & local HTTP bridge
│   │   ├── providers/  # AniList, Nyaa, TMDB (The Provider Pattern)
│   │   └── player.rs   # mpv process management
├── src/                # Svelte frontend (UI, routing, state)
│   ├── lib/
│   │   ├── components/ # UI components (AnimeCard, EpisodeList)
│   │   ├── stores/     # Svelte stores for state management
│   │   └── api/        # Tauri invoke wrappers
├── package.json
└── tauri.conf.json
```

---

### 🗺️ The Master Plan (5 Phases)

#### Phase 1: Environment & Initialization (Day 1 - Morning)
**Goal:** Get the empty Tauri + Svelte app running.

1. **Install Prerequisites:**
   * Rust & Cargo
   * Node.js (v20+) & pnpm
   * System dependencies for Tauri (WebKit2GTK on Linux, Xcode on Mac, WebView2 on Windows).
   * **mpv** (Install it on your OS so you can test it via CLI later).
2. **Initialize Project:**
   ```bash
   pnpm create tauri-app@latest
   # Select: Svelte, TypeScript, Tailwind CSS
   cd kitsune
   pnpm install
   ```
3. **Verify:** Run `pnpm tauri dev`. You should see the default Svelte/Tauri window.

#### Phase 2: The Rust Engine - Torrent to HTTP (Day 1 - Afternoon)
**Goal:** Prove we can stream a magnet link locally. This is the hardest part.

1. **Add Rust Dependencies** in `src-tauri/Cargo.toml`:
   * `axum` (for the local HTTP server)
   * `tokio` (async runtime)
   * `librqbit` OR `cratetorrent` (for the torrent engine. *Pro-tip: `librqbit` is modern and fast. If it's too complex, use a Node.js sidecar with `webtorrent` for the MVP*).
2. **Build the HTTP Bridge:**
   * Write a Rust function that takes a `magnet_link`.
   * Start the torrent engine in sequential mode.
   * Spin up an `axum` server on `127.0.0.1:8765`.
   * Create a route `GET /stream` that reads the downloaded chunks from the torrent engine and streams them back as an HTTP response.
3. **Test:** Open VLC or mpv on your computer and play `http://127.0.0.1:8765/stream`. If it plays, you've won.

#### Phase 3: The Provider Pattern & IPC (Day 2)
**Goal:** Fetch real anime data and pass it to the UI.

1. **Implement Providers (Rust):**
   * Create `src-tauri/src/providers/anilist.rs`. Use the `reqwest` crate to hit the AniList GraphQL API.
   * Create `src-tauri/src/providers/nyaa.rs`. Scrape or use an API wrapper for Nyaa.si to search for the magnet links.
2. **Expose Tauri Commands:**
   * In `commands.rs`, write functions like `#[tauri::command] async fn get_trending_anime() -> Vec<Anime>`.
   * Write `#[tauri::command] async fn play_episode(anilist_id: i32, episode: i32)`. This command will trigger your Torrent-to-HTTP engine from Phase 2.
3. **Frontend Integration (Svelte):**
   * Use `invoke('get_trending_anime')` to fetch data.
   * Build a simple grid UI using Tailwind to display the covers.

#### Phase 4: The Player & UI Overlay (Day 3)
**Goal:** Watch the anime inside the app.

1. **Spawn mpv:**
   * In `player.rs`, use Rust's `std::process::Command` to spawn `mpv`.
   * Pass the local HTTP URL (`http://127.0.0.1:8765/stream`) to mpv.
   * *MVP approach:* Let mpv open in its own window. 
   * *Advanced approach:* Use Tauri's raw window handle APIs to embed the mpv render context directly inside your Svelte webview.
2. **Build the UI:**
   * Create the "Details" page (Synopsis, Episode list).
   * Implement **Virtual Scrolling** for the episode list (crucial for low RAM). Use `svelte-virtual` or write a simple windowing function.

#### Phase 5: Scaling & Polish (Week 2+)
**Goal:** Make it production-ready and scalable to movies.

1. **Database:** Add `rusqlite` or `sqlx` (SQLite) to cache AniList metadata so you don't hit API rate limits.
2. **Movie Providers:** Implement `TMDBProvider` and `YTSProvider` using the exact same Trait interfaces you built in Phase 3.
3. **Auto-Updater:** Configure Tauri's built-in updater (`@tauri-apps/plugin-updater`) so users can get new versions automatically.
4. **Release:** Build the `.msi` (Windows), `.dmg` (Mac), and `.AppImage` (Linux) using `pnpm tauri build`.

---

### 🚀 Your "Start Right Now" Checklist for VS Code

Open your terminal in VS Code and execute this exact sequence:

```bash
# 1. Create the app
pnpm create tauri-app@latest kitsune
cd kitsune
pnpm install

# 2. Add backend dependencies
cd src-tauri
cargo add axum tokio reqwest serde serde_json
# (Add your chosen torrent crate here, e.g., cargo add librqbit)
cd ..

# 3. Add frontend dependencies for routing and virtualization
pnpm add svelte-routing svelte-virtual

# 4. Start the dev server
pnpm tauri dev
```

### 💡 Golden Rules for this Specific Build

1. **Never block the UI thread:** All Rust Tauri commands that do network or disk I/O *must* be `async`. If you block the main thread, the Svelte UI will freeze.
2. **Image sizing:** AniList returns massive images. In your Rust backend, before sending the JSON to Svelte, append `?size=large` or use an image proxy to downscale them to 300px width. This will cut your frontend RAM usage in half.
3. **Handle Seeks Gracefully:** When `mpv` requests a chunk of the video via the local HTTP server, and the torrent hasn't downloaded that chunk yet, your Rust HTTP handler must **pause the response** (don't close the connection), wait for the torrent engine to download that specific piece, and then resume sending bytes. 

