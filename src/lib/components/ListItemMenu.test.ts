import { afterEach, describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

import type { UserListEntry } from "$lib/types";

const setListEntryMock = vi.hoisted(() => vi.fn());
const deleteListEntryMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/auth", () => ({
  setListEntry: setListEntryMock,
  deleteListEntry: deleteListEntryMock,
}));

import ListItemMenu from "./ListItemMenu.svelte";

function entry(overrides: Partial<UserListEntry> = {}): UserListEntry {
  return {
    anime: {
      id: 21,
      provider: "anilist",
      title: { romaji: "One Piece" },
      genres: [],
      streamingEpisodes: [],
      relations: [],
      recommendations: [],
    },
    status: "current",
    progress: 3,
    entryId: 9,
    ...overrides,
  };
}

beforeEach(() => {
  setListEntryMock.mockReset().mockResolvedValue(undefined);
  deleteListEntryMock.mockReset().mockResolvedValue(undefined);
});

/**
 * Give the component a viewport and a trigger position.
 *
 * jsdom reports zeros from `getBoundingClientRect` and does not set
 * `innerWidth`, so without both stubs every render resolves to the same
 * alignment and the flip is untestable.
 */
function setGeometry(triggerLeft: number, viewportWidth: number) {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue({
    left: triggerLeft,
    right: triggerLeft + 28,
    top: 0,
    bottom: 28,
    width: 28,
    height: 28,
    x: triggerLeft,
    y: 0,
    toJSON: () => ({}),
  } as DOMRect);
  vi.spyOn(window, "innerWidth", "get").mockReturnValue(viewportWidth);
}

describe("ListItemMenu", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("starts closed", () => {
    render(ListItemMenu, {
      props: { entry: entry(), onchange: () => {}, onremove: () => {} },
    });

    expect(screen.getByTestId("item-menu-button")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(screen.queryByTestId("item-menu")).toBeNull();
  });

  /// The default: plenty of room to the right, so the menu unfolds that way.
  it("opens rightward by default", async () => {
    setGeometry(0, 1280);

    render(ListItemMenu, {
      props: { entry: entry(), onchange: () => {}, onremove: () => {} },
    });
    await fireEvent.click(screen.getByTestId("item-menu-button"));

    expect(screen.getByTestId("item-menu").className).toContain("left-0");
  });

  /// A card at the right edge would push a right-opening menu off-screen, so
  /// the menu flips to open leftward.
  it("flips to open leftward near the right edge", async () => {
    // 900 + 176 > 1000, so rightward cannot fit.
    setGeometry(900, 1000);

    render(ListItemMenu, {
      props: { entry: entry(), onchange: () => {}, onremove: () => {} },
    });
    await fireEvent.click(screen.getByTestId("item-menu-button"));

    expect(screen.getByTestId("item-menu").className).toContain("right-0");
  });

  it("lists every status plus remove when opened", async () => {
    render(ListItemMenu, {
      props: { entry: entry(), onchange: () => {}, onremove: () => {} },
    });

    await fireEvent.click(screen.getByTestId("item-menu-button"));

    for (const value of ["current", "completed", "paused", "dropped", "planning"]) {
      expect(screen.getByTestId(`item-status-${value}`)).toBeInTheDocument();
    }
    expect(screen.getByTestId("item-remove")).toBeInTheDocument();
  });

  it("changes the status and reports the new one", async () => {
    const onchange = vi.fn();
    render(ListItemMenu, {
      props: { entry: entry(), onchange, onremove: () => {} },
    });

    await fireEvent.click(screen.getByTestId("item-menu-button"));
    await fireEvent.click(screen.getByTestId("item-status-planning"));

    await waitFor(() => expect(setListEntryMock).toHaveBeenCalledWith(21, "planning"));
    expect(onchange).toHaveBeenCalledWith("planning");
  });

  /// Picking the status a work already has is a no-op, not a pointless write.
  it("does nothing when the current status is re-picked", async () => {
    const onchange = vi.fn();
    render(ListItemMenu, {
      props: { entry: entry({ status: "current" }), onchange, onremove: () => {} },
    });

    await fireEvent.click(screen.getByTestId("item-menu-button"));
    await fireEvent.click(screen.getByTestId("item-status-current"));

    expect(setListEntryMock).not.toHaveBeenCalled();
    expect(onchange).not.toHaveBeenCalled();
  });

  it("removes the entry and reports it", async () => {
    const onremove = vi.fn();
    render(ListItemMenu, {
      props: { entry: entry({ entryId: 9 }), onchange: () => {}, onremove },
    });

    await fireEvent.click(screen.getByTestId("item-menu-button"));
    await fireEvent.click(screen.getByTestId("item-remove"));

    await waitFor(() => expect(deleteListEntryMock).toHaveBeenCalledWith(9));
    expect(onremove).toHaveBeenCalled();
  });

  /// A failed remove leaves the row in place rather than claiming success.
  it("does not report a removal that failed", async () => {
    deleteListEntryMock.mockRejectedValue(new Error("offline"));
    const onremove = vi.fn();
    render(ListItemMenu, {
      props: { entry: entry(), onchange: () => {}, onremove },
    });

    await fireEvent.click(screen.getByTestId("item-menu-button"));
    await fireEvent.click(screen.getByTestId("item-remove"));

    await waitFor(() => expect(deleteListEntryMock).toHaveBeenCalled());
    expect(onremove).not.toHaveBeenCalled();
  });

  it("closes on an outside click", async () => {
    render(ListItemMenu, {
      props: { entry: entry(), onchange: () => {}, onremove: () => {} },
    });

    await fireEvent.click(screen.getByTestId("item-menu-button"));
    expect(screen.getByTestId("item-menu")).toBeInTheDocument();

    await fireEvent.click(document.body);

    await waitFor(() => expect(screen.queryByTestId("item-menu")).toBeNull());
  });
});