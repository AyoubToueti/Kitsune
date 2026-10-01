import { describe, it, expect } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";

import FilterBar from "./FilterBar.svelte";
import type { BrowseQuery, MediaTag } from "$lib/types";

const GENRES = ["Action", "Comedy"];

const TAGS: MediaTag[] = [{ name: "Isekai", category: "Theme-Fantasy" }];

/** The button that opens the filter dropdown. */
function filtersButton(): HTMLElement {
  return screen.getByTestId("open-filters");
}

/** The button that opens the sort dropdown. */
function sortButton(): HTMLElement {
  return screen.getByTestId("open-sort");
}

/**
 * The props every render supplies.
 *
 * Kept separate from the optional ones so `params` and friends are not
 * `| undefined` at the call site -- the component requires them, and an
 * all-optional interface would not satisfy it.
 */
interface RequiredBarProps {
  params: URLSearchParams;
  genres: string[];
  current: BrowseQuery;
}

interface BarProps extends RequiredBarProps {
  tags?: MediaTag[];
  base?: string;
  showSearch?: boolean;
  showSort?: boolean;
  extraParams?: Record<string, string>;
}

/** Render with sensible defaults, overriding only what a test cares about. */
function renderBar(overrides: Partial<BarProps> = {}) {
  // Built as one object: the component's props are a closed set, and an inline
  // spread would defeat that check.
  const props: BarProps = {
    params: new URLSearchParams(),
    genres: GENRES,
    tags: TAGS,
    current: { sort: "popularity" },
    ...overrides,
  };

  return render(FilterBar, { props });
}

