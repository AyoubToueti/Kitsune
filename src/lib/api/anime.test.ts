import { describe, it, expect, vi, beforeEach } from "vitest";

// The wrapper module must be mocked before it is imported, so the factory
// is hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import {
  COMMANDS,
  errorMessage,
  getAnime,
  getTrending,
  searchAnime,
} from "./anime";
import type { Anime } from "$lib/types";

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