import { describe, it, expect } from "vitest";

import { RESOLUTION_ORDER, availableResolutions, matchesResolution } from "./resolution";
import type { Release, Resolution } from "$lib/types";

function release(resolution: Resolution): Release {
  return {
    title: `[Group] Show [${resolution}]`,
    indexer: "nyaa",
    magnetUri: "magnet:?xt=urn:btih:abc",
    resolution,
    source: "webdl",
    remux: false,
    trusted: false,
    parsed: { title: "Show", absoluteEpisode: 1 },
    score: 0,
  };
}

describe("RESOLUTION_ORDER", () => {
  it("lists the highest resolution first", () => {
    expect(RESOLUTION_ORDER[0]).toBe("2160p");
    expect(RESOLUTION_ORDER[1]).toBe("1080p");
  });

  it("puts unknown last, below every real resolution", () => {
    // Unknown is the absence of a claim, not a low quality, so ordering it
    // below 360p would misrepresent it.
    expect(RESOLUTION_ORDER[RESOLUTION_ORDER.length - 1]).toBe("unknown");
  });

  it("covers every resolution exactly once", () => {
    // A duplicate would silently drop a chip; a missing one would make a
    // resolution unfilterable.
    expect(new Set(RESOLUTION_ORDER).size).toBe(RESOLUTION_ORDER.length);
  });
});

describe("availableResolutions", () => {
  it("returns nothing for an empty list", () => {
    expect(availableResolutions([])).toEqual([]);
  });

  it("returns only the resolutions present", () => {
    const found = availableResolutions([
      release("1080p"),
      release("720p"),
      release("720p"),
    ]);

    expect(found).toEqual([
      { resolution: "1080p", count: 1 },
      { resolution: "720p", count: 2 },
    ]);
  });

  it("orders by resolution, not by input order", () => {
    // The chips must not reshuffle when the probe re-ranks the releases.
    const found = availableResolutions([
      release("480p"),
      release("2160p"),
      release("1080p"),
    ]);

    expect(found.map((entry) => entry.resolution)).toEqual([
      "2160p",
      "1080p",
      "480p",
    ]);
  });

  it("counts duplicates", () => {
    const found = availableResolutions([
      release("1080p"),
      release("1080p"),
      release("1080p"),
    ]);

    expect(found).toEqual([{ resolution: "1080p", count: 3 }]);
  });

  it("includes unknown when it is present", () => {
    // Dropping it would leave those releases unreachable while a filter is on.
    const found = availableResolutions([release("unknown"), release("1080p")]);

    expect(found.map((entry) => entry.resolution)).toEqual(["1080p", "unknown"]);
  });

  it("handles an unknown-only list", () => {
    expect(availableResolutions([release("unknown")])).toEqual([
      { resolution: "unknown", count: 1 },
    ]);
  });
});

describe("matchesResolution", () => {
  it("keeps everything when nothing is selected", () => {
    // This is what makes clearing the filter safe: an empty selection cannot
    // leave the list empty.
    expect(matchesResolution(release("1080p"), [])).toBe(true);
    expect(matchesResolution(release("unknown"), [])).toBe(true);
  });

  it("keeps only the selected resolutions", () => {
    expect(matchesResolution(release("1080p"), ["1080p", "720p"])).toBe(true);
    expect(matchesResolution(release("720p"), ["1080p", "720p"])).toBe(true);
    expect(matchesResolution(release("480p"), ["1080p", "720p"])).toBe(false);
  });

  it("can select unknown explicitly", () => {
    expect(matchesResolution(release("unknown"), ["unknown"])).toBe(true);
    expect(matchesResolution(release("1080p"), ["unknown"])).toBe(false);
  });
});