import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import TopList from "./TopList.svelte";
import type { Anime } from "$lib/types";

function anime(id: number, title: string, extras: Partial<Anime> = {}): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...extras,
  };
}

const ITEMS = [
  anime(1, "Sousou no Frieren", {
    averageScore: 91,
    popularity: 479645,
    format: "TV",
    episodeCount: 28,
    season: "FALL",
    seasonYear: 2023,
    genres: ["Adventure", "Drama"],
  }),
  anime(2, "Gintama: THE FINAL", { averageScore: 91, format: "MOVIE" }),
];

describe("TopList", () => {
  it("renders a row per title", () => {
    render(TopList, { props: { anime: ITEMS } });

    expect(screen.getByText("Sousou no Frieren")).toBeInTheDocument();
    expect(screen.getByText("Gintama: THE FINAL")).toBeInTheDocument();
  });

  it("numbers the rows from one", () => {
    render(TopList, { props: { anime: ITEMS } });

    const ranks = screen.getAllByTestId("rank").map((el) => el.textContent?.trim());
    expect(ranks).toEqual(["1", "2"]);
  });

  /// A later page must continue the numbering, or every page would restart at 1
  /// and the ranking would read as a set of ties.
  it("continues the numbering from the given rank", () => {
    render(TopList, { props: { anime: ITEMS, startRank: 26 } });

    const ranks = screen.getAllByTestId("rank").map((el) => el.textContent?.trim());
    expect(ranks).toEqual(["26", "27"]);
  });

  it("shows the score and popularity", () => {
    render(TopList, { props: { anime: ITEMS } });

    // Both fixtures score 91, so the assertion is scoped to one row rather than
    // matching whichever the query happened to reach first.
    const row = screen.getByText("Sousou no Frieren").closest("a");
    expect(row).toHaveTextContent("91%");
    expect(row).toHaveTextContent("479,645 users");
  });

  it("joins the season and year into one label", () => {
    render(TopList, { props: { anime: ITEMS } });

    // The provider spells it "FALL"; the label is humanised.
    expect(screen.getByText("Fall 2023")).toBeInTheDocument();
  });

  it("renders a genre chip per genre", () => {
    render(TopList, { props: { anime: ITEMS } });

    expect(screen.getByText("Adventure")).toBeInTheDocument();
    expect(screen.getByText("Drama")).toBeInTheDocument();
  });

  it("links each row to its detail page", () => {
    render(TopList, { props: { anime: ITEMS } });

    expect(screen.getByText("Sousou no Frieren").closest("a")).toHaveAttribute(
      "href",
      "/anime/1",
    );
  });

  /// A title with none of the optional facts must still render a row rather
  /// than an empty one or a crash.
  it("tolerates a title with no score, format or season", () => {
    render(TopList, { props: { anime: [anime(9, "Bare Entry")] } });

    expect(screen.getByText("Bare Entry")).toBeInTheDocument();
    expect(screen.queryByText(/%$/)).toBeNull();
  });
});