import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

import type { Settings } from "$lib/types";

const getSettingsMock = vi.hoisted(() => vi.fn());
const setSettingsMock = vi.hoisted(() => vi.fn());
// The drawer subscribes on mount to stay in sync with other surfaces; a
// never-firing stub is enough, and returning an unlisten matches the real shape.
const onSettingsChangedMock = vi.hoisted(() => vi.fn(async () => () => {}));
vi.mock("$lib/api/settings", () => ({
  getSettings: getSettingsMock,
  setSettings: setSettingsMock,
  onSettingsChanged: onSettingsChangedMock,
}));

const authStatusMock = vi.hoisted(() => vi.fn());
const onAuthChangedMock = vi.hoisted(() => vi.fn());
const beginLoginMock = vi.hoisted(() => vi.fn());
const logoutMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/auth", () => ({
  authStatus: authStatusMock,
  onAuthChanged: onAuthChangedMock,
  beginLogin: beginLoginMock,
  logout: logoutMock,
}));

vi.mock("$lib/api/player", () => ({
  suggestedPlayers: vi.fn(async () => ["mpv", "vlc"]),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
  revealItemInDir: vi.fn(),
}));
// The drawer mounts DiagnosticsPanel, which polls this. Stub it so the drawer
// test stays about the drawer.
vi.mock("$lib/api/diagnostics", () => ({
  getSystemStats: vi.fn(async () => ({
    cpuPercent: 1,
    cpuPercentOfMachine: 1,
    cpuCores: 1,
    rssBytes: 2,
    virtualBytes: 3,
    threadCount: 4,
    processCount: 5,
    uptimeSeconds: 5,
    systemTotalBytes: 6,
    systemAvailableBytes: 7,
  })),
  getLogPath: vi.fn(async () => "/data/kitsune/logs/kitsune.log"),
}));

import SettingsDrawer from "./SettingsDrawer.svelte";

function settings(overrides: Partial<Settings> = {}): Settings {
  return {
    player: "mpv",
    playerArgs: [],
    downloadDir: null,
    preferredResolutions: ["1080p"],
    minSeeders: 0,
    readyFraction: 0.05,
    theme: "system",
    ...overrides,
  };
}

beforeEach(() => {
  getSettingsMock.mockReset().mockResolvedValue(settings());
  setSettingsMock.mockReset().mockImplementation(async (s: Settings) => s);
  authStatusMock.mockReset().mockResolvedValue(false);
  onAuthChangedMock.mockReset().mockResolvedValue(() => {});
  beginLoginMock.mockReset().mockResolvedValue("https://anilist.co/oauth");
  logoutMock.mockReset().mockResolvedValue(undefined);
  document.documentElement.removeAttribute("data-theme");
});

describe("SettingsDrawer", () => {
  it("renders the account, player and appearance sections", async () => {
    render(SettingsDrawer, { props: { open: true, onClose: () => {} } });

    expect(await screen.findByText(/anilist account/i)).toBeInTheDocument();
    expect(screen.getByText(/^player$/i)).toBeInTheDocument();
    expect(screen.getByRole("group", { name: /theme/i })).toBeInTheDocument();
  });

  it("shows the current player value", async () => {
    getSettingsMock.mockResolvedValue(settings({ player: "vlc" }));
    render(SettingsDrawer, { props: { open: true, onClose: () => {} } });

    const input = (await screen.findByLabelText(/program/i)) as HTMLInputElement;
    expect(input.value).toBe("vlc");
  });

  it("persists a theme change and applies it", async () => {
    render(SettingsDrawer, { props: { open: true, onClose: () => {} } });
    await screen.findByText(/anilist account/i);

    await fireEvent.click(screen.getByRole("button", { name: /^dark$/i }));

    await waitFor(() =>
      expect(setSettingsMock).toHaveBeenCalledWith(
        expect.objectContaining({ theme: "dark" }),
      ),
    );
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("does not load while closed", () => {
    render(SettingsDrawer, { props: { open: false, onClose: () => {} } });
    expect(getSettingsMock).not.toHaveBeenCalled();
  });

  it("surfaces the diagnostics panel", async () => {
    render(SettingsDrawer, { props: { open: true, onClose: () => {} } });

    expect(await screen.findByTestId("diagnostics-panel")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /reveal log/i }),
    ).toBeInTheDocument();
  });
});