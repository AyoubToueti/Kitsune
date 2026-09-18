// Episode lists that survive a sparse provider.
//
// AniList exposes `streamingEpisodes` (links to licensed services), but that is
// not an episode list: it is empty for many works and capped by the provider for
// long runners like One Piece. `episodeCount`, by contrast, is reliable. When the
// rich list is missing or short, this synthesises `1..count` so the reader can
// still reach every episode -- and, critically, so the watch page can resolve an
// episode number to search a torrent for.
//
// The number is the load-bearing part. Everything downstream (the torrent
// matcher, the file matcher) keys off "which episode is this", so a synthesised
// entry carries its number in the title where `episodeNumber` can read it back.

import type { Anime, StreamingEpisode } from "./types";

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
 * The episodes to show for a work: the provider's list when it has one, else a
 * synthesised stand-in.
 *
 * The provider list wins when non-empty because it carries real titles and
 * thumbnails. It is not topped up to `episodeCount` -- mixing real and invented
 * entries would make the list look complete while half of it was guesswork.
 */
export function episodesFor(anime: Anime): Episode[] {
  const providerEpisodes = anime.streamingEpisodes ?? [];
  if (providerEpisodes.length > 0) {
    return providerEpisodes.map(toEpisode);
  }
  return synthesizeEpisodes(anime.episodeCount, anime.coverImage);
}

/** Widen a provider entry into the display type. */
function toEpisode(ep: StreamingEpisode): Episode {
  return {
    title: ep.title,
    url: ep.url,
    site: ep.site,
    thumbnail: ep.thumbnail,
  };
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