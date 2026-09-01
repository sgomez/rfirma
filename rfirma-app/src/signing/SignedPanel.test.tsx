import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { SignedPanel } from "./SignedPanel";

const noop = () => {};

function renderPanel(props: Partial<Parameters<typeof SignedPanel>[0]> = {}) {
  return renderWithCatalog(
    <SignedPanel
      document={{ name: "contrato-firmado.pdf", pages: 27 }}
      onSignAnother={noop}
      {...props}
    />,
  );
}

// Grada A: el panel son datos y una devolución de llamada; no habla con nadie.
describe("SignedPanel", () => {
  it("names the file that was written and not the one that was opened", () => {
    renderPanel();

    expect(screen.getByText("contrato-firmado.pdf")).toBeInTheDocument();
  });

  it("says the format, which is always PAdES", () => {
    renderPanel();

    expect(screen.getByText("PAdES")).toBeInTheDocument();
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
    renderPanel({ document: { name: "contrato-firmado.pdf", pages: null } });

    expect(screen.queryByText(/página/)).not.toBeInTheDocument();
  });

  it("offers signing another document as the only way out", async () => {
    const again = vi.fn();
    renderPanel({ onSignAnother: again });

    await userEvent.click(screen.getByRole("button", { name: "Firmar otro documento" }));

    expect(again).toHaveBeenCalledOnce();
  });
});
