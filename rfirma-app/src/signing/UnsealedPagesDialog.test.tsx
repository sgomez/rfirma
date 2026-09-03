import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { UnsealedPagesDialog } from "./UnsealedPagesDialog";

const noop = () => {};

function renderDialog(props: Partial<Parameters<typeof UnsealedPagesDialog>[0]> = {}) {
  return renderWithCatalog(
    <UnsealedPagesDialog fallen={3} chosen={13} onConfirm={noop} onCancel={noop} {...props} />,
  );
}

// Grada A: el diálogo del ID-105/ID-106, contra docs/design/dialogo-paginas-sin-sello.md.
describe("UnsealedPagesDialog", () => {
  it("counts the pages that fall against the chosen set, not the document", () => {
    renderDialog({ fallen: 3, chosen: 13 });

    expect(screen.getByRole("dialog", { name: "3 páginas se quedarán sin sello" })).toBeVisible();
    expect(
      screen.getByText(
        "El recuadro no cabe en 3 de las 13 páginas que has elegido, más pequeñas que aquella " +
          "sobre la que lo colocaste. El documento se firmará igual y la firma será válida en " +
          "todo él, pero en esas páginas no aparecerá el sello.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("El sello aparecerá en 10 de las 13 páginas elegidas."),
    ).toBeInTheDocument();
  });

  // ID-106: nunca una lista de números, ni con doce cayéndose.
  it("never names a fallen page, however many fall", () => {
    renderDialog({ fallen: 12, chosen: 13 });

    expect(screen.getByRole("dialog")).toHaveTextContent("12");
    expect(screen.queryByText(/\b1\b.*\b2\b.*\b3\b/)).not.toBeInTheDocument();
  });

  it("uses the singular for a single page, in the title and the recount", () => {
    renderDialog({ fallen: 1, chosen: 5 });

    expect(screen.getByRole("dialog", { name: "Una página se quedará sin sello" })).toBeVisible();
    expect(
      screen.getByText("El sello aparecerá en 4 de las 5 páginas elegidas."),
    ).toBeInTheDocument();
  });

  it("says 'sin sello', never 'recortadas'", () => {
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