describe("FilterBar", () => {
  describe("the filter dropdown", () => {
    it("starts closed", () => {
      renderBar();

      expect(filtersButton()).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByTestId("filter-dropdown")).toBeNull();
    });

    it("opens when pressed", async () => {
      renderBar();

      await fireEvent.click(filtersButton());

      expect(filtersButton()).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByTestId("filter-dropdown")).toBeInTheDocument();
    });

    it("closes again on a second press", async () => {
      renderBar();

      await fireEvent.click(filtersButton());
      await fireEvent.click(filtersButton());

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
      await fireEvent.click(filtersButton());

      await fireEvent.click(screen.getByTestId("filter-dropdown"));

      expect(screen.getByTestId("filter-dropdown")).toBeInTheDocument();
    });

    it("closes when a click lands outside it", async () => {
      renderBar();
      await fireEvent.click(filtersButton());

      // The window handler ignores clicks with no target, so give it a real one.
      await fireEvent.click(document.body);

      expect(screen.queryByTestId("filter-dropdown")).toBeNull();
    });

    it("hides the search field when asked", async () => {
      renderBar({ showSearch: false });
      await fireEvent.click(filtersButton());

      expect(screen.queryByLabelText("Search")).toBeNull();
    });

    it("offers a reverse toggle beside the sort button", () => {
      renderBar();

      expect(screen.getByTestId("reverse-order")).toHaveAttribute(
        "aria-pressed",
        "false",
      );
    });

    it("points the toggle at order=reverse", () => {
      renderBar();

      expect(screen.getByTestId("reverse-order")).toHaveAttribute(
        "href",
        "/filter?order=reverse",
      );
    });

    it("reads as pressed when the view is reversed", () => {
      renderBar({ current: { sort: "score", reversed: true } });

      expect(screen.getByTestId("reverse-order")).toHaveAttribute(
        "aria-pressed",
        "true",
      );
    });

    it("disables the toggle for a relevance sort", () => {
      // Best match has no reverse, so the control would do nothing.
      renderBar({ current: { sort: "searchMatch" } });

      expect(screen.getByTestId("reverse-order")).toHaveAttribute(
        "aria-disabled",
        "true",
      );
    });

    it("hides the reverse toggle when the sort is hidden", () => {
      renderBar({ showSort: false });

      expect(screen.queryByTestId("reverse-order")).toBeNull();
    });

    /// The Select lists are absolutely-positioned children that extend past
    /// the dropdown box. An `overflow-hidden` wrapper (added to round the
    /// corners) clipped them, so the open list was trimmed and unclickable.
    it("does not clip the panel's dropdowns", async () => {
      renderBar();
      await fireEvent.click(filtersButton());

      const panel = screen.getByTestId("filter-form");
      expect(panel.parentElement?.className).not.toMatch(/overflow-hidden/);
    });

    /// End-to-end through the custom Select: open the filter dropdown, open a
    /// select inside it, and choose. This is the path that was broken.
    it("lets a select inside the dropdown open and choose", async () => {
      renderBar();
      await fireEvent.click(filtersButton());

      await fireEvent.click(screen.getByRole("combobox", { name: "Type" }));
      expect(screen.getByRole("listbox", { name: "Type" })).toBeInTheDocument();

      await fireEvent.click(screen.getByRole("option", { name: "TV" }));

      // The choice landed in the field the form submits.
      const data = new FormData(
        screen.getByTestId("filter-form") as HTMLFormElement,
      );
      expect(data.get("format")).toBe("tv");

      expect(screen.getByTestId("filter-dropdown")).toBeInTheDocument();
    });

    /// In a real browser, choosing an option detaches the option from the DOM
    /// during the click, before the window handler runs. The handler then sees
    /// a detached `event.target`, which is no longer inside the container, and
    /// closed the whole panel. jsdom flushes the Svelte update after the event,
    /// so this has to remove the node mid-bubble to reproduce the timing.
    /// `composedPath()` is captured at dispatch, so it still contains the
    /// container; `event.target` does not.
    it("stays open when the clicked option detaches mid-click", async () => {
      renderBar();
      await fireEvent.click(filtersButton());

      const panel = screen.getByTestId("filter-dropdown");
      panel.addEventListener("click", (event) => {
        // Only an option click detaches: the combobox click that opens the list
        // must be left alone. Bubble phase on the panel runs before the window
        // handler, which is the timing the browser uses too.
        const target = event.target as Element | null;
        if (target?.getAttribute("role") === "option") target.remove();
      });

      await fireEvent.click(screen.getByRole("combobox", { name: "Type" }));
      await fireEvent.click(screen.getByRole("option", { name: "TV" }));

      expect(screen.getByTestId("filter-dropdown")).toBeInTheDocument();
    });

    it("carries extra parameters into the form", async () => {
      renderBar({ extraParams: { q: "naruto" } });
      await fireEvent.click(filtersButton());

      const data = new FormData(
        screen.getByTestId("filter-form") as HTMLFormElement,
      );

      // /search relies on this: without it a submit would reset the term.
      expect(data.get("q")).toBe("naruto");
    });

    it("points the form at the given base", async () => {
      renderBar({ base: "/search" });
      await fireEvent.click(filtersButton());

      expect(screen.getByTestId("filter-form")).toHaveAttribute(
        "action",
        "/search",
      );
    });
  });

  describe("the sort menu", () => {
    it("starts closed", () => {
      renderBar();

      expect(sortButton()).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByTestId("sort-dropdown")).toBeNull();
    });

    it("shows the current sort's label on the button", () => {
      renderBar({ current: { sort: "score" } });

      // The label is the provider's wording, not the wire value.
      expect(sortButton()).toHaveTextContent("Average Score");
    });

    it("opens when pressed", async () => {
      renderBar();

      await fireEvent.click(sortButton());

      expect(screen.getByTestId("sort-dropdown")).toBeInTheDocument();
    });

    it("offers every sort except relevance", async () => {
      renderBar();
      await fireEvent.click(sortButton());

      for (const label of [
        "Title",
        "Popularity",
        "Average Score",
        "Trending",
        "Favorites",
        "Date Added",
        "Release Date",
      ]) {
        expect(screen.getByRole("link", { name: label })).toBeInTheDocument();
      }

      // Relevance means nothing without a query, so it is never offered.
      expect(screen.queryByRole("link", { name: /best match/i })).toBeNull();
    });

    it("marks the active sort", async () => {
      renderBar({ current: { sort: "score" } });
      await fireEvent.click(sortButton());

      expect(
        screen.getByRole("link", { name: "Average Score" }),
      ).toHaveAttribute("aria-current", "true");
    });

    /// The sort is part of the URL, so choosing one is navigation: the link has
    /// to carry the current filters or they would be lost on the way.
    it("keeps the filters when changing sort", async () => {
      renderBar({
        params: new URLSearchParams("tag=Isekai&genre=Action"),
        current: { tags: ["Isekai"], genres: ["Action"], sort: "popularity" },
      });
      await fireEvent.click(sortButton());

      const href =
        screen.getByRole("link", { name: "Average Score" }).getAttribute("href") ??
        "";

      expect(href).toContain("sort=score");
      expect(href).toContain("tag=Isekai");
      expect(href).toContain("genre=Action");
    });

    it("drops the page number, which a new ordering invalidates", async () => {
      renderBar({ params: new URLSearchParams("page=5") });
      await fireEvent.click(sortButton());

      const href =
        screen.getByRole("link", { name: "Trending" }).getAttribute("href") ?? "";

      expect(href).not.toContain("page=");
    });

    it("links back to the given base", async () => {
      renderBar({ params: new URLSearchParams("q=naruto"), base: "/search" });
      await fireEvent.click(sortButton());

      const href =
        screen.getByRole("link", { name: "Popularity" }).getAttribute("href") ??
        "";

      expect(href.startsWith("/search?")).toBe(true);
    });

    it("is hidden when asked", () => {
      renderBar({ showSort: false });

      // /search ranks by relevance, so it offers no ordering.
      expect(screen.queryByTestId("open-sort")).toBeNull();
    });

    /// Opening one dropdown must close the other, or they would overlap.
    it("closes the filter dropdown when the sort menu opens", async () => {
      renderBar();

      await fireEvent.click(filtersButton());
      await fireEvent.click(sortButton());

      expect(screen.queryByTestId("filter-dropdown")).toBeNull();
      expect(screen.getByTestId("sort-dropdown")).toBeInTheDocument();
    });
  });
});