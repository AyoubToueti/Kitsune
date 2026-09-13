import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import FilterPanel from "./FilterPanel.svelte";
import type { BrowseQuery } from "$lib/types";

const GENRES = ["Action", "Comedy", "Slice of Life"];

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