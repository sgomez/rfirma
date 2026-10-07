import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusWindow } from "./StatusWindow";
import { memoryStatus } from "./status";

describe("StatusWindow", () => {
  it("calls onClose when clicking Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusWindow statusPort={memoryStatus()} onClose={onClose} />);

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("calls onClose when pressing Escape", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusWindow statusPort={memoryStatus()} onClose={onClose} />);

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does not call onClose when Escape was default-prevented", () => {
    const onClose = vi.fn();
    renderWithCatalog(<StatusWindow statusPort={memoryStatus()} onClose={onClose} />);

    const event = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    event.preventDefault();
    document.dispatchEvent(event);

    expect(onClose).not.toHaveBeenCalled();
  });

  it("does nothing on Enter", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const statusPort = memoryStatus();
    const withdrawRfirma = vi.spyOn(statusPort, "withdrawRfirma");
    renderWithCatalog(<StatusWindow statusPort={statusPort} onClose={onClose} />);

    await user.keyboard("{Enter}");

    expect(onClose).not.toHaveBeenCalled();
    expect(withdrawRfirma).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
