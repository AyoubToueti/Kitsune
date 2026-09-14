import { describe, it, expect, vi, beforeEach } from "vitest";

// The wrapper module must be mocked before it is imported, so the factory
// is hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import {
  COMMANDS,
  errorMessage,
  getAnime,
  getGenres,
  getList,
  getSchedule,
  getTags,
  getTrending,
} from "./anime";
import { clearApiCache } from "./cache";
import type { Anime, ScheduledEpisode } from "$lib/types";

function sampleAnime(): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: ["Action"],
    streamingEpisodes: [],
  };
}

beforeEach(() => {
  invokeMock.mockReset();
  // The wrappers cache their results, so a value left by one test would be
  // served to the next and make the suite order-dependent.
  clearApiCache();
});

describe("command wrappers", () => {
  it("getTrending calls the trending command with the limit", async () => {
    invokeMock.mockResolvedValue([sampleAnime()]);

    const result = await getTrending(10);

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.trending, { limit: 10 });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(21);
  });

  it("getTrending omits the limit when not given", async () => {
    invokeMock.mockResolvedValue([]);

    await getTrending();

    // `undefined` rather than absent: the Rust side takes Option<u32>, so
    // an explicit undefined deserialises to None.
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.trending, { limit: undefined });
  });

  it("getAnime passes the id", async () => {
    invokeMock.mockResolvedValue(sampleAnime());

    const anime = await getAnime(21);

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.byId, { id: 21 });
    expect(anime?.id).toBe(21);
  });

  it("getAnime resolves to null for a missing id", async () => {
    invokeMock.mockResolvedValue(null);

    expect(await getAnime(999)).toBeNull();
  });

  it("getList passes the filter and limit", async () => {
    invokeMock.mockResolvedValue([sampleAnime()]);

    await getList("topAiring", 12);

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.list, {
      filter: "topAiring",
      limit: 12,
    });
  });

  it("getList sends the filter verbatim, since it is the wire format", async () => {
    invokeMock.mockResolvedValue([]);

    await getList("latestCompleted");

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.list, {
      filter: "latestCompleted",
      limit: undefined,
    });
  });

  it("getGenres takes no arguments", async () => {
    invokeMock.mockResolvedValue(["Action", "Mecha"]);

    const genres = await getGenres();

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.genres);
    expect(genres).toEqual(["Action", "Mecha"]);
  });

  it("getTags takes no arguments", async () => {
    invokeMock.mockResolvedValue([{ name: "Isekai", category: "Theme-Fantasy" }]);

    const tags = await getTags();

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.tags);
    expect(tags).toHaveLength(1);
    // The category is what the filter UI groups by, so it must survive the
    // boundary rather than being flattened to names on the way through.
    expect(tags[0].category).toBe("Theme-Fantasy");
  });

  it("getSchedule passes the window and limit", async () => {
    invokeMock.mockResolvedValue([]);

    await getSchedule(1_789_000_000, 1_789_600_000, 30);

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.schedule, {
      from: 1_789_000_000,
      to: 1_789_600_000,
      limit: 30,
    });
  });

  it("getSchedule returns the mapped entries", async () => {
    const entry: ScheduledEpisode = {
      anime: sampleAnime(),
      airingAt: 1_789_032_600,
      episode: 25,
    };
    invokeMock.mockResolvedValue([entry]);

    const result = await getSchedule(1_789_000_000, 1_789_600_000);

    expect(result).toHaveLength(1);
    expect(result[0].airingAt).toBe(1_789_032_600);
    expect(result[0].episode).toBe(25);
  });

  it("propagates a rejected command", async () => {
    invokeMock.mockRejectedValue("provider returned HTTP 429");

    await expect(getTrending()).rejects.toBe("provider returned HTTP 429");
  });
});

describe("caching", () => {
  it("serves a repeated call without hitting the backend again", async () => {
    invokeMock.mockResolvedValue([sampleAnime()]);

    await getTrending(10);
    await getTrending(10);

    // The second call is the cache's whole reason to exist.
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("still distinguishes calls that differ by argument", async () => {
    invokeMock.mockResolvedValue([]);

    await getTrending(10);
    await getTrending(20);

    // Different limits are different results, so they must not share a key.
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("re-fetches after a failure rather than caching it", async () => {
    invokeMock
      .mockRejectedValueOnce("provider returned HTTP 429")
      .mockResolvedValueOnce([sampleAnime()]);

    await expect(getTrending(10)).rejects.toBe("provider returned HTTP 429");
    const result = await getTrending(10);

    // A rate limit must not stick for the whole TTL.
    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(result).toHaveLength(1);
  });
});

describe("errorMessage", () => {
  it("passes a backend string through", () => {
    expect(errorMessage("provider returned HTTP 429")).toBe(
      "provider returned HTTP 429",
    );
  });

  it("uses an Error's message", () => {
    expect(errorMessage(new Error("network down"))).toBe("network down");
  });

  it("falls back for a blank string", () => {
    expect(errorMessage("   ")).toMatch(/AniList/);
  });

  it("falls back for a non-Error object", () => {
    // A rejection value can be anything; rendering it verbatim would put
    // "[object Object]" in front of the user.
    expect(errorMessage({ unexpected: true })).toMatch(/AniList/);
  });

  it("falls back for undefined", () => {
    expect(errorMessage(undefined)).toMatch(/AniList/);
  });
});