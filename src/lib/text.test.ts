import { describe, it, expect } from "vitest";

import { stripHtml } from "./text";

describe("stripHtml", () => {
  it("removes tags", () => {
    expect(stripHtml("<i>Naruto</i> is a ninja.")).toBe("Naruto is a ninja.");
  });

  it("turns <br> into a space", () => {
    expect(stripHtml("line one<br>line two")).toBe("line one line two");
    expect(stripHtml("a<br/>b")).toBe("a b");
  });

  it("collapses runs of whitespace", () => {
    expect(stripHtml("a   \n\n  b")).toBe("a b");
  });

  it("decodes the common entities", () => {
    expect(stripHtml("He said &quot;hi&quot; &amp; left")).toBe(
      'He said "hi" & left',
    );
    expect(stripHtml("It&#039;s here")).toBe("It's here");
  });

  it("leaves plain text alone", () => {
    expect(stripHtml("A pirate adventure.")).toBe("A pirate adventure.");
  });

  it("returns undefined for absent input", () => {
    expect(stripHtml(undefined)).toBeUndefined();
    expect(stripHtml(null)).toBeUndefined();
  });

  it("returns undefined when only markup was present", () => {
    // Better to omit the paragraph than render an empty one.
    expect(stripHtml("<br><br>")).toBeUndefined();
    expect(stripHtml("   ")).toBeUndefined();
  });
});