import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { Placement } from "../viewer/signatureBox";
import type { Certificate } from "./certificate";
import type { Rubric } from "./rubric";
import { SigningPanel } from "./SigningPanel";
import { DEFAULT_VISIBLE_SIGNATURE, type Layer2Composer } from "./visibleSignature";

const certificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  store: "card",
  status: { kind: "valid" },
  remembered: false,
};

/** Un JPEG de un píxel: lo que devuelve `rubric::normalize`, ya opaco. */
const rubric: Rubric = {
  dataUrl: "data:image/jpeg;base64,/9j/4AAQSkZJRg==",
  width: 240,
  height: 80,
};

/** El compositor de verdad es Rust; aquí basta con que devuelva algo fiel. */
function composerOf(text: string | null): Layer2Composer {
  return { compose: async () => text };
}

const noop = () => {};

/** El recuadro, en espacio de usuario: aquí solo importa que exista. */
const rect = { x0: 100, y0: 100, x1: 300, y1: 180 };

type PanelProps = Partial<Parameters<typeof SigningPanel>[0]>;

function panelWith(props: PanelProps) {
  return (
    <SigningPanel
      document={{ name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000, signatures: 0 }}
      certificate={{ kind: "chosen", certificate, certificates: [certificate] }}
      onChooseCertificate={noop}
      onRetryCertificates={noop}
      onChooseModule={noop}
      signature={DEFAULT_VISIBLE_SIGNATURE}
      onChangeSignature={noop}
      placement={{ rect, pages: { only: [3] } }}
      onPlace={noop}
      pageChoice="single"
      onChangePageChoice={noop}
      viewedPage={3}
      onGoToPage={noop}
      rubric={null}
      rubricFailure={null}
      onChooseRubric={noop}
      signedAt="31/08/26, 12:00:00"
      composer={composerOf(null)}
      destination={{ folder: "Documentos", name: "contrato-firmado.pdf", writable: true }}
      onChangeDestination={noop}
      onSign={noop}
      signing={false}
      failure={null}
      {...props}
    />
  );
}

/** `show` vuelve a pintar con otras props: es el camino de vuelta del ID-99. */
function renderPanel(props: PanelProps = {}) {
  const result = renderWithCatalog(panelWith(props));
  return { ...result, show: (next: PanelProps) => result.rerender(panelWith(next)) };
}

/**
 * El panel con el `placement` de verdad detrás, que es como vive en `App.tsx`.
 *
 * Teclear en el campo son varias pulsaciones seguidas y cada una emite el
 * conjunto: con un espía que no lo aplica, la segunda pulsación escribiría
 * sobre un panel que sigue viendo el conjunto viejo, y lo que se probaría sería
 * el espía.
 */
