import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import MetadataStrip from "./MetadataStrip.svelte";
import type { Anime } from "$lib/types";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 1,
    provider: "anilist",
    title: { romaji: "Test" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

describe("MetadataStrip", () => {
  it("renders all fields when present", () => {
    render(MetadataStrip, {
      props: {
        anime: anime({
          format: "TV",
          episodeCount: 24,
          durationMinutes: 23,
          status: "FINISHED",
          seasonYear: 2023,
          averageScore: 85,
          popularity: 123456,
        }),
      },
    });

    expect(screen.getByText("TV")).toBeInTheDocument();
    expect(screen.getByText("24 eps")).toBeInTheDocument();
    expect(screen.getByText("23 min")).toBeInTheDocument();
    expect(screen.getByText("FINISHED")).toBeInTheDocument();
    expect(screen.getByText("2023")).toBeInTheDocument();
    expect(screen.getByText("85")).toBeInTheDocument();
    // 123456 with locale formatting -> "123,456"
    expect(screen.getByText("123,456")).toBeInTheDocument();
  });

  it("omits absent fields", () => {
    render(MetadataStrip, {
      props: {
        anime: anime({ format: "TV", seasonYear: 2023 }),
      },
    });

    expect(screen.getByText("TV")).toBeInTheDocument();
    expect(screen.getByText("2023")).toBeInTheDocument();
    expect(screen.queryByText(/eps/)).not.toBeInTheDocument();
    expect(screen.queryByText(/min/)).not.toBeInTheDocument();
    expect(screen.queryByText("FINISHED")).not.toBeInTheDocument();
    expect(screen.queryByText("123,456")).not.toBeInTheDocument();
  });

  it("formats episode count and duration with units", () => {
    render(MetadataStrip, {
      props: {
        anime: anime({ episodeCount: 12, durationMinutes: 24 }),
      },
    });

    expect(screen.getByText("12 eps")).toBeInTheDocument();
    expect(screen.getByText("24 min")).toBeInTheDocument();
  });

  it("renders nothing when all fields are absent", () => {
    render(MetadataStrip, { props: { anime: anime() } });

    expect(document.querySelector("dl")).not.toBeInTheDocument();
  });
});