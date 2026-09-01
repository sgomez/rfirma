import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { PreferencesDialog } from "./PreferencesDialog";
import type { Preferences } from "./preferences";

const defaults: Preferences = {
  destination: "Documentos",
  rememberVisibleSignature: true,
  rememberActivity: true,
};

const noop = () => {};

function renderDialog(props: Partial<Parameters<typeof PreferencesDialog>[0]> = {}) {
  return renderWithCatalog(
    <PreferencesDialog
      preferences={defaults}
      destinations={["Documentos"]}
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
   * (`EstadoPreferencias.dc.html:409`). Un solo valor no puede ser los dos, y
   * arreglar uno rompía el otro: el diálogo pide el suyo, y por eso se
   * comprueba que lo pida.
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

    const destination = screen.getByLabelText("Dónde se guarda el documento firmado");
    expect(destination).toHaveValue("Documentos");
    expect(screen.queryByText(/\/home\//)).not.toBeInTheDocument();
  });

  it("offers only the languages whose catalog is complete", () => {
    renderDialog();

    const language = screen.getByLabelText("Idioma");
    expect(language).toHaveValue("es");
    const offered = Array.from(language.querySelectorAll("option")).map((option) => option.value);
    expect(offered).toEqual(["es", "en"]);
  });

  it("changes the language in place", async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.selectOptions(screen.getByLabelText("Idioma"), "en");

    expect(await screen.findByText("Preferences")).toBeInTheDocument();
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
    await user.click(screen.getByRole("button", { name: "Apagar y borrar" }));

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
});
