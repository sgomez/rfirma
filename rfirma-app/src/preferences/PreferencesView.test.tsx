import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { defaults, openTab, renderView } from "./testSupport";

// Grada A: los ajustes son datos, y la vista no habla con nadie.
describe("PreferencesView", () => {
  it("applies a change as it is made, with no Save and no Cancel", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderView({ onChange });
    await openTab(user, "Firma");

    await user.click(screen.getByRole("switch", { name: /Recordar la firma visible/ }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, rememberVisibleSignature: false });
    expect(screen.queryByRole("button", { name: "Guardar" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeInTheDocument();
  });

  it("picks the destination folder with a directory picker and not with a dropdown", async () => {
    // El desplegable recibía una sola opción: un control que finge elegir
    // (ID-65). Lo que hay es un botón que abre el selector del sistema.
    const user = userEvent.setup();
    const onChooseDestination = vi.fn(async () => {});
    renderView({ onChooseDestination });
    await openTab(user, "Firma");

    expect(screen.queryByRole("combobox", { name: "Dónde guardar" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cambiar carpeta…" }));

    expect(onChooseDestination).toHaveBeenCalledOnce();
  });

  it("shows in the section the failure to choose a folder", async () => {
    const user = userEvent.setup();
    renderView({
      onChooseDestination: () => Promise.reject(new Error("no se pudo guardar")),
    });
    await openTab(user, "Firma");

    await user.click(screen.getByRole("button", { name: "Cambiar carpeta…" }));

    expect(await screen.findByText(/no se pudo guardar/)).toBeInTheDocument();
  });

  it("writes the chosen mode through the preferences port", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn(async () => {});
    renderView({
      onChange,
      preferences: {
        ...defaults,
        offersOriginalFolder: true,
        destinationMode: "next_to_the_original",
      },
    });
    await openTab(user, "Firma");

    await user.click(screen.getByRole("radio", { name: "En esta carpeta" }));

    expect(onChange).toHaveBeenCalledWith({
      ...defaults,
      offersOriginalFolder: true,
      destinationMode: "in_the_destination_folder",
    });
  });

  it("reverts to the previous mode and warns in the section when saving it fails", async () => {
    const user = userEvent.setup();
    renderView({
      onChange: () => Promise.reject(new Error("no se pudo guardar")),
      preferences: {
        ...defaults,
        offersOriginalFolder: true,
        destinationMode: "next_to_the_original",
      },
    });
    await openTab(user, "Firma");

    await user.click(screen.getByRole("radio", { name: "En esta carpeta" }));

    expect(await screen.findByText(/no se pudo guardar/)).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Junto al original" })).toBeChecked();
  });

  it("changes the language in place", async () => {
    const user = userEvent.setup();
    renderView();
    await openTab(user, "Apariencia");

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
    renderView({ onChange });
    await openTab(user, "Apariencia");

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
    renderView({ onForgetActivity, onChange });

    await user.click(screen.getByRole("button", { name: "Vaciar la lista" }));

    expect(onForgetActivity).toHaveBeenCalledOnce();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("asks before erasing when Remember my activity is turned off", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn();
    const onChange = vi.fn();
    renderView({ onForgetActivity, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));

    expect(onChange).not.toHaveBeenCalled();
    expect(onForgetActivity).not.toHaveBeenCalled();
    expect(
      screen.getByText(/Se borrarán los documentos recientes y el último certificado usado/),
    ).toBeInTheDocument();
  });

  it("erases what was remembered once the purge is confirmed", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn();
    const onChange = vi.fn();
    renderView({ onForgetActivity, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Borrar y apagar" }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, rememberActivity: false });
    expect(onForgetActivity).toHaveBeenCalledOnce();
  });

  it("keeps what was remembered when the purge is called off", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn();
    const onChange = vi.fn();
    renderView({ onForgetActivity, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onChange).not.toHaveBeenCalled();
    expect(onForgetActivity).not.toHaveBeenCalled();
  });

  it("turns Remember my activity back on without asking", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderView({ preferences: { ...defaults, rememberActivity: false }, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, rememberActivity: true });
  });

  /**
   * Sin confirmación y sin condición (ID-180): no es como «Recordar mi
   * actividad», que borra algo al apagarse. Este interruptor solo cambia si
   * la franja se enseña.
   */
  it("turns Avisar de versiones nuevas off without asking", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderView({ onChange });

    await user.click(screen.getByRole("switch", { name: "Avisar de versiones nuevas" }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, notifyNewVersion: false });
  });

  it("turns the protection against accidental signing off without asking", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderView({ onChange });
    await openTab(user, "Firma");

    await user.click(screen.getByRole("switch", { name: /Esperar 3 segundos antes de firmar/ }));

    expect(onChange).toHaveBeenCalledWith({ ...defaults, consentCountdown: false });
  });

  it("lets the site choose its only accepted certificate once turned on", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderView({ onChange });
    await openTab(user, "Firma");

    const toggle = screen.getByRole("switch", {
      name: /Si solo sirve un certificado y la sede lo permite/,
    });
    expect(toggle).not.toBeChecked();
    await user.click(toggle);

    expect(onChange).toHaveBeenCalledWith({ ...defaults, honourAutomaticSelection: true });
  });

  it("closes on Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderView({ onClose });

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });
  /**
   * `Escape` cierra Preferencias sin depender de dónde esté el foco (ID-352):
   * a diferencia del diálogo que era, esta vista no se lo roba al abrirse.
   */
  it("closes on Escape, wherever the focus is", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderView({ onClose });

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does not call onClose when Escape was default-prevented", () => {
    const onClose = vi.fn();
    renderView({ onClose });

    const event = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    event.preventDefault();
    window.dispatchEvent(event);

    expect(onClose).not.toHaveBeenCalled();
  });

  it("marks the chosen tab and leaves General chosen at the start", async () => {
    const user = userEvent.setup();
    renderView();

    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true");

    await openTab(user, "Apariencia");

    expect(screen.getByRole("tab", { name: "Apariencia" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "false");
  });

  /**
   * El patrón ARIA de pestañas verticales: las flechas arriba/abajo mueven la
   * selección, con vuelta al llegar a un extremo (TD-90).
   */
  it("moves the selection with the arrow keys, wrapping at the ends", async () => {
    const user = userEvent.setup();
    renderView();

    screen.getByRole("tab", { name: "General" }).focus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("tab", { name: "Firma" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: "Firma" })).toHaveFocus();

    await user.keyboard("{ArrowUp}");
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true");

    await user.keyboard("{ArrowUp}");
    expect(screen.getByRole("tab", { name: "Apariencia" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  /**
   * La pantalla ya no atrapa el foco (ID-352): a diferencia de los dos
   * modales que se ponen delante —confirmar el borrado, la contraseña del
   * `.p12`—, Shift+Tab desde la primera pestaña sale de la vista en vez de
   * dar la vuelta, que es lo que deja alcanzable el menú de la cabecera.
   */
  it("lets Shift+Tab leave the screen instead of trapping it, unlike the two modals", async () => {
    const user = userEvent.setup();
    renderView();

    screen.getByRole("tab", { name: "General" }).focus();
    await user.keyboard("{Shift>}{Tab}{/Shift}");

    const index = screen.getByRole("navigation", { name: "Secciones" });
    expect(index.contains(document.activeElement)).toBe(false);
  });

  /**
   * `Escape` sigue cerrando Preferencias entera por encima de todo, tanto si
   * el foco está en una pestaña como en cualquier otro control.
   */
  it("closes Preferences with Escape even while a tab has focus", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderView({ onClose });

    screen.getByRole("tab", { name: "General" }).focus();
    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
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
    renderView({ onChange });
    await openTab(user, "Apariencia");

    await user.click(screen.getByRole("combobox", { name: "Tema" }));
    await user.click(screen.getByRole("option", { name: "Oscuro" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("Algo ha fallado");
    expect(notice).toHaveTextContent("Vuelve a intentarlo.");
    expect(screen.getByRole("tabpanel", { name: "Apariencia" })).toContainElement(notice);
  });

  it("keeps the technical detail of the rejection in the notice", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn(async () => {
      throw new Error("EACCES: permission denied");
    });
    renderView({ onChange });
    await openTab(user, "Firma");

    await user.click(screen.getByRole("switch", { name: /Recordar la firma visible/ }));

    expect(await screen.findByText("EACCES: permission denied")).toBeInTheDocument();
  });

  /** El otro fallo que se tragaba: siempre en Privacidad, pegado a su botón. */
  it("tells the failure to empty the list inside the privacy section", async () => {
    const user = userEvent.setup();
    const onForgetActivity = vi.fn(async () => {
      throw new Error("no se deja borrar");
    });
    renderView({ onForgetActivity });

    await user.click(screen.getByRole("button", { name: "Vaciar la lista" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("Algo ha fallado");
    expect(notice).toHaveTextContent("Vuelve a intentarlo.");
    expect(screen.getByRole("group", { name: "Privacidad" })).toContainElement(notice);
  });

  /** El interruptor no se mueve hasta que se confirma (ID-71). */
  it("leaves the switch on while the purge is being confirmed", async () => {
    const user = userEvent.setup();
    renderView();

    const remember = screen.getByRole("switch", { name: /Recordar mi actividad/ });
    await user.click(remember);

    expect(remember).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveClass("rf-btn--ghost");
    expect(screen.getByRole("button", { name: "Borrar y apagar" })).toHaveClass("rf-btn--primary");
  });

  /** La confirmación es a su vez modal: el teclado no se sale de ella (ID-71). */
  it("keeps the keyboard inside the confirmation while it is in front", async () => {
    const user = userEvent.setup();
    renderView();

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));

    const confirmation = screen
      .getByText(/Se borrarán los documentos recientes y el último certificado usado/)
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
    renderView({ onClose, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));
    await user.keyboard("{Escape}");

    expect(
      screen.queryByText(/Se borrarán los documentos recientes y el último certificado usado/),
    ).not.toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
    expect(onChange).not.toHaveBeenCalled();
  });
});
