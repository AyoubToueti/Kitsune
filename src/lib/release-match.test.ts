import { describe, it, expect } from "vitest";

import { clarityLabel, clarityTitle, matchClarity } from "./release-match";
import type { ParsedRelease, Release } from "$lib/types";

function release(parsed: Partial<ParsedRelease> = {}): Release {
  return {
    title: "[Group] Show [1080p]",
    indexer: "nyaa",
    magnetUri: "magnet:?xt=urn:btih:abc",
    resolution: "1080p",
    source: "webdl",
    remux: false,
    trusted: false,
    parsed: { title: "Show", ...parsed },
    score: 0,
  };
}

describe("matchClarity", () => {
  it("calls a season-and-episode name stated", () => {
    expect(matchClarity(release({ season: 3, episode: 9 }))).toBe("stated");
  });

  it("calls an episode-only name episode", () => {
    // `Show - 09`: the parser fills episode and absolute, leaving season unset.
    expect(
      matchClarity(release({ episode: 9, absoluteEpisode: 9 })),
    ).toBe("episode");
  });

  it("calls a bare trailing number episode", () => {
    expect(matchClarity(release({ episode: 12, absoluteEpisode: 12 }))).toBe(
      "episode",
    );
  });

  it("treats an absolute-only name as episode", () => {
    expect(matchClarity(release({ absoluteEpisode: 5 }))).toBe("episode");
  });

  it("calls a name with no number unclear", () => {
    expect(matchClarity(release())).toBe("unclear");
  });

  it("does not call a season without an episode stated", () => {
    // A stated season alone is not the SxxExx shape; there is no episode to
    // pair it with, so it is not the strongest signal.
    expect(matchClarity(release({ season: 3 }))).toBe("unclear");
  });

  it("does not treat a zero episode as missing", () => {
    // Episode 0 is a real (if unusual) number; `!= null` must not be falsy-checked.
    expect(matchClarity(release({ season: 1, episode: 0 }))).toBe("stated");
    expect(matchClarity(release({ episode: 0 }))).toBe("episode");
  });
});

describe("clarityLabel", () => {
  it("labels a stated match", () => {
    expect(clarityLabel("stated")).toBe("SxxExx");
  });

  it("labels an unclear match", () => {
    expect(clarityLabel("unclear")).toBe("No episode");
  });

  it("says nothing for the ordinary episode case", () => {
    // Marking every row would defeat the point of marking any.
    expect(clarityLabel("episode")).toBeUndefined();
  });
});

describe("clarityTitle", () => {
  it("explains each level in words", () => {
    expect(clarityTitle("stated")).toMatch(/season and episode/i);
    expect(clarityTitle("episode")).toMatch(/episode only/i);
    expect(clarityTitle("unclear")).toMatch(/no episode/i);
  });

  it("never returns an empty explanation", () => {
    for (const level of ["stated", "episode", "unclear"] as const) {
      expect(clarityTitle(level).trim()).not.toBe("");
    }
  });
});