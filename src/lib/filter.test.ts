import { describe, it, expect } from "vitest";

import {
  categoryLabel,
  filterHref,
  groupTagsByCategory,
  parseBrowseQuery,
  yearOptions,
  FORMAT_VALUES,
  SEASON_VALUES,
  SORT_VALUES,
  STATUS_VALUES,
} from "./filter";

/** Parse a query string, as the route would receive it. */
function parse(qs: string) {
  return parseBrowseQuery(new URLSearchParams(qs));
}

describe("parseBrowseQuery", () => {
  it("defaults to popularity with no filters", () => {
    expect(parse("")).toEqual({ sort: "popularity" });
  });

  it("reads a search term", () => {
    expect(parse("search=naruto").search).toBe("naruto");
  });

  it("trims the search term", () => {
    expect(parse("search=%20naruto%20").search).toBe("naruto");
  });

  it("drops a blank search term rather than sending it", () => {
    // An empty filter is not the same as no filter: the provider would treat
    // it as matching nothing.
    expect(parse("search=%20%20")).toEqual({ sort: "popularity" });
  });

  it("reads every genre, not just the first", () => {
    expect(parse("genre=Action&genre=Comedy").genres).toEqual([
      "Action",
      "Comedy",
    ]);
  });

  it("drops blank genres", () => {
    expect(parse("genre=Action&genre=%20").genres).toEqual(["Action"]);
  });

  it("omits genres entirely when none are given", () => {
    expect(parse("").genres).toBeUndefined();
  });

  it("reads every tag, not just the first", () => {
    expect(parse("tag=Isekai&tag=School").tags).toEqual(["Isekai", "School"]);
  });

  it("drops blank tags", () => {
    expect(parse("tag=Isekai&tag=%20").tags).toEqual(["Isekai"]);
  });

  it("omits tags entirely when none are given", () => {
    expect(parse("").tags).toBeUndefined();
  });

  it("reads every excluded tag, not just the first", () => {
    expect(parse("exclude_tag=Harem&exclude_tag=Ecchi").excludedTags).toEqual([
      "Harem",
      "Ecchi",
    ]);
  });

  it("drops blank excluded tags", () => {
    expect(parse("exclude_tag=Harem&exclude_tag=%20").excludedTags).toEqual([
      "Harem",
    ]);
  });

  it("omits excluded tags entirely when none are given", () => {
    expect(parse("").excludedTags).toBeUndefined();
  });

  // The two directions are independent parameters, not a sign on one value:
  // a tag can be included and another excluded in the same view.
  it("keeps inclusions and exclusions separate", () => {
    const query = parse("tag=Isekai&exclude_tag=Harem");

    expect(query.tags).toEqual(["Isekai"]);
    expect(query.excludedTags).toEqual(["Harem"]);
  });

  it("reads each enum filter", () => {
    const query = parse("format=tv&status=finished&season=fall");

    expect(query.format).toBe("tv");
    expect(query.status).toBe("finished");
    expect(query.season).toBe("fall");
  });

  it("drops an unrecognised enum rather than forwarding it", () => {
    // The backend rejects an unknown enum outright, which would surface as a
    // confusing request failure instead of the bad link it is.
    const query = parse("format=bluray&status=nonsense&season=monsoon");

    expect(query.format).toBeUndefined();
    expect(query.status).toBeUndefined();
    expect(query.season).toBeUndefined();
  });

  it("drops an unrecognised sort, falling back to the default", () => {
    expect(parse("sort=chaos").sort).toBe("popularity");
  });

  it("reads a valid sort", () => {
    expect(parse("sort=score").sort).toBe("score");
  });

  it("reads the year", () => {
    expect(parse("year=2019").seasonYear).toBe(2019);
  });

  it("drops a non-numeric year", () => {
    expect(parse("year=abc").seasonYear).toBeUndefined();
  });

  it("drops a year before the supported range", () => {
    expect(parse("year=1800").seasonYear).toBeUndefined();
  });

  it("drops a year too far ahead", () => {
    // Far-future years are typos, not a real filter.
    expect(parse("year=9999").seasonYear).toBeUndefined();
  });

  it("reads the score floor", () => {
    expect(parse("score=70").minScore).toBe(70);
  });

  it("drops a score above the scale", () => {
    expect(parse("score=150").minScore).toBeUndefined();
  });

  it("drops a non-numeric score", () => {
    expect(parse("score=high").minScore).toBeUndefined();
  });

  it("accepts every value the form offers", () => {
    // The form options and this parser must agree: two lists would drift, and
    // the symptom would be a filter that silently does nothing.
    for (const format of FORMAT_VALUES) {
      expect(parse(`format=${format}`).format).toBe(format);
    }
    for (const status of STATUS_VALUES) {
      expect(parse(`status=${status}`).status).toBe(status);
    }
    for (const season of SEASON_VALUES) {
      expect(parse(`season=${season}`).season).toBe(season);
    }
    for (const sort of SORT_VALUES) {
      expect(parse(`sort=${sort}`).sort).toBe(sort);
    }
  });

  it("combines filters", () => {
    const query = parse(
      "search=a&genre=Action&genre=Comedy&format=tv&status=releasing&season=fall&year=2024&score=70&sort=score",
    );

    expect(query).toEqual({
      search: "a",
      genres: ["Action", "Comedy"],
      format: "tv",
      status: "releasing",
      season: "fall",
      seasonYear: 2024,
      minScore: 70,
      sort: "score",
    });
  });
});

