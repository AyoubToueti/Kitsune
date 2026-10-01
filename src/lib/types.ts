// Mirrors of the Rust types in `src-tauri/src/types.rs`.
//
// The Rust side serialises to camelCase, so these match the wire format
// exactly rather than the snake_case field names used internally.

/** Which upstream a given id belongs to. */
export type ProviderId = "anilist" | "tmdb" | "nyaa" | "yts";

/** A work's title in the several forms providers offer. */
export interface Title {
  romaji?: string;
  english?: string;
  native?: string;
  userPreferred?: string;
}

/** A link to an official streaming source. `url` is always present. */
export interface StreamingEpisode {
  title?: string;
  url: string;
  site?: string;
  thumbnail?: string;
}

/**
 * Metadata for a single episode, as a provider reports it.
 *
 * Mirrors the Rust `EpisodeInfo`. Distinct from a synthesised `Episode`: this is
 * catalogue data (title, air date, filler flag) rather than something playable.
 */
export interface EpisodeInfo {
  number: number;
  title?: string;
  /** The provider's own date string, formatted for display by the caller. */
  aired?: string;
  filler: boolean;
  recap: boolean;
}
/**
 * Which curated list to fetch.
 *
 * Mirrors the Rust `ListFilter`. The values are the wire format, so they are
 * passed straight to the backend rather than translated here.
 */
export type ListFilter =
  | "trending"
  | "topAiring"
  | "mostPopular"
  | "topRated"
  | "latestCompleted"
  | "upcoming";

/**
 * A descriptor the provider associates with a work.
 *
 * Distinct from a genre: `category` is what a filter UI groups by, so the two
 * travel together rather than as a bare name.
 */
export interface MediaTag {
  name: string;
  category: string;
  /**
   * The provider's own prose for what the tag means.
   *
   * Optional: the provider may have no description for a tag, and the chip
   * simply renders without a tooltip rather than with an empty one.
   */
  description?: string;
}

/** A work as presented in the UI. */
export interface Anime {
  id: number;
  provider: ProviderId;
      /** The work's MyAnimeList id, when the provider knows it. Used to fetch full episode lists from Jikan. */
      idMal?: number;
  title: Title;
  coverImage?: string;
  bannerImage?: string;
  description?: string;
  episodeCount?: number;
  durationMinutes?: number;
  format?: string;
  genres: string[];
  averageScore?: number;
  popularity?: number;
  status?: string;
  /** Release season in the provider's wording, e.g. "FALL". */
  season?: string;
  seasonYear?: number;
  streamingEpisodes: StreamingEpisode[];
  /**
   * Other works this one is connected to, as the provider links them.
   *
   * Populated only by the single-title lookup; list results leave it empty,
   * since a card has nowhere to render it.
   */
  relations: RelatedAnime[];
  /** Community recommendations, highest-rated first. Detail-only. */
  recommendations: RecommendedAnime[];
  /** The work's trailer, when the provider has one. Detail-only. */
  trailer?: Trailer;
}

/**
 * A work connected to another, as the provider links them.
 *
 * `relationType` is the provider's own vocabulary ("SEQUEL", "PREQUEL",
 * "SIDE_STORY", ...) and is surfaced verbatim: the set is open-ended, so
 * translating it into our own union would drop anything new.
 */
export interface RelatedAnime {
  id: number;
  title: Title;
  coverImage?: string;
  format?: string;
  status?: string;
  episodeCount?: number;
  /** How this work relates to the one being viewed, e.g. "SEQUEL". */
  relationType: string;
}

/** A community recommendation for a work. */
export interface RecommendedAnime {
  /** The recommended work, carried whole so a card renders without a lookup. */
  anime: Anime;
  /** Upvotes the recommendation received on the provider. */
  rating: number;
}

/**
 * A promotional video for a work.
 *
 * `site` and `id` travel apart because the provider reports them separately
 * and each platform spells its watch URL differently; composing the link is
 * the UI's job.
 */
export interface Trailer {
  /** The video id on `site`, e.g. a YouTube video id. */
  id: string;
  /** Hosting platform, e.g. "youtube" or "dailymotion". */
  site: string;
  thumbnail?: string;
}

/**
 * Where a page of results sits in the whole set.
 *
 * Mirrors the Rust `PageInfo`. `lastPage` and `hasNextPage` are what let the
 * UI render pagination at all.
 */
export interface PageInfo {
  /**
   * Total matches. AniList caps this at 5000, so for a very broad query it is
   * a floor rather than an exact count.
   */
  total: number;
  currentPage: number;
  lastPage: number;
  hasNextPage: boolean;
}

