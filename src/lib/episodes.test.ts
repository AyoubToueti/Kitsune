import { describe, it, expect } from "vitest";

import {
  episodesFor,
  episodesFromInfo,
  hasEpisodeData,
  synthesizeEpisodes,
} from "./episodes";
import { episodeNumber } from "./episode";
import type { Anime, EpisodeInfo } from "./types";

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

describe("episodesFromInfo", () => {
    it("carries the number, title, air date and flags through", () => {
      const info: EpisodeInfo[] = [
        {
          number: 7,
          title: "The Newest of Heroes",
          aired: "2024-11-13T00:00:00+00:00",
          filler: true,
          recap: true,
        },
      ];

      const [episode] = episodesFromInfo(info, "https://x.test/c.jpg");

      expect(episode.number).toBe(7);
      expect(episode.title).toBe("The Newest of Heroes");
      expect(episode.aired).toBe("2024-11-13T00:00:00+00:00");
      expect(episode.filler).toBe(true);
      expect(episode.recap).toBe(true);
      expect(episode.thumbnail).toBe("https://x.test/c.jpg");
    });

    it("substitutes a title for an entry the catalogue left unnamed", () => {
      const info: EpisodeInfo[] = [{ number: 3, filler: false, recap: false }];

      expect(episodesFromInfo(info)[0].title).toBe("Episode 3");
    });

    it("produces numbers the shared heuristic reads back", () => {
      // The number is what the torrent search keys off, so it has to survive the
      // round trip through the shared parser.
      const info: EpisodeInfo[] = Array.from({ length: 12 }, (_, i) => ({
        number: i + 1,
        filler: false,
        recap: false,
      }));

      for (const episode of episodesFromInfo(info)) {
        expect(episodeNumber(episode)).toBe(episode.number);
      }
    });
  });

  describe("episodesFor", () => {
    it("prefers the catalogue when it has entries", () => {
      const info: EpisodeInfo[] = [
        {
          number: 1,
          title: "Theatrical Malice",
          filler: false,
          recap: false,
        },
      ];

      const episodes = episodesFor(anime({ episodeCount: 500 }), info);

      expect(episodes).toHaveLength(1);
      expect(episodes[0].number).toBe(1);
      expect(episodes[0].title).toBe("Theatrical Malice");
    });

    it("ignores streaming links, which are not an episode list", () => {
      // AniList attaches the WHOLE FRANCHISE's links to the first season's entry,
      // so a season-1 page could otherwise list episode 66. The catalogue is the
      // only list source; streaming links are for the "where to watch" buttons.
      const anime_ = anime({
        streamingEpisodes: [
          { url: "https://crunchyroll.test/66", title: "Episode 66" },
        ],
        episodeCount: 25,
      });

      const episodes = episodesFor(anime_, []);

      expect(episodes).toHaveLength(25);
      expect(episodes[0].number).toBe(1);
      expect(episodes[24].title).toBe("Episode 25");
    });

    it("falls back to a synthesised list when the catalogue is empty", () => {
      const anime_ = anime({
        episodeCount: 1100,
        coverImage: "https://x.test/c.jpg",
      });

      const episodes = episodesFor(anime_, []);

      expect(episodes).toHaveLength(1100);
      expect(episodes[1099].title).toBe("Episode 1100");
      expect(episodes[1099].thumbnail).toBe("https://x.test/c.jpg");
    });

    it("returns nothing when the work has neither catalogue nor count", () => {
      expect(episodesFor(anime(), [])).toEqual([]);
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