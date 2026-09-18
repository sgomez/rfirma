import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";

describe("StatusView", () => {
  it("renders the title and close button", () => {
    renderWithCatalog(<StatusView onClose={() => {}} />);

    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeInTheDocument();
  });

  it("calls onClose when clicking Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("calls onClose when pressing Escape", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does not call onClose when Escape was default-prevented", () => {
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    const event = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    event.preventDefault();
    window.dispatchEvent(event);

    expect(onClose).not.toHaveBeenCalled();
  });

  it("keeps the close button in a footer that is sibling to the scrollable body (WCAG 2.4.11)", () => {
    const { container } = renderWithCatalog(<StatusView onClose={() => {}} />);

    const body = container.querySelector(".status-view__body");
    const footer = container.querySelector(".status-view__footer");

    expect(body).not.toBeNull();
    expect(footer).not.toBeNull();
    expect(body?.nextElementSibling).toBe(footer);
    expect(footer?.parentElement).toBe(body?.parentElement);
  });
});
