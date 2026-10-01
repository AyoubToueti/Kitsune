import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import ModalFixture from "../../test/ModalFixture.svelte";

afterEach(() => {
  document.body.style.overflow = "";
});

describe("Modal", () => {
  it("renders nothing when closed", () => {
    render(ModalFixture, { props: { open: false } });
    expect(screen.queryByTestId("modal-dialog")).toBeNull();
  });

  it("renders the dialog and footer when open", () => {
    render(ModalFixture, { props: { open: true } });
    expect(screen.getByTestId("modal-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("footer-button")).toBeInTheDocument();
  });

  it("calls onClose when Escape is pressed", async () => {
    const onClose = vi.fn();
    render(ModalFixture, { props: { open: true, onClose } });

    await fireEvent.keyDown(screen.getByTestId("modal-dialog"), {
      key: "Escape",
    });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes on a backdrop click but not on a dialog click", async () => {
    const onClose = vi.fn();
    render(ModalFixture, { props: { open: true, onClose } });

    // A click inside the dialog must not close it.
    await fireEvent.click(screen.getByTestId("first-button"));
    expect(onClose).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByTestId("modal-backdrop"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("locks the page scroll while open and restores it on close", async () => {
    const { rerender } = render(ModalFixture, { props: { open: true } });
    expect(document.body.style.overflow).toBe("hidden");

    await rerender({ open: false });
    expect(document.body.style.overflow).not.toBe("hidden");
  });

  it("restores focus to the element that opened it", async () => {
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();
    expect(document.activeElement).toBe(opener);

    const { rerender } = render(ModalFixture, { props: { open: true } });
    await rerender({ open: false });

    expect(document.activeElement).toBe(opener);
    opener.remove();
  });
});