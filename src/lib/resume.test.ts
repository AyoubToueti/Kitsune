import { describe, it, expect } from "vitest";

import { resumeIndex } from "./resume";

describe("resumeIndex", () => {
  /// Episode 3 is index 2: progress is a 1-based number, `?ep=` a 0-based
  /// index. Getting this wrong resumes one episode off every time.
  it("converts a 1-based episode number to a 0-based index", () => {
    expect(resumeIndex(1)).toBe(0);
    expect(resumeIndex(3)).toBe(2);
    expect(resumeIndex(12)).toBe(11);
  });

  /// A work put on the list but never started has progress 0, which would
  /// otherwise give -1 and select nothing.
  it("floors a never-started work at the first episode", () => {
    expect(resumeIndex(0)).toBe(0);
  });
});