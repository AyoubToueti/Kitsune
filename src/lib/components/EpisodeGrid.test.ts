import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import EpisodeGrid from "./EpisodeGrid.svelte";

describe("EpisodeGrid", () => {
  it("renders the correct number of buttons for a given count", () => {
    render(EpisodeGrid, { props: { count: 12 } });

    // Each button has an aria-label like "Episode 3 — no source configured".
    const buttons = screen.getAllByRole("button");
    expect(buttons).toHaveLength(12);
  });

  it("buttons are disabled (inert)", () => {
    render(EpisodeGrid, { props: { count: 3 } });

    const buttons = screen.getAllByRole("button");
    for (const btn of buttons) {
      expect(btn).toBeDisabled();
    }
  });

  it("button labels match 1 through count", () => {
    render(EpisodeGrid, { props: { count: 5 } });

    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();
  });

  it("shows unknown message when count is absent", () => {
    render(EpisodeGrid, { props: {} });

    expect(screen.getByText(/episode count unknown/i)).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("shows unknown message when count is 0", () => {
    render(EpisodeGrid, { props: { count: 0 } });

    expect(screen.getByText(/episode count unknown/i)).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });
});