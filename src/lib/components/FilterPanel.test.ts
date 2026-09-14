import { describe, it, expect } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";

import FilterPanel from "./FilterPanel.svelte";
import type { BrowseQuery, MediaTag } from "$lib/types";

const GENRES = ["Action", "Comedy", "Slice of Life"];

const TAGS: MediaTag[] = [
  { name: "Isekai", category: "Theme-Fantasy" },
  { name: "Magic", category: "Theme-Fantasy" },
  { name: "School", category: "Setting-Scene" },
  { name: "Shounen", category: "Demographic" },
];

/** The form element, which carries the submit target. */
function form(): HTMLElement {
  return screen.getByTestId("filter-form");
}

/** A labelled control, by its visible label text. */
function control(label: string): HTMLElement {
  return screen.getByLabelText(label) as HTMLElement;
}

describe("FilterPanel", () => {
  it("submits as a GET to /filter", () => {
    render(FilterPanel, { props: { genres: GENRES, current: { sort: "popularity" } } });

    // A plain GET form: the browser builds the query string, so the page works
    // without JavaScript.
    expect(form()).toHaveAttribute("action", "/filter");
    expect(form()).toHaveAttribute("method", "GET");
  });

  it("offers the default sort when nothing is selected", () => {
    render(FilterPanel, { props: { genres: GENRES, current: { sort: "popularity" } } });

    expect(control("Sort")).toHaveValue("popularity");
  });

  it("reflects the applied filters", () => {
    const current: BrowseQuery = {
      format: "tv",
      status: "finished",
      season: "fall",
      seasonYear: 2024,
      minScore: 70,
      sort: "score",
    };

    render(FilterPanel, { props: { genres: GENRES, current } });

    expect(control("Type")).toHaveValue("tv");
    expect(control("Status")).toHaveValue("finished");
    expect(control("Season")).toHaveValue("fall");
    expect(control("Year")).toHaveValue("2024");
    expect(control("Score")).toHaveValue("70");
    expect(control("Sort")).toHaveValue("score");
  });

  it("shows the search term", () => {
    render(FilterPanel, {
      props: { genres: GENRES, current: { search: "naruto", sort: "searchMatch" } },
    });

    expect(control("Search")).toHaveValue("naruto");
  });

  it("renders a checkbox per genre", () => {
    render(FilterPanel, { props: { genres: GENRES, current: { sort: "popularity" } } });

    for (const genre of GENRES) {
      expect(screen.getByRole("checkbox", { name: genre })).toBeInTheDocument();
    }
  });

  it("checks the genres that are applied", () => {
    render(FilterPanel, {
      props: {
        genres: GENRES,
        current: { genres: ["Action", "Comedy"], sort: "popularity" },
      },
    });

    expect(screen.getByRole("checkbox", { name: "Action" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Comedy" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Slice of Life" })).not.toBeChecked();
  });

  it("renders no genre section when the provider gave none", () => {
    render(FilterPanel, { props: { genres: [], current: { sort: "popularity" } } });

    // An empty fieldset would be a heading with nothing under it.
    expect(screen.queryByRole("checkbox")).toBeNull();
  });

  it("does not offer relevance sort, which needs a search term", () => {
    render(FilterPanel, { props: { genres: GENRES, current: { sort: "popularity" } } });

    // `searchMatch` orders by textual relevance and means nothing without a
    // term, so the filter page must not offer it.
    expect(
      screen.queryByRole("option", { name: /best match/i }),
    ).toBeNull();
  });

  it("offers every format", () => {
    render(FilterPanel, { props: { genres: GENRES, current: { sort: "popularity" } } });

    for (const label of ["TV", "Movie", "OVA", "ONA", "Special", "Music"]) {
      expect(screen.getByRole("option", { name: label })).toBeInTheDocument();
    }
  });

  it("renders no catalogue section when there is nothing to show", () => {
    render(FilterPanel, {
      props: { genres: [], tags: [], current: { sort: "popularity" } },
    });

    // An empty fieldset would be a heading with nothing under it.
    expect(screen.queryByRole("group", { name: /genres/i })).toBeNull();
  });

  it("offers a tag search box when a catalogue exists", () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    expect(screen.getByLabelText("Filter tags")).toBeInTheDocument();
  });

  it("offers no tag search box when there are no tags", () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: [], current: { sort: "popularity" } },
    });

    // A search box that can never match anything is worse than none.
    expect(screen.queryByLabelText("Filter tags")).toBeNull();
  });

  it("groups tags under their category", () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    // Categories arrive as provider keys and are humanised for display.
    for (const label of ["Theme / Fantasy", "Setting / Scene", "Demographic"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it("renders a checkbox per tag", () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    for (const tag of TAGS) {
      expect(screen.getByRole("checkbox", { name: tag.name })).toBeInTheDocument();
    }
  });

  it("checks the tags that are applied", () => {
    render(FilterPanel, {
      props: {
        genres: GENRES,
        tags: TAGS,
        current: { tags: ["Isekai"], sort: "popularity" },
      },
    });

    expect(screen.getByRole("checkbox", { name: "Isekai" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Magic" })).not.toBeChecked();
  });

  it("shows every category by default", () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    for (const label of ["Theme / Fantasy", "Setting / Scene", "Demographic"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it("narrows the catalogue to tags matching the search term", async () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    await fireEvent.input(screen.getByLabelText("Filter tags"), {
      target: { value: "school" },
    });

    expect(screen.getByRole("checkbox", { name: "School" })).toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: "Isekai" })).toBeNull();
  });

  it("matches a search term case-insensitively", async () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    await fireEvent.input(screen.getByLabelText("Filter tags"), {
      target: { value: "ISeK" },
    });

    expect(screen.getByRole("checkbox", { name: "Isekai" })).toBeInTheDocument();
  });

  /// A heading with no chips under it is just noise once a search is active.
  it("hides a category whose tags all fail to match", async () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    await fireEvent.input(screen.getByLabelText("Filter tags"), {
      target: { value: "school" },
    });

    expect(screen.getByText("Setting / Scene")).toBeInTheDocument();
    expect(screen.queryByText("Theme / Fantasy")).toBeNull();
  });

  it("says so when a search term matches nothing", async () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    await fireEvent.input(screen.getByLabelText("Filter tags"), {
      target: { value: "zzzz" },
    });

    // Silence would look like a broken list rather than an empty result.
    expect(screen.getByText(/no tags match/i)).toBeInTheDocument();
  });

  it("keeps the catalogue hidden behind the search box out of the submission", () => {
    render(FilterPanel, {
      props: { genres: GENRES, tags: TAGS, current: { sort: "popularity" } },
    });

    // The search box narrows the view; if it were submitted it would become a
    // filter the backend never asked for.
    expect(screen.getByLabelText("Filter tags")).not.toHaveAttribute("name");
  });

  it("has a submit button", () => {
    render(FilterPanel, { props: { genres: GENRES, current: { sort: "popularity" } } });

    expect(screen.getByRole("button", { name: "Filter" })).toBeInTheDocument();
  });

  it("offers a reset back to the unfiltered view", () => {
    render(FilterPanel, { props: { genres: GENRES, current: { sort: "popularity" } } });

    expect(screen.getByRole("link", { name: "Reset" })).toHaveAttribute(
      "href",
      "/filter",
    );
  });
});