import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import DetailPageSkeleton from "./DetailPageSkeleton.svelte";

describe("DetailPageSkeleton", () => {
  it("announces itself as a loading status", () => {
    render(DetailPageSkeleton);

    const status = screen.getByRole("status");
    expect(status).toHaveAttribute("aria-busy", "true");
    expect(status).toHaveTextContent(/loading/i);
  });

  it("draws the episode grid placeholders", () => {
    const { container } = render(DetailPageSkeleton);

    // 8 aspect-video blocks in the episodes region.
    const episodeBlocks = container.querySelectorAll(".aspect-video");
    expect(episodeBlocks).toHaveLength(8);
  });

  it("draws the three related-anime sidebar rows", () => {
    const { container } = render(DetailPageSkeleton);

    // 3 aspect-[2/3] blocks in the sidebar, plus 6 recommendation posters.
    const posterBlocks = container.querySelectorAll(".aspect-\\[2\\/3\\]");
    expect(posterBlocks.length).toBeGreaterThanOrEqual(3);
  });
});