<script lang="ts">
  import { onDestroy } from "svelte";

  import { page } from "$app/state";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { errorMessage, getAnime, getEpisodes } from "$lib/api/anime";
  import {
    addMagnet,
    addTorrent,
    getStreamUrl,
    getTorrentStats,
    openInPlayer,
    removeTorrent,
  } from "$lib/api/player";
  import {
    downloadTorrent,
    onProbeResult,
    probeReleases,
    searchReleases,
  } from "$lib/api/releases";
  import { matchesQuery } from "$lib/release-filter";
  import {
    clarityLabel,
    clarityTitle,
    matchClarity,
    type MatchClarity,
  } from "$lib/release-match";
  import { availableResolutions, matchesResolution } from "$lib/resolution";
  import {
    absoluteOffset,
    episodeNumber as episodeNumberFor,
  } from "$lib/episode";
  import { episodesFor } from "$lib/episodes";
  import { createProgressRecorder } from "$lib/progress";
  import { recordLastPlayed, setListEntry } from "$lib/api/auth";
  import {
    displayTitle,
    titleForms,
    type Anime,
    type EpisodeInfo,
    type HealthBadge,
    type ProbeOutcome,
    type Release,
    type Resolution,
    type SearchMode,
    type TorrentFile,
    type TorrentHandle,
    type TorrentProgress,
  } from "$lib/types";
  import EpisodeList from "$lib/components/EpisodeList.svelte";
  import ExternalPlayerButton from "$lib/components/ExternalPlayerButton.svelte";
  import RelatedAnimeList from "$lib/components/RelatedAnimeList.svelte";
  import ReleaseModeToggle from "$lib/components/ReleaseModeToggle.svelte";
  import ResolutionFilter from "$lib/components/ResolutionFilter.svelte";
  import StreamStatus from "$lib/components/StreamStatus.svelte";

  const id = $derived(Number(page.params.id));

  let anime = $state<Anime | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  /**
   * Marks the current episode as watched once playback actually starts.
   *
   * Built once for the page's life: the set it keeps must survive across
   * re-selections, or switching files back and forth would re-write the same
   * episode every time. A failed write never breaks playback, but it is
   * logged rather than swallowed -- a silently dropped write is exactly what
   * made a watched work fail to appear on the list with no clue why.
   */
  const progress = createProgressRecorder(
    (animeId, episode) => setListEntry(animeId, "current", episode),
    (animeId, episode, error) => {
      console.warn(
        `Could not record progress for ${animeId} episode ${episode ?? "?"}:`,
        error,
      );
    },
  );

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

  /**
   * Which add is the current one, bumped by every new add AND by teardown.
   *
   * An add can still be resolving when the reader navigates away, and the
   * handle it returns names a live torrent that nothing will ever reference.
   * Bumping on teardown as well as on each add makes this single counter the
   * whole answer to "is this result still wanted": equal means yes, otherwise
   * the result belongs to a page that is gone or has moved on.
   *
   * Two adds can also overlap -- a release clicked and then a `.torrent` picked
   * by hand -- and the loser would otherwise never be removed. Sharing one
   * counter between both entry points is what makes that race detectable.
   */
  let addGeneration = 0;

  onDestroy(() => {
    addGeneration += 1;
  });

  /**
   * Take ownership of an add's result, or release it when it is unwanted.
   *
   * False means the page is gone or a later add has superseded this one; the
   * torrent is removed rather than left running, because nothing will ever
   * reference its id again.
   */
  function adopt(handle: TorrentHandle, generation: number): boolean {
    if (generation === addGeneration) return true;

    void removeTorrent(handle.id);
    return false;
  }

  /**
   * Release the current torrent when it is replaced or the page is left.
   *
   * Reading `torrentId` inside the effect is what registers the dependency, so
   * the cleanup fires both when a new torrent is adopted and on unmount. That
   * covers every add that *completed*; the in-flight case is `adopt`'s job.
   */
  $effect(() => {
    const id = torrentId;
    if (id === null) return;

    return () => {
      void removeTorrent(id);
    };
  });
  let streamUrl = $state<string | undefined>(undefined);

  // --- waiting for playback ------------------------------------------------
  //
  // The in-app <video> was removed: on Linux it is a WebKitGTK GStreamer
  // pipeline, one per element and unbounded by default, which is what made
  // playback consume all available memory. Playback now goes through an
  // external player, and this page shows what the torrent is doing while it
  // waits for enough of the episode to be playable.

  /**
   * How much of the chosen file must be downloaded before launching a player.
   *
   * A HEURISTIC, not a guarantee. mpv can usually start once the container
   * header and first cluster are present -- a few MB, which arrive first
   * because librqbit downloads sequentially -- but it may still stall if it
   * needs the file tail (an MKV's `Cues`) for duration or seeking. There is no
   * byte count that is knowably "enough"; this fraction is the trade-off
   * between starting sooner and stalling less.
   */
  const READY_FRACTION = 0.05;

  /** How often the status panel is refreshed while waiting. */
  const POLL_INTERVAL_MS = 500;

  /** The latest snapshot, or `null` before the first poll has answered. */
  let torrentProgress = $state<TorrentProgress | null>(null);
  /** The fraction of the chosen file downloaded so far, 0..1. */
  let fileFraction = $state(0);
  /** The fraction of the whole torrent downloaded so far, 0..1. */
  let torrentFraction = $state(0);
  /** True while a launch is being requested, so it fires only once. */
  let launching = $state(false);
  /** True once the player has been opened for the current selection. */
  let launched = $state(false);

  // --- the indexer search --------------------------------------------------

  let releases = $state<Release[]>([]);
  let chosenRelease = $state<Release | null>(null);
  let searching = $state(false);
  let loadingRelease = $state(false);
  let releaseError = $state<string | null>(null);
  /**
   * Whether the reader wants the one episode or the packs that contain it.
   *
   * A pack is a whole-season or batch upload; the torrent session already picks
   * the chosen episode's file out of one, so flipping this only changes which
   * releases the search returns.
   */
  let releaseMode = $state<SearchMode>("episodes");
  /**
   * Failure from handing a magnet to the OS.
   *
   * Kept apart from `releaseError`, which reports the search or the in-app
   * playback failing. The two come from different actions, and one message
   * covering both would point at the wrong thing.
   */
  let magnetError = $state<string | null>(null);
  /**
   * Failure from saving a release's `.torrent` file.
   *
   * Apart from the other two errors for the same reason they are apart from
   * each other: it names a different action and a different cause.
   */
  let downloadError = $state<string | null>(null);
  /** True while a torrent file is being fetched, so the button can say so. */
  let downloadingTorrent = $state(false);

  // --- swarm health probes -------------------------------------------------
  //
  // A search ranks by what the indexer claimed. The probe asks the torrents
  // themselves, so this holds one verdict per release by position. It is
  // deliberately index-aligned with `releases` rather than keyed by info hash:
  // a release is not guaranteed to carry a hash, and an index cannot go
  // missing.
  //
  // Nothing here is persisted: swarm health changes minute to minute, and a
  // verdict from a previous visit would be a lie.
  let probeOutcomes = $state<(ProbeOutcome | undefined)[]>([]);
  let probing = $state(false);

  // --- the resolution filter ------------------------------------------------
  //
  // Chips are derived from the unfiltered `releases`, not from what is left
  // after filtering: computing them from the filtered list would make a
  // selected chip disappear the moment it was clicked, leaving no way back.
  let resolutionFilter = $state<Resolution[]>([]);

  /**
   * The text typed into the release filter box.
   *
   * Not reset when a new search lands, unlike the resolution chips: it is
   * something the reader typed and it stays visible in the box, so an emptied
   * list is explainable rather than mysterious.
   */
  let releaseQuery = $state("");

  /** The resolutions present in the results, highest first. */
  const resolutionOptions = $derived(availableResolutions(releases));

  /**
   * Turn one resolution's chip on or off.
   *
   * The array is replaced rather than mutated so Svelte's reactivity fires;
   * an in-place push would not re-run the derived list.
   */
  function toggleResolution(resolution: Resolution): void {
    resolutionFilter = resolutionFilter.includes(resolution)
      ? resolutionFilter.filter((value) => value !== resolution)
      : [...resolutionFilter, resolution];
  }

  /** The badge for a release, or undefined while it has not been probed. */
  function badgeFor(index: number): HealthBadge | undefined {
    return probeOutcomes[index]?.badge;
  }

  /** The colour a badge dot is drawn in, or a muted one while unprobed. */
  function badgeClass(badge: HealthBadge | undefined): string {
    switch (badge) {
      case "green":
        return "bg-health-green";
      case "yellow":
        return "bg-health-yellow";
      case "red":
        return "bg-health-red";
      default:
        // Unprobed is not the same as dead, so it gets a neutral pulse rather
        // than a verdict colour.
        return "bg-ink-faint animate-pulse";
    }
  }

  /**
   * The row's emphasis for how explicitly its name states the episode.
   *
   * `stated` rows get the accent border, which already means "this one"
   * elsewhere in the app. `unclear` rows dim, so the eye skips them. The
   * ordinary `episode` case is left exactly as it was -- marking every row would
   * defeat the point of marking any.
   */
  function clarityClass(clarity: MatchClarity): string {
    switch (clarity) {
      case "stated":
        return "border-accent/70 ring-1 ring-accent/40";
      case "unclear":
        return "opacity-60";
      case "episode":
        return "";
    }
  }

  /** A one-line explanation of a release's probe, for the title attribute. */
  function badgeTitle(index: number): string {
    const outcome = probeOutcomes[index];
    if (!outcome) return "Checking swarm health…";

    const parts: string[] = [];
    const seeders = outcome.probe.scrape?.seeders;
    if (seeders !== undefined) parts.push(`${seeders} seeders on tracker`);
    parts.push(
      outcome.probe.metadata?.resolved
        ? "metadata confirmed"
        : "metadata not found",
    );
    return parts.join(" · ");
  }

  /**
   * Releases ordered by the best knowledge available.
   *
   * Before any probe lands this is the backend's static order. As verdicts
   * arrive, a release with a better combined score moves up. The sort is
   * stable, so equal scores keep the static order rather than shuffling.
   */
  const rankedReleases = $derived.by(() => {
    // The original index is carried through both the filter and the sort rather
    // than dropped: it is the key into `probeOutcomes`, and a row that survives
    // filtering still needs ITS OWN badge. Renumbering here would make every row
    // show the badge belonging to whatever release used to sit at that position.
    return releases
      .map((release, index) => ({ release, index }))
      .filter(({ release }) => matchesQuery(release, releaseQuery))
      .filter(({ release }) => matchesResolution(release, resolutionFilter))
      .sort((a, b) => {
        const scoreA = probeOutcomes[a.index]?.combinedScore ?? a.release.score;
        const scoreB = probeOutcomes[b.index]?.combinedScore ?? b.release.score;
        return scoreB - scoreA;
      });
  });

  /**
   * How many releases match the typed query, ignoring the resolution chips.
   *
   * Lets the list say "nothing matched your filter" rather than rendering an
   * empty scroller, which reads as a failure rather than a filter.
   */
  const queryMatches = $derived(
    releases.filter((release) => matchesQuery(release, releaseQuery)).length,
  );

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

  const title = $derived(
    anime ? (displayTitle(anime.title) ?? "Untitled") : null,
  );

  /**
   * The episode catalogue, or `null` while it is being fetched.
   *
   * The list used to come from AniList's `streamingEpisodes`, which is a list of
   * licensed links rather than episodes -- and carries the whole franchise's links
   * on the first season's entry. The catalogue is the real per-episode data.
   */
  let episodeInfo = $state<EpisodeInfo[] | null>(null);

  $effect(() => {
    const malId = anime?.idMal;

    if (malId === undefined) {
      episodeInfo = [];
      return;
    }

    let cancelled = false;
    episodeInfo = null;

    getEpisodes(malId)
      .then((result) => {
        if (cancelled) return;
        episodeInfo = result;
      })
      .catch(() => {
        if (cancelled) return;
        // A missing catalogue is not an error: the synthesised list stands in.
        episodeInfo = [];
      });

    return () => {
      cancelled = true;
    };
  });

  const episodes = $derived(anime ? episodesFor(anime, episodeInfo ?? []) : []);

  /**
   * The episode number of the entry at `index`.
   *
   * Delegates to the shared heuristic rather than repeating it. That one prefers
   * an explicit `number` and only falls back to parsing the title or URL, which
   * matters now that entries come from the catalogue: a real title is an episode
   * *name* ("Theatrical Malice") carrying no number at all, so parsing alone would
   * find nothing and the search would run without an episode.
   */
  function episodeNumber(index: number): number | undefined {
    const ep = episodes[index];
    return ep ? episodeNumberFor(ep) : undefined;
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
   * How much to add to a list position to get the work's absolute number.
   *
   * Zero for an ordinary 1..N season. Non-zero for a later cour, whose releases
   * are named with the running total (`Show - 13`) even though the cour's own
   * list starts at 1. See `absoluteOffset`.
   */
  const episodeOffset = $derived(absoluteOffset(episodes));

  /**
   * The work-wide episode number, when it differs from the cour-relative one.
   *
   * `undefined` when the two agree, so the common case sends one number and the
   * backend does not have to disambiguate a value that was never in doubt.
   */
  const wantedAbsoluteEpisode = $derived.by(() => {
    const relative = wantedEpisode;
    if (relative === undefined || episodeOffset === 0) return undefined;
    return relative + episodeOffset;
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
    const work = anime;
    const episode = wantedEpisode;
    // Read the mode so flipping the toggle re-runs the search. It is part of
    // the request key for the same reason the episode is.
    const mode = releaseMode;

    if (work === null) return;

    // Every title form the work has: an uploader may have used the English or
    // the romaji title, and the backend searches each.
    const forms = titleForms(work.title);
    if (forms.length === 0) return;

    let cancelled = false;
    searching = true;
    releaseError = null;

    searchReleases(forms, episode, wantedAbsoluteEpisode, mode)
      .then((found) => {
        if (cancelled) return;
        releases = found;
        // A new result set may not contain the resolutions the old filter
        // selected, which would leave the list empty with the filter still
        // "on". Resetting keeps the visible chips and the filter in step.
        resolutionFilter = [];
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
   * Probe the found releases for swarm health, once a search settles.
   *
   * Keyed on the release list itself, so it runs when a search produces a new
   * set, not when the stream or file state changes. Results arrive as events
   * and are written into `probeOutcomes` by position; the promise's return
   * value is ignored because the events already delivered it.
   *
   * A failure is swallowed: probing enhances the ranking, so an unreachable
   * DHT should leave the static order alone rather than surface an error the
   * reader cannot act on.
   */
  $effect(() => {
    const found = releases;
    if (found.length === 0) return;

    let cancelled = false;
    probing = true;
    // Reset to the list's length so an index always lands in bounds, even
    // for an event that arrives before this effect finishes setting up.
    probeOutcomes = new Array(found.length).fill(undefined);

    const unlisten = onProbeResult((outcome) => {
      if (cancelled) return;
      if (outcome.index < 0 || outcome.index >= probeOutcomes.length) return;
      // Replace the array rather than mutating a slot: Svelte tracks the
      // binding, and an in-place write would not re-run the derived ranking.
      const next = probeOutcomes.slice();
      next[outcome.index] = outcome;
      probeOutcomes = next;
    });

    probeReleases(found)
      .catch(() => {
        // Already handled by the events; nothing to surface.
      })
      .finally(() => {
        if (cancelled) return;
        probing = false;
      });

    return () => {
      cancelled = true;
      unlisten.then((fn) => fn());
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
    const direct = matchNumber(candidates, number);
    if (direct !== null) return direct;

    // A later cour's releases are named with the running total (`Show - 13`)
    // even though this cour's list starts at 1, so the absolute spelling is
    // tried before giving up. `episodeOffset` is 0 for an ordinary 1..N
    // season, in which case the second attempt would repeat the first.
    const absolute = number + episodeOffset;
    if (absolute === number) return null;

    return matchNumber(candidates, absolute);
  }

  /**
   * The file whose name carries `number` as a delimited token.
   *
   * Split from `fileForEpisode` so the relative and absolute spellings are
   * matched by one rule rather than two that could drift apart.
   */
  function matchNumber(
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

    // A new file is a new wait: clear the previous file's numbers so the panel
    // does not briefly show the last episode's progress as if it were this
    // one's.
    torrentProgress = null;
    fileFraction = 0;
    torrentFraction = 0;
    launched = false;
    launching = false;

    try {
      streamUrl = await getStreamUrl(torrentId, file.idx);
    } catch (err) {
      streamUrl = undefined;
      torrentError = errorMessage(err);
      return;
    }

    // The progress is NOT recorded here. Nothing has played yet -- the reader
    // is still waiting for bytes -- so marking the episode watched would be a
    // lie. It is written by `launch` once a player is actually opened.
  }

  /**
   * Refresh the status panel from the backend.
   *
   * A failure is swallowed rather than surfaced: a dropped poll is transient
   * and the next tick will answer, while flashing an error for every blip
   * would make a working download look broken.
   */
  async function pollProgress(): Promise<void> {
    if (torrentId === null) return;

    let snapshot: TorrentProgress | null;
    try {
      snapshot = await getTorrentStats(torrentId);
    } catch {
      return;
    }
    if (snapshot === null) return;

    torrentProgress = snapshot;
    torrentFraction =
      snapshot.totalBytes > 0
        ? snapshot.progressBytes / snapshot.totalBytes
        : 0;
    fileFraction =
      chosen && chosen.lengthBytes > 0
        ? (snapshot.fileProgress[chosen.idx] ?? 0) / chosen.lengthBytes
        : 0;
  }

  /**
   * Open the external player, once enough of the file is present.
   *
   * Guarded so it fires exactly once per selection: the poll runs several times
   * a second, and every tick past the threshold would otherwise launch another
   * player. The progress writes move here rather than to `play` because this is
   * the moment playback actually begins.
   */
  async function launch(): Promise<void> {
    if (launched || launching) return;
    if (streamUrl === undefined || torrentId === null) return;

    launching = true;
    try {
      // No player argument: the backend uses the reader's stored preference,
      // falling back to its default when none was ever chosen.
      await openInPlayer(streamUrl, undefined, torrentId);
    } catch (err) {
      torrentError = errorMessage(err);
      launching = false;
      return;
    }

    launched = true;
    launching = false;

    // Only now that playback has started. The episode NUMBER is recorded, not
    // the list index: `wantedEpisode` is already derived through
    // `episodeNumberFor`, and sending the index would be off by one on every
    // entry. Recorded even without a number: watching a release with no episode
    // selected still means the reader is watching this work.
    progress.record(id, wantedEpisode);

    // Separately, remember this as the work the reader last OPENED. The resume
    // disc reads this rather than the list, because the list only reorders on a
    // CHANGE -- re-watching the current episode would not move it. Fire and
    // forget: a failed record must never affect playback.
    void recordLastPlayed(id, wantedEpisode).catch(() => {
      // Swallowed deliberately: this is a convenience hint, and the disc falls
      // back to the list when it is missing.
    });
  }

  /**
   * Poll the torrent while a file is selected and the player has not opened.
   *
   * Keyed on `chosen` and `streamUrl`, so a new selection restarts the wait and
   * the cleanup clears the old timer. It stops as soon as `launched` is set,
   * because there is nothing left to wait for and polling a playing torrent
   * every half second would be pure noise.
   */
  $effect(() => {
    const file = chosen;
    const url = streamUrl;
    if (file === null || url === undefined) return;

    // Read once so the effect re-runs when it flips, tearing down the timer.
    void launched;
    if (launched) return;

    // Poll, then check readiness. The first poll runs immediately rather than
    // waiting a full interval, so an already-downloaded file launches at once
    // instead of after an arbitrary delay.
    const tick = () =>
      pollProgress().then(() => {
        if (fileFraction >= READY_FRACTION) void launch();
      });

    void tick();
    const timer = setInterval(() => void tick(), POLL_INTERVAL_MS);

    return () => clearInterval(timer);
  });

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
      const generation = ++addGeneration;
      const handle = await addMagnet(release.magnetUri);
      if (!adopt(handle, generation)) return;

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

  /**
   * Hand a release's magnet to the OS, for the reader's own torrent client.
   *
   * Deliberately does NOT touch the app's engine: `playRelease` already streams
   * a release in-app, and this is the opposite action -- it delegates the
   * download to whatever client the reader has registered for `magnet:`.
   *
   * A machine with no client registered rejects here, which must surface rather
   * than look like a button that does nothing.
   */
  async function openMagnet(release: Release): Promise<void> {
    magnetError = null;
    try {
      await openUrl(release.magnetUri);
    } catch (err) {
      magnetError = errorMessage(err);
    }
  }

  /**
   * A safe default file name for a release's torrent.
   *
   * The release title carries slashes, colons and brackets that a file name
   * cannot hold, so everything outside a conservative set collapses to an
   * underscore. The result is only a default: the reader can rename it in the
   * save dialog.
   */
  function torrentFileName(release: Release): string {
    const safe = release.title.replace(/[^A-Za-z0-9._-]+/g, "_").slice(0, 120);
    return `${safe || "release"}.torrent`;
  }

  /**
   * Save a release's `.torrent` file to a path the reader picks.
   *
   * The fetch happens in the backend, because the webview cannot reach the
   * indexer directly (CORS). Not every feed carries the direct link, so a
   * release without a `torrentUrl` has nothing to download; the button is
   * disabled in that case rather than failing on click.
   */
  async function downloadReleaseTorrent(release: Release): Promise<void> {
    const url = release.torrentUrl;
    if (url === undefined || downloadingTorrent) return;

    const path = await save({
      defaultPath: torrentFileName(release),
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
    });

    // A cancelled save dialog resolves to null, which is not an error.
    if (path === null) return;

    downloadError = null;
    downloadingTorrent = true;
    try {
      await downloadTorrent(url, path);
    } catch (err) {
      downloadError = errorMessage(err);
    } finally {
      downloadingTorrent = false;
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
      const generation = ++addGeneration;
      const handle = await addTorrent(picked);
      if (!adopt(handle, generation)) return;

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

      <StreamStatus
        progress={torrentProgress}
        {fileFraction}
        {torrentFraction}
        empty={streamUrl === undefined}
      />

      <div class="mt-3 flex flex-wrap items-center gap-2">
        <button
          type="button"
          onclick={loadTorrent}
          disabled={loadingTorrent}
          class="rounded-lg border border-border-subtle px-4 py-2 text-sm font-medium text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
        >
          {loadingTorrent
            ? "Loading…"
            : files.length
              ? "Change torrent"
              : "Load torrent"}
        </button>

        <ExternalPlayerButton url={streamUrl} torrentId={torrentId ?? undefined} />
      </div>

      {#if torrentError}
        <p class="mt-2 text-sm text-ink-faint">{torrentError}</p>
      {/if}

      <!-- The releases the app found on its own. The reader no longer has to
           supply a torrent for the common case: picking a row resolves its
           magnet and plays the file matching the selected episode. -->

      {#if files.length > 0}
        <!-- The files the torrent actually holds, since the reader owns the
             copy and its naming may not match AniList's episode list. -->
        <div class="mt-4">
          <h2 class="mb-2 text-sm font-semibold tracking-tight">Files</h2>
          <div
            data-testid="file-scroller"
            class="max-h-55 overflow-y-auto pr-1"
          >
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
        </div>
      {/if}

      <div class="mt-6">
        <div class="mb-2 flex items-center justify-between gap-2">
          <h2 class="text-sm font-semibold tracking-tight">Releases</h2>
          <ReleaseModeToggle mode={releaseMode} onChange={(m) => (releaseMode = m)} />
        </div>

        {#if searching}
          <p class="text-sm text-ink-muted" data-testid="releases-loading">
            Searching…
          </p>
        {:else if releaseError}
          <p class="text-sm text-ink-faint">{releaseError}</p>
        {:else if releases.length === 0}
          <p class="text-sm text-ink-muted" data-testid="releases-empty">
            {releaseMode === "packs"
              ? "No packs found. Load a torrent by hand instead."
              : "No releases found. Load a torrent by hand instead."}
          </p>
        {:else}
          <!-- The box sits outside ResolutionFilter's own guard, which hides
                 the chips when there is only one resolution to choose from. A
                 text filter is still useful in that case. -->
          <div class="mb-2 flex flex-wrap items-center gap-2">
            <input
              type="search"
              bind:value={releaseQuery}
              placeholder="Filter releases…"
              aria-label="Filter releases"
              data-testid="release-filter"
              class="min-w-0 flex-1 rounded-lg border border-border-subtle bg-surface-raised px-3 py-1 text-xs text-ink placeholder:text-ink-faint focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
            <ResolutionFilter
              available={resolutionOptions}
              selected={resolutionFilter}
              onToggle={toggleResolution}
            />
          </div>

          {#if queryMatches === 0}
            <p class="text-sm text-ink-muted" data-testid="releases-no-match">
              No releases match "{releaseQuery.trim()}".
            </p>
          {:else}
            <div
              data-testid="release-scroller"
              class="max-h-60 overflow-y-auto pr-1 m-2"
            >
              <ul class="flex flex-col gap-1" data-testid="releases">
                {#each rankedReleases as { release, index } (release.infoHash ?? release.title)}
                  {@const clarity = matchClarity(release)}
                  <li
                    class="flex items-start gap-2 rounded-lg border px-3 py-2 mb-0.5 text-xs transition-colors {clarityClass(
                      clarity,
                    )} {chosenRelease?.title === release.title
                      ? 'border-accent bg-surface-hover text-ink'
                      : 'border-border-subtle text-ink-muted hover:border-accent hover:text-ink'}"
                    data-clarity={clarity}
                    title={clarityTitle(clarity)}
                  >
                    <!-- A dot rather than a word: the badge is a glanceable signal
                       beside a row already dense with text, and the explanation
                       lives in the title attribute. -->
                    <span
                      class="mt-1.5 size-2 shrink-0 rounded-full {badgeClass(
                        badgeFor(index),
                      )}"
                      data-testid="release-badge"
                      data-badge={badgeFor(index) ?? "pending"}
                      title={badgeTitle(index)}
                      aria-hidden="true"
                    ></span>
                    <button
                      type="button"
                      onclick={() => playRelease(release)}
                      disabled={loadingRelease}
                      data-testid="release-play"
                      class="min-w-0 flex-1 text-left disabled:cursor-not-allowed disabled:opacity-60"
                    >
                      <span class="block truncate">{release.title}</span>
                      {#if clarityLabel(clarity)}
                        <!-- A word as well as the border, so the emphasis does
                           not rely on colour alone. -->
                        <span
                          class="mt-0.5 inline-block rounded px-1 text-[0.6rem] font-medium {clarity ===
                          'stated'
                            ? 'bg-accent/20 text-accent'
                            : 'bg-surface-hover text-ink-faint'}"
                        >
                          {clarityLabel(clarity)}
                        </span>
                      {/if}
                      <span class="mt-0.5 block text-ink-faint">
                        {#if release.resolution !== "unknown"}{release.resolution}{/if}
                        {#if release.source !== "unknown"}· {release.source}{/if}
                        {#if release.seeders !== undefined}· {release.seeders} seeders{/if}
                        {#if formatSize(release.sizeBytes)}· {formatSize(
                            release.sizeBytes,
                          )}{/if}
                      </span>
                    </button>
                    <!-- Hands the magnet to the OS rather than the app's own
                       engine, so the reader's torrent client does the
                       downloading. A sibling, not a child: buttons cannot
                       nest, and the two actions are independent. -->
                    <button
                      type="button"
                      onclick={() => openMagnet(release)}
                      data-testid="release-magnet"
                      aria-label="Open magnet for {release.title}"
                      title="Open in your torrent client"
                      class="shrink-0 rounded border border-border-subtle px-2 py-1 transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent flex items-center justify-center"
                    >
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 32 32"
                        fill="currentColor"
                        class="w-4 h-4 text-ink-muted hover:text-accent transition-colors"
                      >
                        <path
                          d="M30 1.25h-8c-0.414 0-0.75 0.336-0.75 0.75v0 14c0.003 0.067 0.005 0.145 0.005 0.224 0 2.779-2.253 5.031-5.031 5.031-0.079 0-0.157-0.002-0.235-0.005l0.011 0c-0.067 0.003-0.145 0.005-0.223 0.005-2.779 0-5.032-2.253-5.032-5.032 0-0.079 0.002-0.157 0.005-0.234l-0 0.011v-14c-0-0.414-0.336-0.75-0.75-0.75h-8c-0.414 0-0.75 0.336-0.75 0.75v0 14c-0.009 0.187-0.014 0.407-0.014 0.628 0 7.807 6.329 14.136 14.136 14.136 0.221 0 0.44-0.005 0.659-0.015l-0.031 0.001c0.187 0.009 0.407 0.014 0.627 0.014 7.808 0 14.137-6.329 14.137-14.137 0-0.221-0.005-0.44-0.015-0.658l0.001 0.031v-14c-0-0.414-0.336-0.75-0.75-0.75v0zM29.25 2.75v4.5l-6.5 0.002v-4.502zM9.25 2.75v4.499h-6.5v-4.499zM16 29.25c-0.168 0.008-0.365 0.012-0.563 0.012-7.014 0-12.699-5.686-12.699-12.699 0-0.198 0.005-0.395 0.014-0.591l-0.001 0.028v-7.251h6.5v7.251c-0.003 0.068-0.004 0.148-0.004 0.229 0 1.911 0.819 3.631 2.126 4.828l0.005 0.004c1.207 1.050 2.795 1.69 4.532 1.69 0.032 0 0.064-0 0.096-0.001l-0.005 0c0.065 0.002 0.141 0.004 0.217 0.004 3.61 0 6.536-2.926 6.536-6.536 0-0.076-0.001-0.152-0.004-0.228l0 0.011v-7.248l6.5-0.002v7.25c0.008 0.168 0.012 0.365 0.012 0.563 0 7.014-5.686 12.7-12.7 12.7-0.198 0-0.395-0.005-0.591-0.014l0.028 0.001z"
                        />
                      </svg>
                    </button>
                    <!-- Saves the `.torrent` file itself, for a reader who
                         wants the file rather than to hand it straight to a
                         client. Not every feed carries the direct link, so it
                         is disabled when there is nothing to fetch. -->
                    <button
                      type="button"
                      onclick={() => downloadReleaseTorrent(release)}
                      disabled={release.torrentUrl === undefined ||
                        downloadingTorrent}
                      data-testid="release-download"
                      aria-label="Download torrent for {release.title}"
                      title={release.torrentUrl === undefined
                        ? "No torrent file for this release"
                        : "Download the .torrent file"}
                      class="shrink-0 rounded border border-border-subtle px-2 py-1 transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent flex items-center justify-center disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:border-border-subtle disabled:hover:text-ink-muted"
                    >
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 24 24"
                        fill="none"
                        class="w-4 h-4 text-ink-muted transition-colors"
                      >
                        <path
                          stroke="currentColor"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M12 5v8.5m0 0l3-3m-3 3l-3-3M5 15v2a2 2 0 002 2h10a2 2 0 002-2v-2"
                        />
                      </svg>
                    </button>
                  </li>
                {/each}
              </ul>
            </div>
            {#if downloadError}
              <p class="mt-2 text-xs text-danger" role="status">
                Could not download the torrent. <span class="text-ink-muted"
                  >{downloadError}</span
                >
              </p>
            {/if}
            {#if magnetError}
              <p class="mt-2 text-xs text-danger" role="status">
                Could not open a torrent client. <span class="text-ink-muted"
                  >{magnetError}</span
                >
              </p>
            {/if}
          {/if}
        {/if}
      </div>

      {#if episodes.length > 0}
        <div class="mt-6">
          <EpisodeList
            {episodes}
            selected={selectedEpisode}
            onSelect={selectEpisode}
          />
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
