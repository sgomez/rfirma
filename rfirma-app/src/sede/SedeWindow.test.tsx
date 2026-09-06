import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Certificate } from "../signing/certificate";
import { renderWithCatalog } from "../testing/render";
import type { Errand, ErrandStage, SiteErrandPort } from "./errand";
import { noErrand, OUTCOME_CLOSE_MS, UNREACHABLE_AFTER_MS, WAITING_GRACE_MS } from "./errand";
import { SedeWindow } from "./SedeWindow";

/**
 * Grada A: la ventana de sede entera, **por su puerto** (TD-63). No hay
 * backend, no hay canal y no hay Tauri: un doble de `SiteErrandPort` que emite
 * los momentos, y las conductas se leen en la pantalla.
 */

function certificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "handle-1",
    label: "FNMT",
    holderName: "ADA LOVELACE BYRON",
    idNumber: "99999999R",
    issuer: "FNMT-RCM",
    store: "installed",
    status: { kind: "valid", notAfter: 4_102_444_800 },
    remembered: false,
    ...overrides,
  };
}

/** Un puerto que emite el momento que se le pida, y anota lo que se le llama. */
function scriptedErrand(stage: ErrandStage, errand: Partial<Errand> = {}) {
  const calls = {
    consent: vi.fn(),
    submitSecret: vi.fn(),
    cancel: vi.fn(),
    close: vi.fn(),
    lookAgain: vi.fn(),
    installCertificate: vi.fn(),
    installLocalCa: vi.fn(),
  };
  const port: SiteErrandPort = {
    ...noErrand(),
    watch: (onChange) => {
      onChange({ origin: "sede.ejemplo.gob.es", operation: "sign", stage, ...errand });
      return () => {};
    },
    consent: async (id) => calls.consent(id),
    submitSecret: async (secret) => calls.submitSecret(secret),
    cancel: async () => calls.cancel(),
    close: async () => calls.close(),
    lookAgain: async () => calls.lookAgain(),
    installCertificate: async () => calls.installCertificate(),
    installLocalCa: async () => calls.installLocalCa(),
  };
  return { port, calls };
}

/** Deja pasar el tiempo con los relojes falsos, y deja que React repinte. */
async function elapse(ms: number) {
  await act(async () => {
    vi.advanceTimersByTime(ms);
  });
}

