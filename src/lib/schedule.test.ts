import { describe, it, expect } from "vitest";

import {
  buildDayStrip,
  formatTime,
  SCHEDULE_FUTURE_DAYS,
  SCHEDULE_PAST_DAYS,
} from "./schedule";

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

describe("buildDayStrip", () => {
  const now = at(2026, 9, 13, 12);

  it("covers past, today and future", () => {
    const { days } = buildDayStrip(now);

    expect(days).toHaveLength(SCHEDULE_PAST_DAYS + 1 + SCHEDULE_FUTURE_DAYS);
  });

  it("spans at least 25 days, as the layout requires", () => {
    const { days } = buildDayStrip(now);

    // A narrower strip would leave the tab row looking sparse.
    expect(days.length).toBeGreaterThanOrEqual(25);
  });

  it("starts seven days before today", () => {
    const { days } = buildDayStrip(now);

    // Sep 13 minus 7 is Sep 6.
    expect(days[0].key).toBe("2026-09-06");
  });

  it("ends twenty-one days after today", () => {
    const { days } = buildDayStrip(now);

    // Sep 13 plus 21 is Oct 4.
    expect(days[days.length - 1].key).toBe("2026-10-04");
  });

  it("reports today's index so it can be selected by default", () => {
    const { days, todayIndex } = buildDayStrip(now);

    expect(todayIndex).toBe(SCHEDULE_PAST_DAYS);
    expect(days[todayIndex].key).toBe("2026-09-13");
    expect(days[todayIndex].label).toBe("Today");
  });

  it("gives each day a unique key", () => {
    const { days } = buildDayStrip(now);
    const keys = days.map((d) => d.key);

    expect(new Set(keys).size).toBe(keys.length);
  });

  it("orders days chronologically", () => {
    const { days } = buildDayStrip(now);

    for (let i = 1; i < days.length; i++) {
      expect(days[i].from).toBeGreaterThan(days[i - 1].from);
    }
  });

  it("gives each day a one-day window", () => {
    const { days } = buildDayStrip(now);

    for (const day of days) {
      // Inclusive bounds, so a full local day is one second short of 24h.
      expect(day.to - day.from).toBe(24 * 60 * 60 - 1);
    }
  });

  it("labels the current day as Today and the next as Tomorrow", () => {
    const { days, todayIndex } = buildDayStrip(now);

    expect(days[todayIndex].label).toBe("Today");
    expect(days[todayIndex + 1].label).toBe("Tomorrow");
  });

  it("labels a day beyond tomorrow with its date, not prose", () => {
    const { days, todayIndex } = buildDayStrip(now);
    const later = days[todayIndex + 2];

    expect(later.label).not.toBe("Today");
    expect(later.label).not.toBe("Tomorrow");
  });

  it("carries a short weekday and date for the tabs", () => {
    const { days } = buildDayStrip(now);

    // Sep 6 2026 is a Sunday.
    expect(days[0].weekday).toBe("Sun");
    expect(days[0].shortDate).toMatch(/6/);
  });

  it("normalises month rollover rather than producing an invalid date", () => {
    // Late September plus three weeks crosses into October.
    const { days } = buildDayStrip(at(2026, 9, 25));

    expect(days[days.length - 1].key.startsWith("2026-10")).toBe(true);
  });

  it("normalises year rollover", () => {
    const { days } = buildDayStrip(at(2026, 12, 28));

    expect(days[days.length - 1].key.startsWith("2027-01")).toBe(true);
  });

  it("honours explicit past and future spans", () => {
    const { days, todayIndex } = buildDayStrip(now, 2, 3);

    expect(days).toHaveLength(6);
    expect(todayIndex).toBe(2);
  });

  it("brackets midnight so a late broadcast stays on its own day", () => {
    const { days, todayIndex } = buildDayStrip(now);
    const today = days[todayIndex];

    // 23:00 local must fall inside today's window, not tomorrow's. A UTC
    // conversion here would shift it for anyone east of UTC.
    const late = Math.floor(at(2026, 9, 13, 23).getTime() / 1000);

    expect(late).toBeGreaterThanOrEqual(today.from);
    expect(late).toBeLessThanOrEqual(today.to);
  });

  it("starts a day at local midnight", () => {
    const { days, todayIndex } = buildDayStrip(now);

    expect(days[todayIndex].from).toBe(
      Math.floor(at(2026, 9, 13, 0).getTime() / 1000),
    );
  });
});

describe("formatTime", () => {
  it("renders a time with hours and minutes", () => {
    const stamp = Math.floor(at(2026, 9, 13, 21).getTime() / 1000);

    expect(formatTime(stamp)).toMatch(/\d{1,2}:\d{2}/);
  });

  it("reflects the local clock, not UTC", () => {
    const when = at(2026, 9, 13, 21, 30);
    const stamp = Math.floor(when.getTime() / 1000);

    const expected = when.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
    expect(formatTime(stamp)).toBe(expected);
  });
});