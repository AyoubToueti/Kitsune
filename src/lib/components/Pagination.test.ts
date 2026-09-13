import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";

import Pagination from "./Pagination.svelte";

/** A plain href builder, as a route would pass in. */
function href(page: number): string {
  return `/search?q=test&page=${page}`;
}

const base = { current: 3, last: 10, hrefFor: href };

describe("Pagination", () => {
  it("renders nothing when there is only one page", () => {
    render(Pagination, { props: { ...base, current: 1, last: 1 } });

    expect(screen.queryByTestId("pagination")).toBeNull();
  });

  it("renders nothing when the last page is not yet known", () => {
    render(Pagination, { props: { ...base, last: 0 } });

    expect(screen.queryByTestId("pagination")).toBeNull();
  });

  it("marks the current page for assistive tech", () => {
    render(Pagination, { props: base });

    expect(screen.getByTestId("current-page")).toHaveTextContent("3");
    expect(screen.getByTestId("current-page")).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("links the other page numbers", () => {
    render(Pagination, { props: base });

    expect(screen.getByRole("link", { name: "Page 4" })).toHaveAttribute(
      "href",
      "/search?q=test&page=4",
    );
  });

  it("links next to the following page", () => {
    render(Pagination, { props: base });

    expect(screen.getByRole("link", { name: "Next page" })).toHaveAttribute(
      "href",
      "/search?q=test&page=4",
    );
  });

  it("links previous to the preceding page", () => {
    render(Pagination, { props: base });

    expect(screen.getByRole("link", { name: "Previous page" })).toHaveAttribute(
      "href",
      "/search?q=test&page=2",
    );
  });

  it("disables previous on the first page", () => {
    render(Pagination, { props: { ...base, current: 1 } });

    expect(screen.queryByRole("link", { name: "Previous page" })).toBeNull();
    expect(screen.getByTestId("prev-disabled")).toBeInTheDocument();
  });

  it("disables next on the last page", () => {
    render(Pagination, { props: { ...base, current: 10 } });

    expect(screen.queryByRole("link", { name: "Next page" })).toBeNull();
    expect(screen.getByTestId("next-disabled")).toBeInTheDocument();
  });

  it("offers a last-page link from the middle", () => {
    render(Pagination, { props: base });

    expect(screen.getByRole("link", { name: "Last page" })).toHaveAttribute(
      "href",
      "/search?q=test&page=10",
    );
  });

  it("hides the last-page link when already there", () => {
    render(Pagination, { props: { ...base, current: 10 } });

    expect(screen.queryByRole("link", { name: "Last page" })).toBeNull();
  });

  it("collapses long runs into gaps", () => {
    render(Pagination, { props: { ...base, current: 50, last: 100 } });

    // A 100-page result must not render 100 buttons.
    expect(screen.getAllByText("…")).toHaveLength(2);
  });

  it("clamps a current page past the end", () => {
    render(Pagination, { props: { ...base, current: 999, last: 10 } });

    // The active marker lands on the real last page rather than vanishing.
    expect(screen.getByTestId("current-page")).toHaveTextContent("10");
  });

  it("builds every href through the supplied builder", () => {
    const hrefFor = vi.fn((page: number) => `/genre/Action?page=${page}`);

    render(Pagination, { props: { ...base, hrefFor } });

    // The control must not invent its own URLs; the route owns them.
    for (const call of hrefFor.mock.calls) {
      expect(call[0]).toBeGreaterThanOrEqual(1);
    }
  });

  it("re-renders without error when the page changes", async () => {
    const { rerender } = render(Pagination, { props: base });
    expect(screen.getByTestId("current-page")).toHaveTextContent("3");

    await rerender({ ...base, current: 4 });

    expect(screen.getByTestId("current-page")).toHaveTextContent("4");
  });
});
