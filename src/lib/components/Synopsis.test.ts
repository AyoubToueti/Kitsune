import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import Synopsis from "./Synopsis.svelte";

describe("Synopsis", () => {
  it("renders stripped text from light HTML", () => {
    render(Synopsis, {
      props: {
        text: "A boy discovers <i>magic</i>.<br>He fights evil.",
      },
    });

    expect(
      screen.getByText(/A boy discovers magic\. He fights evil\./),
    ).toBeInTheDocument();
  });

  it("decodes common HTML entities", () => {
    render(Synopsis, {
      props: { text: "The &quot;Hero&quot; returns &amp; saves everyone." },
    });

    expect(
      screen.getByText('The "Hero" returns & saves everyone.'),
    ).toBeInTheDocument();
  });

  it("hides when the description is empty after stripping", () => {
    const { container } = render(Synopsis, {
      props: { text: "   <br><br>   " },
    });

    expect(container.querySelector("p")).not.toBeInTheDocument();
  });

  it("hides when the description is absent", () => {
    const { container } = render(Synopsis, { props: {} });

    expect(container.querySelector("p")).not.toBeInTheDocument();
  });

  it("does not inject raw HTML", () => {
    const { container } = render(Synopsis, {
      props: { text: '<img src="x" onerror="alert(1)">' },
    });

    // The tag should be stripped to text, not rendered as an element.
    expect(container.querySelector("img")).not.toBeInTheDocument();
  });
});