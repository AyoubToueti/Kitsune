// Episode lists that survive a sparse provider.
//
// The list comes from Jikan's per-episode catalogue, bridged by AniList's
// `idMal`. AniList's own `streamingEpisodes` is NOT used here: it is a list of
// licensed streaming links, not an episode list, and AniList attaches the whole
// franchise's links to the first season's entry -- so using it produced a
// season-1 list numbered 51..66 for Re:Zero. `StreamingLinks` still reads that
// field for its "where to watch" buttons, which is what it is for.
//
// When Jikan has nothing, this synthesises `1..episodeCount` so the reader can
// still reach every episode -- and, critically, so the watch page can resolve an
// episode number to search a torrent for.
//
// The number is the load-bearing part. Everything downstream (the torrent
// matcher, the file matcher) keys off "which episode is this", so every entry
// carries an explicit number rather than relying on the title being parsed.

import type { Anime, EpisodeInfo } from "./types";

/**
 * An episode as the UI renders it.
 *
 * Deliberately looser than [`StreamingEpisode`]: a synthesised entry has no
 * licensed URL to open, so `url` is optional and the card falls back to
 * selecting the episode rather than opening a link. A `StreamingEpisode` is
 * assignable to this, so the rich and synthesised lists share one renderer.
 */
export interface Episode {
  /** 1-based episode number, when known. */
  number?: number;
  title?: string;
  url?: string;
  site?: string;
  thumbnail?: string;
  /** Air date as the provider spells it, filled in by enrichment. */
  aired?: string;
  /** Anime-original filler, which a viewer may want to skip. */
  filler?: boolean;
  /** A recap of earlier events. */
  recap?: boolean;
}

/**
 * Build `1..count` synthetic episodes.
 *
 * `coverImage` is used as the card background so the grid reads as part of the
 * work rather than a row of blanks. A count that is absent or non-positive
 * yields an empty list: there is nothing honest to show, and the caller renders
 * an "unknown" message instead.
 */
export function synthesizeEpisodes(
  count: number | undefined,
  coverImage?: string,
): Episode[] {
  if (count === undefined || count <= 0) return [];

  return Array.from({ length: count }, (_, i) => {
    const number = i + 1;
    return {
      number,
      // Spelled the way `episodeNumber` reads back, so selection and torrent
      // matching work without a second numbering scheme.
      title: `Episode ${number}`,
      thumbnail: coverImage,
    };
  });
}

/**
 * Turn Jikan's per-episode catalogue into display entries.
 *
 * Every entry keeps its explicit `number`, so the torrent matcher never has to
 * recover it from the title. `coverImage` is used as the card background
 * because Jikan has no per-episode stills; the episode's own title is the real
 * one, and the fallback only fires for a genuinely untitled entry.
 */
export function episodesFromInfo(
  info: EpisodeInfo[],
  coverImage?: string,
): Episode[] {
  return info.map((entry) => ({
    number: entry.number,
    title: entry.title ?? `Episode ${entry.number}`,
    thumbnail: coverImage,
    aired: entry.aired,
    filler: entry.filler,
    recap: entry.recap,
  }));
}

/**
 * The episodes to show for a work.
 *
 * The catalogue wins when it has entries, because it is the real per-episode
 * data: correct numbering, real titles and air dates. Only when it is empty
 * does this fall back to a synthesised `1..episodeCount` stand-in, so a work
 * with no MAL link (or a MAL entry with no episode list) is still navigable.
 *
 * `info` defaults to empty so a caller that has not fetched it yet still gets
 * the fallback rather than a crash.
 */
export function episodesFor(
  anime: Anime,
  info: EpisodeInfo[] = [],
): Episode[] {
  if (info.length > 0) {
    return episodesFromInfo(info, anime.coverImage);
  }
  return synthesizeEpisodes(anime.episodeCount, anime.coverImage);
}

/**
 * Whether a work has any episode information at all.
 *
 * False means neither a provider list nor a count, so the caller should say so
 * rather than render an empty grid that looks broken.
 */
export function hasEpisodeData(anime: Anime): boolean {
  return (
    (anime.streamingEpisodes ?? []).length > 0 || (anime.episodeCount ?? 0) > 0
  );
}

