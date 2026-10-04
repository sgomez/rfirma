import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { UnsealedPagesDialog } from "./UnsealedPagesDialog";

const noop = () => {};

function renderDialog(props: Partial<Parameters<typeof UnsealedPagesDialog>[0]> = {}) {
  return renderWithCatalog(
    <UnsealedPagesDialog fallen={3} onConfirm={noop} onCancel={noop} {...props} />,
  );
}

// Grada A: el diálogo del ID-105/ID-106, contra docs/design/dialogo-paginas-sin-firma-visible.md.
describe("UnsealedPagesDialog", () => {
  it("says how many pages fall and that the signature stays valid", () => {
    renderDialog({ fallen: 3 });

    expect(
      screen.getByRole("dialog", { name: "3 páginas se quedarán sin firma visible" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "El recuadro no cabe en páginas más pequeñas que aquella donde lo colocaste. " +
          "La firma será válida en todo el documento.",
      ),
    ).toBeInTheDocument();
  });

  // ID-106: nunca una lista de números, ni con doce cayéndose.
  it("never names a fallen page, however many fall", () => {
    renderDialog({ fallen: 12 });

    expect(screen.getByRole("dialog")).toHaveTextContent("12");
    expect(screen.queryByText(/\b1\b.*\b2\b.*\b3\b/)).not.toBeInTheDocument();
  });

  it("uses the singular in the title for a single page", () => {
    renderDialog({ fallen: 1 });

    expect(
      screen.getByRole("dialog", { name: "Una página se quedará sin firma visible" }),
    ).toBeVisible();
  });

  it("says 'sin firma visible', never 'recortadas'", () => {
    renderDialog();

    expect(screen.queryByText(/recortad/i)).not.toBeInTheDocument();
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
