import { describe, it, expect } from "vitest";

import { filterByQuery, matchesQuery } from "./release-filter";
import type { Release } from "$lib/types";

function release(title: string): Release {
  return {
    title,
    indexer: "nyaa",
    magnetUri: "magnet:?xt=urn:btih:abc",
    resolution: "1080p",
    source: "webdl",
    remux: false,
    trusted: false,
    parsed: { title: "Show" },
    score: 0,
  };
}

describe("matchesQuery", () => {
  it("matches a substring of the title", () => {
    expect(matchesQuery(release("[G] Show - 05 [1080p]"), "1080")).toBe(true);
  });

  it("is case-insensitive", () => {
    expect(matchesQuery(release("[G] Show - 05 [1080p]"), "SHOW")).toBe(true);
    expect(matchesQuery(release("[G] SHOW - 05"), "show")).toBe(true);
  });

  it("keeps everything when the query is empty", () => {
    expect(matchesQuery(release("[G] Show - 05"), "")).toBe(true);
  });

  it("treats a whitespace-only query as empty", () => {
    expect(matchesQuery(release("[G] Show - 05"), "   ")).toBe(true);
  });

  it("ignores surrounding whitespace", () => {
    expect(matchesQuery(release("[G] Show - 05 [1080p]"), "  1080  ")).toBe(true);
  });

  it("does not match an absent term", () => {
    expect(matchesQuery(release("[G] Show - 05 [1080p]"), "720p")).toBe(false);
  });
});

describe("filterByQuery", () => {
  const releases = [
    release("[SubsPlease] Show - 05 [1080p]"),
    release("[Erai-raws] Show - 05 [720p]"),
    release("[SubsPlease] Show - 06 [1080p]"),
  ];

  it("keeps only the matching releases", () => {
    const kept = filterByQuery(releases, "1080p");
    expect(kept.map((r) => r.title)).toEqual([
      "[SubsPlease] Show - 05 [1080p]",
      "[SubsPlease] Show - 06 [1080p]",
    ]);
  });

  it("preserves the given order", () => {
    // The caller has already ranked these; filtering must not reshuffle them.
    const kept = filterByQuery(releases, "Show");
    expect(kept.map((r) => r.title)).toEqual(releases.map((r) => r.title));
  });

  it("returns the list unchanged for an empty query", () => {
    expect(filterByQuery(releases, "")).toBe(releases);
  });

  it("returns nothing when no release matches", () => {
    expect(filterByQuery(releases, "2160p")).toEqual([]);
  });

  it("narrows a release group", () => {
    const kept = filterByQuery(releases, "erai");
    expect(kept).toHaveLength(1);
    expect(kept[0].title).toContain("720p");
  });
});