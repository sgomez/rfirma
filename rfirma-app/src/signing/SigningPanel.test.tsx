import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import {
  activating,
  type PageChoice,
  type PageSet,
  type PageSets,
  pagesOf,
  placementOf,
  storing,
} from "../viewer/signatureBox";
import type { Certificate } from "./certificate";
import type { Rubric } from "./rubric";
import { SigningPanel } from "./SigningPanel";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

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
      pageSets={{ single: 3, these: null }}
      onChoosePages={noop}
      pageChoice="single"
      onChangePageChoice={noop}
      viewedPage={3}
      onSeal={noop}
      onUnseal={noop}
      rubric={null}
      rubricFailure={null}
      onChooseRubric={noop}
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
 * El panel con **las tres opciones de verdad** detrás, que es como vive en
 * `App.tsx` desde el #188.
 *
 * Teclear en el campo son varias pulsaciones seguidas y cada una emite el
 * conjunto: con un espía que no lo aplica, la segunda pulsación escribiría
 * sobre un panel que sigue viendo el conjunto viejo, y lo que se probaría sería
 * el espía. El recuadro es fijo porque el panel ya no lo compone: solo nombra
 * páginas, y quién las convierte en rectángulo es cosa de `App.tsx`.
 */
function renderLivePanel(props: PanelProps = {}) {
  const chosen: (PageSet | null)[] = [];
  function Live() {
    const choice = props.pageChoice ?? "single";
    const [sets, setSets] = useState<PageSets>({
      single: 3,
      these: choice === "these" ? { only: [3] } : null,
    });
    // La opción elegida también vive fuera del panel, como en `App.tsx`: sin
    // eso, volver a pulsar «Solo 1 página» no dispara nada —el radio sigue
    // marcado— y el viaje de ida y vuelta no se podría probar.
    const [pageChoice, setPageChoice] = useState<PageChoice>(choice);
    return panelWith({
      ...props,
      placement: placementOf(rect, sets, pageChoice),
      pageSets: sets,
      onChoosePages: (next) => {
        chosen.push(next);
        setSets(storing(sets, pageChoice, next, 27));
      },
      pageChoice,
      onChangePageChoice: (next) => {
        setSets(activating(sets, next, pagesOf(sets, pageChoice), 27, 3));
        setPageChoice(next);
      },
    });
  }
  renderWithCatalog(<Live />);
  return { chosen };
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
    expect(screen.getByRole("button", { name: "Quitar el sello" })).toBeInTheDocument();
    for (const label of ["Firmante", "Emisor", "Fecha", "Rúbrica", "Motivo"]) {
      expect(screen.getByRole("checkbox", { name: new RegExp(label) })).toBeInTheDocument();
    }
    expect(screen.queryByRole("checkbox", { name: /DNI/ })).not.toBeInTheDocument();
    await user.type(screen.getByRole("textbox", { name: "Motivo" }), "!");
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

  it("never shows a wildcard in the interface", () => {
    // ID-19: las casillas marcan qué dato aparece, y nunca hay una cadena
    // como `$$SUBJECTCN$$` escrita a mano en el panel.
    renderPanel({});

    expect(document.body.textContent).not.toMatch(/\$\$/);
  });

  it("shows the rubric already normalized, over white, before signing", () => {
    renderPanel({ rubric });

    const thumbnail = screen.getByAltText("Tu rúbrica, tal como se estampará");
    expect(thumbnail).toHaveAttribute("src", rubric.dataUrl);
    expect(screen.getByText(/Se estampa sobre blanco/)).toBeInTheDocument();
  });

  it("cannot tick a rubric that does not exist", () => {
    renderPanel({ rubric: null });

    expect(screen.getByRole("checkbox", { name: /Rúbrica/ })).toBeDisabled();
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
        "Coloca la firma sobre el documento: arrastra un recuadro o pulsa el botón de sellar.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sellar esta página" })).toBeInTheDocument();
    expect(signButton()).toBeDisabled();
  });

  /**
   * #194: el botón vive en el bloque «Colocación», a todo el ancho y bajo los
   * radios, y con él desaparecen los tres mensajes de colocación —incluido el
   * que saltaba a la página del recuadro— porque su etiqueta ya cuenta lo
   * mismo.
   */
  it("offers the seal button in the placement block, and none of the retired messages", () => {
    renderPanel({ signature: visible, placement: { rect, pages: { only: [3] } }, viewedPage: 3 });

    const block = screen.getByText("Colocación").closest("fieldset") as HTMLElement;
    expect(within(block).getByRole("button", { name: "Quitar el sello" })).toBeInTheDocument();
    expect(screen.queryByText("El recuadro está en esta página")).not.toBeInTheDocument();
    expect(screen.queryByText(/El recuadro está en la página/)).not.toBeInTheDocument();
    expect(screen.queryByText("Aún no has colocado la firma")).not.toBeInTheDocument();
  });

  it("seals the page it is looking at when nothing is placed yet", async () => {
    const user = userEvent.setup();
    const onSeal = vi.fn();
    renderPanel({ signature: visible, placement: null, onSeal });

    await user.click(screen.getByRole("button", { name: "Sellar esta página" }));

    expect(onSeal).toHaveBeenCalled();
  });

  /**
   * Con «Todas las páginas» y el recuadro sin colocar, «esta página»
   * mentiría: el conjunto ya está completo y falta el rectángulo.
   */
  it("offers to place the stamp here when «all pages» is chosen and nothing is placed", () => {
    renderPanel({ signature: visible, placement: null, pageChoice: "all" });

    expect(screen.getByRole("button", { name: "Colocar el sello aquí" })).toBeInTheDocument();
  });

  it("offers to unseal the page it is looking at when it already carries the stamp", async () => {
    const user = userEvent.setup();
    const onUnseal = vi.fn();
    renderPanel({
      signature: visible,
      placement: { rect, pages: { only: [3] } },
      viewedPage: 3,
      onUnseal,
    });

    await user.click(screen.getByRole("button", { name: "Quitar el sello" }));

    expect(onUnseal).toHaveBeenCalled();
  });

  /**
   * «Todas las páginas» no tiene conjunto propio que guardar (`storing`,
   * `signatureBox.ts`): quitarle una página de ahí no se va a ninguna parte,
   * `onUnseal` resolvería «todas» en sueltas y `placementOf` las recompondría
   * en «todas» acto seguido, y el botón parecería no hacer nada. No se ofrece.
   */
  it("does not offer to unseal while «all pages» is chosen, even though the page carries the stamp", () => {
    renderPanel({
      signature: visible,
      placement: { rect, pages: "all" },
      pageChoice: "all",
      viewedPage: 3,
    });

    expect(screen.queryByRole("button", { name: "Quitar el sello" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Colocar el sello aquí" })).toBeInTheDocument();
  });

  it("signs invisibly with the switch off, which is the other «no» entirely", () => {
    renderPanel({ signature: { ...visible, enabled: false }, placement: null });

    expect(signButton()).toBeEnabled();
    expect(screen.queryByText("Colocación")).not.toBeInTheDocument();
  });

  it("does not lose the placement when the switch goes off and on again", () => {
    const onChoosePages = vi.fn();
    const { show } = renderPanel({ signature: visible, onChoosePages });

    show({ signature: { ...visible, enabled: false }, onChoosePages });
    show({ signature: visible, onChoosePages });

    expect(onChoosePages).not.toHaveBeenCalled();
    expect(screen.getByText("Página 3")).toBeInTheDocument();
  });

  it("seals what the field says, in the everyday print format", () => {
    const { chosen } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: "1,2-3,10-20" } });

    expect(chosen.at(-1)).toEqual({
      only: [1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
    });
    expect(field()).toHaveValue("1,2-3,10-20");
  });

  it("echoes the pages it is going to seal instead of leaving the field to be read", () => {
    const sealed = { only: [1, 2, 3, 10, 11, 12, 13, 14] };
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages: sealed },
      pageSets: { single: 1, these: sealed },
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
    const { chosen } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: typed } });

    expect(screen.getByText(said)).toBeInTheDocument();
    expect(signButton()).toBeDisabled();
    // Nada se aplica a medias: el conjunto se queda como estaba (ID-22).
    expect(chosen).toEqual([]);
  });

  it("rewrites the field when a page is unsealed from the viewer (ID-99)", () => {
    const props = { signature: visible, pageChoice: "these" as const };
    const sealed = { only: [3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20] };
    const { show } = renderPanel({
      ...props,
      placement: { rect, pages: sealed },
      pageSets: { single: 3, these: sealed },
    });

    expect(field()).toHaveValue("3,10-20");

    const rest = { only: [3, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20] };
    show({ ...props, placement: { rect, pages: rest }, pageSets: { single: 3, these: rest } });

    expect(field()).toHaveValue("3,10-11,13-20");
  });

  /**
   * Elegir una opción **es solo elegirla** (#188). El conjunto de cada una lo
   * guarda quien las tiene las tres, y mientras el panel emitía además un
   * conjunto por su cuenta, la opción que dejabas se reescribía con la que
   * llegaba: de ahí salía que `Solo 1 página` y `Estas páginas` compartieran
   * estado.
   */
  it("asks for the option and does not decide the set that goes with it", async () => {
    const user = userEvent.setup();
    const onChoosePages = vi.fn();
    const onChangePageChoice = vi.fn();
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages: { only: [3, 10, 11] } },
      pageSets: { single: 3, these: { only: [3, 10, 11] } },
      onChoosePages,
      onChangePageChoice,
    });

    await user.click(screen.getByRole("radio", { name: /Solo 1 página/ }));

    expect(onChangePageChoice).toHaveBeenCalledWith("single");
    expect(onChoosePages).not.toHaveBeenCalled();
  });

  it("names every page of the document when «all» is chosen", async () => {
    const user = userEvent.setup();
    const onChangePageChoice = vi.fn();
    renderPanel({ signature: visible, onChangePageChoice });

    expect(screen.getByRole("radio", { name: /Todas las páginas \(27\)/ })).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: /Todas las páginas/ }));

    expect(onChangePageChoice).toHaveBeenCalledWith("all");
  });

  it("offers to seal the page it is looking at, not the one the box is already on", async () => {
    const user = userEvent.setup();
    const onSeal = vi.fn();
    renderPanel({ signature: visible, viewedPage: 7, onSeal });

    await user.click(screen.getByRole("button", { name: "Sellar esta página" }));

    expect(onSeal).toHaveBeenCalled();
  });

  it("warns that the repeated box is one signature field and not one per page", () => {
    renderPanel({ signature: visible, placement: { rect, pages: { only: [3, 4, 5] } } });

    expect(screen.getByText(/es un solo campo de firma repetido, no 3 firmas/)).toBeInTheDocument();
  });

  /**
   * ID-97 y #188, el viaje completo. Con «todas» el conjunto activo ya no
   * nombra la página del gesto, y la que vuelve al elegir «Solo 1 página» es
   * **la que esa opción guarda**, no la más baja del conjunto por casualidad.
   * Su pie la dice todo el rato, incluso mientras manda otra opción.
   */
  it("keeps the page of the box on the round trip single, all and single again", async () => {
    const user = userEvent.setup();
    renderLivePanel({ signature: visible });

    await user.click(screen.getByRole("radio", { name: /Todas las páginas/ }));

    expect(screen.getByText(/en las 27 páginas/)).toBeInTheDocument();
    // El pie de «Solo 1 página» sigue diciendo la página del gesto original.
    expect(screen.getByText("Página 3")).toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /Solo 1 página/ }));

    expect(screen.queryByText(/en las 27 páginas/)).not.toBeInTheDocument();
    expect(screen.getByText("Página 3")).toBeInTheDocument();
  });

  /**
   * Borrar el campo es el paso normal para reescribir el rango. Si el vacío
   * emitiera `onPlace(null)` se llevaría la colocación entera —`rect`
   * incluido— y el campo ya no podría devolverla: habría que volver a arrastrar
   * sobre la hoja.
   */
  it("says the empty field instead of taking the box away with it", () => {
    const { chosen } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: "" } });

    expect(chosen).toEqual([]);
    expect(screen.getByText("Escribe en qué páginas se sella: 1,2-3,10-20.")).toBeInTheDocument();
    expect(signButton()).toBeDisabled();

    // Y el campo devuelve el recuadro, con el mismo sitio y el mismo tamaño.
    fireEvent.change(field(), { target: { value: "5" } });

    expect(chosen.at(-1)).toEqual({ only: [5] });
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
    expect(screen.getByRole("button", { name: "Quitar el sello" })).toBeInTheDocument();

    show({ certificate: { kind: "empty" }, signature: stamping });
    show({ signature: stamping });

    expect(screen.getByRole("button", { name: "Quitar el sello" })).toBeInTheDocument();
  });
});
