import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import NavBar from "./NavBar.svelte";
import SearchBox from "./SearchBox.svelte";

describe("NavBar", () => {
  it("links the brand back to the home page", () => {
    render(NavBar);

    const brand = screen.getByRole("link", { name: "Kitsune" });
    expect(brand).toHaveAttribute("href", "/");
  });

  it("renders the search box", () => {
    render(NavBar);

    // By accessible role, so this fails if the search form loses its role
    // or its label.
    expect(screen.getByRole("search")).toBeInTheDocument();
  });

  it("links to the filter page", () => {
    render(NavBar);

    // The filter page is otherwise unreachable from the UI.
    expect(screen.getByRole("link", { name: "Filter" })).toHaveAttribute(
      "href",
      "/filter",
    );
  });

  it("links to the reader's own list", () => {
    render(NavBar);

    expect(screen.getByRole("link", { name: "My List" })).toHaveAttribute(
      "href",
      "/list",
    );
  });
});

describe("SearchBox", () => {
  it("submits a GET to /search", () => {
    render(SearchBox);

    const form = screen.getByRole("search");
    expect(form).toHaveAttribute("action", "/search");
    expect(form).toHaveAttribute("method", "GET");
  });

  it("names the field q, matching the search route's expectation", () => {
    render(SearchBox);

    // The route reads ?q=, so this name is part of the contract between
    // the two.
    expect(screen.getByRole("searchbox")).toHaveAttribute("name", "q");
  });

  it("labels the input for screen readers", () => {
    render(SearchBox);

    // getByRole with a name only succeeds when the label is associated.
    expect(screen.getByRole("searchbox", { name: "Search anime" })).toBeInTheDocument();
  });

  it("is a search input, so browsers offer the clear button", () => {
    render(SearchBox);

    expect(screen.getByRole("searchbox")).toHaveAttribute("type", "search");
  });
});