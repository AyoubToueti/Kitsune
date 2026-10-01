// Matching a torrent's files against an episode number.
//
// A torrent ships more than the episode: subtitles, cover art, sample clips and
// sometimes a whole batch. The reader selects an episode and expects the right
// video, so this is the rule that turns "episode 7" into one file — or into
// nothing, when the naming genuinely does not line up and the reader must pick
// by hand.
//
// Pure, so the fiddly token-matching is testable without a torrent or a page.
// Shared by the watch page and the episode modal so the two cannot disagree
// about which file an episode is.

import type { TorrentFile } from "./types";

/**
 * The file whose name carries `number` as a delimited token.
 *
 * Split out so the relative and absolute spellings are matched by one rule
 * rather than two that could drift apart. See [`fileForEpisode`].
 */
export function matchNumber(
  files: TorrentFile[],
  number: number,
): TorrentFile | null {
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

/**
 * The file that best matches an episode number, or `null` when nothing does.
 *
 * A release name carries the episode as ` - 03 ` or `E03`, so the number is
 * searched for as a delimited token rather than a bare substring: "03" must
 * not match "1080p". A miss returns nothing and the reader picks by hand,
 * which is the honest outcome when the naming does not line up.
 *
 * `offset` bridges a later cour's numbering: its releases are named with the
 * running total (`Show - 13`) even though its own list starts at 1. It is 0 for
 * an ordinary 1..N season, in which case the absolute attempt would repeat the
 * relative one and is skipped.
 */
export function fileForEpisode(
  files: TorrentFile[],
  number: number,
  offset: number,
): TorrentFile | null {
  const direct = matchNumber(files, number);
  if (direct !== null) return direct;

  const absolute = number + offset;
  if (absolute === number) return null;

  return matchNumber(files, absolute);
}

/** Whether a file looks like something a video player can open. */
export function isPlayable(file: TorrentFile): boolean {
  return /\.(mkv|mp4|avi|webm|mov|m4v)$/i.test(file.name);
}

/** What a file is, for the type icon and label in a file row. */
export type FileKind = "video" | "subtitle" | "image" | "other";

/**
 * Classify a file by its extension.
 *
 * Used to give each row in the file list an icon and a human label, so a
 * torrent that ships subtitles and artwork beside the episode reads clearly
 * rather than as a wall of names. Deliberately coarse: the four buckets are
 * what the row renders, and anything unrecognised is honestly "other".
 */
export function fileKind(file: TorrentFile): FileKind {
  const name = file.name.toLowerCase();
  if (/\.(mkv|mp4|avi|webm|mov|m4v|ts|m2ts)$/.test(name)) return "video";
  if (/\.(ass|srt|ssa|vtt|sub)$/.test(name)) return "subtitle";
  if (/\.(jpg|jpeg|png|gif|webp|bmp)$/.test(name)) return "image";
  return "other";
}

/**
 * The file to start with when nothing matched the episode.
 *
 * The largest playable file, since a full-length episode dwarfs the sample
 * clips and cover art some releases ship beside it. Falls back to the largest
 * file of any kind, so a torrent whose naming is opaque still plays something
 * rather than nothing.
 */
export function bestEffortFile(files: TorrentFile[]): TorrentFile | null {
  const playable = files.filter(isPlayable);
  const pool = playable.length > 0 ? playable : files;
  return [...pool].sort((a, b) => b.lengthBytes - a.lengthBytes)[0] ?? null;
}