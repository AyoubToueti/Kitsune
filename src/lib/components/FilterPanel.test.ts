import { describe, it, expect } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";

import FilterPanel from "./FilterPanel.svelte";
import type { BrowseQuery, MediaTag } from "$lib/types";

const GENRES = ["Action", "Comedy", "Slice of Life"];

const TAGS: MediaTag[] = [
  { name: "Isekai", category: "Theme-Fantasy" },
  { name: "Magic", category: "Theme-Fantasy" },
  { name: "School", category: "Setting-Scene" },
  { name: "Shounen", category: "Demographic" },
];

/** A catalogue where one tag carries prose and the rest do not. */
const TAGS_WITH_DESCRIPTION: MediaTag[] = [
  { name: "Isekai", category: "Theme-Fantasy", description: "Another world." },
  { name: "School", category: "Setting-Scene" },
];

/** The form element, which carries the submit target. */
function form(): HTMLElement {
  return screen.getByTestId("filter-form");
}

/** A labelled control, by its visible label text. */
function control(label: string): HTMLElement {
  return screen.getByLabelText(label) as HTMLElement;
}

/** The disclosure button for the tag catalogue. */
function toggle(): HTMLElement {
  return screen.getByTestId("toggle-catalogue");
}

/** Open the catalogue, since it starts collapsed. */
async function openCatalogue() {
  await fireEvent.click(toggle());
}

/**
 * Everything the form would submit, as the browser would build it.
 *
 * Read from the real form rather than from component state, so a field that
 * exists but is unreachable -- outside the form, or inside a closed section --
 * shows up as absent rather than as a false pass.
 */
function submitted(): {
  tag: string[];
  exclude_tag: string[];
  genre: string[];
} {
  const data = new FormData(form() as HTMLFormElement);
  return {
    tag: data.getAll("tag").map(String),
    exclude_tag: data.getAll("exclude_tag").map(String),
    genre: data.getAll("genre").map(String),
  };
}

/** The props the panel takes, so tests can override just what they care about. */
interface PanelProps {
  genres: string[];
  tags?: MediaTag[];
  current: BrowseQuery;
  base?: string;
}

/** Render with sensible defaults, overriding only what a test cares about. */
function renderPanel(overrides: Partial<PanelProps> = {}) {
  // Built as one object rather than spread inline: the component's props are a
  // closed set, and an inline spread defeats that check.
  const props: PanelProps = {
    genres: GENRES,
    current: { sort: "popularity" },
    ...overrides,
  };

  return render(FilterPanel, { props });
}

