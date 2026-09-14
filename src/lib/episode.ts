// Episode numbering, shared by everything that has to line a provider's
// episode list up with a release's file names.
//
// AniList sends no numeric field for an episode, so the number has to be read
// out of the title or the URL. The heuristic lives here rather than in each
// caller so the detail page, the watch page and the episode list cannot
// disagree about which entry is "episode 3".

import type { StreamingEpisode, TorrentFile } from "./types";

/**
 * The episode number an entry represents, when one can be told.
 *
 * The title is tried before the URL because it is written for humans and so
 * carries the number most reliably; the URL patterns catch entries whose
 * title is a site name or blank.
 */
export function episodeNumber(ep: StreamingEpisode): number | undefined {
  const patterns: RegExp[] = [
    /(?:episode|ep)\.?\s*[-–:]?\s*(\d+)/i, // "Episode 12", "Ep. 12"
    /(?:episode|ep)[-_](\d+)/i, // "episode-12" inside a URL
    /(\d+)\s*$/, // a bare trailing number
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

/** Index of the entry for `number`, or -1 when nothing matches. */
export function indexOfEpisode(
  episodes: StreamingEpisode[],
  number: number,
): number {
  return episodes.findIndex((ep) => episodeNumber(ep) === number);
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