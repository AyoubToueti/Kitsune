import { describe, it, expect } from "vitest";

import {
  absoluteOffset,
  episodeNumber,
  fileForEpisode,
  indexOfEpisode,
  titleHasEpisodeNumber,
} from "./episode";
import type { Episode } from "./episodes";
import type { StreamingEpisode, TorrentFile } from "./types";

function ep(overrides: Partial<StreamingEpisode> = {}): StreamingEpisode {
  return { url: "https://x.test/1", ...overrides };
}

/** An entry that carries an explicit number, as a catalogue entry does. */
function numbered(number: number): Episode {
  return { number, title: `Episode ${number}` };
}

function file(name: string, idx = 0): TorrentFile {
  return { idx, name, lengthBytes: 1_000 };
}

describe("episodeNumber", () => {
  it("reads a number from 'Episode 12'", () => {
    expect(episodeNumber(ep({ title: "Episode 12" }))).toBe(12);
  });

  it("reads a number from an abbreviated title", () => {
    expect(episodeNumber(ep({ title: "Ep. 7 - The Wall" }))).toBe(7);
  });

  it("reads a number from a url when the title has none", () => {
    expect(episodeNumber(ep({ site: "Crunchyroll", url: "https://x.test/episode-5" }))).toBe(5);
  });

  it("prefers the title over the url", () => {
    // A mismatched url must not override an explicit title.
    const entry = ep({ title: "Episode 3", url: "https://x.test/episode-9" });
    expect(episodeNumber(entry)).toBe(3);
  });

  it("returns undefined when no number can be read", () => {
    // The url deliberately carries no number: a trailing digit IS a valid
    // episode number, so a fixture ending in "/1" would prove nothing.
    const entry = ep({ title: "Pilot", site: "Crunchyroll", url: "https://x.test/watch" });
    expect(episodeNumber(entry)).toBeUndefined();
  });

  it("does not read a year as an episode number from the middle of a title", () => {
    // "2000 Years" must not become episode 2000, and the url carries no number.
    const entry = ep({ title: "To You, 2000 Years Later", url: "https://x.test/watch" });
    expect(episodeNumber(entry)).toBeUndefined();
  });
});

describe("indexOfEpisode", () => {
  it("finds the entry for a number", () => {
    const episodes = [
      ep({ title: "Episode 1" }),
      ep({ title: "Episode 2", url: "https://x.test/2" }),
    ];
    expect(indexOfEpisode(episodes, 2)).toBe(1);
  });

  it("returns -1 when nothing matches", () => {
    expect(indexOfEpisode([ep({ title: "Episode 1" })], 9)).toBe(-1);
  });
});

describe("absoluteOffset", () => {
  it("is zero for an ordinary 1..N season", () => {
    const episodes = [
      ep({ title: "Episode 1" }),
      ep({ title: "Episode 2" }),
      ep({ title: "Episode 3" }),
    ];

    expect(absoluteOffset(episodes)).toBe(0);
  });

  it("is the first number minus one for a later cour", () => {
    // Twelve entries numbered 13..24: the lowest number exceeds the length, so
    // the cour restarts at 1 while the work is numbered as a whole.
    const episodes = Array.from({ length: 12 }, (_, i) => numbered(i + 13));

    expect(absoluteOffset(episodes)).toBe(12);
  });

  it("uses the lowest number, not the first entry", () => {
    // Out of order on purpose: the offset must come from the lowest number
    // (13 -> 12), not from the first entry (15 -> 14).
    const episodes = [numbered(15), numbered(13), numbered(14)];

    expect(absoluteOffset(episodes)).toBe(12);
  });

  it("is zero when no entry has a number", () => {
    expect(absoluteOffset([ep({ title: "Episode 1" })])).toBe(0);
  });

  it("is zero for an empty list", () => {
    expect(absoluteOffset([])).toBe(0);
  });
});

describe("titleHasEpisodeNumber", () => {
  it("recognises the forms a title may announce its number in", () => {
    expect(titleHasEpisodeNumber("Episode 7")).toBe(true);
    expect(titleHasEpisodeNumber("Episode 7 - A Name")).toBe(true);
    expect(titleHasEpisodeNumber("Ep. 7")).toBe(true);
    expect(titleHasEpisodeNumber("Ep 7")).toBe(true);
    expect(titleHasEpisodeNumber("E07")).toBe(true);
    // Leading space, as a provider might pad it.
    expect(titleHasEpisodeNumber("  Episode 12")).toBe(true);
  });

  it("leaves an ordinary title alone", () => {
    expect(titleHasEpisodeNumber("The Journey's End")).toBe(false);
    expect(titleHasEpisodeNumber("Theatrical Malice")).toBe(false);
  });

  it("ignores a number that is part of the name", () => {
    // Anchored at the start, so a trailing number is a name rather than an
    // announcement and the caller still prefixes.
    expect(titleHasEpisodeNumber("The Journey's End 2")).toBe(false);
  });

  it("does not treat a URL as an announcement", () => {
    expect(titleHasEpisodeNumber("https://x.test/episode-12")).toBe(false);
  });
});

describe("fileForEpisode", () => {
  const files = [
    file("[Group] Show - 01 [1080p].mkv", 0),
    file("[Group] Show - 02 [1080p].mkv", 1),
    file("[Group] Show - 03 [720p].mkv", 2),
  ];

  it("matches a delimited episode number", () => {
    expect(fileForEpisode(files, 2)?.idx).toBe(1);
  });

  it("does not match a number inside a resolution", () => {
    // Episode 80 must not match the "1080" in a filename.
    expect(fileForEpisode(files, 80)).toBeNull();
  });

  it("matches a zero-padded number spelled without padding", () => {
    expect(fileForEpisode([file("[G] Show E03.mkv", 5)], 3)?.idx).toBe(5);
  });

  it("returns null when nothing lines up", () => {
    expect(fileForEpisode(files, 99)).toBeNull();
  });
});