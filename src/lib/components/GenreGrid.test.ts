import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import GenreGrid from "./GenreGrid.svelte";

describe("GenreGrid", () => {
  it("renders nothing for an empty list", () => {
    render(GenreGrid, { props: { genres: [] } });

    expect(screen.queryByRole("heading")).toBeNull();
  });

  it("shows the section heading", () => {
    render(GenreGrid, { props: { genres: ["Action"] } });

    expect(
      screen.getByRole("heading", { name: /browse by genre/i }),
    ).toBeInTheDocument();
  });

  it("links each genre to its route", () => {
    render(GenreGrid, { props: { genres: ["Action", "Mecha"] } });

    expect(screen.getByRole("link", { name: "Action" })).toHaveAttribute(
      "href",
      "/genre/Action",
    );
    expect(screen.getByRole("link", { name: "Mecha" })).toHaveAttribute(
      "href",
      "/genre/Mecha",
    );
  });

  it("encodes a multi-word genre so the path stays valid", () => {
    render(GenreGrid, { props: { genres: ["Slice of Life"] } });

    expect(screen.getByRole("link", { name: "Slice of Life" })).toHaveAttribute(
      "href",
      "/genre/Slice%20of%20Life",
    );
  });
});