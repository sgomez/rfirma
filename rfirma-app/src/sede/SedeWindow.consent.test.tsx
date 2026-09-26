import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NO_PREVIOUS_SIGNATURES } from "../signing/previousSignatures";
import { previousSignatureOf, reportOf } from "../signing/SigningPanel.testSupport";
import { renderWithCatalog } from "../testing/render";
import type { ErrandStage } from "./errand";
import { SedeWindow } from "./SedeWindow";
import { certificate, elapse, scriptedErrand } from "./sedeWindowFixtures";

/** Grada A: el momento 2, el consentimiento, con su cuenta atrás (TD-63). */

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
        previousSignatures: NO_PREVIOUS_SIGNATURES,
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

  it("orders the body origin, then certificate, then document", () => {
    const { port } = scriptedErrand(consenting());
    const { container } = renderWithCatalog(<SedeWindow errands={port} />);

    const text = container.textContent ?? "";
    expect(text.indexOf("sede.ejemplo.gob.es pide tu firma")).toBeLessThan(
      text.indexOf("Firmarás con"),
    );
    expect(text.indexOf("Firmarás con")).toBeLessThan(text.indexOf("Solicitud de subvención 2026"));
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
          previousSignatures: NO_PREVIOUS_SIGNATURES,
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
          previousSignatures: NO_PREVIOUS_SIGNATURES,
        },
      }),
    );
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(/Firmarás junto a 1 firma anterior/)).toBeInTheDocument();
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
            previousSignatures: NO_PREVIOUS_SIGNATURES,
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
          previousSignatures: NO_PREVIOUS_SIGNATURES,
        },
      }),
    );
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

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
          previousSignatures: NO_PREVIOUS_SIGNATURES,
        },
      }),
    );
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(calls.cancel).toHaveBeenCalled();
  });

  describe("las firmas previas del documento (ID-403, ID-404, ID-406)", () => {
    it.each(["sign", "cosign"] as const)(
      "shows the notice with the right count and states for a %s",
      (kind) => {
        const { port } = scriptedErrand(
          consenting({
            document: {
              title: "Convenio",
              pages: 12,
              sizeBytes: 860_000,
              round: { kind },
              hasUnregisteredSignatures: false,
              previousSignatures: reportOf(
                [
                  previousSignatureOf(),
                  previousSignatureOf({
                    certificateSerialNumber: "2",
                    status: "broken",
                    reason: null,
                  }),
                ],
                { warningCount: 1, tone: "attention" },
              ),
            },
          }),
        );
        renderWithCatalog(<SedeWindow errands={port} />);

        expect(screen.getByText(/Firmarás junto a 2 firmas anteriores/)).toBeInTheDocument();
        expect(screen.getByText(/1 aviso/)).toBeInTheDocument();
      },
    );

    it("signs with no intermediate dialogue when a previous signature is broken", async () => {
      const user = userEvent.setup();
      const { port, calls } = scriptedErrand(
        consenting({
          document: {
            title: "Convenio",
            pages: 12,
            sizeBytes: 860_000,
            round: { kind: "sign" },
            hasUnregisteredSignatures: false,
            previousSignatures: reportOf(
              [previousSignatureOf({ status: "broken", reason: null })],
              {
                warningCount: 1,
                tone: "attention",
              },
            ),
          },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      await user.click(screen.getByRole("button", { name: "Firmar" }));

      expect(calls.consent).toHaveBeenCalledWith("handle-1");
    });

    it("shows the same-signer stripe for the chosen certificate, folded or not", () => {
      const { port } = scriptedErrand(
        consenting({
          document: {
            title: "Convenio",
            pages: 12,
            sizeBytes: 860_000,
            round: { kind: "sign" },
            hasUnregisteredSignatures: false,
            previousSignatures: reportOf([
              previousSignatureOf({
                issuer: certificate().issuer,
                certificateSerialNumber: certificate().certificateSerialNumber,
              }),
            ]),
          },
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Ya lo firmaste tú con este certificado")).toBeInTheDocument();
    });
  });

  it("leaves a calm label, not a warning, when the request has no valid origin", () => {
    const { port } = scriptedErrand(consenting(), { origin: null });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Una página sin identificar pide tu firma.")).toBeInTheDocument();
  });

  it("says a page asks for identity data, unidentified, in the selectcert branch", () => {
    const { port } = scriptedErrand(consenting({ document: null, signing: null }), {
      origin: null,
      operation: "selectcert",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(
      screen.getByText("Una página sin identificar pide tus datos de identidad."),
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
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    expect(screen.getByRole("button", { name: "Firmar" })).toBeInTheDocument();
  });

  it("consents with the chosen certificate's handle", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand(consenting());
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

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
    const { port, calls } = scriptedErrand(consenting({ document: null, signs: 3, signing: null }));
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    await userEvent.click(screen.getByRole("button", { name: "Firmar" }));

    expect(calls.consent).toHaveBeenCalledWith("handle-1");
  });

  it("says «Enviar mis datos», not «Firmar», for selectcert, and does not claim identification", () => {
    const { port } = scriptedErrand(consenting({ document: null, signing: null }), {
      operation: "selectcert",
    });
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    expect(screen.getByRole("button", { name: "Enviar mis datos" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Firmar" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Identificarse" })).not.toBeInTheDocument();
    expect(
      screen.getByText("sede.ejemplo.gob.es pide tus datos de identidad."),
    ).toBeInTheDocument();
  });

  it("spells out what selectcert sends", () => {
    const { port } = scriptedErrand(consenting({ document: null, signing: null }), {
      operation: "selectcert",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(/Se enviarán tu nombre, tu NIF/)).toBeInTheDocument();
  });

  describe("the countdown before signing", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    const elapseCountdown = async () => {
      for (let second = 0; second < 3; second++) await elapse(1000);
    };

    it("counts Firmar (3), (2), (1) down disabled, and enables Firmar after three seconds", async () => {
      const { port } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Firmar (3)" })).toBeDisabled();
      await elapse(1000);
      expect(screen.getByRole("button", { name: "Firmar (2)" })).toBeDisabled();
      await elapse(1000);
      expect(screen.getByRole("button", { name: "Firmar (1)" })).toBeDisabled();
      await elapse(1000);
      expect(screen.getByRole("button", { name: "Firmar" })).toBeEnabled();
    });

    it("focuses Firmar once the countdown ends, so Enter signs with the remembered certificate", async () => {
      const { port, calls } = scriptedErrand(
        consenting({
          certificates: [
            certificate(),
            certificate({ id: "handle-2", label: "Otro", remembered: true }),
          ],
        }),
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapseCountdown();
      const sign = screen.getByRole("button", { name: "Firmar" });
      expect(sign).toHaveFocus();
      fireEvent.click(document.activeElement as HTMLElement);

      expect(calls.consent).toHaveBeenCalledWith("handle-2");
    });

    it("does not take the focus back from what the person moved it to during the countdown", async () => {
      const { port } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} />);

      screen.getByRole("button", { name: "Cancelar" }).focus();
      await elapseCountdown();

      expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
    });

    it("counts down «Enviar mis datos» too: sending an identity is no less final", async () => {
      const { port } = scriptedErrand(consenting({ document: null, signing: null }), {
        operation: "selectcert",
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Enviar mis datos (3)" })).toBeDisabled();
    });

    it("starts enabled and focused, with no number, when the person turned the countdown off", () => {
      const { port } = scriptedErrand(consenting());
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      const sign = screen.getByRole("button", { name: "Firmar" });
      expect(sign).toBeEnabled();
      expect(sign).toHaveFocus();
    });
  });
});
