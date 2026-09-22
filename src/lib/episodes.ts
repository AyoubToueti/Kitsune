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
 * Whether a catalogue entry has already aired.
 *
 * A missing or unparseable date counts as aired. The field is a bonus, and
 * withholding an episode over a malformed date would silently drop data the
 * provider did give us; only a date that parses *and* lies in the future is
 * held back. Airing schedules are also approximate, so a same-day episode is
 * shown rather than hidden for a few hours.
 *
 * `now` is injectable so a test can pin the clock without touching global
 * timer state.
 */
export function isAired(
  aired: string | undefined,
  now: Date = new Date(),
): boolean {
  if (aired == null || aired.trim() === "") return true;

  const at = Date.parse(aired);
  if (Number.isNaN(at)) return true;

  return at <= now.getTime();
}

/**
 * The catalogue entries that have aired, in catalogue order.
 *
 * An unaired episode is filtered out of the list rather than shown disabled:
 * the reader cannot watch it, and a row that does nothing when clicked reads
 * as broken. The count of what was withheld is surfaced separately so the list
 * can say "N of M aired".
 */
export function airedEpisodes(
  info: EpisodeInfo[],
  now: Date = new Date(),
): EpisodeInfo[] {
  return info.filter((entry) => isAired(entry.aired, now));
}

/**
 * The episodes to show for a work.
 *
 * The catalogue wins when it has entries, because it is the real per-episode
 * data: correct numbering, real titles and air dates. Only when it is empty
 * does this fall back to a synthesised `1..episodeCount` stand-in, so a work
 * with no MAL link (or a MAL entry with no episode list) is still navigable.
 *
 * Entries that have not aired yet are dropped. A catalogue that is entirely
 * unaired therefore yields an empty list, which is the honest answer -- it must
 * not fall back to synthesis, because that would invent viewable episodes.
 *
 * A catalogue that is merely *short* is different, and is padded up to the
 * announced count by [`withMissingEpisodes`]: the episodes exist, the provider
 * simply has not published their titles yet.
 *
 * `info` defaults to empty so a caller that has not fetched it yet still gets
 * the fallback rather than a crash. `now` defaults to the current time and is
 * injectable for tests.
 */
export function episodesFor(
  anime: Anime,
  info: EpisodeInfo[] = [],
  now: Date = new Date(),
): Episode[] {
  if (info.length > 0) {
    const known = episodesFromInfo(airedEpisodes(info, now), anime.coverImage);

    // Only a finished work is padded. Every episode of a finished work has
    // aired by definition, so a missing entry is one the provider has not
    // titled yet. On a releasing work the same gap could equally be episodes
    // that have not aired, and adding them would present unwatchable episodes
    // as ready -- so the short list stands and the "N of M aired" label
    // explains it.
    return anime.status === "FINISHED"
      ? withMissingEpisodes(known, anime.episodeCount, anime.coverImage)
      : known;
  }
  return synthesizeEpisodes(anime.episodeCount, anime.coverImage);
}

/**
 * Fill the gaps between a catalogue and the count the provider announced.
 *
 * MyAnimeList is slow to publish a per-episode list for a recent work: it can
 * report a title as having twelve episodes while Jikan's catalogue holds one,
 * which left the grid showing a single card and the reader unable to reach the
 * rest. The announced count is the better authority on *how many* episodes
 * exist, so the missing numbers are added as bare entries.
 *
 * The caller must establish that the missing episodes have aired -- see the
 * `FINISHED` gate in [`episodesFor`]. This function cannot tell an untitled
 * episode from an unaired one, and would happily invent the latter.
 *
 * A bare entry is deliberately generic: it carries a number and nothing else,
 * because nothing else is known. Its title reads `Episode 7`, not a real name,
 * and it has no air date -- so it is never mistaken for catalogue data. What it
 * buys is reachability: the reader can select it, and the watch page can search
 * for it.
 *
 * Only a 1-based list is filled. A later cour is numbered 13..24 while its own
 * list runs 1..12, and padding that to the franchise total would invent the
 * previous cour's episodes. Requiring the lowest number to be 1 keeps the two
 * cases apart.
 *
 * Returns the catalogue untouched when the count is unknown, when the catalogue
 * already covers it, or when the numbering does not look 1-based.
 */
function withMissingEpisodes(
  known: Episode[],
  count: number | undefined,
  coverImage?: string,
): Episode[] {
  if (count === undefined || count <= 0) return known;

  const numbers = known
    .map((episode) => episode.number)
    .filter((number): number is number => number !== undefined);

  // Without numbers there is nothing to compare against, and a guess would put
  // entries in the wrong slots.
  if (numbers.length === 0) return known;

  const lowest = Math.min(...numbers);
  const highest = Math.max(...numbers);

  // `lowest !== 1` means this is not a 1-based cour; `highest > count` means the
  // catalogue is already numbered beyond what was announced.
  if (lowest !== 1 || highest > count) return known;
  if (known.length >= count) return known;

  const present = new Set(numbers);
  const filled = known.slice();

  for (let number = 1; number <= count; number += 1) {
    if (present.has(number)) continue;
    filled.push({
      number,
      title: `Episode ${number}`,
      thumbnail: coverImage,
    });
  }

  return filled.sort((a, b) => (a.number ?? 0) - (b.number ?? 0));
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

