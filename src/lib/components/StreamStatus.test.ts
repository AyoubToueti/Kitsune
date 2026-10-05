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

  it("leads with a buffering headline, not a download one", () => {
    render(StreamStatus, { props: { progress: progress() } });

    // The app only buffers enough to hand to the player; "downloading" would
    // overstate what is happening.
    expect(screen.getByText(/buffering/i)).toBeInTheDocument();
    expect(screen.queryByText(/downloading/i)).not.toBeInTheDocument();
  });

  it("shows a stall warning when bytes stop arriving", () => {
    render(StreamStatus, {
      props: { progress: progress(), staleSeconds: 30 },
    });

    const warning = screen.getByTestId("stream-warning");
    expect(warning).toHaveTextContent(/stalled/i);
    expect(warning).toHaveAttribute("data-level", "error");
  });

  it("shows no warning for a healthy download", () => {
    render(StreamStatus, { props: { progress: progress() } });
    expect(screen.queryByTestId("stream-warning")).toBeNull();
  });

  it("draws a sparkline once there is more than one sample", () => {
    render(StreamStatus, {
      props: { progress: progress(), speedHistory: [1, 2, 3, 2, 0] },
    });

    expect(screen.getByTestId("speed-spark")).toBeInTheDocument();
  });

  it("does not draw a sparkline for a single sample", () => {
    render(StreamStatus, {
      props: { progress: progress(), speedHistory: [1] },
    });

    expect(screen.queryByTestId("speed-spark")).toBeNull();
  });

  it("marks the playhead when a played fraction is given", () => {
    render(StreamStatus, {
      props: { progress: progress(), fileFraction: 0.5, playedFraction: 0.2 },
    });

    expect(screen.getByTestId("played-marker")).toBeInTheDocument();
  });

  it("shows a health word derived from the peers and speed", () => {
    // Slow needs both few peers and a slow connection.
    render(StreamStatus, {
      props: { progress: progress({ peersLive: 2, downloadMbps: 0.2 }) },
    });
    expect(screen.getByTestId("health")).toHaveTextContent(/slow/i);
  });

  it("shows a green notice when the episode is fully buffered", () => {
    render(StreamStatus, {
      props: {
        progress: progress(),
        fileFraction: 1,
        // A full file would otherwise trip the stall check.
        staleSeconds: 999,
      },
    });

    const notice = screen.getByTestId("stream-warning");
    expect(notice).toHaveAttribute("data-level", "ok");
    expect(notice).toHaveTextContent(/fully buffered/i);
  });

  it("hides the swarm stats once the episode is fully buffered", () => {
    render(StreamStatus, {
      props: { progress: progress(), fileFraction: 1, staleSeconds: 999 },
    });

    // The torrent is idle once complete, so peers/speed/ETA/health would all
    // read as a dash. They are hidden rather than shown as noise.
    expect(screen.queryByTestId("connection")).toBeNull();
    expect(screen.queryByTestId("down-speed")).toBeNull();
    expect(screen.queryByTestId("eta")).toBeNull();
    expect(screen.queryByTestId("health")).toBeNull();
  });

  it("keeps the swarm stats while still buffering", () => {
    render(StreamStatus, { props: { progress: progress(), fileFraction: 0.4 } });

    expect(screen.getByTestId("connection")).toBeInTheDocument();
    expect(screen.getByTestId("health")).toBeInTheDocument();
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

  it("shows the chosen file's bytes when they are given", () => {
    render(StreamStatus, {
      props: {
        progress: progress({ progressBytes: 2_000_000, totalBytes: 25_000_000_000 }),
        // The whole torrent is 25 GB, but this one episode is 1.4 GB and 200 MB
        // of it has arrived. The reader cares about the episode, not the pack.
        fileBytes: 200_000_000,
        fileTotalBytes: 1_400_000_000,
      },
    });

    expect(screen.getByTestId("bytes")).toHaveTextContent(/191 MB \/ 1.3 GB/);
    expect(screen.getByTestId("bytes")).not.toHaveTextContent(/25/);
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

  it("shows a Paused headline and connection when paused", () => {
    render(StreamStatus, {
      props: { progress: progress(), paused: true },
    });

    // Both the headline and the connection line read "Paused", so match all.
    expect(screen.getAllByText(/paused/i).length).toBeGreaterThan(0);
    expect(screen.getByTestId("connection")).toHaveTextContent(/paused/i);
  });
});