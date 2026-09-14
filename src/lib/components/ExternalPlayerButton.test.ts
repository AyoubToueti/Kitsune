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

beforeEach(() => {
  openInPlayerMock.mockReset().mockResolvedValue("mpv");
  getPlayerMock.mockReset().mockResolvedValue("mpv");
  setPlayerMock.mockReset().mockResolvedValue(undefined);
  suggestedPlayersMock.mockReset().mockResolvedValue(["mpv", "vlc"]);
});

describe("ExternalPlayerButton", () => {
  it("is disabled without a url", () => {
    render(ExternalPlayerButton, { props: {} });

    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("is enabled once a url is given", () => {
    render(ExternalPlayerButton, { props: { url: "http://127.0.0.1:3030/x" } });

    expect(screen.getByRole("button")).toBeEnabled();
  });

  it("opens the url in the current player", async () => {
    render(ExternalPlayerButton, { props: { url: "http://127.0.0.1:3030/x" } });
    // Let the preference fetch settle before clicking.
    await screen.findByRole("button");

    await fireEvent.click(screen.getByRole("button"));

    expect(openInPlayerMock).toHaveBeenCalledWith("http://127.0.0.1:3030/x", "mpv");
  });

  it("offers the suggested players", async () => {
    render(ExternalPlayerButton, { props: { url: "http://x" } });

    expect(await screen.findByRole("option", { name: "vlc" })).toBeInTheDocument();
  });

  it("uses the stored preference", async () => {
    getPlayerMock.mockResolvedValue("vlc");
    render(ExternalPlayerButton, { props: { url: "http://127.0.0.1:3030/x" } });

    // The select reflects the stored choice once it has loaded.
    await screen.findByRole("option", { name: "vlc" });
    const select = screen.getByLabelText(/external player/i) as HTMLSelectElement;
    expect(select.value).toBe("vlc");
  });

  it("remembers a changed choice", async () => {
    render(ExternalPlayerButton, { props: { url: "http://x" } });
    await screen.findByRole("option", { name: "vlc" });

    const select = screen.getByLabelText(/external player/i);
    await fireEvent.change(select, { target: { value: "vlc" } });

    expect(setPlayerMock).toHaveBeenCalledWith("vlc");
  });

  it("surfaces a launch failure", async () => {
    openInPlayerMock.mockRejectedValue("failed to launch `mpv`");
    render(ExternalPlayerButton, { props: { url: "http://x" } });
    await screen.findByRole("button");

    await fireEvent.click(screen.getByRole("button"));

    expect(await screen.findByText(/failed to launch/i)).toBeInTheDocument();
  });

  it("keeps a stored player that is not in the suggested list", async () => {
    getPlayerMock.mockResolvedValue("my-player");
    suggestedPlayersMock.mockResolvedValue(["mpv", "vlc"]);

    render(ExternalPlayerButton, { props: { url: "http://x" } });

    // Without this option the select would fall back to the first suggestion
    // and silently change the user's choice.
    expect(await screen.findByRole("option", { name: "my-player" })).toBeInTheDocument();
  });
});