import { fireEvent, render, screen } from "@testing-library/react";
import { type ReactNode, useRef, useState } from "react";
import { describe, expect, it } from "vitest";
import { Popover, type PopoverProps } from "./Popover";

function Harness({
  children = "contenido",
  ...props
}: Partial<Omit<PopoverProps, "anchorRef" | "open" | "onClose">> & { children?: ReactNode }) {
  const [open, setOpen] = useState(true);
  const anchor = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button type="button">fuera</button>
      <div ref={anchor}>
        <button type="button" ref={trigger} onClick={() => setOpen(true)}>
          abrir
        </button>
        <Popover
          open={open}
          onClose={() => setOpen(false)}
          anchorRef={anchor}
          returnFocusRef={trigger}
          role="menu"
          tabIndex={-1}
          {...props}
        >
          {children}
        </Popover>
      </div>
    </>
  );
}

describe("Popover", () => {
  it("closes on Escape and gives the focus back to the trigger", () => {
    render(<Harness />);

    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });

    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByRole("button", { name: "abrir" })).toHaveFocus();
  });

  it("closes on a press outside without taking the focus", () => {
    render(<Harness restoreFocus="always" />);
    screen.getByRole("button", { name: "fuera" }).focus();

    fireEvent.pointerDown(screen.getByRole("button", { name: "fuera" }));

    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByRole("button", { name: "fuera" })).toHaveFocus();
  });

  it("stays open on a press inside the panel or its anchor", () => {
    render(<Harness />);

    fireEvent.pointerDown(screen.getByRole("menu"));
    fireEvent.pointerDown(screen.getByRole("button", { name: "abrir" }));

    expect(screen.getByRole("menu")).toBeInTheDocument();
  });

  it("closes on Tab and lets the focus go on its way", () => {
    render(<Harness restoreFocus="always" />);
    screen.getByRole("button", { name: "fuera" }).focus();

    fireEvent.keyDown(screen.getByRole("menu"), { key: "Tab" });

    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByRole("button", { name: "fuera" })).toHaveFocus();
  });

  it("focuses the panel when it opens if asked to", () => {
    render(<Harness initialFocus="panel" />);

    expect(screen.getByRole("menu")).toHaveFocus();
  });

  it("renders in a portal under the anchor, placed with the window's room", () => {
    render(<Harness portal={{ maxHeight: 100000 }} />);

    const panel = screen.getByRole("menu");

    expect(panel.parentElement).toBe(document.body);
    expect(panel.style.position).toBe("fixed");
    expect(panel.style.maxHeight).toBe(`${window.innerHeight - 4 - 8}px`);
  });
});
