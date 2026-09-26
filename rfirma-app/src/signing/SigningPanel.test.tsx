import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { certificate, renderPanel } from "./SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

// Grada A: el panel son datos y devoluciones de llamada; no habla con nadie.
describe("SigningPanel", () => {
  it("covers the certificate, the visible-signature toggle, the page and the model cards", () => {
    renderPanel();

    expect(screen.getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeInTheDocument();
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
    expect(primaries.map((button) => button.textContent)).toEqual(["Firmar como Ada Lovelace"]);

    const buttons = screen.getAllByRole("button");
    expect(buttons.at(-1)).toBe(primaries[0]);
  });

  it("shows no co-signature notice for a document that carries none", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: null },
      previousSignatures: [],
    });

    expect(screen.queryByText(/Firmarás junto a/)).not.toBeInTheDocument();
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
    expect(screen.getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeEnabled();
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
      previousSignatures: [
        {
          name: "Ada Lovelace Byron",
          idNumber: "99999999R",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "1",
          signingTime: "2024-01-01T10:00:00Z",
        },
      ],
    });

    expect(screen.getByText("Firmarás junto a 1 firma anterior")).toBeInTheDocument();
  });

  it("pluralises the co-signature notice with more than one previous signature", () => {
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: [
        {
          name: "Ada Lovelace Byron",
          idNumber: "99999999R",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "1",
          signingTime: "2024-01-01T10:00:00Z",
        },
        {
          name: "Charles Babbage",
          idNumber: "88888888T",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "2",
          signingTime: "2024-01-02T10:00:00Z",
        },
      ],
    });

    expect(screen.getByText("Firmarás junto a 2 firmas anteriores")).toBeInTheDocument();
  });

  it("nace desplegado when a report with two signatures arrives after the panel already mounted", () => {
    const { show } = renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: [],
    });

    show({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: [
        {
          name: "Ada Lovelace Byron",
          idNumber: "99999999R",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "1",
          signingTime: "2024-01-01T10:00:00Z",
        },
        {
          name: "Charles Babbage",
          idNumber: "88888888T",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "2",
          signingTime: "2024-01-02T10:00:00Z",
        },
      ],
    });

    expect(screen.getByRole("button", { name: "Ocultar firmas anteriores" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("does not carry the expanded state of the previous document into the next one", () => {
    const { show } = renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: [
        {
          name: "Ada Lovelace Byron",
          idNumber: "99999999R",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "1",
          signingTime: "2024-01-01T10:00:00Z",
        },
        {
          name: "Charles Babbage",
          idNumber: "88888888T",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "2",
          signingTime: "2024-01-02T10:00:00Z",
        },
      ],
    });
    expect(screen.getByRole("button", { name: "Ocultar firmas anteriores" })).toBeInTheDocument();

    show({
      document: { id: "doc-2", name: "otro.pdf", pages: 3, sizeBytes: 1_000 },
      previousSignatures: [
        {
          name: "Grace Hopper",
          idNumber: "77777777J",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "3",
          signingTime: "2024-02-01T10:00:00Z",
        },
      ],
    });

    expect(screen.getByRole("button", { name: "Ver firmas anteriores" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("toggles the previous-signatures rows, the chevron and aria-expanded when pressed", async () => {
    const user = userEvent.setup();
    renderPanel({
      document: { id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 },
      previousSignatures: [
        {
          name: "Ada Lovelace Byron",
          idNumber: "99999999R",
          organizationIdentifier: null,
          issuer: "AC FNMT Usuarios",
          certificateSerialNumber: "1",
          signingTime: "2024-01-01T10:00:00Z",
        },
      ],
    });

    const rows = () => document.querySelector(".panel__previous-signatures-list");
    const summary = screen.getByRole("button", { name: "Ver firmas anteriores" });
    const chevron = summary.querySelector(".panel__co-signature-chevron");
    expect(summary).toHaveAttribute("aria-expanded", "false");
    expect(chevron).not.toHaveClass("panel__co-signature-chevron--open");
    expect(rows()).not.toBeInTheDocument();

    await user.click(summary);

    const expanded = screen.getByRole("button", { name: "Ocultar firmas anteriores" });
    expect(expanded).toHaveAttribute("aria-expanded", "true");
    expect(expanded.querySelector(".panel__co-signature-chevron")).toHaveClass(
      "panel__co-signature-chevron--open",
    );
    expect(rows()).toBeInTheDocument();

    await user.click(expanded);

    expect(screen.getByRole("button", { name: "Ver firmas anteriores" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(rows()).not.toBeInTheDocument();
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
   * «Reintentar» y «Volver» en vez del botón de certificado.
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

    expect(screen.getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeDisabled();
  });

  it("refuses to sign with a revoked chosen certificate", () => {
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate: { ...certificate, status: { kind: "revoked", reason: "keyCompromise" } },
        certificates: [{ ...certificate, status: { kind: "revoked", reason: "keyCompromise" } }],
      },
    });

    expect(screen.getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeDisabled();
  });

  /** Con varios y nada elegido no hay preselección: el orden de la lista solo
   * dice en qué orden cargaron los módulos, y elegir con qué identidad se firma
   * un documento con validez jurídica no lo hace la aplicación por su cuenta.
   * Sin nada elegido el botón partido es **uno solo**, que abre la lista en
   * vez de firmar (docs/design/panel-de-firma.md § Certificado). */
  it("does not preselect anything when there are several certificates", () => {
    renderPanel({
      certificate: {
        kind: "unchosen",
        certificates: [certificate, { ...certificate, id: "otra" }],
      },
    });

    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "Elegir certificado",
    );
    expect(screen.queryByRole("button", { name: /Firmar/ })).not.toBeInTheDocument();
  });

  /**
   * Ni siquiera con uno solo se elige solo: elegir con qué identidad se firma
   * un documento con validez jurídica no lo hace la aplicación por su cuenta.
   */
  it("does not preselect the sole certificate either", () => {
    renderPanel({ certificate: { kind: "unchosen", certificates: [certificate] } });

    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "Elegir certificado",
    );
    expect(screen.queryByRole("button", { name: /Firmar/ })).not.toBeInTheDocument();
  });

  /**
   * El botón dice el nombre y el primer apellido, con el nombre completo en
   * el `title`; la lista, más abajo, lo sigue mostrando entero.
   */
  it("shortens the holder's name to the given name and the first surname on the button", () => {
    renderPanel();

    const signButton = screen.getByRole("button", { name: "Firmar como Ada Lovelace" });
    expect(signButton).toHaveAttribute("title", "Ada Lovelace Byron");
  });

  it("opens the certificate list upwards, over the footer", async () => {
    const user = userEvent.setup();
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate,
        certificates: [certificate, { ...certificate, id: "otra", holderName: "Grace Hopper" }],
      },
    });

    await user.click(screen.getByRole("combobox", { name: "Certificado" }));

    const list = screen.getByRole("listbox", { name: "Certificados disponibles" });
    expect(list).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(2);
  });

  it("says issuer, store and expiry month on the second line of a usable row", async () => {
    const user = userEvent.setup();
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate,
        certificates: [certificate, { ...certificate, id: "otra", holderName: "Grace Hopper" }],
      },
    });

    await user.click(screen.getByRole("combobox", { name: "Certificado" }));

    const row = screen.getByRole("option", { name: /Grace Hopper/ });
    expect(row).toHaveTextContent(/AC FNMT Usuarios · Tarjeta · Caduca en \d{2}\/\d{4}/);
    expect(row).not.toHaveTextContent("Emitido por");
  });

  it("lists an unusable certificate with its short reason as text and as tooltip", async () => {
    const user = userEvent.setup();
    const onChooseCertificate = vi.fn();
    const revoked = {
      ...certificate,
      id: "revocado",
      holderName: "Grace Hopper",
      status: { kind: "revoked", reason: "keyCompromise" },
    } as const;
    renderPanel({
      certificate: { kind: "chosen", certificate, certificates: [certificate, revoked] },
      onChooseCertificate,
    });

    await user.click(screen.getByRole("combobox", { name: "Certificado" }));
    const row = screen.getByRole("option", { name: /Grace Hopper/ });
    await user.click(row);

    expect(within(row).getByText("Revocado (keyCompromise)")).toBeInTheDocument();
    expect(row).toHaveAttribute("title", "Revocado (keyCompromise)");
    expect(row).toHaveAttribute("aria-disabled", "true");
    expect(onChooseCertificate).not.toHaveBeenCalled();
  });

  it("has no «change» button any more: the trigger is where it changes", () => {
    renderPanel();

    const splitButton = screen
      .getByRole("button", { name: "Firmar como Ada Lovelace" })
      .closest(".certificate-footer");
    expect(splitButton).not.toBeNull();
    expect(
      within(splitButton as HTMLElement).queryByRole("button", { name: "Cambiar" }),
    ).not.toBeInTheDocument();
  });

  it("waits for the certificates without pretending there are none", () => {
    renderPanel({ certificate: { kind: "loading" } });

    expect(screen.getByText("Buscando certificados…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Buscando certificados…" })).toBeDisabled();
    expect(screen.queryByRole("combobox", { name: "Certificado" })).not.toBeInTheDocument();
  });

  it("says who is signing and lets nothing be pressed while signing", () => {
    renderPanel({ signing: true });

    expect(screen.getByRole("button", { name: "Firmando como Ada Lovelace" })).toBeDisabled();
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
