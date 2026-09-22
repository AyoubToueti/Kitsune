import { describe, it, expect, vi, beforeEach } from "vitest";

// The wrapper module must be mocked before it is imported, so the factory is
// hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import {
  PLAYER_COMMANDS,
  addTorrent,
  getPlayer,
  getStreamUrl,
  openInPlayer,
  removeTorrent,
  setPlayer,
  suggestedPlayers,
} from "./player";
import type { TorrentHandle } from "$lib/types";

function handle(): TorrentHandle {
  return {
    id: 3,
    files: [{ idx: 0, name: "episode.mkv", lengthBytes: 1_400_000_000 }],
  };
}

beforeEach(() => {
  invokeMock.mockReset();
});

describe("player command wrappers", () => {
  it("addTorrent passes the path and returns the handle", async () => {
    invokeMock.mockResolvedValue(handle());

    const result = await addTorrent("/tmp/show.torrent");

    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.addTorrent, {
      path: "/tmp/show.torrent",
    });
    expect(result.id).toBe(3);
    expect(result.files[0].name).toBe("episode.mkv");
  });

  it("removeTorrent passes the id", async () => {
    invokeMock.mockResolvedValue(undefined);

    await removeTorrent(3);

    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.removeTorrent, {
      torrentId: 3,
    });
  });

  it("getStreamUrl passes both indices", async () => {
    invokeMock.mockResolvedValue("http://127.0.0.1:3030/torrents/3/stream/0");

    const url = await getStreamUrl(3, 0);

    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.streamUrl, {
      torrentId: 3,
      fileIdx: 0,
    });
    expect(url).toContain("/stream/0");
  });

  it("openInPlayer omits the player when not given", async () => {
    invokeMock.mockResolvedValue("mpv");

    const launched = await openInPlayer("http://127.0.0.1:3030/x");

    // `undefined` rather than absent: the Rust side takes Option<String>, so
    // an explicit undefined deserialises to None and the stored choice applies.
    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.openInPlayer, {
      url: "http://127.0.0.1:3030/x",
      player: undefined,
    });
    expect(launched).toBe("mpv");
  });

  it("openInPlayer forwards an explicit override", async () => {
    invokeMock.mockResolvedValue("vlc");

    await openInPlayer("http://127.0.0.1:3030/x", "vlc");

    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.openInPlayer, {
      url: "http://127.0.0.1:3030/x",
      player: "vlc",
    });
  });

  it("getPlayer returns the stored name", async () => {
    invokeMock.mockResolvedValue("mpv");

    expect(await getPlayer()).toBe("mpv");
    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.getPlayer);
  });

  it("setPlayer passes the name", async () => {
    invokeMock.mockResolvedValue(undefined);

    await setPlayer("vlc");

    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.setPlayer, {
      name: "vlc",
    });
  });

  it("suggestedPlayers returns the list", async () => {
    invokeMock.mockResolvedValue(["mpv", "vlc"]);

    expect(await suggestedPlayers()).toEqual(["mpv", "vlc"]);
    expect(invokeMock).toHaveBeenCalledWith(PLAYER_COMMANDS.suggestedPlayers);
  });

  it("propagates a rejected command", async () => {
    invokeMock.mockRejectedValue("failed to launch `mpv`");

    await expect(openInPlayer("http://x")).rejects.toBe("failed to launch `mpv`");
  });
});