import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Certificate } from "../signing/certificate";
import { renderWithCatalog } from "../testing/render";
import type { Errand, ErrandStage, SiteDocument, SiteErrandPort } from "./errand";
import { CHROME_LOCAL_NETWORK_SETTINGS, noErrand, OUTCOME_CLOSE_MS } from "./errand";
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
    confirmSignatures: vi.fn(),
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
    confirmSignatures: async () => calls.confirmSignatures(),
    submitSecret: async (secret) => calls.submitSecret(secret),
    cancel: async () => calls.cancel(),
    close: async () => calls.close(),
    lookAgain: async () => calls.lookAgain(),
    installCertificate: async () => calls.installCertificate(),
    installLocalCa: async () => calls.installLocalCa(),
  };
  return { port, calls };
}

/** El documento del artboard, el mismo que se enseña al consentir. */
const signedDocument: SiteDocument = {
  title: "Solicitud de subvención 2026",
  pages: 27,
  sizeBytes: 2_400_000,
  round: { kind: "sign" },
  hasUnregisteredSignatures: false,
};

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
    it("shows connecting when waiting for the browser", () => {
      const { port } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Conectando con la sede")).toBeInTheDocument();
    });

    it("crosses into «the request has not arrived» when published by the backend, and never closes", () => {
      const { port, calls } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("La petición no ha llegado")).toBeInTheDocument();
      expect(calls.close).not.toHaveBeenCalled();
      expect(calls.cancel).not.toHaveBeenCalled();
    });

    it("offers two recipes and never diagnoses which one is the problem", () => {
      const { port } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Chrome" })).toHaveAttribute(
        "aria-pressed",
        "true",
      );
      expect(screen.getByRole("button", { name: "Firefox" })).toHaveAttribute(
        "aria-pressed",
        "false",
      );
      expect(screen.getByText(/franja bajo la barra de direcciones/)).toBeInTheDocument();
    });

    it("gives the local CA the screen's only main action, because nothing works without it", () => {
      const { port, calls } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      const install = screen.getByRole("button", { name: "Instalar…" });
      expect(install).toHaveClass("rf-btn--primary");
      fireEvent.click(install);
      expect(calls.installLocalCa).toHaveBeenCalledOnce();
    });

    it("puts the mandatory sentence in the footer, and has no Retry button of its own", () => {
      const { port } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(/vuelve a la sede y pulsa Reintentar/)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Reintentar" })).not.toBeInTheDocument();
    });

    it("abandons the errand when closed while waiting, with no confirmation", () => {
      const { port, calls } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));

      expect(calls.cancel).toHaveBeenCalledOnce();
    });
  });

  describe("1b · the channel that will never open", () => {
    it("shows the repair screen straight away, without waiting for the threshold", () => {
      const { port } = scriptedErrand({ kind: "noChannel", reason: "channelNotOpened" });
      renderWithCatalog(<SedeWindow errands={port} />);

      // Sin relojes falsos y sin adelantar nada: el backend ya sabe que no hay
      // canal, así que «Conectando» sería mentira desde el primer píxel.
      expect(screen.getByText("La petición no ha llegado")).toBeInTheDocument();
    });

    it("gives Chrome's local-network address to copy, and never as something to click", () => {
      const { port } = scriptedErrand({ kind: "noChannel", reason: "localCaMissing" });
      renderWithCatalog(<SedeWindow errands={port} />);

      const address = screen.getByText(CHROME_LOCAL_NETWORK_SETTINGS);
      expect(address).toBeInTheDocument();
      expect(address.closest("a")).toBeNull();
      expect(screen.getByRole("button", { name: /Copiar/ })).toBeInTheDocument();
    });

    it("abandons the errand when closed: nothing has been answered", () => {
      const { port, calls } = scriptedErrand({ kind: "noChannel", reason: "channelNotOpened" });
      renderWithCatalog(<SedeWindow errands={port} />);

      fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));

      expect(calls.cancel).toHaveBeenCalledOnce();
      expect(calls.close).not.toHaveBeenCalled();
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
          round: { kind: "sign" },
          hasUnregisteredSignatures: false,
        },
        signs: null,
        signing: "pdf",
        items: null,
        certificates: [certificate()],
        narrowed: false,
        ...overrides,
      }) satisfies ErrandStage;

    it("names the site plainly and says what it asks for", () => {
      const { port } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(
        screen.getByText("sede.ejemplo.gob.es pide tu firma de un documento PDF."),
      ).toBeInTheDocument();
    });

    it.each([
      ["challenge", "un reto de autenticación"],
      ["xml", "un documento XML"],
      ["invoice", "una factura electrónica"],
    ] as const)("says it asks to sign %s", (signing, what) => {
      const { port } = scriptedErrand(consenting({ signing }));
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(`sede.ejemplo.gob.es pide tu firma de ${what}.`)).toBeInTheDocument();
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
          document: {
            title: null,
            pages: 8,
            sizeBytes: 310_000,
            round: { kind: "sign" },
            hasUnregisteredSignatures: false,
          },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Un PDF sin título")).toBeInTheDocument();
    });

    it("warns that the signature will be a countersignature when the PDF is already signed", () => {
      const { port } = scriptedErrand(
        consenting({
          document: {
            title: "Convenio",
            pages: 12,
            sizeBytes: 860_000,
            round: { kind: "cosign" },
            hasUnregisteredSignatures: false,
          },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(/la tuya será una cofirma/)).toBeInTheDocument();
    });

    it.each([
      ["tree", "todas las firmas que ya tiene"],
      ["leafs", "las últimas firmas que tiene"],
    ] as const)(
      "says a %s countersignature signs over the signatures already there",
      (target, over) => {
        const { port } = scriptedErrand(
          consenting({
            document: {
              title: "Convenio",
              pages: 12,
              sizeBytes: 860_000,
              round: { kind: "counter", target },
              hasUnregisteredSignatures: false,
            },
          }),
        );
        renderWithCatalog(<SedeWindow errands={port} />);

        expect(
          screen.getByText(`Ya viene firmado: la tuya será una contrafirma sobre ${over}.`),
        ).toBeInTheDocument();
        expect(screen.queryByText(/cofirma/)).not.toBeInTheDocument();
      },
    );

    it("warns with an information note, not an alert, when the PDF carries a signature rFirma cannot read", () => {
      const { port } = scriptedErrand(
        consenting({
          document: {
            title: "Convenio",
            pages: 12,
            sizeBytes: 860_000,
            round: { kind: "cosign" },
            hasUnregisteredSignatures: true,
          },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(
        screen.getByText(
          "rFirma no reconoce alguna de las firmas que ya tiene este documento, y al añadir la tuya podrían dejar de verse como válidas",
        ),
      ).toBeInTheDocument();
      expect(screen.queryByText(/firmas sin registrar/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Firmar" })).toBeInTheDocument();
    });

    it("cancels the errand from the unrecognized-signatures note like any other consent", async () => {
      const user = userEvent.setup();
      const { port, calls } = scriptedErrand(
        consenting({
          document: {
            title: "Convenio",
            pages: 12,
            sizeBytes: 860_000,
            round: { kind: "cosign" },
            hasUnregisteredSignatures: true,
          },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      await user.click(screen.getByRole("button", { name: "Cancelar" }));

      expect(calls.cancel).toHaveBeenCalled();
    });

    it("leaves a calm label, not a warning, when the request has no valid origin", () => {
      const { port } = scriptedErrand(consenting(), { origin: null });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Origen sin identificar")).toBeInTheDocument();
      expect(
        screen.getByText(
          "La petición pide firmar un documento PDF y no indica de qué página viene.",
        ),
      ).toBeInTheDocument();
    });

    it("leaves a calm label with no format when the moment does not carry one", () => {
      const { port } = scriptedErrand(consenting({ document: null, signs: 3, signing: null }), {
        origin: null,
      });
      renderWithCatalog(<SedeWindow errands={port} />);

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

    it("says it is a batch and how many signatures it carries, with no document to show", () => {
      const { port } = scriptedErrand(consenting({ document: null, signs: 3, signing: null }));
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Lote de 3 firmas")).toBeInTheDocument();
      expect(
        screen.getByText("Los documentos se quedan en la sede: rFirma firma sin descargarlos."),
      ).toBeInTheDocument();
    });

    it("shows the batch count and a row per element for a local batch", () => {
      const { port } = scriptedErrand(
        consenting({
          document: null,
          signs: 3,
          signing: null,
          items: [
            { id: "001", signing: "pdf", round: { kind: "sign" } },
            { id: "002", signing: "challenge", round: { kind: "cosign" } },
            { id: "003", signing: "xml", round: { kind: "sign" } },
            { id: "004", signing: "invoice", round: { kind: "counter", target: "tree" } },
            { id: "005", signing: "pdf", round: { kind: "counter", target: "leafs" } },
          ],
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Lote de 3 firmas")).toBeInTheDocument();
      expect(screen.getByText("001 — un documento PDF (firma)")).toBeInTheDocument();
      expect(screen.getByText("002 — un reto de autenticación (cofirma)")).toBeInTheDocument();
      expect(screen.getByText("003 — un documento XML (firma)")).toBeInTheDocument();
      expect(
        screen.getByText("004 — una factura electrónica (contrafirma de todas las firmas)"),
      ).toBeInTheDocument();
      expect(
        screen.getByText("005 — un documento PDF (contrafirma de las últimas firmas)"),
      ).toBeInTheDocument();
    });

    it("consents to a batch through the same dropdown and the same button as a signature", async () => {
      const { port, calls } = scriptedErrand(
        consenting({ document: null, signs: 3, signing: null }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      await userEvent.click(screen.getByRole("button", { name: "Firmar" }));

      expect(calls.consent).toHaveBeenCalledWith("handle-1");
    });

    it("says «Identificarse», not «Firmar», for selectcert", () => {
      const { port } = scriptedErrand(consenting({ document: null, signing: null }), {
        operation: "selectcert",
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Identificarse" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Firmar" })).not.toBeInTheDocument();
      expect(screen.getByText("sede.ejemplo.gob.es pide que te identifiques.")).toBeInTheDocument();
    });

    it("spells out what selectcert sends", () => {
      const { port } = scriptedErrand(consenting({ document: null, signing: null }), {
        operation: "selectcert",
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(/Se enviarán tu nombre, tu NIF/)).toBeInTheDocument();
    });
  });

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

  describe("4 · outcome", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    it("still shows what was signed: the outcome is where you check it was that document", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "signed", document: signedDocument },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Solicitud de subvención 2026")).toBeInTheDocument();
      expect(screen.getByText("27 páginas · 2,4 MB")).toBeInTheDocument();
    });

    it("shows no document in a refusal, because there never was one", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "refused", situation: "missingFormat", detail: "format=" },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.queryByText("Solicitud de subvención 2026")).not.toBeInTheDocument();
    });

    it("says rFirma keeps no copy, which is the one thing you cannot deduce", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "signed", document: signedDocument },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Firmado y enviado")).toBeInTheDocument();
      expect(screen.getByText("rFirma no guarda copia.")).toBeInTheDocument();
    });

    it("confirms a plain save with no document row: the person just chose where", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "saved" },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Guardado")).toBeInTheDocument();
      expect(screen.queryByText("Solicitud de subvención 2026")).not.toBeInTheDocument();
    });

    it("says how many files were delivered when the load ends there", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "loaded", fileCount: 2 },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Cargado")).toBeInTheDocument();
      expect(
        screen.getByText("Se han enviado 2 ficheros a sede.ejemplo.gob.es."),
      ).toBeInTheDocument();
    });

    it("confirms the batch is at the site, with how many signatures it carried", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "batchSigned", signs: 3 },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Lote firmado y enviado")).toBeInTheDocument();
      expect(
        screen.getByText("Las 3 firmas del lote ya están en sede.ejemplo.gob.es."),
      ).toBeInTheDocument();
    });

    it("gives a batch refusal its own phrase and leaves the raw detail copiable", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: {
          kind: "refused",
          situation: "batchPresignerUnreachable",
          detail: "presigner: connection refused",
        },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(
        screen.getByText(
          "El servicio de sede.ejemplo.gob.es que prepara el lote no ha contestado.",
        ),
      ).toBeInTheDocument();
      expect(screen.getByText("presigner: connection refused")).toBeInTheDocument();
    });

    it("classifies a cancelled save as its own refusal, with its own phrase", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: {
          kind: "refused",
          situation: "saveCancelled",
          detail: "el dialogo de guardado se cerro sin elegir nada",
        },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(
        screen.getByText(
          "Has cerrado el diálogo de guardado sin elegir dónde guardar el fichero que pedía sede.ejemplo.gob.es.",
        ),
      ).toBeInTheDocument();
    });

    it("adds nothing to a cancellation: the title already says it", () => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "cancelled", document: signedDocument },
      });
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
      const { port, calls } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "signed", document: signedDocument },
      });
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
});
