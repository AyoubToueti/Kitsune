import { describe, it, expect } from "vitest";

import { formatSeason, nextSeason, seasonHref, seasonQuery } from "./season";

describe("nextSeason", () => {
  it("advances within a year", () => {
    // Mid-May is Spring, so the next season is Summer of the same year.
    expect(nextSeason(new Date(2026, 4, 15))).toEqual({
      season: "summer",
      year: 2026,
    });
  });

  it("maps each month to its quarter", () => {
    const at = (month: number) =>
      nextSeason(new Date(2026, month - 1, 15)).season;

    // Winter is Jan-Mar, Spring Apr-Jun, Summer Jul-Sep, Fall Oct-Dec.
    expect([at(1), at(3)]).toEqual(["spring", "spring"]);
    expect([at(4), at(6)]).toEqual(["summer", "summer"]);
    expect([at(7), at(9)]).toEqual(["fall", "fall"]);
    expect([at(10), at(12)]).toEqual(["winter", "winter"]);
  });

  /// The boundary that is easy to miss: December's next season is Winter of the
  /// FOLLOWING year, not Winter of this one.
  it("rolls the year after fall", () => {
    expect(nextSeason(new Date(2026, 11, 31))).toEqual({
      season: "winter",
      year: 2027,
    });
  });

  /// Fall is the last season of its year, so its successor is Winter of the
  /// NEXT year even on Fall's first day. Winter 2026 was Jan-Mar, already past.
  ///
  /// AniList's own convention agrees: its docs note that "Winter 2017 would
  /// also include December 2016 releases", so Winter 2027 is what covers the
  /// December that follows Fall 2026.
  it("rolls the year even on the first day of fall", () => {
    expect(nextSeason(new Date(2026, 9, 1))).toEqual({
      season: "winter",
      year: 2027,
    });
  });

  /// Entering a season must not skip it: on the first day of Summer the answer
  /// is still Fall, not the season after that.
  it("does not skip the season it is entering", () => {
    expect(nextSeason(new Date(2026, 6, 1))).toEqual({
      season: "fall",
      year: 2026,
    });
  });
});

describe("formatSeason", () => {
  it("joins a provider season and a year", () => {
    expect(formatSeason("FALL", 2023)).toBe("Fall 2023");
  });

  it("matches the provider's casing case-insensitively", () => {
    // AniList spells it "FALL"; our own values are lowercase.
    expect(formatSeason("fall", 2023)).toBe("Fall 2023");
    expect(formatSeason("Fall", 2023)).toBe("Fall 2023");
  });

  it("returns the year alone when the season is missing", () => {
    // A film often has a year but no season.
    expect(formatSeason(undefined, 2023)).toBe("2023");
  });

  it("returns the season alone when the year is missing", () => {
    expect(formatSeason("FALL", undefined)).toBe("Fall");
  });

  it("returns nothing when both are missing", () => {
    expect(formatSeason(undefined, undefined)).toBeUndefined();
  });

  /// An unrecognised season must not render as "undefined 2023".
  it("ignores a season it does not recognise", () => {
    expect(formatSeason("MONSOON", 2023)).toBe("2023");
  });
});

describe("seasonQuery", () => {
  it("asks for one season's unreleased titles", () => {
    // Not-yet-released rather than the whole season: a season includes titles
    // that already aired, and "upcoming" should mean upcoming.
    expect(seasonQuery({ season: "fall", year: 2026 })).toEqual({
      season: "fall",
      seasonYear: 2026,
      status: "notYetReleased",
      sort: "popularity",
    });
  });
});

describe("seasonHref", () => {
  it("links to the same season the query asks for", () => {
    expect(seasonHref({ season: "fall", year: 2026 })).toBe(
      "/filter?season=fall&year=2026&status=notYetReleased",
    );
  });

  it("uses the given base path", () => {
    expect(seasonHref({ season: "winter", year: 2027 }, "/search")).toBe(
      "/search?season=winter&year=2027&status=notYetReleased",
    );
  });
});