function renderLivePanel(props: PanelProps = {}) {
  const placed: (Placement | null)[] = [];
  function Live() {
    const [placement, setPlacement] = useState<Placement | null>({ rect, pages: { only: [3] } });
    return panelWith({
      ...props,
      placement,
      onPlace: (next) => {
        placed.push(next);
        setPlacement(next);
      },
    });
  }
  renderWithCatalog(<Live />);
  return { placed };
}

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

  it("covers the certificate, the visible-signature toggle, the page, the checkboxes and the reason", async () => {
    const user = userEvent.setup();
    renderPanel({
      signature: {
        ...DEFAULT_VISIBLE_SIGNATURE,
        fields: { ...DEFAULT_VISIBLE_SIGNATURE.fields, reason: true },
      },
    });

    expect(screen.getByText("Ada Lovelace Byron")).toBeInTheDocument();
    const toggle = screen.getByRole("switch", {
      name: /Estampar un recuadro de firma en el documento/,
    });
    expect(toggle).toBeInTheDocument();
    // El panel lo dibuja con `rf-gap-xs` (8 px, `Main.dc.html:306`); los 16 px
    // son de Preferencias y se piden allí con `switch--wide`.
    expect(toggle.closest(".switch")).not.toHaveClass("switch--wide");
    expect(screen.getByText("El recuadro está en esta página")).toBeInTheDocument();
    for (const label of [
      "Tu rúbrica",
      "Nombre y apellidos",
      "DNI",
      "Fecha y hora de la firma",
      "Un motivo",
    ]) {
      expect(screen.getByRole("checkbox", { name: new RegExp(label) })).toBeInTheDocument();
    }
    await user.type(screen.getByLabelText("Motivo"), "!");
    expect(screen.getByRole("button", { name: "Elegir imagen" })).toBeInTheDocument();
  });

  it("is the only region with a primary button, and it comes last", () => {
    renderPanel();

    const primaries = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button.rf-btn--primary"),
    );
    expect(primaries.map((button) => button.textContent)).toEqual(["Firmar documento"]);

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
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeEnabled();
  });

  it("does not promise a destination it has just said it cannot write to", () => {
    // «Se guardará en» y «No se puede escribir en Documentos» a la vez es una
    // contradicción: el rótulo es la promesa y desaparece con ella.
    renderPanel({ destination: { folder: "Documentos", name: null, writable: false } });

    expect(screen.queryByText("Se guardará en")).not.toBeInTheDocument();
  });

  it("never shows a wildcard, neither in the interface nor in the preview", async () => {
    renderPanel({ composer: composerOf("Firmado por: Ada Lovelace Byron\nDNI: ***9999**") });

    expect(await screen.findByText(/Firmado por: Ada Lovelace Byron/)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/\$\$/);
  });

  it("previews the composed text and not an imitation of its own", async () => {
    // El compositor autoritativo es `signing::layer2_text`: si el panel
    // compusiera el texto por su cuenta, esta cadena no aparecería tal cual.
    renderPanel({ composer: composerOf("Fecha: 31/08/2026 12:00:00 CEST") });

    expect(await screen.findByText("Fecha: 31/08/2026 12:00:00 CEST")).toBeInTheDocument();
  });

  it("shows the rubric already normalized, over white, before signing", () => {
    renderPanel({ rubric });

    const thumbnail = screen.getByAltText("Tu rúbrica, tal como se estampará");
    expect(thumbnail).toHaveAttribute("src", rubric.dataUrl);
    expect(screen.getByText(/Se estampa sobre blanco/)).toBeInTheDocument();
  });

  it("cannot tick a rubric that does not exist", () => {
    renderPanel({ rubric: null });

    expect(screen.getByRole("checkbox", { name: /Tu rúbrica/ })).toBeDisabled();
    expect(screen.getByText("Elige antes una imagen")).toBeInTheDocument();
  });

  it("counts the rubric failure as it is chosen, with the raw detail apart", () => {
    renderPanel({
      rubricFailure: { situation: "notAnAcceptedImage", detail: "image/gif" },
    });

    expect(screen.getByText("Esa imagen no vale como rúbrica")).toBeInTheDocument();
    expect(screen.getByText("image/gif")).toBeInTheDocument();
  });

  it("warns about the co-signature when the document already carries signatures", () => {
    renderPanel({
      document: { name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000, signatures: 1 },
    });

    expect(screen.getByText("Ya lleva 1 firma: la tuya será una cofirma.")).toBeInTheDocument();
  });

  it("offers two ways out when no certificate turned up", async () => {
    const user = userEvent.setup();
    const onRetryCertificates = vi.fn();
    renderPanel({ certificate: { kind: "empty" }, onRetryCertificates });

    expect(screen.getByText(/comprueba que está insertada/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Otro módulo…" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Volver a buscar" }));

    expect(onRetryCertificates).toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  it("shows a token failure as a translated situation with the raw CKR apart", () => {
    renderPanel({
      failure: { situation: "tokenAbsent", detail: "CKR_DEVICE_REMOVED (C_Sign)" },
    });

    expect(screen.getByText("No encontramos la tarjeta")).toBeInTheDocument();
    // El código original, ni traducido ni recortado: está para pegarlo.
    expect(screen.getByText("CKR_DEVICE_REMOVED (C_Sign)")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Volver a intentarlo" })).toBeInTheDocument();
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
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
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
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  /** Con varios y nada elegido no hay preselección: el orden de la lista solo
   * dice en qué orden cargaron los módulos, y elegir con qué identidad se firma
   * un documento con validez jurídica no lo hace la aplicación por su cuenta. */
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
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  /** Con uno solo se elige solo: elegir entre una cosa no es elegir. */
  it("keeps a single certificate chosen and ready to sign", () => {
    renderPanel();

    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "Ada Lovelace Byron",
    );
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeEnabled();
  });

  /** El disparador es ahora el sitio donde se cambia, así que el botón
   * `Cambiar` de la tarjeta ya no existe. El del pie es el del destino. */
  it("has no «change» button in the certificate section any more", () => {
    renderPanel();

    const section = screen.getByRole("region", { name: "Certificado" });
    expect(within(section).queryByRole("button", { name: "Cambiar" })).not.toBeInTheDocument();
  });

  it("waits for the certificates without pretending there are none", () => {
    renderPanel({ certificate: { kind: "loading" } });

    expect(screen.getByText("Buscando certificados…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });
});

// El bloque «Colocación»: los dos noes, el conjunto tecleado y la línea que
// dice la página del recuadro (ID-93, ID-97…ID-100).
describe("SigningPanel · Colocación", () => {
  /** El interruptor encendido, que es donde vive el bloque entero. */
  const visible = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };

  const signButton = () => screen.getByRole("button", { name: "Firmar documento" });
  const field = () => screen.getByLabelText("Páginas donde se sella");

  it("refuses to sign a visible signature that is not placed anywhere, and says what to do", () => {
    renderPanel({ signature: visible, placement: null });

    expect(
      screen.getByText(
        "Coloca la firma sobre el documento: arrastra un recuadro o pulsa el botón que hay bajo la página.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Aún no has colocado la firma")).toBeInTheDocument();
    expect(signButton()).toBeDisabled();
  });

  it("signs invisibly with the switch off, which is the other «no» entirely", () => {
    renderPanel({ signature: { ...visible, enabled: false }, placement: null });

    expect(signButton()).toBeEnabled();
    expect(screen.queryByText("Colocación")).not.toBeInTheDocument();
  });

  it("does not lose the placement when the switch goes off and on again", () => {
    const onPlace = vi.fn();
    const { show } = renderPanel({ signature: visible, onPlace });

    show({ signature: { ...visible, enabled: false }, onPlace });
    show({ signature: visible, onPlace });

    expect(onPlace).not.toHaveBeenCalled();
    expect(screen.getByText("Página 3")).toBeInTheDocument();
  });

  it("seals what the field says, in the everyday print format", () => {
    const { placed } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: "1,2-3,10-20" } });

    expect(placed.at(-1)).toEqual({
      rect,
      pages: { only: [1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20] },
    });
    expect(field()).toHaveValue("1,2-3,10-20");
  });

  it("echoes the pages it is going to seal instead of leaving the field to be read", () => {
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages: { only: [1, 2, 3, 10, 11, 12, 13, 14] } },
    });

    expect(
      screen.getByText("Se sellará en las páginas 1, 2, 3, 10, 11, 12 y 2 más."),
    ).toBeInTheDocument();
  });

  it.each([
    ["3-1", "«3-1» va al revés: el primer número tiene que ser el menor."],
    ["0", "No hay página 0: la primera es la 1."],
    ["99", "El documento tiene 27 páginas y has escrito hasta la 99."],
    ["1;2", "«1;2» no se entiende. Números y rangos separados por comas: 1,2-3,10-20."],
  ])("turns the sign button off and says why for %s", (typed, said) => {
    const { placed } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: typed } });

    expect(screen.getByText(said)).toBeInTheDocument();
    expect(signButton()).toBeDisabled();
    // Nada se aplica a medias: el conjunto se queda como estaba (ID-22).
    expect(placed).toEqual([]);
  });

  it("rewrites the field when a page is unsealed from the viewer (ID-99)", () => {
    const props = { signature: visible, pageChoice: "these" as const };
    const { show } = renderPanel({
      ...props,
      placement: { rect, pages: { only: [3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20] } },
    });

    expect(field()).toHaveValue("3,10-20");

    show({
      ...props,
      placement: { rect, pages: { only: [3, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20] } },
    });

    expect(field()).toHaveValue("3,10-11,13-20");
  });

  it("keeps the page of the box when the option changes", async () => {
    const user = userEvent.setup();
    const onPlace = vi.fn();
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages: { only: [3, 10, 11] } },
      onPlace,
    });

    await user.click(screen.getByRole("radio", { name: /Solo 1 página/ }));

    expect(onPlace).toHaveBeenLastCalledWith({ rect, pages: { only: [3] } });
  });

  it("names every page of the document when «all» is chosen", async () => {
    const user = userEvent.setup();
    const onPlace = vi.fn();
    renderPanel({ signature: visible, onPlace });

    expect(screen.getByRole("radio", { name: /Todas las páginas \(27\)/ })).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: /Todas las páginas/ }));

    expect(onPlace).toHaveBeenLastCalledWith({ rect, pages: "all" });
  });

  it("says the page of the box and not the one on screen, and going there is one click", async () => {
    const user = userEvent.setup();
    const onGoToPage = vi.fn();
    renderPanel({ signature: visible, viewedPage: 7, onGoToPage });

    await user.click(screen.getByRole("button", { name: "El recuadro está en la página 3" }));

    expect(onGoToPage).toHaveBeenCalledWith(3);
  });

  it("warns that the repeated box is one signature field and not one per page", () => {
    renderPanel({ signature: visible, placement: { rect, pages: { only: [3, 4, 5] } } });

    expect(screen.getByText(/es un solo campo de firma repetido, no 3 firmas/)).toBeInTheDocument();
  });
});
