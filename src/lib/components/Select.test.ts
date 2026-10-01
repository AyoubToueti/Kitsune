import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import Select from "./Select.svelte";

const OPTIONS = [
  { value: "", label: "All" },
  { value: "tv", label: "TV" },
  { value: "movie", label: "Movie" },
];

function props(overrides: Record<string, unknown> = {}) {
  return { label: "Type", options: OPTIONS, ...overrides };
}

const trigger = () => screen.getByRole("combobox", { name: "Type" });

describe("Select", () => {
  it("shows the label of the current value", () => {
    render(Select, { props: props({ value: "tv" }) });

    expect(trigger()).toHaveTextContent("TV");
  });

  it("is closed until the trigger is pressed", () => {
    render(Select, { props: props() });

    expect(trigger()).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("opens the list on press", async () => {
    render(Select, { props: props() });

    await fireEvent.click(trigger());

    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(OPTIONS.length);
  });

  it("marks the current option as selected", async () => {
    render(Select, { props: props({ value: "tv" }) });

    await fireEvent.click(trigger());

    expect(screen.getByRole("option", { name: "TV" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("option", { name: "Movie" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });

  it("calls onchange with the chosen value", async () => {
    const onchange = vi.fn();
    render(Select, { props: props({ onchange }) });

    await fireEvent.click(trigger());
    await fireEvent.click(screen.getByRole("option", { name: "Movie" }));

    expect(onchange).toHaveBeenCalledWith("movie");
  });

  it("closes after choosing", async () => {
    render(Select, { props: props() });

    await fireEvent.click(trigger());
    await fireEvent.click(screen.getByRole("option", { name: "TV" }));

    expect(screen.queryByRole("listbox")).toBeNull();
  });

  // The whole point of this component: the open list is ours to style.
  it("renders a themed listbox, not a native select", async () => {
    render(Select, { props: props() });

    await fireEvent.click(trigger());

    const list = screen.getByRole("listbox");
    expect(list.className).toMatch(/bg-surface-raised/);
    expect(list.className).toMatch(/border-border-subtle/);
    // No native `<select>` anywhere: that popup is what we are replacing.
    expect(document.querySelector("select")).toBeNull();
  });

  it("submits its value through a hidden field", () => {
    const { container } = render(Select, {
      props: props({ name: "format", value: "tv" }),
    });

    expect(container.querySelector("input[type='hidden']")).toHaveAttribute(
      "name",
      "format",
    );
    expect(container.querySelector("input[type='hidden']")).toHaveAttribute(
      "value",
      "tv",
    );
  });

  it("renders no hidden field without a name", () => {
    const { container } = render(Select, { props: props() });

    expect(container.querySelector("input[type='hidden']")).toBeNull();
  });

  // --- keyboard -----------------------------------------------------------

  it("opens and moves with the arrow keys", async () => {
    render(Select, { props: props() });

    await fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(screen.getByRole("listbox")).toBeInTheDocument();

    // Second ArrowDown moves from the first option to the second.
    await fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    await fireEvent.keyDown(trigger(), { key: "Enter" });

    // `onchange` is the observable result of the keyboard choice.
    expect(trigger()).toHaveTextContent("TV");
  });

  it("closes on Escape", async () => {
    render(Select, { props: props() });

    await fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    await fireEvent.keyDown(trigger(), { key: "Escape" });

    expect(screen.queryByRole("listbox")).toBeNull();
  });
});