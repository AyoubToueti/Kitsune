import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import Fixture from "./Fixture.svelte";

// These tests do not exercise app behaviour. They prove the frontend test
// harness itself is wired up, so a later failure points at the module under
// test rather than at the tooling.

describe("frontend test harness", () => {
  it("runs basic assertions", () => {
    expect(1 + 1).toBe(2);
  });

  it("provides a jsdom document", () => {
    const el = document.createElement("div");
    el.textContent = "hello";
    document.body.appendChild(el);

    expect(el).toBeInTheDocument();
    expect(el).toHaveTextContent("hello");

    el.remove();
  });

  it("renders a Svelte component with testing-library", () => {
    render(Fixture, { props: { name: "Kitsune" } });

    // Proves the whole path: Svelte 5 compiles under Vitest, jsdom
    // renders it, testing-library queries it, and jest-dom matchers work.
    expect(screen.getByText("Hello, Kitsune!")).toBeInTheDocument();
  });
});