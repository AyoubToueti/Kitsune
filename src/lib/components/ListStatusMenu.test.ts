import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

const authStatusMock = vi.hoisted(() => vi.fn());
const beginLoginMock = vi.hoisted(() => vi.fn());
const getListEntryMock = vi.hoisted(() => vi.fn());
const setListEntryMock = vi.hoisted(() => vi.fn());
const onAuthChangedMock = vi.hoisted(() => vi.fn());
const openUrlMock = vi.hoisted(() => vi.fn());

vi.mock("$lib/api/auth", () => ({
  authStatus: authStatusMock,
  beginLogin: beginLoginMock,
  getListEntry: getListEntryMock,
  setListEntry: setListEntryMock,
  onAuthChanged: onAuthChangedMock,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

import ListStatusMenu from "./ListStatusMenu.svelte";

beforeEach(() => {
  authStatusMock.mockReset().mockResolvedValue(true);
  beginLoginMock.mockReset().mockResolvedValue("https://anilist.co/authorize");
  getListEntryMock.mockReset().mockResolvedValue(null);
  setListEntryMock.mockReset().mockResolvedValue(undefined);
  // Returns an unlisten; never resolves with a listener the component keeps.
  onAuthChangedMock.mockReset().mockResolvedValue(() => {});
  openUrlMock.mockReset().mockResolvedValue(undefined);
});

describe("ListStatusMenu", () => {
  describe("when signed out", () => {
    beforeEach(() => {
      authStatusMock.mockResolvedValue(false);
    });

    it("offers sign-in instead of the menu", async () => {
      render(ListStatusMenu, { props: { mediaId: 1 } });

      expect(
        await screen.findByTestId("list-status-signin"),
      ).toBeInTheDocument();
      expect(screen.queryByTestId("list-status-button")).toBeNull();
    });

    it("opens the authorize url in the browser", async () => {
      render(ListStatusMenu, { props: { mediaId: 1 } });

      await fireEvent.click(await screen.findByTestId("list-status-signin"));

      await waitFor(() =>
        expect(openUrlMock).toHaveBeenCalledWith(
          "https://anilist.co/authorize",
        ),
      );
    });
  });

  describe("when signed in", () => {
    it("prompts to add when the work is not on the list", async () => {
      render(ListStatusMenu, { props: { mediaId: 7 } });

      expect(await screen.findByTestId("list-status-button")).toHaveTextContent(
        "Add to list",
      );
      expect(getListEntryMock).toHaveBeenCalledWith(7);
    });

    it("shows the current status", async () => {
      getListEntryMock.mockResolvedValue({ status: "current", progress: 3 });

      render(ListStatusMenu, { props: { mediaId: 7 } });

      expect(await screen.findByTestId("list-status-button")).toHaveTextContent(
        "Watching",
      );
    });

    it("lists every status when opened", async () => {
      render(ListStatusMenu, { props: { mediaId: 7 } });
      await screen.findByTestId("list-status-button");

      await fireEvent.click(screen.getByTestId("list-status-button"));

      expect(screen.getByTestId("list-status-menu")).toBeInTheDocument();
      for (const value of [
        "current",
        "completed",
        "paused",
        "dropped",
        "planning",
      ]) {
        expect(screen.getByTestId(`list-status-option-${value}`)).toBeInTheDocument();
      }
    });

    it("writes the chosen status and updates the label", async () => {
      render(ListStatusMenu, { props: { mediaId: 7 } });
      await screen.findByTestId("list-status-button");

      await fireEvent.click(screen.getByTestId("list-status-button"));
      await fireEvent.click(screen.getByTestId("list-status-option-planning"));

      await waitFor(() =>
        expect(setListEntryMock).toHaveBeenCalledWith(7, "planning"),
      );
      expect(await screen.findByTestId("list-status-button")).toHaveTextContent(
        "Plan to watch",
      );
    });

    it("reverts and reports when the write fails", async () => {
      setListEntryMock.mockRejectedValue(new Error("offline"));

      render(ListStatusMenu, { props: { mediaId: 7 } });
      await screen.findByTestId("list-status-button");

      await fireEvent.click(screen.getByTestId("list-status-button"));
      await fireEvent.click(screen.getByTestId("list-status-option-completed"));

      expect(await screen.findByTestId("list-status-error")).toBeInTheDocument();
      // Back to the prompt: the optimistic update is undone.
      expect(screen.getByTestId("list-status-button")).toHaveTextContent(
        "Add to list",
      );
    });

    it("closes the menu on an outside click", async () => {
      render(ListStatusMenu, { props: { mediaId: 7 } });
      await screen.findByTestId("list-status-button");

      await fireEvent.click(screen.getByTestId("list-status-button"));
      expect(screen.getByTestId("list-status-menu")).toBeInTheDocument();

      await fireEvent.click(document.body);

      await waitFor(() =>
        expect(screen.queryByTestId("list-status-menu")).toBeNull(),
      );
    });
  });
});