<script lang="ts">
  import { page } from "$app/state";
  import { open } from "@tauri-apps/plugin-dialog";

  import { errorMessage, getAnime } from "$lib/api/anime";
  import { addTorrent, getStreamUrl } from "$lib/api/player";
  import { displayTitle, type Anime, type TorrentFile } from "$lib/types";
  import EpisodeList from "$lib/components/EpisodeList.svelte";
  import ExternalPlayerButton from "$lib/components/ExternalPlayerButton.svelte";
  import RelatedAnimeList from "$lib/components/RelatedAnimeList.svelte";
  import VideoPlayer from "$lib/components/VideoPlayer.svelte";

  const id = $derived(Number(page.params.id));

  let anime = $state<Anime | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // --- the reader's own copy -----------------------------------------------
  //
  // The page never resolves a stream on its own: the reader supplies the
  // torrent and picks the file. That keeps the app a player rather than a
  // source, which is also why nothing here is cached -- a torrent handle dies
  // with its session.

  let torrentId = $state<number | null>(null);
  let files = $state<TorrentFile[]>([]);
  let chosen = $state<TorrentFile | null>(null);
  let streamUrl = $state<string | undefined>(undefined);
  let selectedEpisode = $state<number | undefined>(undefined);
  let loadingTorrent = $state(false);
  let torrentError = $state<string | null>(null);

  $effect(() => {
    const currentId = id;

    let cancelled = false;
    loading = true;
    error = null;
    anime = null;

    getAnime(currentId)
      .then((result) => {
        if (cancelled) return;
        anime = result;
      })
      .catch((err) => {
        if (cancelled) return;
        error = errorMessage(err);
      })
      .finally(() => {
        if (cancelled) return;
        loading = false;
      });

    return () => {
      cancelled = true;
    };
  });

  const title = $derived(anime ? (displayTitle(anime.title) ?? "Untitled") : null);
  const episodes = $derived(anime?.streamingEpisodes ?? []);

  /**
   * The episode number an entry represents, read out of its title or URL.
   *
   * Mirrors the same heuristic EpisodeList uses to jump to an episode, so the
   * preselect and the jump box agree about which entry is "episode 3".
   */
  function episodeNumber(index: number): number | undefined {
    const ep = episodes[index];
    if (!ep) return undefined;

    const patterns: RegExp[] = [
      /(?:episode|ep)\.?\s*[-–:]?\s*(\d+)/i,
      /(?:episode|ep)[-_](\d+)/i,
      /(\d+)\s*$/,
    ];

    for (const source of [ep.title, ep.url]) {
      if (source == null) continue;
      for (const pattern of patterns) {
        const match = source.match(pattern);
        if (match) return Number(match[1]);
      }
    }
    return undefined;
  }

  /**
   * The file that best matches an episode number, or `null` when nothing does.
   *
   * A release name carries the episode as ` - 03 ` or `E03`, so the number is
   * searched for as a delimited token rather than a bare substring: "03" must
   * not match "1080p". A miss returns nothing and the reader picks by hand,
   * which is the honest outcome when the naming does not line up.
   */
  function fileForEpisode(number: number): TorrentFile | null {
    const token = String(number).padStart(2, "0");
    const pattern = new RegExp(`(?:^|[^0-9])0*${number}(?:[^0-9]|$)`);

    return (
      files.find((file) => pattern.test(file.name)) ??
      // Falls back to the zero-padded spelling before giving up, since some
      // releases only ever write "03" and a bare "3" would not match.
      files.find((file) => file.name.includes(token)) ??
      null
    );
  }

  /** Pick the file and resolve its stream URL. */
  async function play(file: TorrentFile): Promise<void> {
    if (torrentId === null) return;

    chosen = file;
    torrentError = null;
    try {
      streamUrl = await getStreamUrl(torrentId, file.idx);
    } catch (err) {
      streamUrl = undefined;
      torrentError = errorMessage(err);
    }
  }

  /** Choose a `.torrent` and preselect the matching file, if any. */
  async function loadTorrent(): Promise<void> {
    if (loadingTorrent) return;

    const picked = await open({
      multiple: false,
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
    });

    // A cancelled picker resolves to null, which is not an error.
    if (typeof picked !== "string") return;

    loadingTorrent = true;
    torrentError = null;
    try {
      const handle = await addTorrent(picked);
      torrentId = handle.id;
      files = handle.files;

      // Preselect from whatever episode the list is showing as current, so
      // loading a torrent for episode 3 does not start at file 1.
      const wanted =
        selectedEpisode !== undefined ? episodeNumber(selectedEpisode) : undefined;
      const match = wanted !== undefined ? fileForEpisode(wanted) : null;

      if (match) {
        await play(match);
      }
    } catch (err) {
      torrentError = errorMessage(err);
    } finally {
      loadingTorrent = false;
    }
  }

  /** Select an episode, switching the playing file when one is loaded. */
  function selectEpisode(index: number): void {
    selectedEpisode = index;

    if (files.length === 0) return;

    const number = episodeNumber(index);
    const match = number !== undefined ? fileForEpisode(number) : null;
    if (match) void play(match);
  }
