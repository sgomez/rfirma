import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { PreferencesDialog } from "./PreferencesDialog";
import type { Preferences } from "./preferences";

const defaults: Preferences = {
  theme: "system",
  destination: "Documentos",
  offersOriginalFolder: false,
  saveNextToOriginal: false,
  rememberVisibleSignature: true,
  rememberActivity: true,
};

const noop = async () => {};

function renderDialog(props: Partial<Parameters<typeof PreferencesDialog>[0]> = {}) {
  return renderWithCatalog(
    <PreferencesDialog
      preferences={defaults}
      onChooseDestination={noop}
      onChange={noop}
      onForgetActivity={noop}
      onClose={noop}
      {...props}
    />,
  );
}

// Grada A: los ajustes son datos, y el diálogo no habla con nadie.
describe("PreferencesDialog", () => {
  it("applies a change as it is made, with no Save and no Cancel", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderDialog({ onChange });

    await user.click(
      screen.getByRole("switch", { name: /Recordar la última configuración de firma visible/ }),
    );

    expect(onChange).toHaveBeenCalledWith({ ...defaults, rememberVisibleSignature: false });
    expect(screen.queryByRole("button", { name: "Guardar" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeInTheDocument();
  });

  /**
   * El interruptor es el mismo componente en el panel de firma y aquí, pero los
   * artboards lo separan distinto del texto: `rf-gap-xs` (8 px) en el panel
   * (`Main.dc.html:306`) y `rf-gap-sm` (16 px) en el diálogo
   * (`PreferenciasPantalla`). Un solo valor no puede ser los dos, y arreglar
   * uno rompía el otro: la pantalla pide el suyo, y por eso se comprueba que
   * lo pida.
   */
  it("asks for the wider spacing the Preferences artboard draws", () => {
    renderDialog();

    for (const name of [
      /Recordar la última configuración de firma visible/,
      /Recordar mi actividad/,
    ]) {
      const control = screen.getByRole("switch", { name });
      expect(control.closest(".switch")).toHaveClass("switch--wide");
    }
  });

  it("shows the destination folder by its name and never by its path", () => {
    renderDialog({ preferences: { ...defaults, destination: "Documentos" } });

    expect(screen.getByText("Dónde se guarda el documento firmado")).toBeInTheDocument();
    expect(screen.getByText("Documentos")).toBeInTheDocument();
    expect(screen.queryByText(/\/home\//)).not.toBeInTheDocument();
  });

  it("picks the destination folder with a directory picker and not with a dropdown", async () => {
    // El desplegable recibía una sola opción: un control que finge elegir
    // (ID-65). Lo que hay es un botón que abre el selector del sistema.
    const user = userEvent.setup();
    const onChooseDestination = vi.fn(async () => {});
    renderDialog({ onChooseDestination });

    expect(
      screen.queryByRole("combobox", { name: "Dónde se guarda el documento firmado" }),
    ).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cambiar carpeta…" }));

    expect(onChooseDestination).toHaveBeenCalledOnce();
  });

  it("shows in the section the failure to choose a folder", async () => {
    const user = userEvent.setup();
    renderDialog({
      onChooseDestination: () => Promise.reject(new Error("no se pudo guardar")),
    });

    await user.click(screen.getByRole("button", { name: "Cambiar carpeta…" }));

    expect(await screen.findByText(/no se pudo guardar/)).toBeInTheDocument();
  });

  // «Junto al documento original» solo cuando el entorno sabe devolver la ruta
  // real del documento (ID-184): donde no la sabe, la opción no aparece y el
  // ajuste se queda en la carpeta con su «Cambiar carpeta…», como antes.
  it("offers Junto al documento original only when the environment allows it", () => {
    renderDialog({ preferences: { ...defaults, offersOriginalFolder: false } });

    expect(screen.queryByRole("radiogroup")).not.toBeInTheDocument();
    expect(screen.queryByText("Junto al documento original")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();
  });

  it("shows the two destination options when the environment allows it", () => {
    renderDialog({ preferences: { ...defaults, offersOriginalFolder: true } });

    expect(screen.getByRole("radiogroup")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Junto al documento original" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "En esta carpeta" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();
  });

  it("starts on En esta carpeta and applies the choice as it is made", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderDialog({
      preferences: { ...defaults, offersOriginalFolder: true },
      onChange,
    });

    expect(screen.getByRole("radio", { name: "En esta carpeta" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Junto al documento original" })).not.toBeChecked();

    await user.click(screen.getByRole("radio", { name: "Junto al documento original" }));

    expect(onChange).toHaveBeenCalledWith({
      ...defaults,
      offersOriginalFolder: true,
      saveNextToOriginal: true,
    });
  });

  it("offers only the languages whose catalog is complete", async () => {
    const user = userEvent.setup();
    renderDialog();

    const language = screen.getByRole("combobox", { name: "Idioma" });
    expect(language).toHaveTextContent("Español");
    await user.click(language);

    const offered = screen.getAllByRole("option").map((option) => option.textContent);
    expect(offered).toEqual(["Español", "English"]);
  });

  it("changes the language in place", async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.click(screen.getByRole("combobox", { name: "Idioma" }));
    await user.click(screen.getByRole("option", { name: "English" }));

    expect(await screen.findByText("Preferences")).toBeInTheDocument();
  });

  /**
   * El tema no lo dibuja el canvas: llegó después, y por eso se comprueba que
   * está y que ofrece los tres valores. `El del sistema` no es «claro»: es no
   * forzar nada.
   */
  it("offers the three themes and applies the chosen one straight away", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderDialog({ onChange });

    const theme = screen.getByRole("combobox", { name: "Tema" });
    expect(theme).toHaveTextContent("El del sistema");
    await user.click(theme);
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual([
      "El del sistema",
      "Claro",
      "Oscuro",
    ]);

    await user.click(screen.getByRole("option", { name: "Oscuro" }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, theme: "dark" });
  });

  it("empties the list without turning the switch off", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn();
    const onChange = vi.fn();
    renderDialog({ onForgetActivity, onChange });

    await user.click(screen.getByRole("button", { name: "Vaciar la lista" }));

    expect(onForgetActivity).toHaveBeenCalledOnce();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("asks before erasing when Remember my activity is turned off", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn();
    const onChange = vi.fn();
    renderDialog({ onForgetActivity, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));

    expect(onChange).not.toHaveBeenCalled();
    expect(onForgetActivity).not.toHaveBeenCalled();
    expect(screen.getByText(/Al apagarlo se borra lo ya recordado/)).toBeInTheDocument();
  });

  it("erases what was remembered once the purge is confirmed", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn();
    const onChange = vi.fn();
    renderDialog({ onForgetActivity, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Borrar y apagar" }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, rememberActivity: false });
    expect(onForgetActivity).toHaveBeenCalledOnce();
  });

  it("keeps what was remembered when the purge is called off", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn();
    const onChange = vi.fn();
    renderDialog({ onForgetActivity, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onChange).not.toHaveBeenCalled();
    expect(onForgetActivity).not.toHaveBeenCalled();
  });

  it("turns Remember my activity back on without asking", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderDialog({ preferences: { ...defaults, rememberActivity: false }, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, rememberActivity: true });
  });

  it("closes on Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderDialog({ onClose });

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });
  /**
   * La pantalla completa sigue siendo un diálogo (ID-68): `Escape` la cierra y
   * el foco entra en ella al abrirse, que es lo que la distingue de una región
   * más de la ventana.
   */
  it("closes on Escape, like the dialog it still is", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderDialog({ onClose });

    expect(screen.getByRole("dialog", { name: "Preferencias" })).toHaveFocus();
    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });

  /** Los cinco ajustes, repartidos en las tres secciones del ID-69. */
  it("lays the settings out in three sections with an index to the left", () => {
    renderDialog();

    const index = screen.getByRole("navigation", { name: "Secciones" });
    expect(
      within(index)
        .getAllByRole("button")
        .map((row) => row.textContent),
    ).toEqual(["Firma", "Privacidad", "Apariencia"]);

    const signing = screen.getByRole("region", { name: "Firma" });
    expect(
      within(signing).getByRole("switch", {
        name: /Recordar la última configuración de firma visible/,
      }),
    ).toBeInTheDocument();
    expect(within(signing).getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();

    const privacy = screen.getByRole("region", { name: "Privacidad" });
    expect(
      within(privacy).getByRole("switch", { name: /Recordar mi actividad/ }),
    ).toBeInTheDocument();
    expect(within(privacy).getByRole("button", { name: "Vaciar la lista" })).toBeInTheDocument();

    const appearance = screen.getByRole("region", { name: "Apariencia" });
    expect(within(appearance).getByRole("combobox", { name: "Tema" })).toBeInTheDocument();
    expect(within(appearance).getByRole("combobox", { name: "Idioma" })).toBeInTheDocument();
  });

  it("marks the chosen section in the index and leaves the first one chosen", async () => {
    const user = userEvent.setup();
    renderDialog();

    const index = screen.getByRole("navigation", { name: "Secciones" });
    expect(within(index).getByRole("button", { name: "Firma" })).toHaveAttribute(
      "aria-current",
      "true",
    );

    await user.click(within(index).getByRole("button", { name: "Apariencia" }));

    expect(within(index).getByRole("button", { name: "Apariencia" })).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(within(index).getByRole("button", { name: "Firma" })).not.toHaveAttribute(
      "aria-current",
    );
  });

  /** Fijo: un botón de cierre que se va con el desplazamiento no está (ID-69). */
  it("keeps Cerrar in a footer outside the column that scrolls", () => {
    const { container } = renderDialog();

    const close = screen.getByRole("button", { name: "Cerrar" });
    expect(close.closest(".preferences__footer")).not.toBeNull();
    expect(container.querySelector(".preferences__content")?.contains(close)).toBe(false);
  });

  /**
   * El aviso va **en la sección donde se pulsó** y no en una franja común
   * arriba (ID-70): con tres secciones, un aviso común obliga a leer el texto
   * para saber qué se rompió.
   */
  it("shows the failure to save inside the section where the setting was pressed", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn(async () => {
      throw new Error("no se deja escribir");
    });
    renderDialog({ onChange });

    await user.click(screen.getByRole("combobox", { name: "Tema" }));
    await user.click(screen.getByRole("option", { name: "Oscuro" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("No hemos podido guardar el ajuste");
    expect(notice).toHaveTextContent("Hemos vuelto al valor anterior");
    expect(screen.getByRole("region", { name: "Apariencia" })).toContainElement(notice);
    expect(screen.getByRole("region", { name: "Firma" })).not.toContainElement(notice);
  });

  it("keeps the technical detail of the rejection in the notice", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn(async () => {
      throw new Error("EACCES: permission denied");
    });
    renderDialog({ onChange });

    await user.click(
      screen.getByRole("switch", { name: /Recordar la última configuración de firma visible/ }),
    );

    expect(await screen.findByText("EACCES: permission denied")).toBeInTheDocument();
  });

  /** El otro fallo que se tragaba: siempre en Privacidad, pegado a su botón. */
  it("says the recents are still saved when emptying the list fails", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn(async () => {
      throw new Error("no se deja borrar");
    });
    renderDialog({ onForgetActivity });

    await user.click(screen.getByRole("button", { name: "Vaciar la lista" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("No hemos podido vaciar la lista");
    expect(notice).toHaveTextContent("siguen guardados");
    expect(screen.getByRole("region", { name: "Privacidad" })).toContainElement(notice);
  });

  it("says nothing when the setting is saved", async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.click(screen.getByRole("switch", { name: /Recordar la última configuración/ }));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  /** El interruptor no se mueve hasta que se confirma (ID-71). */
  it("leaves the switch on while the purge is being confirmed", async () => {
    const user = userEvent.setup();
    renderDialog();

    const remember = screen.getByRole("switch", { name: /Recordar mi actividad/ });
    await user.click(remember);

    expect(remember).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveClass("rf-btn--ghost");
    expect(screen.getByRole("button", { name: "Borrar y apagar" })).toHaveClass("rf-btn--primary");
  });

  /** La confirmación es a su vez modal: el teclado no se sale de ella (ID-71). */
  it("keeps the keyboard inside the confirmation while it is in front", async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));

    const confirmation = screen
      .getByText(/Al apagarlo se borra lo ya recordado/)
      .closest(".rf-dialog") as HTMLElement;
    expect(confirmation.contains(document.activeElement)).toBe(true);

    // Dos botones: al tercer tabulador ya ha dado la vuelta en vez de irse al
    // índice de secciones que queda detrás.
    await user.tab();
    await user.tab();
    await user.tab();

    expect(confirmation.contains(document.activeElement)).toBe(true);
  });

  it("calls the confirmation off with Escape, without closing the screen", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onChange = vi.fn();
    renderDialog({ onClose, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));
    await user.keyboard("{Escape}");

    expect(screen.queryByText(/Al apagarlo se borra lo ya recordado/)).not.toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
    expect(onChange).not.toHaveBeenCalled();
  });
});
