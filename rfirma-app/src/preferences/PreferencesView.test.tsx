import { screen, within } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Certificate } from "../signing/certificate";
import { renderWithCatalog } from "../testing/render";
import { PreferencesView } from "./PreferencesView";
import type { Preferences } from "./preferences";

const defaults: Preferences = {
  theme: "system",
  destination: "Documentos",
  offersOriginalFolder: false,
  rememberVisibleSignature: true,
  rememberActivity: true,
  notifyNewVersion: true,
  setupWizardSeen: false,
};

const noop = async () => {};

/** `2030-01-15T00:00:00Z`, en segundos desde la época. */
const IN_2030 = 1_894_752_000;

/** `2020-01-15T00:00:00Z`, en segundos desde la época. */
const IN_2020 = 1_579_046_400;

function anInstalledCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef",
    label: "FNMT-GEMELO",
    holderName: "Ada Lovelace Byron",
    idNumber: "IDCES-00000000T",
    issuer: "FNMT-RCM",
    store: "installed",
    status: { kind: "valid", notAfter: IN_2030 },
    remembered: false,
    ...overrides,
  };
}

function renderView(props: Partial<Parameters<typeof PreferencesView>[0]> = {}) {
  return renderWithCatalog(
    <PreferencesView
      preferences={defaults}
      onChooseDestination={noop}
      onChange={noop}
      onForgetActivity={noop}
      installedCertificates={[]}
      onInstallCertificate={async () => true}
      onRemoveCertificate={noop}
      onClose={noop}
      {...props}
    />,
  );
}

/** Pasa al panel de la pestaña dada, por su nombre en el índice. */
async function openTab(user: UserEvent, name: string) {
  await user.click(screen.getByRole("tab", { name }));
}

