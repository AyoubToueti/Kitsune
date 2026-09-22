// Episode numbering, shared by everything that has to line a provider's
// episode list up with a release's file names.
//
// AniList sends no numeric field for an episode, so the number usually has to be
// read out of the title or the URL. A synthesised entry, by contrast, carries an
// explicit `number` (see `episodes.ts`). Both forms are accepted here so the
// detail page, the watch page and the episode list cannot disagree about which
// entry is "episode 3".

import type { Episode } from "./episodes";
import type { TorrentFile } from "./types";

/**
 * The episode number an entry represents, when one can be told.
 *
 * An explicit `number` wins outright: a synthesised entry knows its own number
 * and re-deriving it from the title would be both redundant and fragile.
 *
 * Otherwise the title is tried before the URL because it is written for humans
 * and so carries the number most reliably; the URL patterns catch entries whose
 * title is a site name or blank.
 */
export function episodeNumber(ep: Episode): number | undefined {
  if (ep.number !== undefined) return ep.number;

  const patterns: RegExp[] = [
    /(?:episode|ep)\.?\s*[-–:]?\s*(\d+)/i, // "Episode 12", "Ep. 12"
    /(?:episode|ep)[-_](\d+)/i, // "episode-12" inside a URL
    /(\d+)\s*$/, // a bare trailing number
  ];

  for (const source of [ep.title, ep.url]) {
    if (source == null) continue;
    for (const pattern of patterns) {
      const match = pattern.exec(source);
      if (match) return Number(match[1]);
    }
  }
  return undefined;
}

/**
 * Whether a title already announces its own episode number.
 *
 * Some titles carry the number themselves: a padded catalogue entry is titled
 * `Episode 7`, and a provider may spell one `Ep. 7 - The Name`. Prefixing those
 * again would read "Episode 7 - Episode 7", so the caller leaves them as they
 * are.
 *
 * Anchored at the start of the string on purpose. A number appearing later is
 * part of a name ("The Journey's End 2") rather than an announcement, and
 * matching it would suppress the prefix on an entry that needs one. A URL is
 * also unaffected, because it starts with its scheme rather than a marker.
 *
 * `E07` is accepted as well as `Episode 7` and `Ep. 7`, since some sources
 * spell it that way. The `e` alternative cannot swallow "Episode": after the
 * `e` comes `p`, not a digit.
 */
export function titleHasEpisodeNumber(title: string): boolean {
  return /^\s*(?:ep(?:isode)?\.?\s*|e)\d+/i.test(title);
}

/** Index of the entry for `number`, or -1 when nothing matches. */
export function indexOfEpisode(episodes: Episode[], number: number): number {
  return episodes.findIndex((ep) => episodeNumber(ep) === number);
}

/**
 * How much to add to a list position to get the work's absolute episode number.
 *
 * A later cour is often numbered 13..24 while its releases are named
 * `Show - 13`; a torrent indexer will not find `Show - 01` for it. The offset
 * bridges the two: it is the lowest number in the list minus one, but only when
 * that lowest number is greater than the list length.
 *
 * That guard is what keeps an ordinary 1..N season at zero. A season numbered
 * 13..24 is twelve entries whose lowest number (13) exceeds the length (12), so
 * the offset is 12: position 0 becomes 13, matching the releases. A season
 * numbered 1..12 has a lowest number (1) below the length, so the offset is 0
 * and nothing changes.
 *
 * The lowest number is read from the entries, not assumed to be the first one,
 * so a catalogue that starts mid-season is still handled. Entries with no
 * recoverable number are ignored; a list where none of them has one yields 0,
 * because there is nothing to offset from.
 */
export function absoluteOffset(episodes: Episode[]): number {
  const numbers = episodes
    .map((ep) => episodeNumber(ep))
    .filter((n): n is number => n !== undefined);

  if (numbers.length === 0) return 0;

  const lowest = Math.min(...numbers);
  return lowest > episodes.length ? lowest - 1 : 0;
}

/**
 * The file in a torrent that best matches an episode number.
 *
 * The number is matched as a delimited token so "03" does not match "1080p" or
 * "2023". A release whose naming does not line up returns `null` and the reader
 * picks by hand, which is the honest outcome.
 */
export function fileForEpisode(
  files: TorrentFile[],
  number: number,
): TorrentFile | null {
  // `0*` lets the same pattern match a zero-padded number, so "03" and "3"
  // both resolve without a looser second pass. A substring fallback was tried
  // and dropped: it made episode 80 match the "80" inside "1080p".
  const pattern = new RegExp(`(?:^|[^0-9])0*${number}(?:[^0-9]|$)`);

  return files.find((file) => pattern.test(file.name)) ?? null;
}