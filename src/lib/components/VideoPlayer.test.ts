import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import VideoPlayer from "./VideoPlayer.svelte";

describe("VideoPlayer", () => {
  it("shows the empty state when there is no source", () => {
    render(VideoPlayer, { props: {} });

    expect(screen.getByText(/load a torrent/i)).toBeInTheDocument();
    expect(document.querySelector("video")).toBeNull();
  });

  it("renders a video element when given a source", () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    const video = document.querySelector("video");
    expect(video).not.toBeNull();
    expect(video?.getAttribute("src")).toBe("http://127.0.0.1:3030/x");
  });

  it("shows a buffering hint until the video can play", () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    expect(screen.getByText(/buffering/i)).toBeInTheDocument();
  });

  it("hides the buffering hint once the video can play", async () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    await fireEvent.canPlay(document.querySelector("video")!);

    expect(screen.queryByText(/buffering/i)).toBeNull();
  });

  it("reports a playback failure rather than a black frame", async () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    await fireEvent.error(document.querySelector("video")!);

    expect(screen.getByText(/could not play/i)).toBeInTheDocument();
    // The external player is the suggested escape hatch.
    expect(screen.getByText(/external player/i)).toBeInTheDocument();
  });

  it("uses the given title as the accessible name", () => {
    render(VideoPlayer, {
      props: { src: "http://127.0.0.1:3030/x", title: "Episode 3" },
    });

    expect(document.querySelector("video")?.getAttribute("title")).toBe("Episode 3");
  });

  it("falls back to a generic accessible name", () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    expect(document.querySelector("video")?.getAttribute("title")).toBe("Video player");
  });
});