// Grada A: los ajustes son datos, y la vista no habla con nadie.
describe("PreferencesView", () => {
  it("applies a change as it is made, with no Save and no Cancel", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderView({ onChange });
    await openTab(user, "Firma");

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
   * lo pida, en las dos pestañas donde aparece.
   */
  it("asks for the wider spacing the Preferences artboard draws", async () => {
    const user = userEvent.setup();
    renderView();

    expect(
      screen.getByRole("switch", { name: /Recordar mi actividad/ }).closest(".switch"),
    ).toHaveClass("switch--wide");

    await openTab(user, "Firma");
    expect(
      screen
        .getByRole("switch", { name: /Recordar la última configuración de firma visible/ })
        .closest(".switch"),
    ).toHaveClass("switch--wide");
  });

  it("shows the destination folder by its name and never by its path", async () => {
    const user = userEvent.setup();
    renderView({ preferences: { ...defaults, destination: "Documentos" } });
    await openTab(user, "Firma");

    expect(screen.getByText("Dónde se guarda el documento firmado")).toBeInTheDocument();
    expect(screen.getByText("Documentos")).toBeInTheDocument();
    expect(screen.queryByText(/\/home\//)).not.toBeInTheDocument();
  });

  it("picks the destination folder with a directory picker and not with a dropdown", async () => {
    // El desplegable recibía una sola opción: un control que finge elegir
    // (ID-65). Lo que hay es un botón que abre el selector del sistema.
    const user = userEvent.setup();
    const onChooseDestination = vi.fn(async () => {});
    renderView({ onChooseDestination });
    await openTab(user, "Firma");

    expect(
      screen.queryByRole("combobox", { name: "Dónde se guarda el documento firmado" }),
    ).not.toBeInTheDocument();
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

  // «Junto al documento original» solo cuando el entorno sabe devolver la ruta
  // real del documento (ID-184): donde no la sabe, la opción no aparece y el
  // ajuste se queda en la carpeta con su «Cambiar carpeta…», como antes.
  it("offers Junto al documento original only when the environment allows it", async () => {
    const user = userEvent.setup();
    renderView({ preferences: { ...defaults, offersOriginalFolder: false } });
    await openTab(user, "Firma");

    expect(screen.queryByText("Junto al documento original")).not.toBeInTheDocument();
    expect(screen.queryByText("En esta carpeta")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();
  });

  // El destino lo decide el documento, no la persona (ADR-0011): las dos
  // frases son un estado que se enseña, no un control que finge elegir entre
  // ellas.
  it("shows the two destination states as text, never as a choice", async () => {
    const user = userEvent.setup();
    renderView({ preferences: { ...defaults, offersOriginalFolder: true } });
    await openTab(user, "Firma");

    expect(screen.queryByRole("radiogroup")).not.toBeInTheDocument();
    expect(screen.queryByRole("radio")).not.toBeInTheDocument();
    expect(screen.getByText("Junto al documento original")).toBeInTheDocument();
    expect(screen.getByText("En esta carpeta")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();
  });

  it("offers every language whose catalog is complete", async () => {
    const user = userEvent.setup();
    renderView();
    await openTab(user, "Apariencia");

    const language = screen.getByRole("combobox", { name: "Idioma" });
    expect(language).toHaveTextContent("Español");
    await user.click(language);

    const offered = screen.getAllByRole("option").map((option) => option.textContent);
    expect(offered).toEqual(["Español", "Català", "Euskara", "Galego", "English"]);
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
    expect(screen.getByText(/Al apagarlo se borra lo ya recordado/)).toBeInTheDocument();
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
  it("turns Avisarme cuando haya una versión nueva off without asking", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    renderView({ onChange });

    await user.click(
      screen.getByRole("switch", { name: "Avisarme cuando haya una versión nueva" }),
    );

    expect(onChange).toHaveBeenCalledWith({ ...defaults, notifyNewVersion: false });
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

  /**
   * El índice es el patrón ARIA de pestañas: solo el panel activo está en
   * pantalla, y se entra siempre en *General*, con *Privacidad* dentro como
   * grupo con su propio encabezado.
   */
  it("lays the settings out in sections with a permanent index to the left", async () => {
    const user = userEvent.setup();
    renderView();

    const index = screen.getByRole("navigation", { name: "Secciones" });
    expect(
      within(index)
        .getAllByRole("tab")
        .map((tab) => tab.textContent),
    ).toEqual(["General", "Firma", "Certificados", "Apariencia"]);

    const general = screen.getByRole("tabpanel", { name: "General" });
    const privacy = within(general).getByRole("group", { name: "Privacidad" });
    expect(
      within(privacy).getByRole("switch", { name: /Recordar mi actividad/ }),
    ).toBeInTheDocument();
    expect(within(privacy).getByRole("button", { name: "Vaciar la lista" })).toBeInTheDocument();
    expect(
      within(privacy).getByRole("switch", { name: /Avisarme cuando haya una versión nueva/ }),
    ).toBeInTheDocument();

    await openTab(user, "Firma");
    const signing = screen.getByRole("tabpanel", { name: "Firma" });
    expect(
      within(signing).getByRole("switch", {
        name: /Recordar la última configuración de firma visible/,
      }),
    ).toBeInTheDocument();
    expect(within(signing).getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();

    await openTab(user, "Apariencia");
    const appearance = screen.getByRole("tabpanel", { name: "Apariencia" });
    expect(within(appearance).getByRole("combobox", { name: "Tema" })).toBeInTheDocument();
    expect(within(appearance).getByRole("combobox", { name: "Idioma" })).toBeInTheDocument();
  });

  /** Solo el panel activo está en pantalla: no hay dos a la vez. */
  it("shows only the active panel, never two at once", async () => {
    const user = userEvent.setup();
    renderView();

    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    expect(screen.queryByRole("switch", { name: /Recordar la última/ })).not.toBeInTheDocument();

    await openTab(user, "Firma");

    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    expect(screen.queryByRole("switch", { name: /Recordar mi actividad/ })).not.toBeInTheDocument();
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

  /** Fijo: un botón de cierre que se va con el desplazamiento no está (ID-69). */
  it("keeps Cerrar in a footer outside the column that scrolls", () => {
    const { container } = renderView();

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
    renderView({ onChange });
    await openTab(user, "Apariencia");

    await user.click(screen.getByRole("combobox", { name: "Tema" }));
    await user.click(screen.getByRole("option", { name: "Oscuro" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("No hemos podido guardar el ajuste");
    expect(notice).toHaveTextContent("Hemos vuelto al valor anterior");
    expect(screen.getByRole("tabpanel", { name: "Apariencia" })).toContainElement(notice);
  });

  it("keeps the technical detail of the rejection in the notice", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn(async () => {
      throw new Error("EACCES: permission denied");
    });
    renderView({ onChange });
    await openTab(user, "Firma");

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
    renderView({ onForgetActivity });

    await user.click(screen.getByRole("button", { name: "Vaciar la lista" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("No hemos podido vaciar la lista");
    expect(notice).toHaveTextContent("siguen guardados");
    expect(screen.getByRole("group", { name: "Privacidad" })).toContainElement(notice);
  });

  it("says nothing when the setting is saved", async () => {
    const user = userEvent.setup();
    renderView();
    await openTab(user, "Firma");

    await user.click(screen.getByRole("switch", { name: /Recordar la última configuración/ }));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
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
    renderView({ onClose, onChange });

    await user.click(screen.getByRole("switch", { name: /Recordar mi actividad/ }));
    await user.keyboard("{Escape}");

    expect(screen.queryByText(/Al apagarlo se borra lo ya recordado/)).not.toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
    expect(onChange).not.toHaveBeenCalled();
  });

  /**
   * Certificados en fichero (docs/design/preferencias.md): una lista y dos
   * gestos, y ni una casilla por almacén ni un diálogo anidado (ID-198).
   */
  describe("certificates in a file", () => {
    it("lists an installed certificate by its holder and never by its file", async () => {
      const user = userEvent.setup();
      renderView({ installedCertificates: [anInstalledCertificate()] });
      await openTab(user, "Certificados");

      const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
      expect(within(certificates).getByText("Ada Lovelace Byron")).toBeInTheDocument();
      expect(
        within(certificates).getByText(/IDCES-00000000T · Emitido por FNMT-RCM · caduca el /),
      ).toBeInTheDocument();
      expect(certificates.textContent).not.toMatch(/[/\\]/);
    });

    it("offers the two gestures and nothing else", async () => {
      const user = userEvent.setup();
      renderView({ installedCertificates: [anInstalledCertificate()] });
      await openTab(user, "Certificados");

      const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
      expect(within(certificates).getByRole("button", { name: "Añadir…" })).toBeInTheDocument();
      expect(
        within(certificates).getByRole("button", {
          name: "Quitar el certificado de Ada Lovelace Byron",
        }),
      ).toBeInTheDocument();
      expect(within(certificates).queryAllByRole("checkbox")).toHaveLength(0);
      expect(within(certificates).queryAllByRole("switch")).toHaveLength(0);
    });

    it("says nothing is installed yet, without instructions inside the box", async () => {
      const user = userEvent.setup();
      renderView({ installedCertificates: [] });
      await openTab(user, "Certificados");

      const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
      expect(
        within(certificates).getByText("Todavía no has instalado ninguno"),
      ).toBeInTheDocument();
    });

    /** Un caducado se queda: que desaparezca no le explica nada a quien lo instaló. */
    it("keeps an expired certificate in the list, with its badge", async () => {
      const user = userEvent.setup();
      renderView({
        installedCertificates: [
          anInstalledCertificate({ status: { kind: "expired", notAfter: IN_2020 } }),
        ],
      });
      await openTab(user, "Certificados");

      const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
      expect(within(certificates).getByText("Ada Lovelace Byron")).toBeInTheDocument();
      expect(within(certificates).getByText("Caducado")).toBeInTheDocument();
    });

    it("asks for the password of the file and installs with it", async () => {
      const user = userEvent.setup();
      const onInstallCertificate = vi.fn(async () => true);
      renderView({ onInstallCertificate });
      await openTab(user, "Certificados");

      await user.click(screen.getByRole("button", { name: "Añadir…" }));
      await user.type(screen.getByLabelText("Contraseña"), "hunter2");
      await user.click(screen.getByRole("button", { name: "Continuar" }));

      expect(onInstallCertificate).toHaveBeenCalledWith("hunter2");
    });

    it("calls the password off with Escape, without closing the screen", async () => {
      const user = userEvent.setup();
      const onClose = vi.fn();
      const onInstallCertificate = vi.fn(async () => true);
      renderView({ onClose, onInstallCertificate });
      await openTab(user, "Certificados");

      await user.click(screen.getByRole("button", { name: "Añadir…" }));
      await user.keyboard("{Escape}");

      expect(screen.queryByLabelText("Contraseña")).not.toBeInTheDocument();
      expect(onInstallCertificate).not.toHaveBeenCalled();
      expect(onClose).not.toHaveBeenCalled();
    });

    /**
     * ID-197 + ID-211: se rechaza al instalar y en un solo renglón. Ni la
     * curva, ni el mecanismo, ni «instala uno de clave RSA».
     */
    it("says an elliptic key does not work, in a single line", async () => {
      const user = userEvent.setup();
      const onInstallCertificate = vi.fn(async () => {
        throw { situation: "keyNotRsa", detail: "FIRMA: la clave no es RSA" };
      });
      renderView({ onInstallCertificate });
      await openTab(user, "Certificados");

      await user.click(screen.getByRole("button", { name: "Añadir…" }));
      await user.click(screen.getByRole("button", { name: "Continuar" }));

      const notice = await screen.findByRole("alert");
      expect(notice).toHaveTextContent("Ese certificado no es compatible con rFirma");
      expect(notice.textContent).not.toMatch(/RSA|elíptica|curva/);
      expect(within(notice).queryByText("Detalle técnico")).not.toBeInTheDocument();
      expect(screen.getByRole("tabpanel", { name: "Certificados" })).toHaveTextContent(
        "Todavía no has instalado ninguno",
      );
    });

    /** Cerrar el selector sin elegir nada no es un fallo: no se cuenta nada. */
    it("says nothing when the file picker was closed without choosing anything", async () => {
      const user = userEvent.setup();
      renderView({ onInstallCertificate: async () => false });
      await openTab(user, "Certificados");

      await user.click(screen.getByRole("button", { name: "Añadir…" }));
      await user.click(screen.getByRole("button", { name: "Continuar" }));

      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    });

    it("removes an installed certificate by its handle", async () => {
      const user = userEvent.setup();
      const onRemoveCertificate = vi.fn(async () => {});
      renderView({
        installedCertificates: [anInstalledCertificate({ id: "2a01" })],
        onRemoveCertificate,
      });
      await openTab(user, "Certificados");

      await user.click(
        screen.getByRole("button", { name: "Quitar el certificado de Ada Lovelace Byron" }),
      );

      expect(onRemoveCertificate).toHaveBeenCalledWith("2a01");
    });

    it("shows in the section that the certificate could not be removed", async () => {
      const user = userEvent.setup();
      renderView({
        installedCertificates: [anInstalledCertificate()],
        onRemoveCertificate: async () => {
          throw { situation: "certificateNotFound", detail: "ya no esta" };
        },
      });
      await openTab(user, "Certificados");

      await user.click(
        screen.getByRole("button", { name: "Quitar el certificado de Ada Lovelace Byron" }),
      );

      const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
      expect(await within(certificates).findByRole("alert")).toBeInTheDocument();
    });
  });
});
