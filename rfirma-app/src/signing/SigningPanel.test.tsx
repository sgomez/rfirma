import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  certificate,
  previousSignatureOf,
  renderPanel,
  reportOf,
} from "./SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

const TWO_SIGNATURES = [
  previousSignatureOf({ certificateSerialNumber: "1" }),
  previousSignatureOf({
    name: "Charles Babbage",
    idNumber: "88888888T",
    certificateSerialNumber: "2",
    signingTime: "2024-01-02T10:00:00Z",
  }),
];

// Grada A: el panel son datos y devoluciones de llamada; no habla con nadie.
describe("SigningPanel", () => {
  it("covers the certificate, the visible-signature toggle, the page and the model cards", () => {
    renderPanel();

    expect(screen.getByRole("button", { name: "Firmar" })).toBeInTheDocument();
    const toggle = screen.getByRole("switch", { name: "Firma visible" });
    expect(toggle.closest(".switch")).toHaveClass("switch--trailing");
    expect(toggle).toHaveAttribute("title", "Quitar la firma visible");
    expect(screen.getByText("En la página 3")).toBeInTheDocument();
    for (const label of ["Completa", "Solo rúbrica", "Personalizada"]) {
      expect(screen.getByRole("radio", { name: label })).toBeInTheDocument();
    }
    expect(screen.getByRole("switch", { name: "Con rúbrica" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cargar…" })).toBeInTheDocument();
  });

  it("is the only region with a primary button, and it comes last", () => {
    renderPanel();

    const primaries = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button.rf-btn--primary"),
    );
    expect(primaries.map((button) => button.textContent)).toEqual(["Firmar"]);

    const buttons = screen.getAllByRole("button");
    expect(buttons.at(-1)).toBe(primaries[0]);
  });

  it("shows no co-signature notice for a document that carries none", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: null },
      previousSignatures: reportOf([]),
    });

    expect(screen.queryByText(/Junto a 2 firmas/)).not.toBeInTheDocument();
  });

  it("shows the destination folder and the file name in their own lines, and never the whole path", () => {
    renderPanel({
      destination: { folder: "Documentos", name: "contrato-firmado.pdf", writable: true },
    });

    // El artboard parte la fila en dos: «Guardar en» como rótulo, con
    // `Cambiar` a su derecha, y la caja del destino debajo con la carpeta y
    // el nombre en dos líneas separadas.
    expect(screen.getByText("Guardar en")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar" })).toBeInTheDocument();
    expect(screen.getByText("Documentos")).toBeInTheDocument();
    expect(screen.getByText("contrato-firmado.pdf")).toBeInTheDocument();
    expect(screen.queryByText(/\/home\//)).not.toBeInTheDocument();
  });

  it("carries the full folder and file name in the title of their own line", () => {
    renderPanel({
      destination: { folder: "Documentos", name: "contrato-firmado.pdf", writable: true },
    });

    expect(screen.getByText("Documentos").closest("[title]")).toHaveAttribute(
      "title",
      "Documentos",
    );
    expect(screen.getByText("contrato-firmado.pdf").closest("[title]")).toHaveAttribute(
      "title",
      "contrato-firmado.pdf",
    );
  });

  it("shortens a long name through the middle and keeps its suffix and extension", () => {
    renderPanel({
      destination: {
        folder: "Documentos",
        name: `contrato-de-arrendamiento-${"largo-".repeat(6)}firmado-2.pdf`,
        writable: true,
      },
    });

    const shown = screen.getByText(/contrato-de-/);
    expect(shown.textContent).toContain("…");
    expect(shown.textContent?.endsWith("-firmado-2.pdf")).toBe(true);
  });

  it("keeps the sign button alive when the destination cannot be written to", () => {
    renderPanel({ destination: { folder: "Documentos", name: null, writable: false } });

    expect(
      screen.getByText(
        (_, element) => element?.textContent === "No se puede escribir en Documentos",
        {
          selector: "span",
        },
      ),
    ).toBeInTheDocument();
    // El diseño pone la carpeta en negrita dentro de la frase.
    expect(screen.getByText("Documentos", { selector: "strong" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Firmar" })).toBeEnabled();
  });

  it("carries the full folder in the title of the unwritable message, even shortened", () => {
    const folder = "Documentos-de-la-empresa-que-no-caben-en-una-sola-linea-del-pie";
    renderPanel({ destination: { folder, name: null, writable: false } });

    expect(
      screen.getByText((_, element) => element?.textContent?.startsWith("No se puede") ?? false, {
        selector: "span",
      }),
    ).toHaveAttribute("title", folder);
  });

  it("keeps the label even when the destination cannot be written to", () => {
    // El artboard no quita la fila «Guardar en · Cambiar» con el destino roto:
    // solo cambia la caja de debajo.
    renderPanel({ destination: { folder: "Documentos", name: null, writable: false } });

    expect(screen.getByText("Guardar en")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar" })).toBeInTheDocument();
  });

  it("never shows a wildcard in the interface", () => {
    // ID-19: el modelo elegido dice qué aparece, y nunca hay una cadena
    // como `$$SUBJECTCN$$` escrita a mano en el panel.
    renderPanel({});

    expect(document.body.textContent).not.toMatch(/\$\$/);
  });

  it("warns about the co-signature when the document already carries signatures", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([previousSignatureOf()]),
    });

    expect(screen.getByText("Junto a 1 firma")).toBeInTheDocument();
  });

  it("shows the same-certificate strip, even folded, when the chosen certificate signed before", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([
        previousSignatureOf({
          idNumber: certificate.idNumber,
          organizationIdentifier: certificate.organizationIdentifier,
          issuer: certificate.issuer,
          certificateSerialNumber: certificate.certificateSerialNumber,
        }),
      ]),
    });

    expect(screen.getByText("Ya lo firmaste tú con este certificado")).toBeInTheDocument();
  });

  it("shows the other-certificate strip for a renewed certificate: same NIF and entity, other serial", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([
        previousSignatureOf({
          idNumber: certificate.idNumber,
          organizationIdentifier: certificate.organizationIdentifier,
          issuer: certificate.issuer,
          certificateSerialNumber: "9999999999",
        }),
      ]),
    });

    expect(screen.getByText("Ya lo firmaste tú, con otro certificado tuyo")).toBeInTheDocument();
  });

  it("shows no strip without a certificate chosen, even with a matching previous signature", () => {
    renderPanel({
      certificate: { kind: "unchosen", certificates: [certificate] },
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([
        previousSignatureOf({
          idNumber: certificate.idNumber,
          organizationIdentifier: certificate.organizationIdentifier,
          issuer: certificate.issuer,
          certificateSerialNumber: certificate.certificateSerialNumber,
        }),
      ]),
    });

    expect(screen.queryByText(/Ya lo firmaste tú/)).not.toBeInTheDocument();
  });

  it("pluralises the co-signature notice with more than one previous signature", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf(TWO_SIGNATURES),
    });

    expect(screen.getByText("Junto a 2 firmas")).toBeInTheDocument();
  });

  it("keeps the notice to one line, whatever the number of signatures", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([...TWO_SIGNATURES, ...TWO_SIGNATURES, ...TWO_SIGNATURES]),
    });

    expect(screen.getByText("Junto a 6 firmas")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Ver firmas →" })).toHaveLength(1);
    expect(document.querySelector(".panel__previous-signatures-list")).not.toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("says only the count, with the quiet tone, when every signature is valid", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf(TWO_SIGNATURES),
    });

    expect(screen.getByText("Junto a 2 firmas")).toBeInTheDocument();
    expect(document.querySelector(".panel__co-signature")).toHaveClass(
      "panel__co-signature--valid",
    );
  });

  it("counts «caducadas» when every problem is an expiry", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([
        previousSignatureOf({ validity: "expired" }),
        previousSignatureOf({ validity: "expired", certificateSerialNumber: "2" }),
        previousSignatureOf({ certificateSerialNumber: "3" }),
      ]),
    });

    expect(screen.getByText("Junto a 3 firmas · 2 caducadas")).toBeInTheDocument();
    expect(document.querySelector(".panel__co-signature")).toHaveClass(
      "panel__co-signature--expired",
    );
  });

  it("counts «problemas», expired ones included, as soon as one signature is invalid", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([
        previousSignatureOf({ validity: "expired" }),
        previousSignatureOf({ validity: "invalid", certificateSerialNumber: "2" }),
        previousSignatureOf({ certificateSerialNumber: "3" }),
      ]),
    });

    expect(screen.getByText("Junto a 3 firmas · 2 problemas")).toBeInTheDocument();
    expect(document.querySelector(".panel__co-signature")).toHaveClass(
      "panel__co-signature--invalid",
    );
  });

  it("counts a finding of the document as a problem even with every signature valid", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([previousSignatureOf()], {
        findings: ["modifiedAfterLastSignature"],
      }),
    });

    expect(screen.getByText("Junto a 1 firma · 1 problema")).toBeInTheDocument();
    expect(document.querySelector(".panel__co-signature")).toHaveClass(
      "panel__co-signature--invalid",
    );
  });

  it("opens the «Ver firmas» dialog from «Ver firmas →» and closes it", async () => {
    const user = userEvent.setup();
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: reportOf([previousSignatureOf()]),
    });

    await user.click(screen.getByRole("button", { name: "Ver firmas →" }));

    const dialog = screen.getByRole("dialog", { name: "Firmas del documento" });
    await user.click(within(dialog).getByRole("button", { name: "Cerrar" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("offers two ways out when no certificate turned up, in the footer", async () => {
    const user = userEvent.setup();
    const onRetryCertificates = vi.fn();
    renderPanel({ certificate: { kind: "empty" }, onRetryCertificates });

    expect(screen.getByText("Sin certificados")).toBeInTheDocument();
    expect(screen.getByText("No hay ningún certificado con el que firmar.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Añadir un certificado…" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Volver a buscar" }));

    expect(onRetryCertificates).toHaveBeenCalled();
    // Sin certificados no hay con qué firmar, así que tampoco hay botón que
    // deshabilitar: el pie ofrece las dos salidas y nada más.
    expect(screen.queryByRole("combobox", { name: "Certificado" })).not.toBeInTheDocument();
  });

  /**
   * Error al firmar (docs/design/panel-de-firma.md § Error al firmar): la
   * zona que se desliza se sustituye por la tarjeta del fallo y el pie ofrece
   * «Reintentar» y «Volver» en vez de «Firmar».
   */
  it("shows a token failure as a translated situation with the raw CKR apart", () => {
    renderPanel({
      failure: { situation: "tokenAbsent", detail: "CKR_DEVICE_REMOVED (C_Sign)" },
    });

    // El título es siempre el fijo; la situación clasificada baja a ser la
    // causa (docs/design/panel-de-firma.md § Estados → Error al firmar).
    expect(screen.getByText("No se ha podido firmar")).toBeInTheDocument();
    expect(screen.getByText("No encontramos la tarjeta")).toBeInTheDocument();
    // El código original, ni traducido ni recortado: está para pegarlo.
    expect(screen.getByText("CKR_DEVICE_REMOVED (C_Sign)")).toBeInTheDocument();
    expect(
      screen.getByText("El documento sigue como estaba: no se ha guardado nada."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copiar detalle" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reintentar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Volver" })).toBeInTheDocument();
    // La firma visible no aporta nada mientras el documento sigue igual: se
    // esconde entera en vez de enseñarla al lado del error.
    expect(screen.queryByText("Firma visible")).not.toBeInTheDocument();
  });

  /**
   * Firmando (docs/design/panel-de-firma.md § Estados → Firmando): el
   * interruptor y el bloque de colocación se atenúan al 35 %.
   */
  it("dims the toggle and the placement controls while signing", () => {
    renderPanel({ signing: true });

    expect(
      screen.getByRole("switch", { name: "Firma visible" }).closest(".panel__toggle"),
    ).toHaveClass("panel__toggle--dim");
    expect(screen.getByRole("radiogroup").closest(".panel__controls--dim")).not.toBeNull();
  });

  it("does not dim the toggle or the placement controls otherwise", () => {
    renderPanel({ signing: false });

    expect(
      screen.getByRole("switch", { name: "Firma visible" }).closest(".panel__toggle"),
    ).not.toHaveClass("panel__toggle--dim");
    expect(screen.getByRole("radiogroup").closest(".panel__controls--dim")).toBeNull();
  });

  it("dims «Cambiar» to the same 35 % while signing", () => {
    renderPanel({ signing: true });

    expect(screen.getByRole("button", { name: "Cambiar" })).toHaveClass("panel__controls--dim");
  });

  it("keeps the destination box and calls onBack from the error's «Volver»", async () => {
    const user = userEvent.setup();
    const onBack = vi.fn();
    renderPanel({
      failure: { situation: "tokenAbsent", detail: "CKR_DEVICE_REMOVED (C_Sign)" },
      onBack,
    });

    expect(screen.getByText("contrato-firmado.pdf")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Volver" }));

    expect(onBack).toHaveBeenCalled();
  });

  // Uno recordado puede caducar entre sesiones (ADR-0010).
  it("refuses to sign with an expired chosen certificate", () => {
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate: { ...certificate, status: { kind: "expired", notAfter: 1_767_225_600 } },
        certificates: [{ ...certificate, status: { kind: "expired", notAfter: 1_767_225_600 } }],
      },
    });

    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
  });

  it("refuses to sign with a revoked chosen certificate", () => {
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate: { ...certificate, status: { kind: "revoked", reason: "keyCompromise" } },
        certificates: [{ ...certificate, status: { kind: "revoked", reason: "keyCompromise" } }],
      },
    });

    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
  });

  /** Con varios y nada elegido no hay preselección: el orden de la lista solo
   * dice en qué orden cargaron los módulos, y elegir con qué identidad se firma
   * un documento con validez jurídica no lo hace la aplicación por su cuenta.
   * Sin nada elegido, «Firmar» está atenuado. */
  it("does not preselect anything when there are several certificates", () => {
    renderPanel({
      certificate: {
        kind: "unchosen",
        certificates: [certificate, { ...certificate, id: "otra" }],
      },
    });

    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "Elige un certificado",
    );
    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
  });

  /**
   * Ni siquiera con uno solo se elige solo: elegir con qué identidad se firma
   * un documento con validez jurídica no lo hace la aplicación por su cuenta.
   */
  it("does not preselect the sole certificate either", () => {
    renderPanel({ certificate: { kind: "unchosen", certificates: [certificate] } });

    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "Elige un certificado",
    );
    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
  });

  it("names the chosen certificate's holder in the title of «Firmar»", () => {
    renderPanel();

    expect(screen.getByRole("button", { name: "Firmar" })).toHaveAttribute(
      "title",
      "Firmar con el certificado de Ada Lovelace Byron",
    );
  });

  it("puts the certificate selector first, above the co-signature notice", () => {
    renderPanel({ previousSignatures: reportOf(TWO_SIGNATURES) });

    const selector = screen.getByRole("combobox", { name: "Certificado" });
    const scroll = selector.closest(".panel__scroll");
    expect(scroll?.firstElementChild?.contains(selector)).toBe(true);
    expect(screen.getByText(/Junto a 2 firmas/)).toBeInTheDocument();
  });

  it("chooses the certificate from the selector, not from the footer", async () => {
    const user = userEvent.setup();
    const onChooseCertificate = vi.fn();
    const grace = { ...certificate, id: "otra", holderName: "Grace Hopper" };
    renderPanel({
      certificate: { kind: "unchosen", certificates: [certificate, grace] },
      onChooseCertificate,
    });

    await user.click(screen.getByRole("combobox", { name: "Certificado" }));
    await user.click(screen.getByRole("option", { name: /Grace Hopper/ }));

    expect(onChooseCertificate).toHaveBeenCalledWith(grace);
  });

  it("has no split button: the footer says just «Firmar»", () => {
    renderPanel();

    const footer = screen.getByRole("button", { name: "Firmar" }).closest("footer");
    expect(footer).not.toBeNull();
    expect(within(footer as HTMLElement).queryByRole("combobox")).not.toBeInTheDocument();
    expect(screen.queryByText(/Firmar como|Elegir certificado/)).not.toBeInTheDocument();
  });

  it("waits for the certificates without pretending there are none", () => {
    renderPanel({ certificate: { kind: "loading" } });

    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "Buscando certificados…",
    );
    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
    expect(screen.queryByText("Sin certificados")).not.toBeInTheDocument();
  });

  it("says «Firmando…» and lets nothing be pressed while signing", () => {
    renderPanel({ signing: true });

    expect(screen.getByRole("button", { name: "Firmando…" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Certificado" })).toBeDisabled();
  });
});

describe("la firma visible, sin certificado elegido", () => {
  const visible = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };
  const unchosen = { kind: "unchosen", certificates: [certificate] } as const;

  it("keeps the switch off and disabled, with a notice below, until a certificate is chosen", () => {
    renderPanel({ certificate: unchosen, signature: visible });

    const toggle = screen.getByRole("switch", { name: "Firma visible" });
    expect(toggle).toHaveAttribute("aria-checked", "false");
    expect(toggle).toBeDisabled();
    expect(
      screen.getByText("Elige un certificado para añadir una firma visible."),
    ).toBeInTheDocument();
    expect(screen.queryByRole("radio", { name: "Completa" })).not.toBeInTheDocument();
  });

  it("drops the notice and frees the switch once a certificate is chosen", () => {
    renderPanel({ signature: { ...visible, enabled: false } });

    expect(screen.getByRole("switch", { name: "Firma visible" })).toBeEnabled();
    expect(
      screen.queryByText("Elige un certificado para añadir una firma visible."),
    ).not.toBeInTheDocument();
  });

  it("brings the placement back when a certificate that went away comes back", () => {
    const { show } = renderPanel({ signature: visible });
    expect(screen.getByText("En la página 3")).toBeInTheDocument();

    show({ certificate: { kind: "empty" }, signature: visible });
    expect(screen.queryByText("En la página 3")).not.toBeInTheDocument();
    show({ signature: visible });

    expect(screen.getByText("En la página 3")).toBeInTheDocument();
  });
});