describe("filterHref", () => {
  it("drops the page parameter for the first page", () => {
    // One canonical URL per view, rather than both /filter and /filter?page=1.
    const params = new URLSearchParams("format=tv&page=3");

    expect(filterHref(params, 1)).toBe("/filter?format=tv");
  });

  it("keeps the filters when moving to another page", () => {
    const params = new URLSearchParams("format=tv&season=fall");

    expect(filterHref(params, 2)).toBe("/filter?format=tv&season=fall&page=2");
  });

  it("preserves repeated genre parameters", () => {
    const params = new URLSearchParams("genre=Action&genre=Comedy");

    expect(filterHref(params, 2)).toBe(
      "/filter?genre=Action&genre=Comedy&page=2",
    );
  });

  it("returns a bare path when there is nothing left", () => {
    expect(filterHref(new URLSearchParams("page=2"), 1)).toBe("/filter");
  });

  it("keeps excluded tags alongside inclusions when paging", () => {
    const params = new URLSearchParams("tag=Isekai&exclude_tag=Harem");

    expect(filterHref(params, 2)).toBe(
      "/filter?tag=Isekai&exclude_tag=Harem&page=2",
    );
  });

  // A shared filter bar has to link back to whichever route hosts it, so the
  // base path cannot be hard-coded to /filter.
  it("uses the given base path", () => {
    const params = new URLSearchParams("format=tv");

    expect(filterHref(params, 2, "/search")).toBe("/search?format=tv&page=2");
  });

  it("returns a bare base path when there is nothing left", () => {
    expect(filterHref(new URLSearchParams("page=2"), 1, "/search")).toBe(
      "/search",
    );
  });

  it("does not mutate the parameters it is given", () => {
    // The caller's params object is the live URL; mutating it would corrupt
    // the page it was read from.
    const params = new URLSearchParams("format=tv");

    filterHref(params, 5);

    expect(params.get("page")).toBeNull();
    expect(params.toString()).toBe("format=tv");
  });
});

describe("groupTagsByCategory", () => {
  it("collects tags under their category", () => {
    const groups = groupTagsByCategory([
      { name: "Isekai", category: "Theme-Fantasy" },
      { name: "Magic", category: "Theme-Fantasy" },
      { name: "School", category: "Setting-Scene" },
    ]);

    expect(groups.map((g) => g.category)).toEqual([
      "Theme-Fantasy",
      "Setting-Scene",
    ]);
    expect(groups[0].tags.map((t) => t.name)).toEqual(["Isekai", "Magic"]);
    expect(groups[1].tags.map((t) => t.name)).toEqual(["School"]);
  });

  it("keeps each tag's description, which the chip tooltip needs", () => {
    // The whole tag travels rather than just its name: dropping the
    // description here would silently empty every tooltip.
    const groups = groupTagsByCategory([
      { name: "Isekai", category: "Theme-Fantasy", description: "Another world." },
    ]);

    expect(groups[0].tags[0].description).toBe("Another world.");
  });

  it("preserves the provider's category order", () => {
    // Re-sorting here would silently undo whatever order the provider chose.
    const groups = groupTagsByCategory([
      { name: "Shounen", category: "Demographic" },
      { name: "Isekai", category: "Theme-Fantasy" },
    ]);

    expect(groups.map((g) => g.category)).toEqual([
      "Demographic",
      "Theme-Fantasy",
    ]);
  });

  it("returns nothing for an empty catalogue", () => {
    expect(groupTagsByCategory([])).toEqual([]);
  });
});

describe("categoryLabel", () => {
  it("splits on the first hyphen", () => {
    expect(categoryLabel("Cast-Main Cast")).toBe("Cast / Main Cast");
  });

  it("leaves later hyphens alone", () => {
    // "Theme-Game-Sport" is three levels, not a typo. Replacing every hyphen
    // would render it "Theme / Game / Sport", which is a different taxonomy.
    expect(categoryLabel("Theme-Game-Sport")).toBe("Theme / Game-Sport");
  });

  it("passes a category with no hyphen through unchanged", () => {
    expect(categoryLabel("Technical")).toBe("Technical");
  });
});

describe("yearOptions", () => {
  it("starts at next year, so upcoming seasons are reachable", () => {
    const newest = new Date().getFullYear() + 1;

    expect(yearOptions()[0]).toBe(newest);
  });

  it("ends at the earliest supported year", () => {
    const years = yearOptions();

    expect(years[years.length - 1]).toBe(1960);
  });

  it("descends without gaps", () => {
    const years = yearOptions();

    for (let i = 1; i < years.length; i++) {
      expect(years[i - 1] - years[i]).toBe(1);
    }
  });
});