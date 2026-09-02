import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { SignedPanel } from "./SignedPanel";

const noop = () => {};

function renderPanel(props: Partial<Parameters<typeof SignedPanel>[0]> = {}) {
  return renderWithCatalog(
    <SignedPanel
      document={{ name: "contrato-firmado.pdf", pages: 27, sizeBytes: 2_400_000 }}
      onOpenDocument={noop}
      onOpenFolder={noop}
      onSignAgain={noop}
      {...props}
    />,
  );
}

// Grada A: el panel son datos y tres devoluciones de llamada; no habla con nadie.
describe("SignedPanel", () => {
  it("names the file that was written and not the one that was opened", () => {
    renderPanel();

    expect(screen.getByText("contrato-firmado.pdf")).toBeInTheDocument();
  });

  it("shows the pages and the size the postsign already knew", () => {
    // El tamaño no lo calcula el panel ni se relee del fichero: llega contado
    // desde la escritura (ID-77).
    renderPanel();

    expect(screen.getByText("27 páginas · 2,4 MB")).toBeInTheDocument();
  });

  it("says the format, which is always PAdES", () => {
    renderPanel();

    expect(screen.getByText("PAdES")).toBeInTheDocument();
  });

  /**
   * El encabezado `Resumen` se queda con una sola insignia debajo porque
   * **guarda el sitio de la ficha 14** (ID-78): ahí irán el número de firmas y
   * la tarjeta de cada una. Esta prueba es lo que impide que alguien lo quite
   * por parecer vacío.
   */
  it("keeps the summary heading that holds the place of the signature cards", () => {
    renderPanel();

    expect(screen.getByRole("region", { name: "Resumen" })).toBeInTheDocument();
  });

  it("does not take up space with what nobody counted", () => {
    // Ni la insignia con el número de firmas ni las tarjetas de cada firma se
    // montan: nadie lee todavía las firmas del PDF resultante, y un hueco con
    // un guion diría «no tiene», que es falso (ID-44).
    renderPanel();

    expect(screen.queryByText(/firmas?$/)).not.toBeInTheDocument();
    expect(screen.queryByText(/—|–/)).not.toBeInTheDocument();
  });

  it("does not invent a page count it was not given", () => {
    renderPanel({
      document: { name: "contrato-firmado.pdf", pages: null, sizeBytes: 2_400_000 },
    });

    expect(screen.queryByText(/página/)).not.toBeInTheDocument();
    // Y el tamaño, que sí se sabe, se sigue enseñando solo.
    expect(screen.getByText("2,4 MB")).toBeInTheDocument();
  });

  /**
   * Los tres botones del pie, en el orden y con la jerarquía del ID-79. Los dos
   * de abrir no son comodidad: bajo el arenero son la única forma que tiene el
   * usuario de llegar a un fichero cuya ruta nunca ve (ADR-0011).
   */
  it("offers three ways out, stacked and in the hierarchy of the artboard", () => {
    renderPanel();

    const buttons = screen.getAllByRole("button");
    expect(buttons.map((button) => button.textContent)).toEqual([
      "Abrir el PDF",
      "Abrir la carpeta",
      "Volver a firmar",
    ]);
    expect(buttons[0]).toHaveClass("rf-btn--primary");
    expect(buttons[1]).toHaveClass("rf-btn--secondary");
    expect(buttons[2]).toHaveClass("rf-btn--ghost");
  });

  it("no longer offers signing another document", () => {
    // Lo hubo y se retira: la bandeja ya ofrece abrir y aceptar arrastre, y dos
    // caminos para lo mismo es uno de más (ID-79).
    renderPanel();

    expect(screen.queryByRole("button", { name: "Firmar otro documento" })).not.toBeInTheDocument();
  });

  it("opens the signed PDF", async () => {
    const open = vi.fn();
    renderPanel({ onOpenDocument: open });

    await userEvent.click(screen.getByRole("button", { name: "Abrir el PDF" }));

    expect(open).toHaveBeenCalledOnce();
  });

  it("opens the folder where it landed", async () => {
    const open = vi.fn();
    renderPanel({ onOpenFolder: open });

    await userEvent.click(screen.getByRole("button", { name: "Abrir la carpeta" }));

    expect(open).toHaveBeenCalledOnce();
  });

  it("goes back to sign the same document again", async () => {
    const again = vi.fn();
    renderPanel({ onSignAgain: again });

    await userEvent.click(screen.getByRole("button", { name: "Volver a firmar" }));

    expect(again).toHaveBeenCalledOnce();
  });

  it("says why it could not open instead of leaving the button doing nothing", () => {
    // Un botón que no hace nada y no dice por qué deja al usuario sin ninguna
    // forma de llegar a lo que acaba de firmar.
    renderPanel({
      failure: { situation: "unknown", detail: "no portal responded", attemptsLeft: null },
    });

    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});
