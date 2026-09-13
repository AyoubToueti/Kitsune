import { describe, it, expect } from "vitest";

import { formatTime, groupByDay } from "./schedule";
import type { Anime, ScheduledEpisode } from "$lib/types";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
  };
}

/** A local-time Date, so the tests do not depend on the runner's timezone. */
function at(
  year: number,
  month: number,
  day: number,
  hour = 0,
  minute = 0,
): Date {
  return new Date(year, month - 1, day, hour, minute);
}

function entry(id: number, when: Date, episode?: number): ScheduledEpisode {
  return {
    anime: anime(id, `Title ${id}`),
    airingAt: Math.floor(when.getTime() / 1000),
    episode,
  };
}

describe("groupByDay", () => {
  const now = at(2026, 9, 13, 12, 0);

  it("returns nothing for an empty list", () => {
    expect(groupByDay([], now)).toEqual([]);
  });

  it("labels the current day as Today", () => {
    const days = groupByDay([entry(1, at(2026, 9, 13, 21, 30))], now);

    expect(days).toHaveLength(1);
    expect(days[0].label).toBe("Today");
  });

  it("labels the next day as Tomorrow", () => {
    const days = groupByDay([entry(1, at(2026, 9, 14, 9, 0))], now);

    expect(days[0].label).toBe("Tomorrow");
  });

  it("labels a later day with its date, not Today or Tomorrow", () => {
    const days = groupByDay([entry(1, at(2026, 9, 20, 9, 0))], now);

    expect(days[0].label).not.toBe("Today");
    expect(days[0].label).not.toBe("Tomorrow");
    expect(days[0].label).toMatch(/20/);
  });

  it("groups entries from the same day together", () => {
    const days = groupByDay(
      [
        entry(1, at(2026, 9, 13, 21, 30)),
        entry(2, at(2026, 9, 13, 22, 0)),
        entry(3, at(2026, 9, 14, 9, 0)),
      ],
      now,
    );

    expect(days).toHaveLength(2);
    expect(days[0].entries).toHaveLength(2);
    expect(days[1].entries).toHaveLength(1);
  });

  it("orders days earliest first", () => {
    // Deliberately supplied out of order.
    const days = groupByDay(
      [
        entry(1, at(2026, 9, 15, 9, 0)),
        entry(2, at(2026, 9, 13, 9, 0)),
        entry(3, at(2026, 9, 14, 9, 0)),
      ],
      now,
    );

    expect(days.map((d) => d.entries[0].anime.id)).toEqual([2, 3, 1]);
  });

  it("orders entries within a day by airing time", () => {
    const days = groupByDay(
      [
        entry(1, at(2026, 9, 13, 23, 0)),
        entry(2, at(2026, 9, 13, 20, 0)),
        entry(3, at(2026, 9, 13, 22, 0)),
      ],
      now,
    );

    expect(days[0].entries.map((e) => e.anime.id)).toEqual([2, 3, 1]);
  });

  it("keeps a late-night broadcast on its own local day", () => {
    // 23:30 local is the same local day. Using a UTC conversion here would
    // push it to the next day for anyone east of UTC.
    const days = groupByDay([entry(1, at(2026, 9, 13, 23, 30))], now);

    expect(days[0].label).toBe("Today");
  });

  it("gives each group a stable key", () => {
    const days = groupByDay([entry(1, at(2026, 9, 13, 21, 0))], now);

    expect(days[0].key).toBe("2026-09-13");
  });

  it("does not mutate the caller's array", () => {
    const input = [
      entry(1, at(2026, 9, 14, 9, 0)),
      entry(2, at(2026, 9, 13, 9, 0)),
    ];
    const before = input.map((e) => e.anime.id);

    groupByDay(input, now);

    expect(input.map((e) => e.anime.id)).toEqual(before);
  });
});

describe("formatTime", () => {
  it("renders a time with hours and minutes", () => {
    const stamp = Math.floor(at(2026, 9, 13, 21, 5).getTime() / 1000);

    expect(formatTime(stamp)).toMatch(/\d{1,2}:\d{2}/);
  });

  it("reflects the local clock, not UTC", () => {
    const when = at(2026, 9, 13, 21, 30);
    const stamp = Math.floor(when.getTime() / 1000);

    // Whatever the offset, the rendered time must match the local Date.
    const expected = when.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
    expect(formatTime(stamp)).toBe(expected);
  });
});