describe("SedeWindow", () => {
  it("does not mount at all when no site has called", () => {
    renderWithCatalog(<SedeWindow errands={noErrand()} />);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  describe("1 · waiting for the channel", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    it("paints nothing during the grace delay: the happy path must not flash", async () => {
      const { port } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

      await elapse(WAITING_GRACE_MS);

      expect(screen.getByText("Conectando con la sede")).toBeInTheDocument();
    });

    it("crosses its single threshold into «the request has not arrived», and never closes", async () => {
      const { port, calls } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapse(UNREACHABLE_AFTER_MS);

      expect(screen.getByText("La petición no ha llegado")).toBeInTheDocument();
      expect(calls.close).not.toHaveBeenCalled();
      expect(calls.cancel).not.toHaveBeenCalled();
    });

    it("offers two recipes and never diagnoses which one is the problem", async () => {
      const { port } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapse(UNREACHABLE_AFTER_MS);

      expect(screen.getByRole("tab", { name: "Chrome" })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("tab", { name: "Firefox" })).toHaveAttribute(
        "aria-selected",
        "false",
      );
      expect(screen.getByText(/franja bajo la barra de direcciones/)).toBeInTheDocument();
    });

    it("puts the mandatory sentence in the footer, and has no Retry button of its own", async () => {
      const { port } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapse(UNREACHABLE_AFTER_MS);

      expect(screen.getByText(/vuelve a la sede y pulsa Reintentar/)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Reintentar" })).not.toBeInTheDocument();
    });

    it("abandons the errand when closed while waiting, with no confirmation", async () => {
      // `fireEvent` y no `userEvent`: con relojes falsos, `userEvent` espera a
      // temporizadores reales que nadie va a adelantar y la prueba se cuelga.
      const { port, calls } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);
      await elapse(WAITING_GRACE_MS);

      fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));

      expect(calls.cancel).toHaveBeenCalledOnce();
    });
  });

  describe("2 · consent", () => {
    const consenting = (overrides: Partial<Extract<ErrandStage, { kind: "consent" }>> = {}) =>
      ({
        kind: "consent",
        document: {
          title: "Solicitud de subvención 2026",
          pages: 27,
          sizeBytes: 2_400_000,
          signatures: 0,
        },
        certificates: [certificate()],
        narrowed: false,
        ...overrides,
      }) satisfies ErrandStage;

    it("names the site plainly and says what it asks for", () => {
      const { port } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("sede.ejemplo.gob.es pide tu firma.")).toBeInTheDocument();
    });

    it("shows the PDF metadata title, pages, size — never a file name", () => {
      const { port } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Solicitud de subvención 2026")).toBeInTheDocument();
      expect(screen.getByText("27 páginas · 2,4 MB")).toBeInTheDocument();
    });

    it("names an untitled PDF for what it is instead of inventing one", () => {
      const { port } = scriptedErrand(
        consenting({
          document: { title: null, pages: 8, sizeBytes: 310_000, signatures: 0 },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Un PDF sin título")).toBeInTheDocument();
    });

    it("warns that the signature will be a countersignature when the PDF is already signed", () => {
      const { port } = scriptedErrand(
        consenting({
          document: { title: "Convenio", pages: 12, sizeBytes: 860_000, signatures: 1 },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(/la tuya será una cofirma/)).toBeInTheDocument();
    });

    it("leaves a calm label, not a warning, when the request has no valid origin", () => {
      const { port } = scriptedErrand(consenting(), { origin: null });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Origen sin identificar")).toBeInTheDocument();
      expect(
        screen.getByText("La petición pide una firma y no indica de qué página viene."),
      ).toBeInTheDocument();
    });

    it("says the site narrowed the list, and never what it discarded nor why", () => {
      const { port } = scriptedErrand(consenting({ narrowed: true }));
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(
        screen.getByText("sede.ejemplo.gob.es ha limitado los certificados válidos."),
      ).toBeInTheDocument();
      expect(screen.queryByText(/criterio|descartad/i)).not.toBeInTheDocument();
    });

    it("appears with a single certificate too: a site never causes a silent signature", () => {
      const { port } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Firmar" })).toBeInTheDocument();
    });

    it("consents with the chosen certificate's handle", async () => {
      const user = userEvent.setup();
      const { port, calls } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} />);

      await user.click(screen.getByRole("button", { name: "Firmar" }));

      expect(calls.consent).toHaveBeenCalledWith("handle-1");
    });

    it("says «Identificarse», not «Firmar», for selectcert", () => {
      const { port } = scriptedErrand(consenting({ document: null }), {
        operation: "selectcert",
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Identificarse" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Firmar" })).not.toBeInTheDocument();
      expect(screen.getByText("sede.ejemplo.gob.es pide que te identifiques.")).toBeInTheDocument();
    });

    it("spells out what selectcert sends", () => {
      const { port } = scriptedErrand(consenting({ document: null }), {
        operation: "selectcert",
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(/Se enviarán tu nombre, tu NIF/)).toBeInTheDocument();
    });
  });

  describe("the store's secret", () => {
    it("has no screen of its own: it is the same dialog as the local route", () => {
      const { port } = scriptedErrand({
        kind: "secret",
        certificate: certificate({ store: "card" }),
        failure: null,
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByLabelText("PIN")).toBeInTheDocument();
      // Y debajo sigue el momento de firma, que es donde el trámite está.
      expect(screen.getByText("Firmando")).toBeInTheDocument();
    });

    it("hands the typed secret back through the port", async () => {
      const user = userEvent.setup();
      const { port, calls } = scriptedErrand({
        kind: "secret",
        certificate: certificate({ store: "card" }),
        failure: null,
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      await user.type(screen.getByLabelText("PIN"), "1234");
      await user.click(screen.getByRole("button", { name: "Firmar" }));

      expect(calls.submitSecret).toHaveBeenCalledWith("1234");
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

  describe("4 · outcome", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    it("says rFirma keeps no copy, which is the one thing you cannot deduce", () => {
      const { port } = scriptedErrand({ kind: "outcome", outcome: { kind: "signed" } });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Firmado y enviado")).toBeInTheDocument();
      expect(screen.getByText("rFirma no guarda copia.")).toBeInTheDocument();
    });

    it("adds nothing to a cancellation: the title already says it", () => {
      const { port } = scriptedErrand({ kind: "outcome", outcome: { kind: "cancelled" } });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Has cancelado la firma")).toBeInTheDocument();
      expect(screen.queryByText(/no se ha firmado nada/i)).not.toBeInTheDocument();
    });

    it("states a refusal without blaming anyone, and leaves the raw detail copiable", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: {
          kind: "refused",
          situation: "appendedSignaturePage",
          detail: "signaturePages=append",
        },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("rFirma ha rechazado la petición")).toBeInTheDocument();
      expect(
        screen.getByText(
          "sede.ejemplo.gob.es pide colocar la firma en una página añadida al final, y rFirma no hace eso.",
        ),
      ).toBeInTheDocument();
      expect(screen.getByText("signaturePages=append")).toBeInTheDocument();
      expect(screen.queryByText(/el fallo es de/i)).not.toBeInTheDocument();
    });

    it("closes by itself after fifteen seconds, and not before", async () => {
      const { port, calls } = scriptedErrand({ kind: "outcome", outcome: { kind: "signed" } });
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapse(OUTCOME_CLOSE_MS - 1_000);
      expect(calls.close).not.toHaveBeenCalled();

      await elapse(1_000);
      expect(calls.close).toHaveBeenCalledOnce();
    });
  });

  describe("5 · no usable certificate", () => {
    it("offers the fix when there is none installed, because the fix is not the site's", () => {
      const { port } = scriptedErrand({ kind: "noCertificate", reason: "none", owned: 0 });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("No tienes ningún certificado")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Instalar un certificado…" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Volver a buscar" })).toBeInTheDocument();
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

  describe("the window's shape", () => {
    it("has no application header, no menu, no tray and no destination footer", () => {
      const { port } = scriptedErrand({ kind: "outcome", outcome: { kind: "cancelled" } });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.queryByRole("banner")).not.toBeInTheDocument();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(screen.queryByText(/se guardará en/i)).not.toBeInTheDocument();
    });
  });
});
