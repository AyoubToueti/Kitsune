import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import ReleaseModeToggle from "./ReleaseModeToggle.svelte";

describe("ReleaseModeToggle", () => {
  it("renders both segments", () => {
    render(ReleaseModeToggle, { props: { mode: "episodes" } });

    expect(screen.getByText("Episodes")).toBeInTheDocument();
    expect(screen.getByText("Packs")).toBeInTheDocument();
  });

  it("marks the current mode pressed and the other not", () => {
    render(ReleaseModeToggle, { props: { mode: "packs" } });

    expect(screen.getByText("Packs")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("Episodes")).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("calls onChange with the clicked segment's mode", async () => {
    const onChange = vi.fn();
    render(ReleaseModeToggle, { props: { mode: "episodes", onChange } });

    await fireEvent.click(screen.getByText("Packs"));

    expect(onChange).toHaveBeenCalledWith("packs");
  });

  it("does not crash without an onChange handler", async () => {
    // A read-only render must not throw on click.
    render(ReleaseModeToggle, { props: { mode: "episodes" } });

    await fireEvent.click(screen.getByText("Packs"));
  });
});