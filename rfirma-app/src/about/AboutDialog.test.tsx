import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { AboutDialog } from "./AboutDialog";

const noop = () => {};

// Grada A. Lo que se comprueba es el **contenido** obligatorio, no la estética.
describe("AboutDialog", () => {
  it("declares that rFirma is not the official client", () => {
    renderWithCatalog(<AboutDialog version="0.1.0" onClose={noop} />);

    const notice = screen.getByText(/Proyecto independiente/);
    expect(notice).toHaveTextContent(/no está relacionada con AutoFirma/);
    expect(notice).toHaveTextContent(/ni cuenta con su respaldo/);
  });

  it("states that neither the document nor the private key leaves the computer", () => {
    renderWithCatalog(<AboutDialog version="0.1.0" onClose={noop} />);

    expect(
      screen.getByText(/El documento y la clave privada no salen de tu ordenador/),
    ).toBeInTheDocument();
  });

  it("shows the version", () => {
    renderWithCatalog(<AboutDialog version="0.1.0" onClose={noop} />);

    expect(screen.getByText("Versión 0.1.0")).toBeInTheDocument();
  });

  it("shows both licences", async () => {
    const user = userEvent.setup();
    renderWithCatalog(<AboutDialog version="0.1.0" onClose={noop} />);

    await user.click(screen.getByRole("button", { name: "Ver las licencias" }));

    expect(screen.getByText("rFirma: EUPL-1.2.")).toBeInTheDocument();
    expect(
      screen.getByText("Bibliotecas de Cliente @firma: GPL-2.0+ / EUPL-1.1."),
    ).toBeInTheDocument();
  });

  it("closes on Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<AboutDialog version="0.1.0" onClose={onClose} />);

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });
});
