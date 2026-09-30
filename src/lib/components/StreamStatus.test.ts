import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { TorrentProgress } from "$lib/types";

import StreamStatus from "./StreamStatus.svelte";

/** A snapshot with every field set, overridable per test. */
function progress(overrides: Partial<TorrentProgress> = {}): TorrentProgress {
  return {
    state: "live",
    progressBytes: 1_000,
    totalBytes: 2_000,
    fileProgress: [500, 500],
    finished: false,
    error: null,
    downloadMbps: 4.2,
    uploadMbps: 0.5,
    etaSeconds: 240,
    peersLive: 3,
    peersConnecting: 0,
    peersQueued: 0,
    peersSeen: 10,
    ...overrides,
  };
}

describe("StreamStatus", () => {
  it("shows the empty state when no torrent is loaded", () => {
    render(StreamStatus, { props: { empty: true } });

    expect(
      screen.getByText(/load a torrent to start watching/i),
    ).toBeInTheDocument();
    // The bars belong to a loaded torrent, so none should be drawn yet.
    expect(screen.queryByTestId("stream-status")).not.toBeInTheDocument();
  });

  it("renders both progress bars when a torrent is loaded", () => {
    render(StreamStatus, {
      props: {
        progress: progress(),
        fileFraction: 0.5,
        torrentFraction: 0.25,
      },
    });

    expect(screen.getByTestId("file-bar")).toBeInTheDocument();
    expect(screen.getByTestId("torrent-bar")).toBeInTheDocument();
  });

  it("labels the two bars with their own percentages", () => {
    render(StreamStatus, {
      props: {
        progress: progress(),
        // The chosen file is half done; the torrent as a whole is a quarter.
        fileFraction: 0.5,
        torrentFraction: 0.25,
      },
    });

    expect(screen.getByTestId("file-percent")).toHaveTextContent("50%");
    expect(screen.getByTestId("torrent-percent")).toHaveTextContent("25%");
  });

  it("reports a live peer count as connected", () => {
    render(StreamStatus, { props: { progress: progress({ peersLive: 3 }) } });

    expect(screen.getByTestId("connection")).toHaveTextContent(
      /connected \(3 peers\)/i,
    );
  });

  it("reports connecting while a handshake is in flight", () => {
    render(StreamStatus, {
      props: {
        progress: progress({ peersLive: 0, peersConnecting: 2 }),
      },
    });

    expect(screen.getByTestId("connection")).toHaveTextContent(/connecting/i);
  });

  it("reports searching when no peer is known yet", () => {
    render(StreamStatus, {
      props: {
        progress: progress({ peersLive: 0, peersConnecting: 0, peersQueued: 0 }),
      },
    });

    expect(screen.getByTestId("connection")).toHaveTextContent(
      /searching for peers/i,
    );
  });

  it("formats the download speed in MiB/s", () => {
    render(StreamStatus, { props: { progress: progress({ downloadMbps: 4.2 }) } });

    expect(screen.getByTestId("down-speed")).toHaveTextContent("4.2 MiB/s");
  });

  it("shows a dash for a stalled download speed", () => {
    render(StreamStatus, { props: { progress: progress({ downloadMbps: 0 }) } });

    expect(screen.getByTestId("down-speed")).toHaveTextContent("—");
  });

  it("formats a multi-minute ETA", () => {
    render(StreamStatus, { props: { progress: progress({ etaSeconds: 240 }) } });

    expect(screen.getByTestId("eta")).toHaveTextContent("4m 0s");
  });

  it("shows a dash when there is no ETA to report", () => {
    render(StreamStatus, { props: { progress: progress({ etaSeconds: null }) } });

    expect(screen.getByTestId("eta")).toHaveTextContent("—");
  });

  it("renders the downloaded and total bytes", () => {
    render(StreamStatus, {
      props: { progress: progress({ progressBytes: 1_500_000, totalBytes: 3_000_000 }) },
    });

    // 1_500_000 / 3_000_000 bytes, in binary units.
    expect(screen.getByTestId("bytes")).toHaveTextContent(/1.4 MB \/ 2.9 MB/);
  });

  it("surfaces a torrent error message", () => {
    render(StreamStatus, {
      props: {
        progress: progress({ state: "error", error: "tracker unreachable" }),
      },
    });

    expect(screen.getByText(/tracker unreachable/i)).toBeInTheDocument();
  });

  it("says stats are pending before the first poll lands", () => {
    render(StreamStatus, { props: { progress: null } });

    expect(screen.getByTestId("connection")).toHaveTextContent(
      /waiting for stats/i,
    );
  });

  it("clamps a fraction above 1 to 100%", () => {
    render(StreamStatus, {
      props: { progress: progress(), fileFraction: 1.5, torrentFraction: 2 },
    });

    expect(screen.getByTestId("file-percent")).toHaveTextContent("100%");
    expect(screen.getByTestId("torrent-percent")).toHaveTextContent("100%");
  });
});