describe("FilterPanel", () => {
  describe("the form itself", () => {
    it("submits as a GET to /filter", () => {
      renderPanel();

      // A plain GET form: the browser builds the query string.
      expect(form()).toHaveAttribute("action", "/filter");
      expect(form()).toHaveAttribute("method", "GET");
    });

    it("submits to the given base path instead", () => {
      renderPanel({ base: "/search" });

      // A shared panel has to post back to whichever route hosts it, or the
      // search term would be dropped on submit.
      expect(form()).toHaveAttribute("action", "/search");
    });

    it("resets to the base path rather than always /filter", () => {
      renderPanel({ base: "/search" });

      expect(screen.getByRole("link", { name: "Reset" })).toHaveAttribute(
        "href",
        "/search",
      );
    });

    /// The sort control lives in the toolbar now, but the panel still has to
    /// submit it: a GET replace the whole query string, so omitting it would
    /// reset the ordering every time a filter changed.
    it("submits the sort without offering a control for it", () => {
      renderPanel();

      const data = new FormData(form() as HTMLFormElement);
      expect(data.get("sort")).toBe("popularity");
      expect(screen.queryByLabelText("Sort")).toBeNull();
    });

    it("reflects the applied filters", () => {
      renderPanel({
        current: {
          format: "tv",
          status: "finished",
          season: "fall",
          seasonYear: 2024,
          minScore: 70,
          sort: "score",
        },
      });

      expect(control("Type")).toHaveValue("tv");
      expect(control("Status")).toHaveValue("finished");
      expect(control("Season")).toHaveValue("fall");
      expect(control("Year")).toHaveValue("2024");
      expect(control("Score")).toHaveValue("70");
      expect(new FormData(form() as HTMLFormElement).get("sort")).toBe("score");
    });

    it("shows the search term", () => {
      renderPanel({ current: { search: "naruto", sort: "searchMatch" } });

      expect(control("Search")).toHaveValue("naruto");
    });

    it("does not offer relevance sort, which needs a search term", () => {
      renderPanel();

      // `searchMatch` orders by textual relevance and means nothing without a
      // term, so this form must not offer it.
      expect(screen.queryByRole("option", { name: /best match/i })).toBeNull();
    });

    it("offers every format", () => {
      renderPanel();

      for (const label of ["TV", "Movie", "OVA", "ONA", "Special", "Music"]) {
        expect(screen.getByRole("option", { name: label })).toBeInTheDocument();
      }
    });

    it("has a submit button", () => {
      renderPanel();

      expect(screen.getByRole("button", { name: "Filter" })).toBeInTheDocument();
    });

    it("offers a reset back to the unfiltered view", () => {
      renderPanel();

      expect(screen.getByRole("link", { name: "Reset" })).toHaveAttribute(
        "href",
        "/filter",
      );
    });
  });

  describe("the catalogue disclosure", () => {
    it("renders no disclosure when there is nothing to show", () => {
      renderPanel({ genres: [], tags: [] });

      // A toggle that opens an empty box is worse than no toggle.
      expect(screen.queryByTestId("toggle-catalogue")).toBeNull();
    });

    it("starts collapsed", () => {
      renderPanel({ tags: TAGS });

      expect(toggle()).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByTestId("tag-scroll")).toBeNull();
    });

    it("opens the catalogue when followed", async () => {
      renderPanel({ tags: TAGS });

      await openCatalogue();

      expect(toggle()).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByTestId("tag-scroll")).toBeInTheDocument();
    });

    it("closes again on a second press", async () => {
      renderPanel({ tags: TAGS });

      await openCatalogue();
      await openCatalogue();

      expect(toggle()).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByTestId("tag-scroll")).toBeNull();
    });

    it("counts the active tags on the toggle", async () => {
      renderPanel({
        tags: TAGS,
        current: { tags: ["Isekai"], excludedTags: ["School"], sort: "popularity" },
      });

      // The count is the only hint that tags are set while the catalogue is shut.
      expect(screen.getByTestId("active-tag-count")).toHaveTextContent("2");
    });

    it("shows no count when no tags are active", () => {
      renderPanel({ tags: TAGS });

      expect(screen.queryByTestId("active-tag-count")).toBeNull();
    });

    it("submits active tags even while collapsed", () => {
      renderPanel({
        tags: TAGS,
        current: { tags: ["Isekai"], sort: "popularity" },
      });

      // The hidden fields must live outside the collapse, or changing an
      // unrelated filter with the catalogue shut would silently drop them.
      expect(screen.queryByTestId("tag-scroll")).toBeNull();
      expect(submitted().tag).toEqual(["Isekai"]);
    });
  });

  describe("genres", () => {
    it("renders a checkbox per genre", async () => {
      renderPanel();
      await openCatalogue();

      for (const genre of GENRES) {
        expect(screen.getByRole("checkbox", { name: genre })).toBeInTheDocument();
      }
    });

    it("checks the genres that are applied", async () => {
      renderPanel({ current: { genres: ["Action", "Comedy"], sort: "popularity" } });
      await openCatalogue();

      expect(screen.getByRole("checkbox", { name: "Action" })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Comedy" })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Slice of Life" })).not.toBeChecked();
    });

    it("submits a checked genre", async () => {
      renderPanel();
      await openCatalogue();

      await fireEvent.click(screen.getByRole("checkbox", { name: "Action" }));

      expect(submitted().genre).toEqual(["Action"]);
    });

    it("renders no genre section when the provider gave none", () => {
      renderPanel({ genres: [], tags: [] });

      // Nothing to disclose, so nothing is offered.
      expect(screen.queryByRole("checkbox")).toBeNull();
    });
  });

  describe("the tag catalogue", () => {
    it("offers a tag search box", async () => {
      renderPanel({ tags: TAGS });
      await openCatalogue();

      expect(screen.getByLabelText("Filter tags")).toBeInTheDocument();
    });

    it("offers no tag search box when there are no tags", async () => {
      renderPanel({ tags: [] });
      await openCatalogue();

      // A search box that can never match anything is worse than none.
      expect(screen.queryByLabelText("Filter tags")).toBeNull();
    });

    it("groups tags under their category", async () => {
      renderPanel({ tags: TAGS });
      await openCatalogue();

      // Categories arrive as provider keys and are humanised for display.
      for (const label of ["Theme / Fantasy", "Setting / Scene", "Demographic"]) {
        expect(screen.getByText(label)).toBeInTheDocument();
      }
    });

    it("renders a chip per tag", async () => {
      renderPanel({ tags: TAGS });
      await openCatalogue();

      for (const tag of TAGS) {
        expect(screen.getByRole("button", { name: tag.name })).toBeInTheDocument();
      }
    });

    it("shows the provider's description as a tooltip", async () => {
      renderPanel({ tags: TAGS_WITH_DESCRIPTION });
      await openCatalogue();

      expect(screen.getByRole("button", { name: "Isekai" })).toHaveAttribute(
        "title",
        "Another world.",
      );
    });

    // A tag with no prose must not get `title=""`, which some browsers render
    // as an empty tooltip box rather than no tooltip at all.
    it("omits the tooltip for a tag with no description", async () => {
      renderPanel({ tags: TAGS_WITH_DESCRIPTION });
      await openCatalogue();

      expect(screen.getByRole("button", { name: "School" })).not.toHaveAttribute(
        "title",
      );
    });

    it("keeps the tag search box out of the submission", async () => {
      renderPanel({ tags: TAGS });
      await openCatalogue();

      // The box narrows the view; if it were submitted it would become a filter
      // the backend never asked for.
      expect(screen.getByLabelText("Filter tags")).not.toHaveAttribute("name");
    });
  });

  describe("filtering the catalogue", () => {
    /** Open the catalogue and type into its search box. */
    async function search(term: string) {
      renderPanel({ tags: TAGS });
      await openCatalogue();
      await fireEvent.input(screen.getByLabelText("Filter tags"), {
        target: { value: term },
      });
    }

    it("narrows the catalogue to matching tags", async () => {
      await search("school");

      expect(screen.getByRole("button", { name: "School" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Isekai" })).toBeNull();
    });

    it("matches case-insensitively", async () => {
      await search("ISeK");

      expect(screen.getByRole("button", { name: "Isekai" })).toBeInTheDocument();
    });

    // A heading with no chips under it is just noise once a search is active.
    it("hides a category whose tags all fail to match", async () => {
      await search("school");

      expect(screen.getByText("Setting / Scene")).toBeInTheDocument();
      expect(screen.queryByText("Theme / Fantasy")).toBeNull();
    });

    it("says so when a search term matches nothing", async () => {
      await search("zzzz");

      // Silence would look like a broken list rather than an empty result.
      expect(screen.getByText(/no tags match/i)).toBeInTheDocument();
    });
  });

  describe("tag chip states", () => {
    /** Open the catalogue and hand back a named chip. */
    async function chip(name: string): Promise<HTMLElement> {
      await openCatalogue();
      return screen.getByRole("button", { name });
    }

    it("seeds an included chip from the applied filters", async () => {
      renderPanel({
        tags: TAGS,
        current: { tags: ["Isekai"], sort: "popularity" },
      });

      expect(await chip("Isekai")).toHaveAttribute("data-state", "include");
      expect(screen.getByRole("button", { name: "Magic" })).toHaveAttribute(
        "data-state",
        "off",
      );
    });

    it("seeds an excluded chip without touching its neighbours", async () => {
      renderPanel({
        tags: TAGS,
        current: { excludedTags: ["School"], sort: "popularity" },
      });

      expect(await chip("School")).toHaveAttribute("data-state", "exclude");
      expect(screen.getByRole("button", { name: "Isekai" })).toHaveAttribute(
        "data-state",
        "off",
      );
    });

    it("cycles a tag off -> include -> exclude -> off", async () => {
      renderPanel({ tags: TAGS });
      const isekai = await chip("Isekai");

      // Untouched: present but inert.
      expect(isekai).toHaveAttribute("data-state", "off");
      expect(submitted().tag).toEqual([]);
      expect(submitted().exclude_tag).toEqual([]);

      await fireEvent.click(isekai);
      expect(isekai).toHaveAttribute("data-state", "include");
      expect(submitted().tag).toEqual(["Isekai"]);

      await fireEvent.click(isekai);
      expect(isekai).toHaveAttribute("data-state", "exclude");
      expect(submitted().exclude_tag).toEqual(["Isekai"]);
      // The include must be gone, or the tag would be required AND rejected.
      expect(submitted().tag).toEqual([]);

      await fireEvent.click(isekai);
      expect(isekai).toHaveAttribute("data-state", "off");
      expect(submitted().exclude_tag).toEqual([]);
    });

    it("tracks each tag's state independently", async () => {
      renderPanel({ tags: TAGS });
      const isekai = await chip("Isekai");
      const school = screen.getByRole("button", { name: "School" });

      await fireEvent.click(isekai);
      await fireEvent.click(school);
      await fireEvent.click(school);

      expect(isekai).toHaveAttribute("data-state", "include");
      expect(school).toHaveAttribute("data-state", "exclude");
    });

    it("submits inclusions and exclusions as separate parameters", async () => {
      renderPanel({ tags: TAGS });
      const isekai = await chip("Isekai");
      const school = screen.getByRole("button", { name: "School" });

      await fireEvent.click(isekai);
      await fireEvent.click(school);
      await fireEvent.click(school);

      const { tag, exclude_tag } = submitted();
      expect(tag).toEqual(["Isekai"]);
      expect(exclude_tag).toEqual(["School"]);
    });
  });
});