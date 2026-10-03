import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NO_PREVIOUS_SIGNATURES } from "../signing/previousSignatures";
import { previousSignatureOf, reportOf } from "../signing/SigningPanel.testSupport";
import { renderWithCatalog } from "../testing/render";
import type { ErrandStage } from "./errand";
import { SedeWindow } from "./SedeWindow";
import { certificate, elapse, scriptedErrand, signedDocument } from "./sedeWindowFixtures";

/** Grada A: la variante de origen «orden de terminal» de la ventana de sede (`-certgui`). */

const DOCUMENT_PATH = "/home/ada/contratos/convenio.pdf";
const terminal = { terminalOrder: { documentPath: DOCUMENT_PATH }, origin: null };

const consent = (overrides: Partial<Extract<ErrandStage, { kind: "consent" }>> = {}) =>
  ({
    kind: "consent",
    document: signedDocument,
    signs: null,
    signing: "pdf",
    items: null,
    certificates: [certificate()],
    narrowed: true,
    ...overrides,
  }) satisfies ErrandStage;

describe("terminal order origin", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("names the document in the title with its full path in the tooltip", async () => {
    const { port } = scriptedErrand(consent(), terminal);
    renderWithCatalog(<SedeWindow errands={port} />);
    await elapse(0);

    expect(screen.getByText("Firmar convenio.pdf")).toHaveAttribute("title", DOCUMENT_PATH);
  });

  it("shortens a long name in the middle, keeping the start, the end and the extension", async () => {
    const name = `${"a".repeat(30)}-informe-final-revisado.pdf`;
    const { port } = scriptedErrand(consent(), {
      ...terminal,
      terminalOrder: { documentPath: `/tmp/${name}` },
    });
    renderWithCatalog(<SedeWindow errands={port} />);
    await elapse(0);

    const title = screen.getByText(/^Firmar a+….*\.pdf$/);
    expect(title).toHaveAttribute("title", `/tmp/${name}`);
    expect(title.textContent?.length).toBeLessThan(name.length);
  });

  it("shows nothing aimed at the site and keeps the certificate select and Sign", async () => {
    const { port } = scriptedErrand(consent(), terminal);
    renderWithCatalog(<SedeWindow errands={port} />);
    await elapse(0);

    expect(screen.queryByText(/pide tu firma/)).toBeNull();
    expect(screen.queryByText(/sede/i)).toBeNull();
    expect(screen.queryByText(/Solicitud de subvención/)).toBeNull();
    expect(screen.getByRole("button", { name: /^Firmar/ })).toBeInTheDocument();
  });

  it("consents with the chosen certificate", async () => {
    vi.useRealTimers();
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand(consent(), terminal);
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    await user.click(await screen.findByRole("button", { name: "Firmar" }));

    expect(calls.consent).toHaveBeenCalledWith("handle-1");
  });

  it("shows the previous signatures box when the document has them", async () => {
    const { port } = scriptedErrand(
      consent({
        document: {
          ...signedDocument,
          previousSignatures: reportOf([previousSignatureOf()], {
            warningCount: 0,
            tone: "information",
          }),
        },
      }),
      terminal,
    );
    renderWithCatalog(<SedeWindow errands={port} />);
    await elapse(0);

    expect(screen.getByText(/Firmarás junto a 1 firma anterior/)).toBeInTheDocument();
  });

  it("omits the previous signatures box when there are none", async () => {
    const { port } = scriptedErrand(
      consent({ document: { ...signedDocument, previousSignatures: NO_PREVIOUS_SIGNATURES } }),
      terminal,
    );
    renderWithCatalog(<SedeWindow errands={port} />);
    await elapse(0);

    expect(screen.queryByText(/Firmarás junto a/)).toBeNull();
  });

  describe("without a usable certificate", () => {
    it("says none, with no body, and offers to install", () => {
      const { port } = scriptedErrand(
        { kind: "noCertificate", reason: "none", owned: 0 },
        terminal,
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("No tienes ningún certificado")).toBeInTheDocument();
      expect(screen.queryByText(/Necesitas uno instalado/)).toBeNull();
      expect(screen.queryByText(/FNMT/)).toBeNull();
      expect(screen.getByRole("button", { name: "Instalar un certificado…" })).toBeInTheDocument();
    });

    it("blames the --filter when it leaves none, and only offers to close", async () => {
      vi.useRealTimers();
      const user = userEvent.setup();
      const { port, calls } = scriptedErrand(
        { kind: "noCertificate", reason: "excluded", owned: 3 },
        terminal,
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(
        screen.getByText("Tu --filter no deja ninguno de tus 3 certificados"),
      ).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Instalar un certificado…" })).toBeNull();
      expect(screen.queryByText(/sede/i)).toBeNull();

      await user.click(screen.getByRole("button", { name: "Cerrar" }));
      expect(calls.cancel).toHaveBeenCalledOnce();
    });
  });
});