/** One page of results, with the metadata needed to ask for another. */
export interface AnimePage {
  items: Anime[];
  pageInfo: PageInfo;
}

/** How to order results. Mirrors the Rust `SortOption`. */
export type SortOption =
  | "popularity"
  | "score"
  | "newest"
  | "titleAz"
  | "trending"
  | "favorites"
  /**
   * Most recently added to the provider's catalogue, which is not the same as
   * most recently aired -- that is `newest`.
   */
  | "dateAdded"
  /**
   * Closest textual match. Only meaningful alongside a search term; without
   * one the provider falls back to its default order.
   */
  | "searchMatch";

/** Which release status to keep. */
export type StatusFilter = "releasing" | "finished" | "notYetReleased";

/**
 * Where a work sits on the reader's own AniList list.
 *
 * Distinct from `StatusFilter`: that one is the WORK's release state ("is it
 * still airing"), this is the READER's relationship to it ("am I watching it").
 * A finished show can be `current` on someone's list, so the two are not
 * interchangeable.
 *
 * Mirrors the Rust `ListStatus`. The values are the wire format the backend
 * accepts, so they pass straight through rather than being translated here.
 */
export type ListStatus =
  | "current"
  | "planning"
  | "completed"
  | "dropped"
  | "paused";

/** The reader's own entry for a work, when it has one. */
export interface ListEntry {
  status: ListStatus;
  /** Episodes watched so far. */
  progress: number;
}

/**
 * One entry in the reader's "Continue Watching" row.
 *
 * Carries the work AND how far the reader got, because the row's resume button
 * needs the episode number -- which is not part of the catalogue `Anime` shape.
 */
export interface ContinueWatchingItem {
  anime: Anime;
  /** Episodes watched so far, `0` for one never started. */
  progress: number;
}

/**
 * The work the reader most recently opened, kept locally.
 *
 * Distinct from the AniList list: `updatedAt` there records the last CHANGE, so
 * re-watching the episode you are already on does not move you up. "What did I
 * last open" is a different question, and this is the local answer to it.
 */
export interface LastPlayed {
  animeId: number;
  /** The episode number, when one was known. Absent for a film or a play with no selection. */
  episode?: number;
  /** Unix seconds when it was recorded. */
  at: number;
}

/**
 * One entry in the reader's own list, for the My List page.
 *
 * Carries the status so the page can group by it, and the list-entry id so the
 * row's menu can remove it. `entryId` is the LIST ENTRY's id, not the media id.
 */
export interface UserListEntry {
  anime: Anime;
  /** Which list the work is on. */
  status: ListStatus;
  /** Episodes watched so far, `0` for one never started. */
  progress: number;
  /** The id of the list entry itself, needed to delete it. */
  entryId: number;
}

/** Which release format to keep. */
export type FormatFilter = "tv" | "movie" | "ova" | "ona" | "special" | "music";

/** Which release season to keep. */
export type SeasonFilter = "winter" | "spring" | "summer" | "fall";

/**
 * A browse request.
 *
 * Every filter is optional, so the UI sends only what the user set. `sort` is
 * required because there is always an order, even when nothing is filtered.
 */
export interface BrowseQuery {
  search?: string;
  genres?: string[];
  /**
   * Tags to require, by name. Combined with AND, like `genres`.
   *
   * These are the provider's tag namespace, not its genres. Only names that
   * came from `getTags()` may be sent: an unknown tag matches nothing rather
   * than erroring, so a typo would look like an empty result set.
   */
  tags?: string[];
  /**
   * Tags to reject, by name. Combined with AND, like `tags`.
   *
   * The provider's rank floor governs this direction too, so excluding a tag
   * only drops works carrying it at or above that rank. Only names that came
   * from `getTags()` may be sent, for the same reason as `tags`.
   */
  excludedTags?: string[];
  format?: FormatFilter;
  status?: StatusFilter;
  season?: SeasonFilter;
  seasonYear?: number;
  /** Minimum average score on the provider's own scale (AniList: 0-100). */
  minScore?: number;
  sort: SortOption;
  /**
   * Reverse the sort's natural direction.
   *
   * A boolean rather than `asc`/`desc`, because the natural direction differs
   * per sort -- score is highest-first, title is A-Z -- so "ascending" would
   * need a per-sort default table to mean anything. This says only "the
   * opposite of what this sort normally does". Absent means natural.
   */
  reversed?: boolean;
}

/**
 * One upcoming broadcast, as listed on a schedule.
 *
 * Distinct from an episode of a release: this is the provider announcing that
 * a work airs at a given time, not something this app can play.
 */
