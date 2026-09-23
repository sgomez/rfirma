import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { certificate, scriptedErrand } from "./sedeWindowFixtures";
import { SedeWindow } from "./SedeWindow";

/**
 * Grada A: el momento 2b (la confirmación que exige el validador), el 3 (la
 * firma) y el fichero que pide el diálogo del portal (TD-63).
 */

describe("2b · confirming what the validator flags", () => {
  it("asks in rFirma's own words, and offers exactly two ways out", () => {
    const { port } = scriptedErrand({
      kind: "confirming",
      messageCode: "pdfShadowAttackSuspect",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(/se ha modificado después de la última firma/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancelar" })).toBeInTheDocument();
  });

  it("says what a modified form is, which is another thing entirely", () => {
    const { port } = scriptedErrand({
      kind: "confirming",
      messageCode: "signingModifiedPdfForm",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(
      screen.getByText(/formulario cuyos campos se han cambiado después de firmarlo/i),
    ).toBeInTheDocument();
  });

  it("names the code when the message is one rFirma has no words for", () => {
    const { port } = scriptedErrand({ kind: "confirming", messageCode: "somethingNewer" });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(/somethingNewer/)).toBeInTheDocument();
  });

  it("goes on with the signature when the person confirms", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand({
      kind: "confirming",
      messageCode: "pdfShadowAttackSuspect",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Continuar" }));

    expect(calls.confirmSignatures).toHaveBeenCalled();
    expect(calls.cancel).not.toHaveBeenCalled();
  });

  it("hands the confirmation on only once, however many times the button is pressed", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand({
      kind: "confirming",
      messageCode: "pdfShadowAttackSuspect",
    });
    calls.confirmSignatures.mockReturnValue(new Promise(() => {}));
    renderWithCatalog(<SedeWindow errands={port} />);
    const going = screen.getByRole("button", { name: "Continuar" });

    await user.click(going);
    await user.click(going);

    expect(calls.confirmSignatures).toHaveBeenCalledOnce();
    expect(going).toBeDisabled();
  });

  it("abandons the errand when the person refuses, which is what the site gets", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand({
      kind: "confirming",
      messageCode: "pdfShadowAttackSuspect",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(calls.cancel).toHaveBeenCalled();
    expect(calls.confirmSignatures).not.toHaveBeenCalled();
  });
});

describe("3 · signing", () => {
  it("names no cryptographic phase, only the certificate the person just chose", () => {
    const { port } = scriptedErrand({
      kind: "signing",
      certificate: certificate(),
      phase: "signing",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Firmando")).toBeInTheDocument();
    expect(screen.getByText("Con ADA LOVELACE BYRON · 99999999R.")).toBeInTheDocument();
    expect(screen.queryByText(/prefirma|posfirma/i)).not.toBeInTheDocument();
  });

  it("can still be cancelled while rFirma signs: the site has received nothing", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand({
      kind: "signing",
      certificate: certificate(),
      phase: "signing",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(calls.cancel).toHaveBeenCalledOnce();
  });

  it("empties the footer once the answer is on its way, rather than lying with a button", () => {
    const { port } = scriptedErrand({
      kind: "signing",
      certificate: certificate(),
      phase: "returning",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Enviando la firma a sede.ejemplo.gob.es")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Cancelar" })).not.toBeInTheDocument();
  });

  it("moves the bar between the two moments so they do not look the same", () => {
    const { port } = scriptedErrand({
      kind: "signing",
      certificate: certificate(),
      phase: "returning",
    });
    const { rerender } = renderWithCatalog(<SedeWindow errands={port} />);
    const returning = screen.getByRole("progressbar").getAttribute("aria-valuenow");

    const earlier = scriptedErrand({
      kind: "signing",
      certificate: certificate(),
      phase: "signing",
    });
    rerender(<SedeWindow errands={earlier.port} />);

    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).not.toBe(returning);
  });
});

describe("the file the portal is asking about", () => {
  it("names the file the site proposed, and never a path", () => {
    const { port } = scriptedErrand({ kind: "saving", filename: "firma.pdf" });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Guardando firma.pdf")).toBeInTheDocument();
    expect(screen.queryByText(/\//)).not.toBeInTheDocument();
  });

  it("names the file as what it is when the site proposed none", () => {
    const { port } = scriptedErrand({ kind: "saving", filename: null });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Guardando el fichero")).toBeInTheDocument();
  });

  it("says whether the site asked for one file or several", () => {
    const { port } = scriptedErrand({ kind: "loading", multiple: true });
    const { rerender } = renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Cargando varios ficheros")).toBeInTheDocument();

    rerender(<SedeWindow errands={scriptedErrand({ kind: "loading", multiple: false }).port} />);

    expect(screen.getByText("Cargando un fichero")).toBeInTheDocument();
  });

  it("offers no action of its own: the person answers inside the portal dialog", () => {
    const { port } = scriptedErrand({ kind: "loading", multiple: false });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});
