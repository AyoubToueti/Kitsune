// Where a trailer can be watched, and where it can be embedded.
//
// AniList reports the platform and the video id separately, and each platform
// spells its URL differently. Keeping the mapping here means the detail page
// and the card modal compose the same URL, and an unknown site has a single
// definition of "no trailer".

import type { Trailer } from "./types";

/** The watch URL for a trailer, or `null` for a site we cannot link to. */
export function trailerWatchUrl(trailer: Trailer): string | null {
  switch (trailer.site.toLowerCase()) {
    case "youtube":
      return `https://www.youtube.com/watch?v=${trailer.id}`;
    case "dailymotion":
      return `https://www.dailymotion.com/video/${trailer.id}`;
    default:
      return null;
  }
}

/**
 * The embed URL for a trailer, or `null` for a site we cannot embed.
 *
 * Autoplay is requested because the embed only ever appears after an explicit
 * click on a play affordance.
 */
export function trailerEmbedUrl(trailer: Trailer): string | null {
  switch (trailer.site.toLowerCase()) {
    case "youtube":
      return `https://www.youtube.com/embed/${trailer.id}?autoplay=1`;
    case "dailymotion":
      return `https://www.dailymotion.com/embed/video/${trailer.id}?autoplay=1`;
    default:
      return null;
  }
}

/** Whether a trailer can be played in an embed. */
export function isPlayableTrailer(
  trailer: Trailer | undefined,
): trailer is Trailer {
  return trailer !== undefined && trailerEmbedUrl(trailer) !== null;
}