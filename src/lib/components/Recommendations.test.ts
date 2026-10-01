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

    // A card renders two links to the same work (poster + Watch); both must
    // point at the detail route.
    for (const link of screen.getAllByRole("link")) {
      expect(link).toHaveAttribute("href", "/anime/16498");
    }
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
});