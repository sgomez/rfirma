import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { certificate, renderPanel } from "./SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

// Grada A: el panel son datos y devoluciones de llamada; no habla con nadie.
describe("SigningPanel", () => {
  it("counts the pages in singular when the document has only one", () => {
    renderPanel({
      document: { name: "instancia.pdf", pages: 1, sizeBytes: null, signatures: null },
    });

    expect(screen.getByText("1 página")).toBeInTheDocument();
  });

  it("counts the pages in plural when the document has more than one", () => {
    renderPanel();

    expect(screen.getByText(/^27 páginas/)).toBeInTheDocument();
  });

  it("covers the certificate, the visible-signature toggle, the page and the model cards", () => {
    renderPanel();

    expect(screen.getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeInTheDocument();
    const toggle = screen.getByRole("switch", {
      name: /Estampar un recuadro de firma en el documento/,
    });
    expect(toggle).toBeInTheDocument();
    // El panel lo dibuja con `rf-gap-xs` (8 px, `Main.dc.html:306`); los 16 px
    // son de Preferencias y se piden allí con `switch--wide`.
    expect(toggle.closest(".switch")).not.toHaveClass("switch--wide");
    expect(screen.getByRole("button", { name: "Quitarla de aquí" })).toBeInTheDocument();
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

  // El artboard enseña «27 páginas · 2,4 MB» y un resumen de firmas que hoy
  // nadie calcula. Lo desconocido **no ocupa sitio**: ni un guion, ni un «—»,
  // ni un marcador de posición.
  it("paints nothing at all in place of what nobody knows yet", () => {
    renderPanel({
      document: { name: "contrato.pdf", pages: 27, sizeBytes: null, signatures: null },
    });

    // La línea de metadatos dice las páginas y **nada más**: sin el separador
    // que precedería al tamaño, y sin tamaño.
    expect(screen.getByText("27 páginas")).toBeInTheDocument();
    expect(screen.getByText("27 páginas").textContent).toBe("27 páginas");
    expect(screen.queryByText(/—|–|\bMB\b|\bkB\b/)).not.toBeInTheDocument();
    expect(screen.queryByText(/cofirma/)).not.toBeInTheDocument();
  });

  it("shows the destination folder and the file name, and never the whole path", () => {
    renderPanel({
      destination: { folder: "Documentos", name: "contrato-firmado.pdf", writable: true },
    });

    // El artboard parte la fila en dos: «Se guardará en» como rótulo y el
    // destino debajo, junto al icono de carpeta. El destino son **dos cosas**:
    // la carpeta precedida de `…/` y el nombre con el que va a caer (ID-63).
    expect(screen.getByText("Se guardará en")).toBeInTheDocument();
    expect(screen.getByText("…/Documentos/")).toBeInTheDocument();
    expect(screen.getByText(/contrato-firmado\.pdf/)).toBeInTheDocument();
    expect(screen.queryByText(/\/home\//)).not.toBeInTheDocument();
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

    expect(screen.getByText("No se puede escribir en Documentos")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeEnabled();
  });

  it("does not promise a destination it has just said it cannot write to", () => {
    // «Se guardará en» y «No se puede escribir en Documentos» a la vez es una
    // contradicción: el rótulo es la promesa y desaparece con ella.
    renderPanel({ destination: { folder: "Documentos", name: null, writable: false } });

    expect(screen.queryByText("Se guardará en")).not.toBeInTheDocument();
  });

  it("never shows a wildcard in the interface", () => {
    // ID-19: el modelo elegido dice qué aparece, y nunca hay una cadena
    // como `$$SUBJECTCN$$` escrita a mano en el panel.
    renderPanel({});

    expect(document.body.textContent).not.toMatch(/\$\$/);
  });

  it("warns about the co-signature when the document already carries signatures", () => {
    renderPanel({
      document: { name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000, signatures: 1 },
    });

    expect(screen.getByText("Ya lleva 1 firma: la tuya será una cofirma.")).toBeInTheDocument();
  });

  it("offers two ways out when no certificate turned up, in the footer", async () => {
    const user = userEvent.setup();
    const onRetryCertificates = vi.fn();
    renderPanel({ certificate: { kind: "empty" }, onRetryCertificates });

    expect(screen.getByText(/comprueba que está insertada/)).toBeInTheDocument();
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
    expect(screen.getByRole("button", { name: "Copiar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Volver a intentarlo" })).toBeInTheDocument();
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

    expect(screen.getByRole("switch", { name: /Estampar un recuadro/ })).toHaveClass("switch__control");
    expect(screen.getByRole("switch", { name: /Estampar un recuadro/ }).closest(".panel__toggle")).toHaveClass("panel__toggle--dim");
    expect(screen.getByText("Colocación").closest(".panel__controls--dim")).not.toBeNull();
  });

  it("does not dim the toggle or the placement controls otherwise", () => {
    renderPanel({ signing: false });

    expect(screen.getByRole("switch", { name: /Estampar un recuadro/ }).closest(".panel__toggle")).not.toHaveClass(
      "panel__toggle--dim",
    );
    expect(screen.getByText("Colocación").closest(".panel__controls--dim")).toBeNull();
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

  it("warns about an expired certificate and refuses to sign with it", () => {
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate: { ...certificate, status: { kind: "expired", notAfter: 1_767_225_600 } },
        certificates: [{ ...certificate, status: { kind: "expired", notAfter: 1_767_225_600 } }],
      },
    });

    expect(screen.getByRole("alert")).toHaveTextContent(/El certificado caducó el/);
    expect(screen.getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeDisabled();
  });

  it("warns about a revoked certificate and refuses to sign with it", () => {
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate: { ...certificate, status: { kind: "revoked", reason: "keyCompromise" } },
        certificates: [{ ...certificate, status: { kind: "revoked", reason: "keyCompromise" } }],
      },
    });

    expect(screen.getByRole("alert")).toHaveTextContent(/revocado/);
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
});

/**
 * ID-108. El estado del sello en sí lo cuenta ahora la pastilla flotante del
 * visor (#202) — ver `DocumentViewer.test.tsx` § «el estado del sello,
 * flotando sobre la botonera». Lo que sigue siendo del panel es el bloque
 * entero, apagado sin certificado, y que la colocación sobrevive a que el
 * certificado desaparezca y vuelva.
 */
describe("el bloque de firma visible, sin certificado", () => {
  const stamping = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };

  function toggle() {
    return screen.getByRole("switch", {
      name: /Estampar un recuadro de firma en el documento/,
    });
  }

  /**
   * ID-108. El bloque entero apagado y en gris, y el interruptor **en «no»**:
   * pintarlo encendido dentro de un bloque inerte prometía un recuadro que no
   * hay, porque sin certificado no hay sello que dibujar.
   */
  it("turns the whole visible-signature block off, switch included, without a certificate", () => {
    renderPanel({
      certificate: { kind: "unchosen", certificates: [certificate] },
      signature: stamping,
    });

    expect(toggle()).toHaveAttribute("aria-checked", "false");
    expect(
      screen.getByText("Elige un certificado para colocar la firma visible"),
    ).toBeInTheDocument();
    // Y nada de lo que hay dentro del bloque, que es lo que no se puede decidir.
    expect(screen.queryByRole("checkbox", { name: /Firmante/ })).not.toBeInTheDocument();
  });

  it("keeps the placement across a certificate that comes and goes", () => {
    const { show } = renderPanel({ signature: stamping });
    expect(screen.getByRole("button", { name: "Quitarla de aquí" })).toBeInTheDocument();

    show({ certificate: { kind: "empty" }, signature: stamping });
    show({ signature: stamping });

    expect(screen.getByRole("button", { name: "Quitarla de aquí" })).toBeInTheDocument();
  });
});
