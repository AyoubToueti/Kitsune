import { describe, it, expect } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";

import FilterBar from "./FilterBar.svelte";
import type { BrowseQuery, MediaTag } from "$lib/types";

const GENRES = ["Action", "Comedy"];

const TAGS: MediaTag[] = [{ name: "Isekai", category: "Theme-Fantasy" }];

/** The button that opens the dropdown. */
function toggle(): HTMLElement {
  return screen.getByTestId("open-filters");
}

/** Render with sensible defaults, overriding only what a test cares about. */
function renderBar(
  overrides: {
    genres?: string[];
    tags?: MediaTag[];
    current?: BrowseQuery;
    base?: string;
    showSearch?: boolean;
    showSort?: boolean;
    extraParams?: Record<string, string>;
  } = {},
) {
  const props = {
    genres: GENRES,
    tags: TAGS,
    current: { sort: "popularity" } as BrowseQuery,
    ...overrides,
  };

  return render(FilterBar, { props });
}

describe("FilterBar", () => {
  it("starts with the dropdown closed", () => {
    renderBar();

    expect(toggle()).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId("filter-dropdown")).toBeNull();
  });

  it("opens the dropdown when pressed", async () => {
    renderBar();

    await fireEvent.click(toggle());

    expect(toggle()).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId("filter-dropdown")).toBeInTheDocument();
  });

  it("closes again on a second press", async () => {
    renderBar();

    await fireEvent.click(toggle());
    await fireEvent.click(toggle());

    expect(screen.queryByTestId("filter-dropdown")).toBeNull();
  });

  it("shows no count when nothing is applied", () => {
    renderBar();

    expect(screen.queryByTestId("filter-count")).toBeNull();
  });

  it("counts every applied filter", () => {
    renderBar({
      current: {
        tags: ["Isekai"],
        excludedTags: ["Harem"],
        genres: ["Action"],
        sort: "popularity",
      },
    });

    // The badge is the only sign of active filters while the dropdown is shut.
    expect(screen.getByTestId("filter-count")).toHaveTextContent("3");
  });

  /// Clicking a control inside the dropdown must not close it, or the panel
  /// would vanish the moment the user touched it.
  it("stays open when a click lands inside it", async () => {
    renderBar();
    await fireEvent.click(toggle());

    const panel = screen.getByTestId("filter-dropdown");
    await fireEvent.click(panel);

    expect(screen.getByTestId("filter-dropdown")).toBeInTheDocument();
  });

  it("closes when a click lands outside it", async () => {
    renderBar();
    await fireEvent.click(toggle());

    // The window handler ignores clicks with no target, so give it a real one.
    await fireEvent.click(document.body);

    expect(screen.queryByTestId("filter-dropdown")).toBeNull();
  });

  it("hides the search field when asked", async () => {
    renderBar({ showSearch: false });
    await fireEvent.click(toggle());

    expect(screen.queryByLabelText("Search")).toBeNull();
  });

  it("hides the sort control when asked", async () => {
    renderBar({ showSort: false });
    await fireEvent.click(toggle());

    expect(screen.queryByLabelText("Sort")).toBeNull();
  });

  it("carries extra parameters into the form", async () => {
    renderBar({ extraParams: { q: "naruto" } });
    await fireEvent.click(toggle());

    const data = new FormData(screen.getByTestId("filter-form") as HTMLFormElement);

    // /search relies on this: without it a submit would reset the term.
    expect(data.get("q")).toBe("naruto");
  });

  it("points the form at the given base", async () => {
    renderBar({ base: "/search" });
    await fireEvent.click(toggle());

    expect(screen.getByTestId("filter-form")).toHaveAttribute("action", "/search");
  });
});