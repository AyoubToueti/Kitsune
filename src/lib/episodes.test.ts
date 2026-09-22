import { describe, it, expect } from "vitest";

import {
  airedEpisodes,
  episodesFor,
  episodesFromInfo,
  hasEpisodeData,
  isAired,
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

      // The count matches the catalogue, so nothing is padded and the real
      // title is what the reader sees.
      const episodes = episodesFor(anime({ episodeCount: 1 }), info);

      expect(episodes).toHaveLength(1);
      expect(episodes[0].number).toBe(1);
      expect(episodes[0].title).toBe("Theatrical Malice");
    });

    it("pads a short catalogue up to the announced count", () => {
      // The case this exists for: MAL reports twelve episodes but has only
      // published the first. The grid must still reach all twelve.
      const info: EpisodeInfo[] = [
        {
          number: 1,
          title: "The Future Is In Our Hands",
          aired: "2026-07-04T00:00:00+00:00",
          filler: false,
          recap: false,
        },
      ];

      const episodes = episodesFor(
        anime({ episodeCount: 12, status: "FINISHED" }),
        info,
      );

      expect(episodes).toHaveLength(12);
      expect(episodes.map((ep) => ep.number)).toEqual([
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
      ]);
      // The catalogue entry keeps its real title; the invented ones are bare.
      expect(episodes[0].title).toBe("The Future Is In Our Hands");
      expect(episodes[1].title).toBe("Episode 2");
      // A padded entry carries no air date, so it cannot be mistaken for
      // catalogue data.
      expect(episodes[1].aired).toBeUndefined();
    });

    it("fills a hole in the middle of the catalogue", () => {
      const info: EpisodeInfo[] = [
        { number: 1, title: "One", filler: false, recap: false },
        { number: 3, title: "Three", filler: false, recap: false },
      ];

      const episodes = episodesFor(
        anime({ episodeCount: 3, status: "FINISHED" }),
        info,
      );

      expect(episodes.map((ep) => ep.number)).toEqual([1, 2, 3]);
      expect(episodes[1].title).toBe("Episode 2");
      expect(episodes[2].title).toBe("Three");
    });

    it("does not pad a later cour numbered from 13", () => {
      // A cour listed as 13..24 is twelve episodes of a longer work. Padding it
      // to the franchise total would invent the previous cour's episodes.
      const info: EpisodeInfo[] = Array.from({ length: 12 }, (_, i) => ({
        number: i + 13,
        title: `Episode ${i + 13}`,
        filler: false,
        recap: false,
      }));

      const episodes = episodesFor(
        anime({ episodeCount: 24, status: "FINISHED" }),
        info,
      );

      expect(episodes).toHaveLength(12);
      expect(episodes[0].number).toBe(13);
    });

    it("does not pad a work that is still airing", () => {
      // The gap on a releasing work may be episodes that have not aired, so
      // inventing them would present unwatchable episodes as ready.
      const info: EpisodeInfo[] = [
        { number: 1, title: "One", filler: false, recap: false },
      ];

      const episodes = episodesFor(
        anime({ episodeCount: 12, status: "RELEASING" }),
        info,
      );

      expect(episodes).toHaveLength(1);
    });

    it("does not pad when the status is unknown", () => {
      const info: EpisodeInfo[] = [
        { number: 1, title: "One", filler: false, recap: false },
      ];

      expect(episodesFor(anime({ episodeCount: 12 }), info)).toHaveLength(1);
    });

    it("leaves the catalogue alone when the count is unknown", () => {
      const info: EpisodeInfo[] = [
        { number: 1, title: "One", filler: false, recap: false },
      ];

      expect(episodesFor(anime(), info)).toHaveLength(1);
    });

    it("does not pad when the catalogue already covers the count", () => {
      const info: EpisodeInfo[] = [
        { number: 1, title: "One", filler: false, recap: false },
        { number: 2, title: "Two", filler: false, recap: false },
      ];

      expect(episodesFor(anime({ episodeCount: 2 }), info)).toHaveLength(2);
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

describe("isAired", () => {
  const now = new Date("2024-06-01T00:00:00Z");

  it("treats a missing date as aired", () => {
    expect(isAired(undefined, now)).toBe(true);
  });

  it("treats an empty date as aired", () => {
    expect(isAired("", now)).toBe(true);
  });

  it("treats an unparseable date as aired", () => {
    expect(isAired("not a date", now)).toBe(true);
  });

  it("accepts a timestamp in the past", () => {
    expect(isAired("2024-01-02T00:00:00+00:00", now)).toBe(true);
  });

  it("accepts a date-only string in the past", () => {
    expect(isAired("2024-01-02", now)).toBe(true);
  });

  it("rejects a timestamp in the future", () => {
    expect(isAired("2024-10-02T00:00:00+00:00", now)).toBe(false);
  });

  it("accepts an episode airing exactly now", () => {
    expect(isAired("2024-06-01T00:00:00Z", now)).toBe(true);
  });
});

describe("airedEpisodes", () => {
  const now = new Date("2024-06-01T00:00:00Z");

  function entry(number: number, aired?: string): EpisodeInfo {
    return {
      number,
      title: `Episode ${number}`,
      filler: false,
      recap: false,
      aired,
    };
  }

  it("drops the entries that have not aired", () => {
    const kept = airedEpisodes(
      [entry(1, "2024-01-01"), entry(2, "2024-10-01"), entry(3, "2024-02-01")],
      now,
    );

    expect(kept.map((e) => e.number)).toEqual([1, 3]);
  });

  it("keeps entries with no date at all", () => {
    const kept = airedEpisodes([entry(1), entry(2)], now);

    expect(kept.map((e) => e.number)).toEqual([1, 2]);
  });
});

describe("episodesFor with air dates", () => {
  const now = new Date("2024-06-01T00:00:00Z");

  function entry(number: number, aired: string): EpisodeInfo {
    return {
      number,
      title: `Episode ${number}`,
      filler: false,
      recap: false,
      aired,
    };
  }

  it("hides an episode that has not aired", () => {
    const info = [entry(1, "2024-01-01"), entry(2, "2024-10-01")];

    const episodes = episodesFor(anime({ episodeCount: 2 }), info, now);

    expect(episodes.map((e) => e.number)).toEqual([1]);
  });

  // An entirely unaired catalogue must NOT fall back to synthesis: that would
  // invent viewable episodes for a work whose first episode is still to come.
  it("yields an empty list when nothing has aired", () => {
    const info = [entry(1, "2025-01-01")];

    expect(episodesFor(anime({ episodeCount: 12 }), info, now)).toEqual([]);
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