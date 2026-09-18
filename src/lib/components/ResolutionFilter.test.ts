import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import type { ResolutionCount } from "$lib/resolution";
import ResolutionFilter from "./ResolutionFilter.svelte";

function available(...entries: [ResolutionCount["resolution"], number][]): ResolutionCount[] {
  return entries.map(([resolution, count]) => ({ resolution, count }));
}

describe("ResolutionFilter", () => {
  it("renders one chip per available resolution", () => {
    render(ResolutionFilter, {
      props: {
        available: available(["1080p", 3], ["720p", 1]),
      },
    });

    const chips = screen.getAllByRole("button");
    expect(chips).toHaveLength(2);
    expect(chips[0]).toHaveTextContent("1080p");
    expect(chips[1]).toHaveTextContent("720p");
  });

  it("shows the count on each chip", () => {
    // Two entries, because a lone chip is deliberately not rendered at all.
    render(ResolutionFilter, {
      props: { available: available(["1080p", 3], ["720p", 1]) },
    });

    expect(screen.getByText("1080p").closest("button")).toHaveTextContent("3");
    expect(screen.getByText("720p").closest("button")).toHaveTextContent("1");
  });

  it("marks the selected resolutions as pressed", () => {
    render(ResolutionFilter, {
      props: {
        available: available(["1080p", 1], ["720p", 1]),
        selected: ["720p"],
      },
    });

    expect(screen.getByText("1080p").closest("button")).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(screen.getByText("720p").closest("button")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("reports the resolution that was clicked", async () => {
    const onToggle = vi.fn();
    render(ResolutionFilter, {
      props: {
        available: available(["1080p", 1], ["720p", 1]),
        onToggle,
      },
    });

    await fireEvent.click(screen.getByText("720p"));

    expect(onToggle).toHaveBeenCalledWith("720p");
  });

  it("renders nothing when only one resolution is available", () => {
    // A lone chip cannot change the view, so offering it is noise.
    render(ResolutionFilter, {
      props: { available: available(["1080p", 4]) },
    });

    expect(screen.queryByTestId("resolution-filter")).not.toBeInTheDocument();
  });

  it("renders nothing when there are no resolutions", () => {
    render(ResolutionFilter, { props: { available: [] } });

    expect(screen.queryByTestId("resolution-filter")).not.toBeInTheDocument();
  });

  it("keeps the order it was given", () => {
    // The caller sorts by resolution; the component must not reorder, or a
    // re-ranked release list would reshuffle the chips.
    render(ResolutionFilter, {
      props: {
        available: available(["2160p", 1], ["1080p", 2], ["480p", 1]),
      },
    });

    const labels = screen
      .getAllByRole("button")
      .map((button) => button.getAttribute("data-resolution"));

    expect(labels).toEqual(["2160p", "1080p", "480p"]);
  });
});