export interface ScheduledEpisode {
  /** The work being aired, carried whole so a card needs no second lookup. */
  anime: Anime;
  /**
   * When it airs, as a unix timestamp in seconds.
   *
   * An absolute instant, not a formatted string: the backend has no idea what
   * timezone the viewer is in, so local formatting happens in the UI.
   */
  airingAt: number;
  /** Episode number being aired, when the provider states one. */
  episode?: number;
}

/**
 * One file inside a torrent.
 *
 * Mirrors the Rust `TorrentFile`. `idx` is what the stream URL is built from,
 * so it is the value the UI passes back rather than the array position.
 */
export interface TorrentFile {
  idx: number;
  name: string;
  lengthBytes: number;
}

/**
 * A torrent that has been added, with the files it resolved.
 *
 * Mirrors the Rust `TorrentHandle`. `id` and a file's `idx` together identify
 * one stream URL.
 */
export interface TorrentHandle {
  id: number;
  files: TorrentFile[];
}

/**
 * A download-progress snapshot for one torrent.
 *
 * Mirrors the Rust `TorrentProgress`. `fileProgress` is indexed by the same
 * `idx` used in a stream URL, so the chosen episode's own progress can be read
 * out of it. The live-only fields (`downloadMbps`, peers, `etaSeconds`) are
 * zero/`null` while the torrent is not running, so the panel never has to
 * special-case a missing section.
 */
export interface TorrentProgress {
  /** `"initializing"`, `"live"`, `"paused"` or `"error"`. */
  state: string;
  progressBytes: number;
  totalBytes: number;
  /** Bytes downloaded per file, indexed by the stream URL's `idx`. */
  fileProgress: number[];
  finished: boolean;
  /** The torrent's failure message, when `state` is `"error"`. */
  error: string | null;
  /** Download rate in MiB/s. */
  downloadMbps: number;
  /** Upload rate in MiB/s. */
  uploadMbps: number;
  /** Estimated seconds until completion, when it can be derived. */
  etaSeconds: number | null;
  peersLive: number;
  peersConnecting: number;
  peersQueued: number;
  peersSeen: number;
}

/**
 * Vertical resolution a release is encoded at.
 *
 * Mirrors the Rust `Resolution`. `"unknown"` is the absence of a claim, not a
 * low quality, so it must never be treated as `"360p"`.
 */
export type Resolution =
  | "unknown"
  | "360p"
  | "480p"
  | "540p"
  | "576p"
  | "720p"
  | "1080p"
  | "2160p";

/**
 * Where a release's video came from.
 *
 * Mirrors the Rust `ReleaseSource`. Ordered roughly worst-to-best for anime,
 * though the backend already encodes that ordering and the UI only displays it.
 */
export type ReleaseSource =
  | "unknown"
  | "sdtv"
  | "tvrip"
  | "dsr"
  | "pdtv"
  | "dvd"
  | "hdtv"
  | "webrip"
  | "webdl"
  | "bdrip"
  | "brrip"
  | "bluray"
  | "rawhd";

/**
 * What the backend's release-name parser learned about a name.
 *
 * Mirrors the Rust `ParsedRelease`. Carried so the UI can show the episode a
 * release claims rather than re-deriving it from the title.
 */
export interface ParsedRelease {
  title?: string;
  season?: number;
  episode?: number;
  /** Episode number counting from the work's first episode. */
  absoluteEpisode?: number;
  /** Fansub or release group, e.g. "SubsPlease". */
  subgroup?: string;
}

/**
 * A candidate release found by an indexer, ready to play.
 *
 * Mirrors the Rust `Release`. The backend has already parsed the name, judged
 * the quality and ranked the set, so the UI renders `title` and `score`
 * without re-deriving anything.
 */
export interface Release {
  /** The release name as the indexer spelled it, never rewritten. */
  title: string;
  indexer: ProviderId;
  /** Full magnet URI, validated by the backend. */
  magnetUri: string;
  /** Direct `.torrent` download URL, when the indexer exposed one. */
  torrentUrl?: string;
  infoHash?: string;
  sizeBytes?: number;
  seeders?: number;
  leechers?: number;
  resolution: Resolution;
  source: ReleaseSource;
  /** True when the release is a remux of another, which is a better copy. */
  remux: boolean;
  /** Whether the indexer marks the uploader as trusted. */
  trusted: boolean;
  parsed: ParsedRelease;
  /** The ranker's verdict: higher is a better pick for the request. */
  score: number;
}

/**
 * What the viewer prefers when several releases are equally correct.
 *
 * Mirrors the Rust `ReleasePreference`. Every field is optional; the backend
 * fills in a default (1080p then 720p, no seeder floor) when none is sent.
 */
