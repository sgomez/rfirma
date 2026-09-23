import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RenderErrorBoundary } from "../errors/RenderErrorBoundary";
import type { Certificate } from "../signing/certificate";
import { renderWithCatalog } from "../testing/render";
import { OUTCOME_CLOSE_MS } from "./errand";
import { elapse, scriptedErrand, signedDocument } from "./sedeWindowFixtures";
import { SedeWindow } from "./SedeWindow";

/**
 * Grada A: el momento 5 (sin certificado utilizable), el fallo de un hijo y la
 * forma de la ventana (TD-63).
 */

describe("5 · no usable certificate", () => {
  it("offers the fix when there is none installed, because the fix is not the site's", () => {
    const { port } = scriptedErrand({ kind: "noCertificate", reason: "none", owned: 0 });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("No tienes ningún certificado")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Instalar un certificado…" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Volver a buscar" })).toBeInTheDocument();
    // Sin «Cerrar» la única salida sería la cruz del sistema, que ya no pasa
    // por aquí: la atiende `CloseRequested` en el backend.
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeInTheDocument();
  });

  /*
   * La barra de título es la del sistema, así que la cruz **no la pinta esta
   * ventana** y no hay dos puertas que comparar: la del pie es la única que
   * pasa por aquí. Irse por la del sistema llega a `CloseRequested`, y de que
   * eso abandone el trámite responde el backend (ID-340).
   */
  it("leaves through the footer, and the window paints no cross of its own", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand({ kind: "noCertificate", reason: "none", owned: 0 });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByRole("button", { name: "Cerrar la ventana" })).toBeNull();

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    // La sede no ha recibido nada: irse es abandonar el trámite, que es lo
    // único que libera el `idsession`.
    expect(calls.cancel).toHaveBeenCalledOnce();
    expect(calls.close).not.toHaveBeenCalled();
  });

  it("leaves no main action when the site excluded them all: installing another fixes nothing", () => {
    const { port } = scriptedErrand({ kind: "noCertificate", reason: "excluded", owned: 3 });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(
      screen.getByText("sede.ejemplo.gob.es no acepta ninguno de tus 3 certificados"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Instalar un certificado…" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Volver a buscar" })).not.toBeInTheDocument();
  });

  it("never enumerates what the site discarded", () => {
    const { port } = scriptedErrand({ kind: "noCertificate", reason: "excluded", owned: 3 });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByText(/ADA LOVELACE/)).not.toBeInTheDocument();
    expect(screen.queryByText(/criterio/i)).not.toBeInTheDocument();
  });

  it("looks again through the port, for a certificate installed with the window open", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand({ kind: "noCertificate", reason: "none", owned: 0 });
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Volver a buscar" }));

    expect(calls.lookAgain).toHaveBeenCalledOnce();
  });
});

describe("when a child throws", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // React registra el fallo en la consola además de pasarlo al boundary.
    vi.spyOn(console, "error").mockImplementation(() => {});
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("shows the failure screen instead of closing itself", async () => {
    // `SedeConsent` da por hecho que `certificates` existe desde su primera
    // línea: es el hijo más sencillo de hacer lanzar sin tocar su código.
    const { port, calls } = scriptedErrand({
      kind: "consent",
      document: null,
      signs: null,
      signing: "pdf",
      items: null,
      certificates: undefined as unknown as Certificate[],
      narrowed: false,
    });
    renderWithCatalog(
      <RenderErrorBoundary>
        <SedeWindow errands={port} />
      </RenderErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await elapse(OUTCOME_CLOSE_MS * 2);

    expect(calls.close).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});

describe("the window's shape", () => {
  it("has no application header, no menu, no tray and no destination footer", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "cancelled", document: signedDocument },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByRole("banner")).not.toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.queryByText(/se guardará en/i)).not.toBeInTheDocument();
  });
});
