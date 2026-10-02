import { describe, it, expect } from "vitest";

import {
  isPlayableTrailer,
  trailerEmbedUrl,
  trailerWatchUrl,
} from "./trailer";

describe("trailerWatchUrl", () => {
  it("builds a YouTube watch link", () => {
    expect(trailerWatchUrl({ id: "abc", site: "youtube" })).toBe(
      "https://www.youtube.com/watch?v=abc",
    );
  });

  it("builds a Dailymotion watch link", () => {
    expect(trailerWatchUrl({ id: "xyz", site: "dailymotion" })).toBe(
      "https://www.dailymotion.com/video/xyz",
    );
  });

  it("matches the site case-insensitively", () => {
    expect(trailerWatchUrl({ id: "abc", site: "YouTube" })).toBe(
      "https://www.youtube.com/watch?v=abc",
    );
  });

  it("returns null for a site it cannot link", () => {
    expect(trailerWatchUrl({ id: "abc", site: "vimeo" })).toBeNull();
  });
});

describe("trailerEmbedUrl", () => {
  it("builds an autoplaying YouTube embed", () => {
    expect(trailerEmbedUrl({ id: "abc", site: "youtube" })).toBe(
      "https://www.youtube.com/embed/abc?autoplay=1",
    );
  });

  it("returns null for a site it cannot embed", () => {
    expect(trailerEmbedUrl({ id: "abc", site: "vimeo" })).toBeNull();
  });
});

describe("isPlayableTrailer", () => {
  it("is false for undefined", () => {
    expect(isPlayableTrailer(undefined)).toBe(false);
  });

  it("is true for an embeddable site", () => {
    expect(isPlayableTrailer({ id: "abc", site: "youtube" })).toBe(true);
  });

  it("is false for a non-embeddable site", () => {
    expect(isPlayableTrailer({ id: "abc", site: "vimeo" })).toBe(false);
  });
});