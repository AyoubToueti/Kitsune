import { describe, it, expect } from "vitest";

import {
  bestEffortFile,
  fileForEpisode,
  fileKind,
  firstTargetFile,
  isPlayable,
  matchNumber,
  matchesFileQuery,
  playableCount,
} from "./torrent-files";
import type { TorrentFile } from "./types";

function file(name: string, lengthBytes = 1_000_000): TorrentFile {
  return { idx: 0, name, lengthBytes };
}

describe("playableCount", () => {
  it("counts only playable videos", () => {
    const files = [
      file("Show - 01.mkv"),
      file("Show - 01.srt"),
      file("cover.jpg"),
      file("Show - 02.mp4"),
    ];
    expect(playableCount(files)).toBe(2);
  });

  it("is zero for a torrent with no video", () => {
    expect(playableCount([file("a.srt"), file("b.nfo")])).toBe(0);
  });
});

describe("firstTargetFile", () => {
  it("waits on any torrent with more than one video", () => {
    const files = [file("Show - 01.mkv"), file("Show - 02.mkv")];
    expect(firstTargetFile(files)).toBeNull();
  });

  it("waits even when one name looks like the episode", () => {
    // A pack's "S01E21" contains "01", which must not be taken for episode 1.
    const files = [
      file("Show.S01E21.1080p.mkv"),
      file("Show.S01E22.1080p.mkv"),
    ];
    expect(firstTargetFile(files)).toBeNull();
  });

  it("starts the lone video", () => {
    const files = [file("something-else.mkv"), file("notes.nfo")];
    expect(firstTargetFile(files)?.name).toBe("something-else.mkv");
  });

  it("starts a torrent whose only playable file is one video", () => {
    // Episode + subtitle is one video, not a choice to make.
    const files = [file("Show - 03.mkv"), file("Show - 03.ass")];
    expect(firstTargetFile(files)?.name).toBe("Show - 03.mkv");
  });
});

describe("matchesFileQuery", () => {
  it("matches an empty query", () => {
    expect(matchesFileQuery(file("Show - 01.mkv"), "")).toBe(true);
  });

  it("matches a case-insensitive substring", () => {
    expect(matchesFileQuery(file("Show - 01.MKV"), "01.mkv")).toBe(true);
  });

  it("ignores surrounding whitespace in the query", () => {
    expect(matchesFileQuery(file("Show - 01.mkv"), "  01  ")).toBe(true);
  });

  it("rejects a non-match", () => {
    expect(matchesFileQuery(file("Show - 01.mkv"), "extra")).toBe(false);
  });
});

describe("matchNumber", () => {
  it("matches a delimited episode number", () => {
    const files = [file("[SubsPlease] Show - 07 (1080p).mkv")];
    expect(matchNumber(files, 7)?.name).toContain("07");
  });

  it("does not match the number inside a resolution", () => {
    // "03" must not match "1080p" or "2023".
    const files = [file("Show 1080p 2023.mkv")];
    expect(matchNumber(files, 3)).toBeNull();
  });

  it("matches a zero-padded number", () => {
    const files = [file("Show - 03.mkv")];
    expect(matchNumber(files, 3)?.name).toContain("03");
  });

  it("falls back to a substring match for an unpadded name", () => {
    const files = [file("Show 07.mkv")];
    expect(matchNumber(files, 7)?.name).toContain("07");
  });

  it("returns null when nothing matches", () => {
    expect(matchNumber([file("Show - 12.mkv")], 7)).toBeNull();
  });
});

describe("fileForEpisode", () => {
  it("prefers the relative number", () => {
    const files = [file("Show - 07.mkv")];
    expect(fileForEpisode(files, 7, 0)?.name).toContain("07");
  });

  it("tries the absolute spelling when the relative misses", () => {
    // A later cour's release named with the running total.
    const files = [file("Show - 13.mkv")];
    expect(fileForEpisode(files, 1, 12)?.name).toContain("13");
  });

  it("does not re-try when the offset is zero", () => {
    const files = [file("Show - 13.mkv")];
    expect(fileForEpisode(files, 1, 0)).toBeNull();
  });

  it("returns null when neither spelling matches", () => {
    const files = [file("Show - 99.mkv")];
    expect(fileForEpisode(files, 1, 12)).toBeNull();
  });
});

describe("isPlayable", () => {
  it("accepts common video extensions", () => {
    expect(isPlayable(file("a.mkv"))).toBe(true);
    expect(isPlayable(file("a.MP4"))).toBe(true);
  });

  it("rejects non-video files", () => {
    expect(isPlayable(file("a.ass"))).toBe(false);
    expect(isPlayable(file("a.jpg"))).toBe(false);
  });
});

describe("bestEffortFile", () => {
  it("picks the largest playable file", () => {
    const files = [
      file("cover.jpg", 9_000_000),
      file("episode.mkv", 1_000_000_000),
      file("sample.mkv", 5_000_000),
    ];
    expect(bestEffortFile(files)?.name).toBe("episode.mkv");
  });

  it("falls back to the largest file of any kind", () => {
    const files = [file("a.txt", 10), file("b.nfo", 99)];
    expect(bestEffortFile(files)?.name).toBe("b.nfo");
  });

  it("returns null for an empty list", () => {
    expect(bestEffortFile([])).toBeNull();
  });
});

describe("fileKind", () => {
  it("classifies common video files", () => {
    expect(fileKind(file("episode.mkv"))).toBe("video");
    expect(fileKind(file("episode.MP4"))).toBe("video");
    expect(fileKind(file("episode.webm"))).toBe("video");
  });

  it("classifies subtitle files", () => {
    expect(fileKind(file("episode.ass"))).toBe("subtitle");
    expect(fileKind(file("episode.en.srt"))).toBe("subtitle");
    expect(fileKind(file("episode.vtt"))).toBe("subtitle");
  });

  it("classifies image files", () => {
    expect(fileKind(file("cover.jpg"))).toBe("image");
    expect(fileKind(file("poster.png"))).toBe("image");
  });

  it("falls back to other for anything unrecognised", () => {
    expect(fileKind(file("readme.nfo"))).toBe("other");
    expect(fileKind(file("no-extension"))).toBe("other");
  });
});