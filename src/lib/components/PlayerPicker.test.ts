import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

const listPlayersMock = vi.hoisted(() => vi.fn());
const openInPlayerChoiceMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/player", () => ({
  listPlayers: listPlayersMock,
  openInPlayerChoice: openInPlayerChoiceMock,
}));

const setDefaultPlayerMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/settings", () => ({
  setDefaultPlayer: setDefaultPlayerMock,
}));

import PlayerPicker from "./PlayerPicker.svelte";

/** One player, with an icon. */
function withIcon() {
  return {
    id: "/usr/bin/vlc",
    name: "vlc",
    program: "/usr/bin/vlc",
    extraArgs: [],
    isDefault: true,
    icon: "data:image/png;base64,AAAA",
  };
}

/** One player, with no icon resolved. */
function withoutIcon() {
  return {
    id: "/usr/bin/mpv",
    name: "mpv",
    program: "/usr/bin/mpv",
    extraArgs: [],
    isDefault: false,
  };
}

beforeEach(() => {
  listPlayersMock.mockReset().mockResolvedValue([]);
  openInPlayerChoiceMock.mockReset().mockResolvedValue("/usr/bin/vlc");
  setDefaultPlayerMock.mockReset().mockResolvedValue(undefined);
});

describe("PlayerPicker", () => {
  it("lists a player with its icon when one resolved", async () => {
    listPlayersMock.mockResolvedValue([withIcon()]);
    render(PlayerPicker, {
      props: { open: true, url: "http://x", onClose: () => {} },
    });

    expect(await screen.findByTestId("player-icon")).toBeInTheDocument();
    expect(screen.queryByTestId("player-icon-fallback")).toBeNull();
  });

  it("falls back to a glyph when no icon resolved", async () => {
    listPlayersMock.mockResolvedValue([withoutIcon()]);
    render(PlayerPicker, {
      props: { open: true, url: "http://x", onClose: () => {} },
    });

    expect(await screen.findByTestId("player-icon-fallback")).toBeInTheDocument();
    expect(screen.queryByTestId("player-icon")).toBeNull();
  });

  it("launches the picked player and closes", async () => {
    listPlayersMock.mockResolvedValue([withoutIcon()]);
    const onClose = vi.fn();
    const onLaunched = vi.fn();
    render(PlayerPicker, {
      props: { open: true, url: "http://x", torrentId: 7, onClose, onLaunched },
    });

    await fireEvent.click(await screen.findByTestId("player-option"));

    expect(openInPlayerChoiceMock).toHaveBeenCalledWith(
      "http://x",
      "/usr/bin/mpv",
      [],
      7,
    );
    // `onLaunched` gets whatever the backend resolved to, which the mock sets.
    await waitFor(() => expect(onLaunched).toHaveBeenCalledWith("/usr/bin/vlc"));
    expect(onClose).toHaveBeenCalled();
  });

  it("shows an empty state when no players are installed", async () => {
    listPlayersMock.mockResolvedValue([]);
    render(PlayerPicker, {
      props: { open: true, url: "http://x", onClose: () => {} },
    });

    expect(await screen.findByTestId("player-picker-empty")).toBeInTheDocument();
  });

  it("pre-selects the stored default", async () => {
    listPlayersMock.mockResolvedValue([withIcon()]);
    render(PlayerPicker, {
      props: { open: true, url: "http://x", onClose: () => {} },
    });

    const option = await screen.findByTestId("player-option");
    expect(option.dataset.selected).toBe("true");
  });

  it("Always saves the default, launches, and closes", async () => {
    listPlayersMock.mockResolvedValue([withIcon()]);
    const onClose = vi.fn();
    render(PlayerPicker, {
      props: { open: true, url: "http://x", torrentId: 3, onClose },
    });

    await screen.findByTestId("player-option");
    await fireEvent.click(screen.getByTestId("player-always"));

    // Persisted as the default (the backend also clears askEveryTime)...
    await waitFor(() =>
      expect(setDefaultPlayerMock).toHaveBeenCalledWith("/usr/bin/vlc", []),
    );
    // ...then launched.
    expect(openInPlayerChoiceMock).toHaveBeenCalledWith(
      "http://x",
      "/usr/bin/vlc",
      [],
      3,
    );
    expect(onClose).toHaveBeenCalled();
  });
});