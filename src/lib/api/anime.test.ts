import { describe, it, expect, vi, beforeEach } from "vitest";

// The wrapper module must be mocked before it is imported, so the factory
// is hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import {
  COMMANDS,
  errorMessage,
  getAnime,
  getByGenre,
  getGenres,
  getList,
  getSchedule,
  getTrending,
  searchAnime,
} from "./anime";
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

  it("searchAnime passes the query through", async () => {
    invokeMock.mockResolvedValue([sampleAnime()]);

    await searchAnime("one piece", 5);

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.search, {
      query: "one piece",
      limit: 5,
    });
  });

  it("searchAnime returns an empty list unchanged", async () => {
    invokeMock.mockResolvedValue([]);

    expect(await searchAnime("")).toEqual([]);
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

  it("getByGenre passes the genre through", async () => {
    invokeMock.mockResolvedValue([sampleAnime()]);

    await getByGenre("Slice of Life", 20);

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.byGenre, {
      genre: "Slice of Life",
      limit: 20,
    });
  });

  it("getGenres takes no arguments", async () => {
    invokeMock.mockResolvedValue(["Action", "Mecha"]);

    const genres = await getGenres();

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.genres);
    expect(genres).toEqual(["Action", "Mecha"]);
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