export interface ReleasePreference {
  /** Resolutions to prefer, in order. The first match wins. */
  preferredResolutions?: Resolution[];
  /** Releases below this many seeders are dropped when better ones exist. */
  minSeeders?: number;
}

/**
 * The reader's stored preferences.
 *
 * Mirrors the Rust `Settings` exactly: the backend writes the whole object and
 * the frontend reads the whole object, so there is no partial-update surface.
 */
export interface Settings {
  /** The external player binary, e.g. `mpv` or `flatpak`. */
  player: string;
  /**
   * Extra arguments passed before the stream URL, e.g. `["run",
   * "io.mpv.Mpv"]`. Each element is one argv entry; nothing is shell-parsed.
   */
  playerArgs: string[];
  /** Download directory, or `null` for the default temp directory. */
  downloadDir: string | null;
  /** Resolutions to prefer, in order. */
  preferredResolutions: Resolution[];
  /** Releases below this many seeders are dropped when better ones exist. */
  minSeeders: number;
  /** Fraction of the file to buffer before launching the player, 0..1. */
  readyFraction: number;
  /** `"light"`, `"dark"` or `"system"`. */
  theme: string;
}

/**
 * A coarse health verdict for a release, from one probe.
 *
 * Mirrors the Rust `HealthBadge`. Three levels rather than a number because the
 * evidence behind them is coarse: confirmed alive, some sign of life, nothing.
 */
export type HealthBadge = "green" | "yellow" | "red";

/**
 * What one tracker reported about a release, live.
 *
 * Mirrors the Rust `TrackerScrape`. These are the tracker's own numbers, asked
 * for directly, rather than the seeder count an indexer's web page displayed
 * when the feed was generated.
 */
export interface TrackerScrape {
  seeders: number;
  leechers: number;
  /** Times the torrent was completed, not a current peer count. */
  completed: number;
  /** The tracker that answered, since a count is only as good as its source. */
  trackerUrl: string;
  durationMs: number;
}

/**
 * What the metadata probe learned about a release.
 *
 * Mirrors the Rust `MetadataProbe`. `resolved` is the headline: a magnet whose
 * info dictionary a peer actually served is a torrent that exists, which is a
 * stronger statement than any seeder count.
 */
export interface MetadataProbe {
  resolved: boolean;
  fileCount?: number;
  totalBytes?: number;
  durationMs: number;
}

/** The outcome of probing one release. Mirrors the Rust `ProbeResult`. */
export interface ProbeResult {
  infoHash: string;
  /** The best tracker answer, absent when no tracker replied. */
  scrape?: TrackerScrape;
  /** Absent only when the probe was skipped entirely. */
  metadata?: MetadataProbe;
  totalDurationMs: number;
}

/**
 * One probe's effect on one release, as delivered by a `probe-result` event.
 *
 * Mirrors the Rust `ProbeOutcome`. `index` is the release's position in the
 * list that was sent to `probe_releases`, which is how an event is matched back
 * to the release it describes.
 */
export interface ProbeOutcome {
  index: number;
  probe: ProbeResult;
  badge: HealthBadge;
  /** The release's static score plus the probe's contribution. */
  combinedScore: number;
}

/**
 * Best available title, following the same preference order as the Rust
 * `Title::display`: user's own choice, then English, then romaji, then the
 * native form.
 *
 * Returns `undefined` only when every form is missing or blank, so callers
 * can decide how to render a genuinely untitled entry.
 */
export function displayTitle(title: Title): string | undefined {
  const candidates = [title.userPreferred, title.english, title.romaji, title.native];
  return candidates.find((c) => c != null && c.trim() !== "");
}

/**
 * Every title form worth searching an indexer for, best first.
 *
 * A work has an English title and a romaji one, and an uploader may have used
 * either, so both are searched. The native form is deliberately excluded: the
 * Nyaa category queried is "Anime / English-translated", where a Japanese title
 * essentially never appears, so searching it would only cost a request.
 *
 * Duplicates are collapsed case-insensitively and blank forms are dropped.
 * Returns an empty array only when every form is missing or blank.
 */
export function titleForms(title: Title): string[] {
  const candidates = [title.userPreferred, title.english, title.romaji];
  const seen = new Set<string>();
  const forms: string[] = [];

  for (const candidate of candidates) {
    const trimmed = candidate?.trim();
    if (!trimmed) continue;

    const key = trimmed.toLowerCase();
    if (seen.has(key)) continue;

    seen.add(key);
    forms.push(trimmed);
  }

  return forms;
}