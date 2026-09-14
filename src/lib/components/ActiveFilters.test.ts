import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import ActiveFilters from "./ActiveFilters.svelte";
import type { BrowseQuery } from "$lib/types";

/** Render the bar against a query string, as the page would. */
function renderWith(qs: string, current: BrowseQuery) {
  const params = new URLSearchParams(qs);
  render(ActiveFilters, { props: { params, current } });
  return params;
}

/** The bar itself, for asserting on presence. */
function bar(): HTMLElement | null {
  return screen.queryByTestId("active-filters");
}

describe("ActiveFilters", () => {
  it("renders nothing when no filters are applied", () => {
    renderWith("", { sort: "popularity" });

    // An empty bar would be chrome with nothing to say.
    expect(bar()).toBeNull();
  });

  it("shows a pill per applied tag", () => {
    renderWith("tag=Isekai&tag=School", {
      tags: ["Isekai", "School"],
      sort: "popularity",
    });

    expect(screen.getByRole("link", { name: /remove isekai filter/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /remove school filter/i })).toBeInTheDocument();
  });

  it("labels an excluded tag so it cannot be read as a requirement", () => {
    renderWith("exclude_tag=Harem", {
      excludedTags: ["Harem"],
      sort: "popularity",
    });

    // A bare "Harem" pill would be indistinguishable from an inclusion.
    expect(screen.getByText("Not Harem")).toBeInTheDocument();
  });

  it("shows the search term as a pill", () => {
    renderWith("search=naruto", { search: "naruto", sort: "popularity" });

    expect(screen.getByText("“naruto”")).toBeInTheDocument();
  });

  it("labels enum filters with their human names", () => {
    renderWith("format=tv&status=finished", {
      format: "tv",
      status: "finished",
      sort: "popularity",
    });

    // "tv" and "finished" are wire values, not something to show a user.
    expect(screen.getByText("TV")).toBeInTheDocument();
    expect(screen.getByText("Finished airing")).toBeInTheDocument();
  });

  it("removes only the clicked filter", () => {
    renderWith("tag=Isekai&tag=School", {
      tags: ["Isekai", "School"],
      sort: "popularity",
    });

    const href = screen
      .getByRole("link", { name: /remove isekai filter/i })
      .getAttribute("href");

    // The other tag must survive: dropping every `tag` would be the easy bug.
    expect(href).toContain("tag=School");
    expect(href).not.toContain("Isekai");
  });

  it("drops the page number when removing a filter", () => {
    renderWith("tag=Isekai&page=5", { tags: ["Isekai"], sort: "popularity" });

    const href = screen
      .getByRole("link", { name: /remove isekai filter/i })
      .getAttribute("href");

    // Page 5 of a different filter set is not a meaningful place to land.
    expect(href).not.toContain("page=");
  });

  it("offers Clear all only when there is more than one filter", () => {
    renderWith("tag=Isekai", { tags: ["Isekai"], sort: "popularity" });
    expect(screen.queryByTestId("clear-all")).toBeNull();
  });

  it("Clear all keeps parameters it does not own", () => {
    renderWith("tag=Isekai&q=naruto", { tags: ["Isekai"], sort: "popularity" });

    // `q` belongs to /search. Clearing filters must not clear the search term.
    expect(screen.queryByTestId("clear-all")).toBeNull();
  });

  it("removes every filter when Clear all is followed", () => {
    renderWith("tag=Isekai&format=tv", {
      tags: ["Isekai"],
      format: "tv",
      sort: "popularity",
    });

    const href = screen.getByTestId("clear-all").getAttribute("href");

    expect(href).not.toContain("tag=");
    expect(href).not.toContain("format=");
  });

  it("points removal links at the given base path", () => {
    const params = new URLSearchParams("tag=Isekai");
    render(ActiveFilters, {
      props: { params, current: { tags: ["Isekai"], sort: "popularity" }, base: "/search" },
    });

    expect(
      screen.getByRole("link", { name: /remove isekai filter/i }),
    ).toHaveAttribute("href", "/search");
  });
});