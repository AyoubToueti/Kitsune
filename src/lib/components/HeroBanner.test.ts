import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import HeroBanner from "./HeroBanner.svelte";
import type { Anime } from "$lib/types";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

/** Every rendered image src, for asserting which image was chosen. */
function imgSources(): string[] {
  return Array.from(document.querySelectorAll("img")).map(
    (img) => img.getAttribute("src") ?? "",
  );
}

describe("HeroBanner", () => {
  it("shows the title as a heading", () => {
    render(HeroBanner, { props: { anime: anime() } });

    expect(
      screen.getByRole("heading", { name: "One Piece" }),
    ).toBeInTheDocument();
  });

  it("links to the detail route for its id", () => {
    render(HeroBanner, { props: { anime: anime({ id: 7 }) } });

    expect(screen.getByRole("link", { name: "View details" })).toHaveAttribute(
      "href",
      "/anime/7",
    );
  });

  it("uses the banner as the backdrop when present", () => {
    render(HeroBanner, {
      props: {
        anime: anime({
          bannerImage: "https://example.test/banner.jpg",
          coverImage: "https://example.test/cover.jpg",
        }),
      },
    });

    expect(imgSources()).toContain("https://example.test/banner.jpg");
  });

  it("falls back to the cover when there is no banner", () => {
    render(HeroBanner, {
      props: { anime: anime({ coverImage: "https://example.test/cover.jpg" }) },
    });

    expect(imgSources()).toContain("https://example.test/cover.jpg");
  });

  it("renders no images when neither banner nor cover is present", () => {
    render(HeroBanner, { props: { anime: anime() } });

    expect(imgSources()).toHaveLength(0);
  });

  it("shows score, format and year", () => {
    render(HeroBanner, {
      props: {
        anime: anime({ averageScore: 88, format: "TV", seasonYear: 1999 }),
      },
    });

    expect(screen.getByText("88")).toBeInTheDocument();
    expect(screen.getByText("TV")).toBeInTheDocument();
    expect(screen.getByText("1999")).toBeInTheDocument();
  });

  it("lists at most four genres", () => {
    render(HeroBanner, {
      props: { anime: anime({ genres: ["A", "B", "C", "D", "E", "F"] }) },
    });

    for (const shown of ["A", "B", "C", "D"]) {
      expect(screen.getByText(shown)).toBeInTheDocument();
    }
    expect(screen.queryByText("E")).not.toBeInTheDocument();
  });

  it("strips html from the synopsis instead of rendering it", () => {
    render(HeroBanner, {
      props: { anime: anime({ description: "<i>A</i> pirate<br>adventure." }) },
    });

    expect(screen.getByText("A pirate adventure.")).toBeInTheDocument();
    // The raw markup must not survive anywhere in the DOM.
    expect(document.body.innerHTML).not.toContain("<i>");
  });

  it("omits the synopsis when the description is only markup", () => {
    render(HeroBanner, { props: { anime: anime({ description: "<br>" }) } });

    expect(document.querySelectorAll("p")).toHaveLength(0);
  });
});