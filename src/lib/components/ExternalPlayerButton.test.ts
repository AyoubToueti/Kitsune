import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

const openInPlayerMock = vi.hoisted(() => vi.fn());

vi.mock("$lib/api/player", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/player")>("$lib/api/player");
  return {
    ...actual,
    openInPlayer: openInPlayerMock,
  };
});

import ExternalPlayerButton from "./ExternalPlayerButton.svelte";

/** The launch button. */
const launchButton = () =>
  screen.getByRole("button", { name: /open in external player/i });

beforeEach(() => {
  openInPlayerMock.mockReset().mockResolvedValue("mpv");
});

describe("ExternalPlayerButton", () => {
  it("is disabled without a url", () => {
    render(ExternalPlayerButton, { props: {} });

    expect(launchButton()).toBeDisabled();
  });

  it("is enabled once a url is given", () => {
    render(ExternalPlayerButton, { props: { url: "http://127.0.0.1:3030/x" } });

    expect(launchButton()).toBeEnabled();
  });

  it("launches with the stored player (no explicit argument)", async () => {
    render(ExternalPlayerButton, { props: { url: "http://127.0.0.1:3030/x" } });

    await fireEvent.click(launchButton());

    // No player argument: the backend uses the reader's stored choice, so a
    // change in settings applies without this component knowing about it.
    // `undefined` for the id rather than absent: no torrent means nothing to hold.
    expect(openInPlayerMock).toHaveBeenCalledWith(
      "http://127.0.0.1:3030/x",
      undefined,
      undefined,
    );
  });

  it("passes the torrent id so the backend can hold it", async () => {
    render(ExternalPlayerButton, {
      props: { url: "http://127.0.0.1:3030/x", torrentId: 5 },
    });

    await fireEvent.click(launchButton());

    expect(openInPlayerMock).toHaveBeenCalledWith(
      "http://127.0.0.1:3030/x",
      undefined,
      5,
    );
  });

  it("surfaces a launch failure", async () => {
    openInPlayerMock.mockRejectedValue("failed to launch `mpv`");
    render(ExternalPlayerButton, { props: { url: "http://x" } });

    await fireEvent.click(launchButton());

    expect(await screen.findByText(/failed to launch/i)).toBeInTheDocument();
  });
});