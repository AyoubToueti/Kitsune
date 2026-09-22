import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import VideoPlayer from "./VideoPlayer.svelte";

/**
 * jsdom implements neither `pause` nor `load`, and destroy calls both.
 *
 * Patched once at module scope rather than in `beforeEach`. Restoring them in
 * an `afterEach` does NOT work: vitest runs those hooks LIFO, so the restore
 * lands before @testing-library/svelte's cleanup, and teardown then reaches the
 * real unimplemented methods -- which is what fills the console with "Not
 * implemented" for every test that rendered a source.
 */
const pauseMock = vi.fn();
const loadMock = vi.fn();
HTMLMediaElement.prototype.pause = pauseMock;
HTMLMediaElement.prototype.load = loadMock;

beforeEach(() => {
  // Cleared, never restored: the patches must outlive each test.
  pauseMock.mockClear();
  loadMock.mockClear();
});

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

  it("does not show a buffering hint before playback starts", () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    // The poster is showing at this point. "Buffering…" over a still image
    // would claim something is loading when nothing has been asked to play.
    expect(screen.queryByText(/buffering/i)).toBeNull();
  });

  it("shows a buffering hint once playback starts but before it can play", async () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    // The hint is gated on playback having started: before that the poster is
    // showing, and "Buffering…" over a still image would be a lie.
    await fireEvent.play(document.querySelector("video")!);

    expect(screen.getByText(/buffering/i)).toBeInTheDocument();
  });

  it("hides the buffering hint once the video can play", async () => {
    render(VideoPlayer, { props: { src: "http://127.0.0.1:3030/x" } });

    const video = document.querySelector("video")!;
    await fireEvent.play(video);
    await fireEvent.canPlay(video);

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

  it("releases the stream when the component is destroyed", () => {
    const { unmount } = render(VideoPlayer, {
      props: { src: "http://127.0.0.1:3030/x" },
    });

    const video = document.querySelector("video")!;
    unmount();

    // Detaching the node does not stop WebKitGTK's pipeline, which is why
    // leaving the watch page used to leave the episode audible. Clearing the
    // source and reloading is what actually aborts the stream.
    expect(video.getAttribute("src")).toBeNull();
    expect(loadMock).toHaveBeenCalled();
  });

  it("does not touch the element when there was no source", () => {
    const { unmount } = render(VideoPlayer, { props: {} });

    unmount();

    // No element to release: the empty state renders no <video> at all.
    expect(loadMock).not.toHaveBeenCalled();
  });
});