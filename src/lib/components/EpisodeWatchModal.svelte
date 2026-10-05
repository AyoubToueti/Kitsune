<script lang="ts">
  import { untrack } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { downloadTorrent } from "$lib/api/releases";
  import {
    absoluteOffset,
    episodeNumber as episodeNumberFor,
  } from "$lib/episode";
  import { matchesQuery } from "$lib/release-filter";
  import { clarityLabel, clarityTitle, matchClarity } from "$lib/release-match";
  import {
    badgeClass,
    badgeTitle,
    clarityClass,
    formatSize,
  } from "$lib/release-display";
  import { createReleaseSearch } from "$lib/release-search.svelte";
  import { createSpeedTest } from "$lib/speed-test.svelte";
  import { compactResult } from "$lib/speed-test";
  import {
    fileKind,
    matchesFileQuery,
    type FileKind,
  } from "$lib/torrent-files";
  import { availableResolutions, matchesResolution } from "$lib/resolution";
  import { nowPlayingSession, startPlaying } from "$lib/now-playing.svelte";
  import {
    titleForms,
    type Anime,
    type Release,
    type Resolution,
    type SearchMode,
  } from "$lib/types";
  import type { Episode } from "$lib/episodes";

  import FileFilter from "./FileFilter.svelte";
  import Modal from "./Modal.svelte";
  import ReleaseModeToggle from "./ReleaseModeToggle.svelte";
  import ResolutionFilter from "./ResolutionFilter.svelte";
  import StreamStatus from "./StreamStatus.svelte";

  let {
    open: isOpen,
    anime,
    episodes,
    episodeIndex,
    onClose,
  }: {
    open: boolean;
    anime: Anime;
    episodes: Episode[];
    episodeIndex?: number;
    onClose: () => void;
  } = $props();

  // --- Episode Derivations --------------------------------------------------

  const episode = $derived(
    episodeIndex !== undefined ? episodes[episodeIndex] : undefined,
  );
  const wantedEpisode = $derived(
    episode !== undefined ? episodeNumberFor(episode) : undefined,
  );
  const episodeOffset = $derived(absoluteOffset(episodes));
  const wantedAbsoluteEpisode = $derived.by(() => {
    if (wantedEpisode === undefined || episodeOffset === 0) return undefined;
    return wantedEpisode + episodeOffset;
  });

  const displayNumber = $derived(
    episodeIndex !== undefined ? episodeIndex + 1 : undefined,
  );

  // --- Shared State & Search ------------------------------------------------

  let releaseMode = $state<SearchMode>("episodes");

  const search = createReleaseSearch(() => {
    if (!isOpen) return null;
    const forms = titleForms(anime.title);
    if (forms.length === 0) return null;
    return {
      titles: forms,
      episode: wantedEpisode,
      absoluteEpisode: wantedAbsoluteEpisode,
      mode: releaseMode,
    };
  });

  const session = nowPlayingSession();
  const speed = createSpeedTest();
  const speedResult = $derived(
    speed.result ? compactResult(speed.result) : null,
  );

  $effect(() => {
    if (!isOpen) return;
    return speed.start();
  });

  $effect(() => {
    if (!isOpen) return;
    if (episodeIndex === undefined) return;
    untrack(() => startPlaying(anime, episodeIndex, episodes));
  });

  // --- Filtering & Sorting --------------------------------------------------

  let releaseQuery = $state("");
  let resolutionFilter = $state<Resolution[]>([]);
  let fileQuery = $state("");

  const filteredFiles = $derived(
    session.files.filter((file) => matchesFileQuery(file, fileQuery)),
  );

  const resolutionOptions = $derived(availableResolutions(search.releases));

  function toggleResolution(resolution: Resolution): void {
    resolutionFilter = resolutionFilter.includes(resolution)
      ? resolutionFilter.filter((value) => value !== resolution)
      : [...resolutionFilter, resolution];
  }

  const rankedReleases = $derived.by(() => {
    return search.releases
      .map((release, index) => ({ release, index }))
      .filter(({ release }) => matchesQuery(release, releaseQuery))
      .filter(({ release }) => matchesResolution(release, resolutionFilter))
      .sort((a, b) => {
        const scoreA =
          search.badgeFor(a.index)?.combinedScore ?? a.release.score;
        const scoreB =
          search.badgeFor(b.index)?.combinedScore ?? b.release.score;
        return scoreB - scoreA;
      });
  });

  const queryMatches = $derived(
    search.releases.filter((release) => matchesQuery(release, releaseQuery))
      .length,
  );

  // --- Actions --------------------------------------------------------------

  async function pickRelease(release: Release): Promise<void> {
    await session.playRelease(release);
  }

  async function loadTorrentByHand(): Promise<void> {
    const picked = await open({
      multiple: false,
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
    });
    if (typeof picked !== "string") return;
    await session.loadTorrentFile(picked);
  }

  function changeTorrent(): void {
    if (session.files.length > 0) {
      session.reset();
      return;
    }
    void loadTorrentByHand();
  }

  async function openMagnet(release: Release): Promise<void> {
    try {
      await openUrl(release.magnetUri);
    } catch {
      // Ignored
    }
  }

  async function downloadReleaseTorrent(release: Release): Promise<void> {
    const url = release.torrentUrl;
    if (url === undefined) return;

    const path = await save({
      defaultPath: torrentFileName(release),
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
    });
    if (path === null) return;

    try {
      await downloadTorrent(url, path);
    } catch {
      // Non-fatal
    }
  }

  function torrentFileName(release: Release): string {
    const safe = release.title.replace(/[^A-Za-z0-9._-]+/g, "_").slice(0, 120);
    return `${safe || "release"}.torrent`;
  }

  const stage = $derived(session.files.length > 0 ? "files" : "releases");
  const showStatus = $derived(
    session.chosen !== null && session.streamUrl !== undefined,
  );