</script>

{#if loading}
  <p class="py-16 text-center text-ink-muted">Loading…</p>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load details.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if anime === null}
  <div class="py-16 text-center">
    <p class="text-ink">Anime not found.</p>
  </div>
{:else}
  <div class="grid gap-6 lg:grid-cols-[1fr_18rem]">
    <!-- Player column -->
    <div class="min-w-0">
      <h1 class="mb-3 text-lg font-semibold tracking-tight">
        {title}
        {#if selectedEpisode !== undefined}
          <span class="text-ink-muted">· Episode {selectedEpisode + 1}</span>
        {/if}
      </h1>

<VideoPlayer src={streamUrl} title={title ?? "Video player"} />

      <div class="mt-3 flex flex-wrap items-center gap-2">
        <button
          type="button"
          onclick={loadTorrent}
          disabled={loadingTorrent}
          class="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
        >
          {loadingTorrent ? "Loading…" : files.length ? "Change torrent" : "Load torrent"}
        </button>

        <ExternalPlayerButton url={streamUrl} />
      </div>

      {#if torrentError}
        <p class="mt-2 text-sm text-ink-faint">{torrentError}</p>
      {/if}

      {#if files.length > 0}
        <!-- The files the torrent actually holds, since the reader owns the
             copy and its naming may not match AniList's episode list. -->
        <div class="mt-4">
          <h2 class="mb-2 text-sm font-semibold tracking-tight">Files</h2>
          <ul class="flex flex-col gap-1" data-testid="torrent-files">
            {#each files as file (file.idx)}
              <li>
                <button
                  type="button"
                  onclick={() => play(file)}
                  class="w-full truncate rounded-lg border px-3 py-2 text-left text-xs transition-colors {chosen?.idx ===
                  file.idx
                    ? 'border-accent bg-surface-hover text-ink'
                    : 'border-border-subtle text-ink-muted hover:border-accent hover:text-ink'}"
                >
                  {file.name}
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if episodes.length > 0}
        <div class="mt-6">
          <EpisodeList {episodes} selected={selectedEpisode} onSelect={selectEpisode} />
        </div>
      {/if}
    </div>

    <!-- Info sidebar -->
    <aside class="min-w-0">
      {#if anime.coverImage}
        <img
          src={anime.coverImage}
          alt=""
          class="mb-3 w-full rounded-lg object-cover"
        />
      {/if}

      <!-- Not a heading: the page's <h1> above the player already names the
           work, and a second heading with the same text is noise for anyone
           navigating by headings. -->
      <p class="text-base font-semibold tracking-tight">{title}</p>

      {#if anime.description}
        <p class="mt-2 line-clamp-6 text-xs leading-relaxed text-ink-muted">
          {anime.description.replace(/<[^>]*>/g, "")}
        </p>
      {/if}

      <a
        href={`/anime/${anime.id}`}
        class="mt-3 inline-block rounded-lg border border-border-subtle px-3 py-1.5 text-xs text-ink-muted transition-colors hover:border-accent hover:text-accent"
      >
        View detail
      </a>

      <div class="mt-6">
        <RelatedAnimeList relations={anime.relations} />
      </div>
    </aside>
  </div>
{/if}