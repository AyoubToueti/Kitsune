import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

const openInPlayerMock = vi.hoisted(() => vi.fn());
const getPlayerMock = vi.hoisted(() => vi.fn());
const setPlayerMock = vi.hoisted(() => vi.fn());
const suggestedPlayersMock = vi.hoisted(() => vi.fn());

vi.mock("$lib/api/player", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/player")>("$lib/api/player");
  return {
    ...actual,
    openInPlayer: openInPlayerMock,
    getPlayer: getPlayerMock,
    setPlayer: setPlayerMock,
    suggestedPlayers: suggestedPlayersMock,
  };
});

import ExternalPlayerButton from "./ExternalPlayerButton.svelte";

/** The launch button, distinguished from the dropdown's own button. */
const launchButton = () =>
  screen.getByRole("button", { name: /open in external player/i });
/** The themed dropdown trigger. */
const picker = () => screen.getByRole("combobox", { name: "External player" });

beforeEach(() => {
  openInPlayerMock.mockReset().mockResolvedValue("mpv");
  getPlayerMock.mockReset().mockResolvedValue("mpv");
  setPlayerMock.mockReset().mockResolvedValue(undefined);
  suggestedPlayersMock.mockReset().mockResolvedValue(["mpv", "vlc"]);
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

  it("opens the url in the current player", async () => {
    render(ExternalPlayerButton, { props: { url: "http://127.0.0.1:3030/x" } });
    await screen.findByRole("button", { name: /open/i });

    await fireEvent.click(launchButton());

    // `undefined` rather than absent: no torrent means nothing to hold.
    expect(openInPlayerMock).toHaveBeenCalledWith(
      "http://127.0.0.1:3030/x",
      "mpv",
      undefined,
    );
  });

  it("passes the torrent id so the backend can hold it", async () => {
    render(ExternalPlayerButton, {
      props: { url: "http://127.0.0.1:3030/x", torrentId: 5 },
    });
    await screen.findByRole("button", { name: /open/i });

    await fireEvent.click(launchButton());

    expect(openInPlayerMock).toHaveBeenCalledWith(
      "http://127.0.0.1:3030/x",
      "mpv",
      5,
    );
  });

  it("offers the suggested players", async () => {
    render(ExternalPlayerButton, { props: { url: "http://x" } });

    await fireEvent.click(picker());

    expect(screen.getByRole("option", { name: "vlc" })).toBeInTheDocument();
  });

  it("shows the stored preference", async () => {
    getPlayerMock.mockResolvedValue("vlc");
    render(ExternalPlayerButton, { props: { url: "http://127.0.0.1:3030/x" } });

    // The trigger reflects the stored choice once it has loaded.
    expect(await screen.findByText("vlc")).toBeInTheDocument();
  });

  it("remembers a changed choice", async () => {
    render(ExternalPlayerButton, { props: { url: "http://x" } });
    await screen.findByText("mpv");

    await fireEvent.click(picker());
    await fireEvent.click(screen.getByRole("option", { name: "vlc" }));

    expect(setPlayerMock).toHaveBeenCalledWith("vlc");
  });

  it("surfaces a launch failure", async () => {
    openInPlayerMock.mockRejectedValue("failed to launch `mpv`");
    render(ExternalPlayerButton, { props: { url: "http://x" } });
    await screen.findByRole("button", { name: /open/i });

    await fireEvent.click(launchButton());

    expect(await screen.findByText(/failed to launch/i)).toBeInTheDocument();
  });

  it("keeps a stored player that is not in the suggested list", async () => {
    getPlayerMock.mockResolvedValue("my-player");
    suggestedPlayersMock.mockResolvedValue(["mpv", "vlc"]);

    render(ExternalPlayerButton, { props: { url: "http://x" } });
    await screen.findByText("my-player");

    // Without this option the picker would fall back to a suggestion and
    // silently change the user's choice.
    await fireEvent.click(picker());
    expect(screen.getByRole("option", { name: "my-player" })).toBeInTheDocument();
  });
});