</script>

<Modal open={isOpen} {onClose} label="Watch episode">
  <!-- Modal Header -->
  <header class="flex items-start gap-4 border-b border-border-subtle p-5">
    {#if episode?.thumbnail}
      <div
        class="relative aspect-video w-44 shrink-0 overflow-hidden rounded-lg border border-border-subtle bg-surface-raised shadow-sm"
      >
        <img
          src={episode.thumbnail}
          alt={episode.title ?? "Episode thumbnail"}
          class="h-full w-full object-cover"
        />
      </div>
    {/if}

    <div class="min-w-0 flex-1 pt-0.5">
      {#if displayNumber !== undefined}
        <span
          class="inline-block rounded bg-accent/10 px-2 py-0.5 text-[0.65rem] font-bold tracking-widest text-accent uppercase"
        >
          Episode {displayNumber}
        </span>
      {/if}
      <h2 class="mt-1.5 truncate text-lg font-bold text-ink tracking-tight">
        {episode?.title ?? "Watch episode"}
      </h2>
      {#if episode?.aired}
        <p class="mt-0.5 text-xs text-ink-faint">Aired {episode.aired}</p>
      {/if}
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#if episode?.filler}
          <span
            class="rounded bg-danger/20 px-1.5 py-0.5 text-[0.65rem] font-semibold text-danger"
            >Filler</span
          >
        {/if}
        {#if episode?.recap}
          <span
            class="rounded bg-health-yellow/20 px-1.5 py-0.5 text-[0.65rem] font-semibold text-health-yellow"
            >Recap</span
          >
        {/if}
      </div>
    </div>

    <button
      type="button"
      onclick={onClose}
      aria-label="Close modal"
      class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-border-subtle bg-surface-hover text-ink-muted transition-colors hover:border-accent hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      <svg class="size-4 fill-none stroke-current stroke-2" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="M6 18L18 6M6 6l12 12"
        />
      </svg>
    </button>
  </header>

  <!-- Modal Body -->
  <div class="min-h-0 flex-1 overflow-y-auto p-5" data-testid="modal-body">
    {#if stage === "files"}
      <div data-testid="stage-files" class="space-y-4">
        {#if showStatus}
          <div
            class="rounded-xl border border-border-subtle bg-surface-raised p-4"
          >
            <StreamStatus
              progress={session.progress}
              fileFraction={session.fileFraction}
              torrentFraction={session.torrentFraction}
              speedHistory={session.speedHistory}
              staleSeconds={session.staleSeconds}
              empty={false}
              paused={session.paused}
              fileBytes={session.chosen
                ? (session.progress?.fileProgress[session.chosen.idx] ?? 0)
                : undefined}
              fileTotalBytes={session.chosen?.lengthBytes}
            >
              {#snippet actions()}
                <div class="flex items-center gap-1.5">
                  <!-- Pause / Resume Button -->
                  <button
                    type="button"
                    onclick={() =>
                      session.paused ? session.resume() : session.pause()}
                    disabled={session.loading}
                    title={session.paused ? "Resume" : "Pause"}
                    aria-label={session.paused
                      ? "Resume playback"
                      : "Pause playback"}
                    data-testid="buffer-pause"
                    class="group relative flex size-8 items-center justify-center rounded-full border border-border-subtle bg-surface text-ink transition-all hover:border-accent hover:bg-accent hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50"
                  >
                    {#if session.paused}
                      <!-- Play / Resume Icon -->
                      <svg
                        class="size-3.5 fill-current ml-0.5"
                        viewBox="0 0 24 24"
                      >
                        <path d="M8 5v14l11-7z" />
                      </svg>
                    {:else}
                      <!-- Pause Icon -->
                      <svg class="size-3.5 fill-current" viewBox="0 0 24 24">
                        <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                      </svg>
                    {/if}
                  </button>

                  <!-- Cancel / Stop Button -->
                  <button
                    type="button"
                    onclick={() => session.reset()}
                    disabled={session.loading}
                    title="Cancel"
                    aria-label="Cancel streaming"
                    data-testid="buffer-cancel"
                    class="group relative flex size-8 items-center justify-center rounded-full border border-border-subtle bg-surface text-ink-muted transition-all hover:border-danger hover:bg-danger hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50"
                  >
                    <!-- Stop / Cancel Icon -->
                    <svg class="size-3.5 fill-current" viewBox="0 0 24 24">
                      <path d="M6 6h12v12H6z" />
                    </svg>
                  </button>
                </div>
              {/snippet}
            </StreamStatus>
            {#if session.launched}
              <p
                class="mt-3 flex items-center justify-center gap-2 text-xs font-medium text-ink-muted"
                data-testid="playing-banner"
              >
                <span
                  class="size-2 shrink-0 rounded-full bg-health-green motion-safe:animate-pulse"
                  aria-hidden="true"
                ></span>
                Playing in external player — feel free to close this window.
              </p>
            {:else}
              <p class="mt-3 text-center text-xs text-ink-muted">
                Buffering file for playback…
              </p>
            {/if}
          </div>
        {/if}

        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-2 min-w-0">
            <button
              type="button"
              onclick={() => session.reset()}
              class="flex items-center gap-1 text-xs font-medium text-accent hover:underline focus:outline-none"
            >
              ← Back to releases
            </button>
            <span class="text-ink-faint">·</span>
            <p class="truncate text-xs text-ink-faint">
              {session.chosenRelease
                ? session.chosenRelease.title
                : "Files inside torrent"}
            </p>
          </div>
          <span class="shrink-0 text-xs font-medium text-ink-faint">
            {session.files.length}
            {session.files.length === 1 ? "file" : "files"}
          </span>
        </div>

        {#if session.paused && session.chosen === null}
          <p
            class="rounded-lg border border-accent/30 bg-accent/10 px-3 py-2 text-xs text-accent-hover"
            role="status"
            data-testid="pick-file-hint"
          >
            Paused — pick a file to start buffering.
          </p>
        {/if}

        {#if session.files.length > 4}
          <FileFilter
            value={fileQuery}
            onInput={(next) => (fileQuery = next)}
          />
        {/if}

        <div class="max-h-72 overflow-y-auto pr-1">
          <ul class="flex flex-col gap-2" data-testid="torrent-files">
            {#each filteredFiles as file (file.idx)}
              {@const isChosen = session.chosen?.idx === file.idx}
              <li>
                <button
                  type="button"
                  onclick={() => session.play(file)}
                  class="group flex w-full items-center gap-3 rounded-xl border p-3 text-left text-xs transition-all {isChosen
                    ? 'border-accent bg-accent/5 text-ink ring-1 ring-accent/30'
                    : 'border-border-subtle bg-surface-raised text-ink-muted hover:border-accent/60 hover:bg-surface-hover hover:text-ink'}"
                >
                  <span
                    aria-hidden="true"
                    class="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border-subtle bg-surface-hover font-bold"
                  >
                    {#if fileKind(file) === "video"}
                      <svg
                        class="size-4 text-accent fill-current"
                        viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg
                      >
                    {:else if fileKind(file) === "subtitle"}
                      <span class="text-xs text-health-yellow font-serif"
                        >CC</span
                      >
                    {:else if fileKind(file) === "image"}
                      <svg
                        class="size-4 text-health-green fill-none stroke-current stroke-2"
                        viewBox="0 0 24 24"
                        ><rect
                          x="3"
                          y="3"
                          width="18"
                          height="18"
                          rx="2"
                        /><circle cx="8.5" cy="8.5" r="1.5" /><path
                          d="M21 15l-5-5L5 21"
                        /></svg
                      >
                    {:else}
                      <svg
                        class="size-4 text-ink-faint fill-none stroke-current stroke-2"
                        viewBox="0 0 24 24"
                        ><path
                          d="M13 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V9z"
                        /><path d="M13 2v7h7" /></svg
                      >
                    {/if}
                  </span>

                  <span class="min-w-0 flex-1">
                    <span class="block truncate font-medium text-ink"
                      >{file.name}</span
                    >
                    <span class="mt-0.5 block text-[10.5px] text-ink-faint">
                      {formatSize(file.lengthBytes) ?? "Unknown size"}
                    </span>
                  </span>

                  <!-- Dynamic Play / Playing Action Badge -->
                  <span
                    class="shrink-0 rounded-lg border px-3 py-1 text-[11px] font-semibold transition-colors {isChosen
                      ? 'border-accent bg-accent text-white shadow-sm'
                      : 'border-border-subtle bg-surface text-ink group-hover:border-accent group-hover:bg-accent group-hover:text-white'}"
                  >
                    {isChosen ? "Playing..." : "Play"}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      </div>
    {:else}
      <div data-testid="stage-releases">
        <!-- Persistent Stage Controls -->
        <div class="mb-3 flex items-center justify-between gap-2">
          <h3 class="text-xs font-bold uppercase tracking-wider text-ink-faint">
            Available Releases
          </h3>
          <ReleaseModeToggle
            mode={releaseMode}
            disabled={search.searching || session.loadingRelease}
            onChange={(m) => (releaseMode = m)}
          />
        </div>

        {#if search.searching}
          <div class="py-12 text-center" data-testid="stage-searching">
            <div
              class="mx-auto size-7 animate-spin rounded-full border-2 border-border-subtle border-t-accent"
            ></div>
            <p class="mt-4 text-xs font-medium text-ink-muted">
              Searching indexers for {releaseMode === "packs"
                ? "season packs"
                : "episode releases"}{displayNumber !== undefined
                ? ` (Episode ${displayNumber})`
                : ""}…
            </p>
          </div>
        {:else if search.error}
          <div
            class="my-6 rounded-xl border border-danger/30 bg-danger/5 p-4 text-center"
          >
            <p class="text-xs font-medium text-danger">{search.error}</p>
          </div>
        {:else if search.releases.length === 0}
          <div class="py-10 text-center" data-testid="releases-empty">
            <p class="text-xs text-ink-muted">
              {releaseMode === "packs"
                ? "No season packs found for this title."
                : "No episode releases found."}
            </p>
            <button
              type="button"
              onclick={loadTorrentByHand}
              class="mt-3 text-xs font-medium text-accent hover:underline focus:outline-none"
            >
              Load local .torrent file instead
            </button>
          </div>
        {:else}
          <div class="mb-3 flex flex-wrap items-center gap-2">
            <input
              type="search"
              bind:value={releaseQuery}
              placeholder="Filter releases…"
              aria-label="Filter releases"
              data-testid="release-filter"
              class="min-w-0 flex-1 rounded-full border border-border-subtle bg-surface-raised px-4 py-2 text-xs text-ink placeholder:text-ink-faint focus:border-accent focus:bg-surface focus:outline-none focus:ring-2 focus:ring-accent/20"
            />
            <ResolutionFilter
              available={resolutionOptions}
              selected={resolutionFilter}
              onToggle={toggleResolution}
            />
          </div>

          {#if queryMatches === 0}
            <p
              class="py-8 text-center text-xs text-ink-muted"
              data-testid="releases-no-match"
            >
              No releases match "<span class="font-semibold"
                >{releaseQuery.trim()}</span
              >"
            </p>
          {:else}
            <ul class="flex max-h-72 flex-col gap-2 overflow-y-auto pr-1">
              {#each rankedReleases as { release, index } (release.infoHash ?? release.title)}
                {@const clarity = matchClarity(release)}
                {@const outcome = search.badgeFor(index)}
                <li
                  class="group flex items-center gap-3 rounded-xl border p-3 text-xs transition-all {clarityClass(
                    clarity,
                  )} {session.chosenRelease?.title === release.title
                    ? 'border-accent bg-accent/5 text-ink'
                    : 'border-border-subtle bg-surface-raised text-ink-muted hover:border-accent/60 hover:bg-surface-hover hover:text-ink'}"
                  data-clarity={clarity}
                  title={clarityTitle(clarity)}
                >
                  <span
                    class="size-2 shrink-0 rounded-full {badgeClass(
                      outcome?.badge,
                    )}"
                    data-testid="release-badge"
                    data-badge={outcome?.badge ?? "pending"}
                    title={badgeTitle(outcome)}
                    aria-hidden="true"
                  ></span>

                  <button
                    type="button"
                    onclick={() => pickRelease(release)}
                    disabled={session.loadingRelease}
                    data-testid="release-play"
                    class="min-w-0 flex-1 text-left focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    <span class="block truncate font-semibold text-ink"
                      >{release.title}</span
                    >
                    <div
                      class="mt-1 flex flex-wrap items-center gap-2 text-[10.5px] text-ink-faint"
                    >
                      {#if clarityLabel(clarity)}
                        <span
                          class="rounded px-1.5 py-0.2 text-[9.5px] font-bold uppercase {clarity ===
                          'stated'
                            ? 'bg-accent/20 text-accent'
                            : 'bg-surface-hover text-ink-faint'}"
                        >
                          {clarityLabel(clarity)}
                        </span>
                      {/if}
                      {#if release.resolution !== "unknown"}<span
                          >{release.resolution}</span
                        >{/if}
                      {#if release.source !== "unknown"}<span
                          >· {release.source}</span
                        >{/if}
                      {#if release.seeders !== undefined}
                        <span class="text-health-green font-medium"
                          >· {release.seeders} seeders</span
                        >
                      {/if}
                      {#if formatSize(release.sizeBytes)}<span
                          >· {formatSize(release.sizeBytes)}</span
                        >{/if}
                    </div>
                  </button>

                  <div class="flex items-center gap-1 shrink-0">
                    <button
                      type="button"
                      onclick={() => openMagnet(release)}
                      data-testid="release-magnet"
                      aria-label="Open magnet link"
                      title="Open in external torrent client"
                      class="flex size-7 items-center justify-center rounded-lg border border-border-subtle bg-surface text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                    >
                      <svg
                        class="size-3.5 fill-none stroke-current stroke-2"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
                        />
                      </svg>
                    </button>
                    <button
                      type="button"
                      onclick={() => downloadReleaseTorrent(release)}
                      disabled={release.torrentUrl === undefined}
                      data-testid="release-download"
                      aria-label="Download torrent file"
                      title={release.torrentUrl === undefined
                        ? "No torrent file available"
                        : "Download .torrent file"}
                      class="flex size-7 items-center justify-center rounded-lg border border-border-subtle bg-surface text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-30 disabled:hover:border-border-subtle disabled:hover:text-ink-muted"
                    >
                      <svg
                        class="size-3.5 fill-none stroke-current stroke-2"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                        />
                      </svg>
                    </button>
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}

        {#if session.error}
          <p class="mt-3 text-xs text-danger font-medium" role="status">
            {session.error}
          </p>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Modal Footer -->
  {#snippet footer()}
    <div
      class="flex flex-wrap items-center gap-3 border-t border-border-subtle bg-surface-raised px-5 py-3.5"
    >
      <button
        type="button"
        onclick={changeTorrent}
        disabled={session.loading}
        class="rounded-lg border border-border-subtle bg-surface px-3.5 py-1.5 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50"
      >
        {session.loading
          ? "Loading…"
          : session.files.length > 0
            ? "Change torrent"
            : "Load local torrent"}
      </button>

      <div class="flex items-center gap-2">
        <button
          type="button"
          onclick={speed.run}
          disabled={speed.running}
          data-testid="modal-speed-test"
          title="Test network connection speed"
          class="rounded-lg border border-border-subtle bg-surface px-3.5 py-1.5 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50"
        >
          {#if speed.running && speed.progress}
            Speed test ({speed.progress.percent}%)
          {:else if speed.running}
            Testing…
          {:else}
            Speed test
          {/if}
        </button>

        {#if speedResult}
          <span
            class="text-xs font-semibold text-accent-hover"
            data-testid="modal-speed-result"
          >
            {speedResult}
          </span>
        {/if}

        {#if speed.error}
          <span class="text-xs text-danger" data-testid="modal-speed-error">
            {speed.error}
          </span>
        {/if}
      </div>

      <div class="flex-1"></div>

      <button
        type="button"
        onclick={onClose}
        class="rounded-lg px-4 py-1.5 text-xs font-medium text-ink-muted transition-colors hover:bg-surface-hover hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        Close
      </button>
    </div>
  {/snippet}
</Modal>
