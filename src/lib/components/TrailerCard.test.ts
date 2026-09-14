import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import type { Trailer } from "$lib/types";

const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: openUrlMock,
}));

import TrailerCard from "./TrailerCard.svelte";

function trailer(overrides: Partial<Trailer> = {}): Trailer {
  return {
    id: "LHtdKWJdif4",
    site: "youtube",
    thumbnail: "https://example.test/trailer.jpg",
    ...overrides,
  };
}

describe("TrailerCard", () => {
  it("renders a play button when a trailer is present", () => {
    render(TrailerCard, { props: { trailer: trailer() } });

    expect(screen.getByRole("button", { name: /play trailer/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /trailer/i })).toBeInTheDocument();
  });

  it("opens an in-app modal with the YouTube embed", async () => {
    render(TrailerCard, { props: { trailer: trailer({ site: "youtube", id: "abc123" }) } });

    await fireEvent.click(screen.getByRole("button", { name: /play trailer/i }));

    const frame = screen.getByTitle("Trailer Player");
    expect(frame.getAttribute("src")).toBe(
      "https://www.youtube.com/embed/abc123?autoplay=1",
    );
  });

  it("opens an in-app modal with the Dailymotion embed", async () => {
    render(TrailerCard, {
      props: { trailer: trailer({ site: "dailymotion", id: "x9abc" }) },
    });

    await fireEvent.click(screen.getByRole("button", { name: /play trailer/i }));

    const frame = screen.getByTitle("Trailer Player");
    expect(frame.getAttribute("src")).toBe(
      "https://www.dailymotion.com/embed/video/x9abc?autoplay=1",
    );
  });

  it("is case-insensitive about the site", async () => {
    render(TrailerCard, { props: { trailer: trailer({ site: "YouTube" }) } });

    await fireEvent.click(screen.getByRole("button", { name: /play trailer/i }));

    expect(screen.getByTitle("Trailer Player").getAttribute("src")).toContain(
      "youtube.com/embed",
    );
  });

  it("closes the modal when Escape is pressed", async () => {
    render(TrailerCard, { props: { trailer: trailer() } });

    await fireEvent.click(screen.getByRole("button", { name: /play trailer/i }));
    expect(screen.getByTitle("Trailer Player")).toBeInTheDocument();

    await fireEvent.keyDown(window, { key: "Escape" });

    expect(screen.queryByTitle("Trailer Player")).toBeNull();
  });

  it("renders nothing for an unknown site, since there is no url to build", () => {
    const { container } = render(TrailerCard, {
      props: { trailer: trailer({ site: "vimeo" }) },
    });

    expect(container.querySelector("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /trailer/i })).toBeNull();
  });

  it("renders nothing when the trailer is absent", () => {
    const { container } = render(TrailerCard, { props: {} });

    expect(container.querySelector("button")).not.toBeInTheDocument();
  });

  it("renders the thumbnail when present", () => {
    render(TrailerCard, {
      props: { trailer: trailer({ thumbnail: "https://example.test/thumb.jpg" }) },
    });

    expect(document.querySelector("img")?.getAttribute("src")).toBe(
      "https://example.test/thumb.jpg",
    );
  });
});