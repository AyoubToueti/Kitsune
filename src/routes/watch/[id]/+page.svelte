<script lang="ts">
  import { page } from "$app/state";
  import { open } from "@tauri-apps/plugin-dialog";

  import { errorMessage, getAnime } from "$lib/api/anime";
  import { addMagnet, addTorrent, getStreamUrl } from "$lib/api/player";
  import { searchReleases } from "$lib/api/releases";
  import {
    displayTitle,
    type Anime,
    type Release,
    type TorrentFile,
  } from "$lib/types";
  import EpisodeList from "$lib/components/EpisodeList.svelte";
  import ExternalPlayerButton from "$lib/components/ExternalPlayerButton.svelte";
  import RelatedAnimeList from "$lib/components/RelatedAnimeList.svelte";
  import VideoPlayer from "$lib/components/VideoPlayer.svelte";

  const id = $derived(Number(page.params.id));

  let anime = $state<Anime | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // --- resolving a stream --------------------------------------------------
  //
  // The page can resolve a stream two ways. It asks the indexers for releases
  // and moves the chosen magnet itself -- the Sonarr-style path, which is the
  // default -- or the reader supplies a `.torrent` by hand when the search
  // missed. Either way the result is the same shape: a torrent handle whose
  // files are matched against the selected episode.
  //
  // Nothing here is cached: a torrent handle dies with its session.

  let torrentId = $state<number | null>(null);
  let files = $state<TorrentFile[]>([]);
  let chosen = $state<TorrentFile | null>(null);
  let streamUrl = $state<string | undefined>(undefined);

  // --- the indexer search --------------------------------------------------

  let releases = $state<Release[]>([]);
  let chosenRelease = $state<Release | null>(null);
  let searching = $state(false);
  let loadingRelease = $state(false);
  let releaseError = $state<string | null>(null);

  let loadingTorrent = $state(false);
  let torrentError = $state<string | null>(null);

  /**
   * The episode the detail page linked to, from `?ep=`.
   *
   * Initialised from the URL so arriving from the episode grid lands on the
   * right entry rather than the top of the list. A nonsense value is treated
   * as "no selection" instead of being clamped to an arbitrary episode.
   */
  let selectedEpisode = $state<number | undefined>(undefined);

  const requestedEpisode = $derived.by(() => {
    const raw = page.url.searchParams.get("ep");
    if (raw === null) return undefined;
    const value = Number(raw);
    return Number.isInteger(value) && value >= 0 ? value : undefined;
  });

  // Applied once the episode list has loaded, so an out-of-range value can be
  // ignored rather than shown as a phantom selection.
  $effect(() => {
    const wanted = requestedEpisode;
    const count = episodes.length;
    if (wanted === undefined || count === 0) return;
    if (wanted < count) selectedEpisode = wanted;
  });

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
   * The episode number to search for and to match files against.
   *
   * Without a selection the search is the work's title alone, which is what
   * finds a film or a batch. With one, the number narrows it to that episode.
   */
  const wantedEpisode = $derived.by(() => {
    if (selectedEpisode === undefined) return undefined;
    return episodeNumber(selectedEpisode);
  });

  /**
   * Search the indexers whenever the work or the wanted episode changes.
   *
   * The title and the episode number are the whole key: re-running the search
   * on a stream URL or a file list would fire it for reasons the user never
   * asked about. A search in flight is abandoned when the key changes, so a
   * slow response for episode 1 cannot overwrite the results for episode 2.
   */
  $effect(() => {
    const work = title;
    const episode = wantedEpisode;

    if (work === null) return;

    let cancelled = false;
    searching = true;
    releaseError = null;

    searchReleases(work, episode)
      .then((found) => {
        if (cancelled) return;
        releases = found;
      })
      .catch((err) => {
        if (cancelled) return;
        releases = [];
        releaseError = errorMessage(err);
      })
      .finally(() => {
        if (cancelled) return;
        searching = false;
      });

    return () => {
      cancelled = true;
    };
  });

  /**
   * The file that best matches an episode number, or `null` when nothing does.
   *
   * A release name carries the episode as ` - 03 ` or `E03`, so the number is
   * searched for as a delimited token rather than a bare substring: "03" must
   * not match "1080p". A miss returns nothing and the reader picks by hand,
   * which is the honest outcome when the naming does not line up.
   *
   * Takes the candidate list rather than reading `files`, so the same rule can
   * be applied to a freshly-added handle before it is committed to state.
   */
  function fileForEpisode(
    candidates: TorrentFile[],
    number: number,
  ): TorrentFile | null {
    const token = String(number).padStart(2, "0");
    const pattern = new RegExp(`(?:^|[^0-9])0*${number}(?:[^0-9]|$)`);

    return (
      candidates.find((file) => pattern.test(file.name)) ??
      // Falls back to the zero-padded spelling before giving up, since some
      // releases only ever write "03" and a bare "3" would not match.
      candidates.find((file) => file.name.includes(token)) ??
      null
    );
  }

  /** Whether a file looks like something the video element can open. */
  function isPlayable(file: TorrentFile): boolean {
    return /\.(mkv|mp4|avi|webm|mov|m4v)$/i.test(file.name);
  }

  /**
   * The file to start with when nothing matched the episode.
   *
   * The largest playable file, since a full-length episode dwarfs the sample
   * clips and cover art some releases ship beside it. Falls back to the largest
   * file of any kind, so a torrent whose naming is opaque still plays something
   * rather than nothing.
   */
  function bestEffortFile(candidates: TorrentFile[]): TorrentFile | null {
    const playable = candidates.filter(isPlayable);
    const pool = playable.length > 0 ? playable : candidates;
    return [...pool].sort((a, b) => b.lengthBytes - a.lengthBytes)[0] ?? null;
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

  /**
   * Move a found release: add its magnet, then play the right file inside it.
   *
   * The episode match is tried first and the best-effort file second, so a
   * release whose files do not spell the episode still starts on something
   * watchable. The whole handle is kept, so the file list below stays usable
   * for switching by hand.
   */
  async function playRelease(release: Release): Promise<void> {
    if (loadingRelease) return;

    chosenRelease = release;
    releaseError = null;
    loadingRelease = true;
    try {
      const handle = await addMagnet(release.magnetUri);
      torrentId = handle.id;
      files = handle.files;

      const wanted = wantedEpisode;
      const match =
        wanted !== undefined ? fileForEpisode(handle.files, wanted) : null;
      const target = match ?? bestEffortFile(handle.files);
      if (target) await play(target);
    } catch (err) {
      releaseError = errorMessage(err);
    } finally {
      loadingRelease = false;
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
      const wanted = wantedEpisode;
      const match =
        wanted !== undefined ? fileForEpisode(handle.files, wanted) : null;

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
    const match = number !== undefined ? fileForEpisode(files, number) : null;
    if (match) void play(match);
  }

  /** A human size for a release row, e.g. "1.4 GB". */
  function formatSize(bytes?: number): string | undefined {
    if (bytes === undefined || bytes <= 0) return undefined;
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${value.toFixed(value < 10 && unit > 0 ? 1 : 0)} ${units[unit]}`;
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

      <VideoPlayer
        src={streamUrl}
        title={title ?? "Video player"}
        poster={anime.bannerImage}
      />

      <div class="mt-3 flex flex-wrap items-center gap-2">
        <button
          type="button"
          onclick={loadTorrent}
          disabled={loadingTorrent}
          class="rounded-lg border border-border-subtle px-4 py-2 text-sm font-medium text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
        >
          {loadingTorrent ? "Loading…" : files.length ? "Change torrent" : "Load torrent"}
        </button>

        <ExternalPlayerButton url={streamUrl} />
      </div>

      {#if torrentError}
        <p class="mt-2 text-sm text-ink-faint">{torrentError}</p>
      {/if}

      <!-- The releases the app found on its own. The reader no longer has to
           supply a torrent for the common case: picking a row resolves its
           magnet and plays the file matching the selected episode. -->
      <div class="mt-6">
        <h2 class="mb-2 text-sm font-semibold tracking-tight">Releases</h2>

        {#if searching}
          <p class="text-sm text-ink-muted" data-testid="releases-loading">Searching…</p>
        {:else if releaseError}
          <p class="text-sm text-ink-faint">{releaseError}</p>
        {:else if releases.length === 0}
          <p class="text-sm text-ink-muted" data-testid="releases-empty">
            No releases found. Load a torrent by hand instead.
          </p>
        {:else}
          <ul class="flex flex-col gap-1" data-testid="releases">
            {#each releases as release (release.infoHash ?? release.title)}
              <li>
                <button
                  type="button"
                  onclick={() => playRelease(release)}
                  disabled={loadingRelease}
                  class="w-full rounded-lg border px-3 py-2 text-left text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-60 {chosenRelease?.title ===
                  release.title
                    ? 'border-accent bg-surface-hover text-ink'
                    : 'border-border-subtle text-ink-muted hover:border-accent hover:text-ink'}"
                >
                  <span class="block truncate">{release.title}</span>
                  <span class="mt-0.5 block text-ink-faint">
                    {#if release.resolution !== "unknown"}{release.resolution}{/if}
                    {#if release.source !== "unknown"}· {release.source}{/if}
                    {#if release.seeders !== undefined}· {release.seeders} seeders{/if}
                    {#if formatSize(release.sizeBytes)}· {formatSize(release.sizeBytes)}{/if}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

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