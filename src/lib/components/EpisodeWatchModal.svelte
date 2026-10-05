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
  import { fileKind, type FileKind } from "$lib/torrent-files";
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

  import Modal from "./Modal.svelte";
  import ReleaseModeToggle from "./ReleaseModeToggle.svelte";
  import ResolutionFilter from "./ResolutionFilter.svelte";
  import StreamStatus from "./StreamStatus.svelte";

  /**
   * The in-place episode watcher.
   *
   * Opened from an episode card on the detail page, it searches the indexers,
   * lets the reader pick a release, lists the files inside it, and launches the
   * external player -- all without leaving the page.
   */
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
    /** The selected episode's index into `episodes`, when one is chosen. */
    episodeIndex?: number;
    onClose: () => void;
  } = $props();

  // --- the episode ---------------------------------------------------------

  const episode = $derived(
    episodeIndex !== undefined ? episodes[episodeIndex] : undefined,
  );
  const wantedEpisode = $derived(
    episode !== undefined ? episodeNumberFor(episode) : undefined,
  );
  const episodeOffset = $derived(absoluteOffset(episodes));
  const wantedAbsoluteEpisode = $derived.by(() => {
    const relative = wantedEpisode;
    if (relative === undefined || episodeOffset === 0) return undefined;
    return relative + episodeOffset;
  });

  const displayNumber = $derived(
    episodeIndex !== undefined ? episodeIndex + 1 : undefined,
  );

  // --- the shared state machines -------------------------------------------

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
    const index = episodeIndex;
    if (index === undefined) return;
    untrack(() => startPlaying(anime, index, episodes));
  });

  // --- filtering the release list ------------------------------------------

  let releaseQuery = $state("");
  let resolutionFilter = $state<Resolution[]>([]);

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

  /** Move the release's magnet into the session. */
  async function pickRelease(release: Release): Promise<void> {
    await session.playRelease(release);
  }

  /** Load a `.torrent` from disk and hand it to the session. */
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

  // --- file-row presentation ----------------------------------------------

  function kindIcon(kind: FileKind): string {
    switch (kind) {
      case "video":
        return "▶";
      case "subtitle":
        return "字";
      case "image":
        return "▤";
      case "other":
        return "·";
    }
  }

  function kindClass(kind: FileKind): string {
    switch (kind) {
      case "video":
        return "text-accent-hover border-accent/40";
      case "subtitle":
        return "text-health-yellow";
      case "image":
        return "text-health-green";
      case "other":
        return "text-ink-faint";
    }
  }

  function kindLabel(kind: FileKind): string {
    switch (kind) {
      case "video":
        return "Video";
      case "subtitle":
        return "Subtitle";
      case "image":
        return "Image";
      case "other":
        return "File";
    }
  }

  // --- stage derivation ----------------------------------------------------
  // Only toggles between "files" and "releases". Searching is rendered in-place
  // inside the releases view so persistent controls like the mode toggle stay mounted.

  const stage = $derived.by(() => {
    if (session.files.length > 0) return "files";
    return "releases";
  });

  const showStatus = $derived(
    session.chosen !== null && session.streamUrl !== undefined,
  );
</script>

