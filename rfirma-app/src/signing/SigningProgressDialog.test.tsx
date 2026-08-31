import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { SigningStage } from "./flow";
import { SigningProgressDialog } from "./SigningProgressDialog";

function renderProgress(stage: SigningStage) {
  return renderWithCatalog(<SigningProgressDialog stage={stage} />);
}

// Grada A: el diálogo solo sabe en qué etapa va.
describe("SigningProgressDialog", () => {
  it("blocks the window while the three stages run", () => {
    const { container } = renderProgress("sign");

    const dialog = screen.getByRole("dialog", { name: "Firmando el documento…" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(container.querySelector(".rf-scrim")).toBeInTheDocument();
    // No hay salida: una vez empezada la firma en la tarjeta no hay marcha atrás.
    expect(within(dialog).queryAllByRole("button")).toEqual([]);
  });

  it("warns about not removing the card", () => {
    renderProgress("presign");

    expect(screen.getByText("No retires la tarjeta hasta que termine.")).toBeInTheDocument();
  });

  it("names the three stages in plain language, with the domain term alongside", () => {
    renderProgress("presign");

    const stages = screen.getAllByRole("listitem").map((item) => item.textContent);
    expect(stages[0]).toContain("Preparando la firma");
    expect(stages[0]).toContain("(prefirma)");
    expect(stages[1]).toContain("Firmando en la tarjeta");
    // La firma no lleva paréntesis: ya dice exactamente lo que pasa.
    expect(stages[1]).not.toContain("(");
    expect(stages[2]).toContain("Ensamblando el PDF");
    expect(stages[2]).toContain("(postfirma)");
  });

  it("marks what is done, what is under way and what is still pending", () => {
    renderProgress("sign");

    const stages = screen.getAllByRole("listitem");
    expect(stages[0]).toHaveTextContent("Hecha");
    expect(stages[1]).toHaveTextContent("En curso");
    expect(stages[2]).toHaveTextContent("Pendiente");
  });

  it("advances the bar with the stage", () => {
    renderProgress("postsign");

    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "3");
    expect(bar).toHaveAttribute("aria-valuemax", "3");
  });
});
