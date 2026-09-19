import { describe, it, expect } from "vitest";

import { displayTitle, titleForms } from "./types";

describe("titleForms", () => {
  it("returns the English and romaji forms, best first", () => {
    const forms = titleForms({
      english: "Attack on Titan",
      romaji: "Shingeki no Kyojin",
    });

    expect(forms).toEqual(["Attack on Titan", "Shingeki no Kyojin"]);
  });

  it("puts the user's own choice first", () => {
    const forms = titleForms({
      userPreferred: "SnK",
      english: "Attack on Titan",
      romaji: "Shingeki no Kyojin",
    });

    expect(forms[0]).toBe("SnK");
  });

  it("excludes the native form", () => {
    // The indexer category queried is English-translated, so a Japanese title
    // would only cost a request.
    const forms = titleForms({
      english: "Attack on Titan",
      native: "進撃の巨人",
    });

    expect(forms).toEqual(["Attack on Titan"]);
    expect(forms).not.toContain("進撃の巨人");
  });

  it("drops blanks and trims what is left", () => {
    const forms = titleForms({ english: "  Attack on Titan  ", romaji: "   " });

    expect(forms).toEqual(["Attack on Titan"]);
  });

  it("collapses duplicates case-insensitively", () => {
    const forms = titleForms({
      english: "Attack on Titan",
      romaji: "attack on titan",
    });

    expect(forms).toEqual(["Attack on Titan"]);
  });

  it("is empty when every form is missing", () => {
    expect(titleForms({})).toEqual([]);
  });
});

describe("displayTitle", () => {
  it("still falls back to the native form for headings", () => {
    // titleForms excludes native, but displayTitle must keep it: a heading with
    // no English or romaji title still has something to show.
    expect(displayTitle({ native: "進撃の巨人" })).toBe("進撃の巨人");
  });
});