<Modal open={isOpen} {onClose} label="Watch episode">
  <!-- Header: the episode being watched. -->
  <header class="flex gap-4 border-b border-border-subtle p-5">
    {#if episode?.thumbnail}
      <div
        class="relative aspect-video w-48 shrink-0 overflow-hidden rounded-lg border border-border-subtle"
      >
        <img
          src={episode.thumbnail}
          alt=""
          class="h-full w-full object-cover"
        />
      </div>
    {/if}

    <div class="min-w-0 flex-1">
      {#if displayNumber !== undefined}
        <p
          class="text-[0.7rem] font-bold tracking-[0.12em] text-accent uppercase"
        >
          Episode {displayNumber}
        </p>
      {/if}
      <h2 class="mt-1 text-xl font-semibold tracking-tight">
        {episode?.title ?? "Watch episode"}
      </h2>
      {#if episode?.aired}
        <p class="mt-1 text-xs text-ink-faint">Aired {episode.aired}</p>
      {/if}
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#if episode?.filler}
          <span
            class="rounded bg-danger/20 px-1.5 py-0.5 text-[0.65rem] font-semibold text-danger"
          >
            Filler
          </span>
        {/if}
        {#if episode?.recap}
          <span
            class="rounded bg-health-yellow/20 px-1.5 py-0.5 text-[0.65rem] font-semibold text-health-yellow"
          >
            Recap
          </span>
        {/if}
      </div>
    </div>

    <button
      type="button"
      onclick={onClose}
      aria-label="Close"
      class="size-8 shrink-0 self-start rounded-lg border border-border-subtle bg-surface-hover text-ink-muted transition-colors hover:border-accent hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      ✕
    </button>
  </header>

  <!-- Body: the staged content. -->
  <div class="min-h-0 flex-1 overflow-y-auto p-5" data-testid="modal-body">
    {#if stage === "files"}
      <div data-testid="stage-files">
        {#if showStatus}
          <div class="mb-4">
            <StreamStatus
              progress={session.progress}
              fileFraction={session.fileFraction}
              torrentFraction={session.torrentFraction}
              speedHistory={session.speedHistory}
              staleSeconds={session.staleSeconds}
              empty={false}
            />
            {#if session.launched}
              <p
                class="mt-2 flex items-center justify-center gap-2 text-xs text-ink-muted"
                data-testid="playing-banner"
              >
                <span
                  class="size-1.5 shrink-0 rounded-full bg-health-green motion-safe:animate-pulse"
                  aria-hidden="true"
                ></span>
                Playing in your external player — you can close this window.
              </p>
            {:else}
              <p class="mt-2 text-center text-xs text-ink-muted">
                Waiting for enough of the file to start playback…
              </p>
            {/if}
          </div>
        {/if}

        <div class="mb-2 flex items-center justify-between gap-3">
          <p class="text-xs text-ink-faint">
            Pick the file to play{session.chosenRelease
              ? ` from ${session.chosenRelease.title}`
              : ""}.
          </p>
          <span class="shrink-0 text-[11px] text-ink-faint">
            {session.files.length} file{session.files.length === 1 ? "" : "s"}
          </span>
        </div>

        <ul class="flex flex-col gap-1.5" data-testid="torrent-files">
          {#each session.files as file (file.idx)}
            {@const kind = fileKind(file)}
            <li>
              <button
                type="button"
                onclick={() => session.play(file)}
                class="flex w-full items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-xs transition-colors {session
                  .chosen?.idx === file.idx
                  ? 'border-accent bg-surface-hover text-ink ring-1 ring-accent/30'
                  : 'border-border-subtle bg-surface-raised text-ink-muted hover:border-accent hover:text-ink'}"
              >
                <span
                  aria-hidden="true"
                  class="grid size-8 shrink-0 place-items-center rounded-lg border border-border-subtle bg-surface-hover text-sm {kindClass(
                    kind,
                  )}"
                >
                  {kindIcon(kind)}
                </span>

                <span class="min-w-0 flex-1">
                  <span class="block truncate">{file.name}</span>
                  <span class="mt-0.5 block text-[10.5px] text-ink-faint">
                    {kindLabel(kind)} · {formatSize(file.lengthBytes) ?? "—"}
                    {#if session.chosen?.idx === file.idx}
                      <span
                        class="ml-1.5 inline-block rounded bg-accent/20 px-1.5 text-[9.5px] font-semibold tracking-wide text-accent-hover uppercase"
                        >Matched</span
                      >
                    {/if}
                  </span>
                </span>

                <span
                  class="shrink-0 rounded-lg border border-border-subtle px-3 py-1 text-[11px] font-semibold {session
                    .chosen?.idx === file.idx
                    ? 'border-accent bg-accent text-white'
                    : 'text-ink-muted'}"
                >
                  Play
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {:else}
      <div data-testid="stage-releases">
        <!-- Persistent Header: Stays rendered during search and results -->
        <div class="mb-3 flex items-center justify-between gap-2">
          <h3 class="text-xs font-semibold tracking-tight text-ink-muted">
            Releases
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
              class="mx-auto size-6 animate-spin rounded-full border-2 border-border-subtle border-t-accent"
            ></div>
            <p class="mt-4 text-sm text-ink-muted">
              Searching indexers for {releaseMode === "packs"
                ? "packs"
                : "episodes"}{displayNumber !== undefined
                ? ` (Episode ${displayNumber})`
                : ""}…
            </p>
          </div>
        {:else if search.error}
          <p class="py-8 text-center text-sm text-ink-faint">{search.error}</p>
        {:else if search.releases.length === 0}
          <p
            class="py-8 text-center text-sm text-ink-muted"
            data-testid="releases-empty"
          >
            {releaseMode === "packs"
              ? "No packs found. Load a torrent by hand instead."
              : "No releases found. Load a torrent by hand instead."}
          </p>
        {:else}
          <div class="mb-3 flex flex-wrap items-center gap-2">
            <input
              type="search"
              bind:value={releaseQuery}
              placeholder="Filter releases…"
              aria-label="Filter releases"
              data-testid="release-filter"
              class="min-w-0 flex-1 rounded-full border border-border-subtle bg-surface px-4 py-2 text-xs text-ink placeholder:text-ink-faint focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
            <ResolutionFilter
              available={resolutionOptions}
              selected={resolutionFilter}
              onToggle={toggleResolution}
            />
          </div>

          {#if queryMatches === 0}
            <p
              class="py-6 text-center text-sm text-ink-muted"
              data-testid="releases-no-match"
            >
              No releases match "{releaseQuery.trim()}".
            </p>
          {:else}
            <ul class="flex max-h-72 flex-col gap-1.5 overflow-y-auto pr-1">
              {#each rankedReleases as { release, index } (release.infoHash ?? release.title)}
                {@const clarity = matchClarity(release)}
                {@const outcome = search.badgeFor(index)}
                <li
                  class="flex items-start gap-2.5 rounded-lg border px-3 py-2.5 text-xs transition-colors {clarityClass(
                    clarity,
                  )} {session.chosenRelease?.title === release.title
                    ? 'border-accent bg-surface-hover text-ink'
                    : 'border-border-subtle text-ink-muted hover:border-accent hover:text-ink'}"
                  data-clarity={clarity}
                  title={clarityTitle(clarity)}
                >
                  <span
                    class="mt-1.5 size-2 shrink-0 rounded-full {badgeClass(
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
                    class="min-w-0 flex-1 text-left disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    <span class="block truncate">{release.title}</span>
                    {#if clarityLabel(clarity)}
                      <span
                        class="mt-1 inline-block rounded px-1.5 text-[0.6rem] font-medium {clarity ===
                        'stated'
                          ? 'bg-accent/20 text-accent'
                          : 'bg-surface-hover text-ink-faint'}"
                      >
                        {clarityLabel(clarity)}
                      </span>
                    {/if}
                    <span class="mt-1 block text-ink-faint">
                      {#if release.resolution !== "unknown"}{release.resolution}{/if}
                      {#if release.source !== "unknown"}· {release.source}{/if}
                      {#if release.seeders !== undefined}· {release.seeders} seeders{/if}
                      {#if formatSize(release.sizeBytes)}· {formatSize(
                          release.sizeBytes,
                        )}{/if}
                    </span>
                  </button>

                  <button
                    type="button"
                    onclick={() => openMagnet(release)}
                    data-testid="release-magnet"
                    aria-label="Open magnet for {release.title}"
                    title="Open in your torrent client"
                    class="flex size-7 shrink-0 items-center justify-center rounded border border-border-subtle text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  >
                    🧲
                  </button>
                  <button
                    type="button"
                    onclick={() => downloadReleaseTorrent(release)}
                    disabled={release.torrentUrl === undefined}
                    data-testid="release-download"
                    aria-label="Download torrent for {release.title}"
                    title={release.torrentUrl === undefined
                      ? "No torrent file for this release"
                      : "Download the .torrent file"}
                    class="flex size-7 shrink-0 items-center justify-center rounded border border-border-subtle text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:border-border-subtle disabled:hover:text-ink-muted"
                  >
                    ⬇
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}

        {#if session.error}
          <p class="mt-3 text-xs text-danger" role="status">{session.error}</p>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Footer -->
  {#snippet footer()}
    <div
      class="flex flex-wrap items-center gap-2 border-t border-border-subtle bg-surface-raised px-5 py-3.5"
    >
      <button
        type="button"
        onclick={changeTorrent}
        disabled={session.loading}
        class="rounded-lg border border-border-subtle bg-surface-hover px-4 py-2 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
      >
        {session.loading
          ? "Loading…"
          : session.files.length > 0
            ? "Change torrent"
            : "Load local torrent file"}
      </button>

      <button
        type="button"
        onclick={speed.run}
        disabled={speed.running}
        data-testid="modal-speed-test"
        title="Test your internet speed"
        class="rounded-lg border border-border-subtle bg-surface-hover px-4 py-2 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
      >
        {#if speed.running && speed.progress}
          Testing… {speed.progress.percent}%
        {:else if speed.running}
          Testing…
        {:else}
          Test speed
        {/if}
      </button>

      {#if speedResult}
        <span
          class="text-[11px] text-ink-muted"
          data-testid="modal-speed-result"
        >
          {speedResult}
        </span>
      {/if}

      {#if speed.error}
        <span class="text-[11px] text-danger" data-testid="modal-speed-error">
          {speed.error}
        </span>
      {/if}

      <div class="flex-1"></div>

      <button
        type="button"
        onclick={onClose}
        class="rounded-lg px-4 py-2 text-xs font-medium text-ink-muted transition-colors hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        Close
      </button>
    </div>
  {/snippet}
</Modal>
