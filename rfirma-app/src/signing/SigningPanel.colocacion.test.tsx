import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { rect, renderLivePanel, renderPanel } from "./SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

describe("SigningPanel · Colocación", () => {
  /** El interruptor encendido, que es donde vive el bloque entero. */
  const visible = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };

  const signButton = () => screen.getByRole("button", { name: "Firmar como Ada Lovelace" });
  const field = () => screen.getByLabelText("Páginas donde se sella");

  it("refuses to sign a visible signature that is not placed anywhere, and says what to do", () => {
    renderPanel({ signature: visible, placement: null });

    expect(
      screen.getByText(
        "Coloca la firma sobre el documento: arrastra un recuadro o pulsa el botón de sellar.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ponerla aquí" })).toBeInTheDocument();
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
    expect(within(block).getByRole("button", { name: "Quitarla de aquí" })).toBeInTheDocument();
    expect(screen.queryByText("El recuadro está en esta página")).not.toBeInTheDocument();
    expect(screen.queryByText(/El recuadro está en la página/)).not.toBeInTheDocument();
    expect(screen.queryByText("Aún no has colocado la firma")).not.toBeInTheDocument();
  });

  it("seals the page it is looking at when nothing is placed yet", async () => {
    const user = userEvent.setup();
    const onSeal = vi.fn();
    renderPanel({ signature: visible, placement: null, onSeal });

    await user.click(screen.getByRole("button", { name: "Ponerla aquí" }));

    expect(onSeal).toHaveBeenCalled();
  });

  /**
   * Con «Todas las páginas» y el recuadro sin colocar, «esta página»
   * mentiría: el conjunto ya está completo y falta el rectángulo.
   */
  it("offers to place the stamp here when «all pages» is chosen and nothing is placed", () => {
    renderPanel({ signature: visible, placement: null, pageChoice: "all" });

    expect(screen.getByRole("button", { name: "Ponerla aquí" })).toBeInTheDocument();
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

    await user.click(screen.getByRole("button", { name: "Quitarla de aquí" }));

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

    expect(screen.queryByRole("button", { name: "Quitarla de aquí" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ponerla aquí" })).toBeInTheDocument();
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

    await user.click(screen.getByRole("button", { name: "Ponerla aquí" }));

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
