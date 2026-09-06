import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { UnregisteredSignaturesDialog } from "./UnregisteredSignaturesDialog";

const noop = () => {};

function renderDialog(props: Partial<Parameters<typeof UnregisteredSignaturesDialog>[0]> = {}) {
  return renderWithCatalog(
    <UnregisteredSignaturesDialog onConfirm={noop} onCancel={noop} {...props} />,
  );
}

// Grada A: el aviso del ID-297…ID-301 y del ID-305.
describe("UnregisteredSignaturesDialog", () => {
  it("says the previous signatures are not understood, and that signing is still possible", () => {
    renderDialog();

    expect(
      screen.getByRole("dialog", { name: "Este PDF trae firmas que no entendemos" }),
    ).toBeVisible();
    expect(screen.getByRole("dialog")).toHaveTextContent("Puedes firmar igual");
  });

  // ID-305: ni recuento, ni titulares, ni un veredicto sobre lo que ya había.
  it("never counts the previous signatures nor calls any of them valid", () => {
    renderDialog();

    const dialog = screen.getByRole("dialog");
    expect(dialog).not.toHaveTextContent(/\d/);
    expect(dialog).not.toHaveTextContent(/válida[s]? desde|son válidas|firma válida de/i);
  });

  it("signs anyway on confirm, and cancels without signing", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    renderDialog({ onConfirm, onCancel });

    await user.click(screen.getByRole("button", { name: "Firmar de todos modos" }));
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onCancel).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
