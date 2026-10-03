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

    // 8 episode blocks plus the trailer card in the sidebar.
    const videoBlocks = container.querySelectorAll(".aspect-video");
    expect(videoBlocks).toHaveLength(9);
  });

  it("draws the three related-anime sidebar rows", () => {
    const { container } = render(DetailPageSkeleton);

    // 3 aspect-2/3 blocks in the sidebar, plus 6 recommendation posters.
    // Tailwind v4 names the utility `aspect-2/3` (the old arbitrary-value form
    // `aspect-[2/3]` is gone), so the selector follows the emitted class.
    const posterBlocks = container.querySelectorAll(".aspect-2\\/3");
    expect(posterBlocks.length).toBeGreaterThanOrEqual(3);
  });
});