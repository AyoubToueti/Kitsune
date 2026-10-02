import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { RecommendedAnime } from "$lib/types";

import Recommendations from "./Recommendations.svelte";

function recommendation(
  id: number,
  title: string,
  rating = 10,
): RecommendedAnime {
  return {
    anime: {
      id,
      provider: "anilist",
      title: { romaji: title },
      genres: [],
      streamingEpisodes: [],
      relations: [],
      recommendations: [],
    },
    rating,
  };
}

describe("Recommendations", () => {
  it("renders one card per recommendation", () => {
    render(Recommendations, {
      props: {
        recommendations: [
          recommendation(1, "Fullmetal Alchemist"),
          recommendation(2, "Steins;Gate"),
        ],
      },
    });

    expect(screen.getByText("Fullmetal Alchemist")).toBeInTheDocument();
    expect(screen.getByText("Steins;Gate")).toBeInTheDocument();
  });

  it("links each card to the recommended anime", () => {
    render(Recommendations, {
      props: { recommendations: [recommendation(16498, "Fullmetal Alchemist")] },
    });

    // A card renders two links to the same work (poster + Watch). The poster
    // points at the detail route; the Watch button starts at the first episode.
    const hrefs = screen
      .getAllByRole("link")
      .map((link) => link.getAttribute("href"));
    expect(hrefs).toContain("/anime/16498");
    expect(hrefs).toContain("/anime/16498?ep=0");
  });

  it("renders nothing when the list is empty", () => {
    const { container } = render(Recommendations, {
      props: { recommendations: [] },
    });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /recommended/i })).toBeNull();
  });

  it("renders nothing when the prop is undefined", () => {
    const { container } = render(Recommendations, { props: {} });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
  });

  it("links to the full recommendations page when given the work id", () => {
    render(Recommendations, {
      props: {
        recommendations: [recommendation(1, "Fullmetal Alchemist")],
        workId: 42,
      },
    });

    expect(screen.getByTestId("view-more-recommendations")).toHaveAttribute(
      "href",
      "/anime/42/recommendations",
    );
  });

  it("omits the view-more link without a work id", () => {
    // The row is rendered in isolation by every other test here, so the link
    // must not depend on the recommendations themselves.
    render(Recommendations, {
      props: { recommendations: [recommendation(1, "Fullmetal Alchemist")] },
    });

    expect(screen.queryByTestId("view-more-recommendations")).toBeNull();
  });

  it("offers voting when given a work id", () => {
    render(Recommendations, {
      props: {
        recommendations: [recommendation(1, "Fullmetal Alchemist")],
        workId: 42,
      },
    });

    expect(screen.getByTestId("vote-up")).toBeInTheDocument();
    expect(screen.getByTestId("vote-down")).toBeInTheDocument();
  });

  it("omits voting without a work id", () => {
    // No base work means no (base, recommended) pair to key a vote by.
    render(Recommendations, {
      props: { recommendations: [recommendation(1, "Fullmetal Alchemist")] },
    });

    expect(screen.queryByTestId("vote-up")).toBeNull();
  });

  it("caps the row at six recommendations", () => {
    // Eight recommendations but only six slots. jsdom does not apply the
    // responsive breakpoints, so all six slots are in the DOM and the seventh
    // and eighth are never rendered at all.
    const many = Array.from({ length: 8 }, (_, i) =>
      recommendation(i + 1, `Title ${i + 1}`),
    );

    render(Recommendations, { props: { recommendations: many } });

    expect(screen.getAllByTestId("anime-card")).toHaveLength(6);
    expect(screen.queryByText("Title 7")).toBeNull();
    expect(screen.queryByText("Title 8")).toBeNull();
  });

  it("reveals the extra slots by breakpoint rather than scrolling", () => {
    const { container } = render(Recommendations, {
      props: {
        recommendations: Array.from({ length: 6 }, (_, i) =>
          recommendation(i + 1, `Title ${i + 1}`),
        ),
      },
    });

    // No horizontal scroller: the row wraps and the late slots are hidden until
    // the window is wide enough.
    expect(container.querySelector("ul")?.className).not.toMatch(
      /overflow-x-auto/,
    );
    const items = container.querySelectorAll("li");
    expect(items[5].className).toMatch(/hidden/);
    expect(items[5].className).toMatch(/xl:block/);
  });
});