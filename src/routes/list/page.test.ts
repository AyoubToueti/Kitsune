import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@testing-library/svelte";

import type { ListStatus, UserListEntry } from "$lib/types";

const authStatusMock = vi.hoisted(() => vi.fn());
const beginLoginMock = vi.hoisted(() => vi.fn());
const getUserListMock = vi.hoisted(() => vi.fn());
const onAuthChangedMock = vi.hoisted(() => vi.fn());
const setListEntryMock = vi.hoisted(() => vi.fn());
const deleteListEntryMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/auth", () => ({
  authStatus: authStatusMock,
  beginLogin: beginLoginMock,
  getUserList: getUserListMock,
  onAuthChanged: onAuthChangedMock,
  setListEntry: setListEntryMock,
  deleteListEntry: deleteListEntryMock,
}));

const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

import Page from "./+page.svelte";

function entry(
  id: number,
  title: string,
  status: ListStatus,
  entryId = id,
): UserListEntry {
  return {
    anime: {
      id,
      provider: "anilist",
      title: { romaji: title },
      genres: [],
      streamingEpisodes: [],
      relations: [],
      recommendations: [],
    },
    status,
    progress: 0,
    entryId,
  };
}

/** The card titles currently rendered in the grid. */
function shownTitles(): string[] {
  return within(screen.getByTestId("list-grid"))
    .getAllByRole("listitem")
    .map((li) => li.textContent ?? "");
}

beforeEach(() => {
  authStatusMock.mockReset().mockResolvedValue(true);
  beginLoginMock.mockReset().mockResolvedValue("https://anilist.co/authorize");
  getUserListMock.mockReset().mockResolvedValue([]);
  onAuthChangedMock.mockReset().mockResolvedValue(() => {});
  setListEntryMock.mockReset().mockResolvedValue(undefined);
  deleteListEntryMock.mockReset().mockResolvedValue(undefined);
  openUrlMock.mockReset().mockResolvedValue(undefined);
});

describe("My List page", () => {
  it("prompts to sign in when signed out", async () => {
    authStatusMock.mockResolvedValue(false);

    render(Page);

    expect(await screen.findByTestId("list-signin")).toBeInTheDocument();
    // No request is made for a reader with no account.
    expect(getUserListMock).not.toHaveBeenCalled();
  });

  it("opens the authorize url from the sign-in prompt", async () => {
    authStatusMock.mockResolvedValue(false);

    render(Page);
    await fireEvent.click(await screen.findByTestId("list-signin"));

    await waitFor(() =>
      expect(openUrlMock).toHaveBeenCalledWith("https://anilist.co/authorize"),
    );
  });

  it("shows every entry on the All tab", async () => {
    getUserListMock.mockResolvedValue([
      entry(1, "One Piece", "current"),
      entry(2, "Naruto", "completed"),
    ]);

    render(Page);

    await screen.findByTestId("list-grid");
    expect(shownTitles()).toHaveLength(2);
  });

  it("filters to one status when a tab is chosen", async () => {
    getUserListMock.mockResolvedValue([
      entry(1, "One Piece", "current"),
      entry(2, "Naruto", "completed"),
    ]);

    render(Page);
    await screen.findByTestId("list-grid");

    await fireEvent.click(screen.getByTestId("list-tab-completed"));

    await waitFor(() => {
      const titles = shownTitles();
      expect(titles).toHaveLength(1);
      expect(titles[0]).toContain("Naruto");
    });
  });

  /// The count is what tells a reader a tab is not empty without opening it.
  it("counts each tab", async () => {
    getUserListMock.mockResolvedValue([
      entry(1, "One Piece", "current"),
      entry(2, "Naruto", "current"),
      entry(3, "Bleach", "completed"),
    ]);

    render(Page);

    await screen.findByTestId("list-grid");
    await waitFor(() =>
      expect(screen.getByTestId("list-tab-current")).toHaveTextContent("2"),
    );
    expect(screen.getByTestId("list-tab-completed")).toHaveTextContent("1");
  });

  it("says so when a tab is empty", async () => {
    getUserListMock.mockResolvedValue([entry(1, "One Piece", "current")]);

    render(Page);
    await screen.findByTestId("list-grid");

    await fireEvent.click(screen.getByTestId("list-tab-dropped"));

    expect(await screen.findByTestId("list-empty")).toBeInTheDocument();
  });

  it("reports a load failure and offers a retry", async () => {
    getUserListMock.mockRejectedValue(new Error("offline"));

    render(Page);

    expect(await screen.findByText(/could not load/i)).toBeInTheDocument();

    getUserListMock.mockResolvedValue([entry(1, "One Piece", "current")]);
    await fireEvent.click(screen.getByRole("button", { name: /try again/i }));

    expect(await screen.findByTestId("list-grid")).toBeInTheDocument();
  });

  /// Removing a work from its card menu drops it from the grid without a
  /// refetch, so the page stays put.
  it("drops a removed work from the grid", async () => {
    getUserListMock.mockResolvedValue([
      entry(1, "One Piece", "current"),
      entry(2, "Naruto", "current"),
    ]);

    render(Page);
    await screen.findByTestId("list-grid");

    // First card's menu, scoped so the second card's button is not matched.
    const cards = within(screen.getByTestId("list-grid")).getAllByRole("listitem");
    await fireEvent.click(
      within(cards[0]).getByTestId("item-menu-button"),
    );
    await fireEvent.click(within(cards[0]).getByTestId("item-remove"));

    await waitFor(() => expect(deleteListEntryMock).toHaveBeenCalledWith(1));
    await waitFor(() => expect(shownTitles()).toHaveLength(1));
  });

  /// Moving a work to another status re-groups it locally: it leaves the tab it
  /// was on and appears under the new one.
  it("re-groups a work whose status changed", async () => {
    getUserListMock.mockResolvedValue([entry(1, "One Piece", "current")]);

    render(Page);
    await screen.findByTestId("list-grid");

    // On the Watching tab, change to Completed.
    const cards = within(screen.getByTestId("list-grid")).getAllByRole("listitem");
    await fireEvent.click(within(cards[0]).getByTestId("item-menu-button"));
    await fireEvent.click(within(cards[0]).getByTestId("item-status-completed"));

    await waitFor(() => expect(setListEntryMock).toHaveBeenCalledWith(1, "completed"));

    // It has left Watching...
    await fireEvent.click(screen.getByTestId("list-tab-current"));
    expect(await screen.findByTestId("list-empty")).toBeInTheDocument();

    // ...and is now under Completed.
    await fireEvent.click(screen.getByTestId("list-tab-completed"));
    await waitFor(() => expect(shownTitles()[0]).toContain("One Piece"));
  });
});