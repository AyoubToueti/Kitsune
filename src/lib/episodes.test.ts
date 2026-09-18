import { describe, it, expect } from "vitest";

import { episodesFor, hasEpisodeData, synthesizeEpisodes } from "./episodes";
import { episodeNumber } from "./episode";
import type { Anime, StreamingEpisode } from "./types";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 1,
    provider: "anilist",
    title: { romaji: "Test" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

describe("synthesizeEpisodes", () => {
  it("builds 1..count", () => {
    const episodes = synthesizeEpisodes(3);

    expect(episodes.map((e) => e.number)).toEqual([1, 2, 3]);
    expect(episodes.map((e) => e.title)).toEqual([
      "Episode 1",
      "Episode 2",
      "Episode 3",
    ]);
  });

  it("uses the cover as every card's background", () => {
    const episodes = synthesizeEpisodes(2, "https://example.test/cover.jpg");

    expect(episodes.every((e) => e.thumbnail === "https://example.test/cover.jpg")).toBe(
      true,
    );
  });

  it("returns nothing for a missing or non-positive count", () => {
    expect(synthesizeEpisodes(undefined)).toEqual([]);
    expect(synthesizeEpisodes(0)).toEqual([]);
    expect(synthesizeEpisodes(-5)).toEqual([]);
  });

  it("produces numbers the shared heuristic reads back", () => {
    // The synthesised title is what the torrent search keys off, so it has to
    // round-trip through the same parser the provider list uses.
    for (const ep of synthesizeEpisodes(12)) {
      expect(episodeNumber(ep)).toBe(ep.number);
    }
  });
});

describe("episodesFor", () => {
  it("prefers the provider's list when it has one", () => {
    const real: StreamingEpisode[] = [
      { url: "https://crunchyroll.test/1", title: "Episode 1" },
    ];
    const anime_ = anime({ streamingEpisodes: real, episodeCount: 500 });

    const episodes = episodesFor(anime_);

    expect(episodes).toHaveLength(1);
    expect(episodes[0].url).toBe("https://crunchyroll.test/1");
  });

  it("falls back to a synthesised list when the provider has none", () => {
    const anime_ = anime({ episodeCount: 1100, coverImage: "https://x.test/c.jpg" });

    const episodes = episodesFor(anime_);

    expect(episodes).toHaveLength(1100);
    expect(episodes[1099].title).toBe("Episode 1100");
    expect(episodes[1099].thumbnail).toBe("https://x.test/c.jpg");
  });

  it("does not top up a short provider list with invented entries", () => {
    // One Piece returns ~60 streaming links for 1000+ episodes. Showing 60 real
    // ones is honest; padding to 1000 would look complete while lying.
    const anime_ = anime({
      streamingEpisodes: [{ url: "https://crunchyroll.test/1", title: "Episode 1" }],
      episodeCount: 1100,
    });

    expect(episodesFor(anime_)).toHaveLength(1);
  });

  it("returns nothing when the work has no episode data at all", () => {
    expect(episodesFor(anime())).toEqual([]);
  });
});

describe("hasEpisodeData", () => {
  it("is true with a provider list", () => {
    expect(
      hasEpisodeData(anime({ streamingEpisodes: [{ url: "https://x.test/1" }] })),
    ).toBe(true);
  });

  it("is true with only a count", () => {
    expect(hasEpisodeData(anime({ episodeCount: 12 }))).toBe(true);
  });

  it("is false with neither", () => {
    expect(hasEpisodeData(anime())).toBe(false);
  });
});