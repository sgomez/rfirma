import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { inMemoryExternalDestinationOpener } from "../desktop/externalDestination";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import type { LanguageTag } from "../i18n/languages";
import { inMemoryLanguagePreference } from "../i18n/preference";
import { ErrorNotice } from "./ErrorNotice";

/** **Grada A** (`vitest`, carril rápido). ID-29 y ADR-0009. */

function renderIn(language: LanguageTag, children: ReactNode) {
  const preference = inMemoryLanguagePreference(language);
  return render(
    <LanguageProvider i18n={createI18n(language)} preference={preference}>
      {children}
    </LanguageProvider>,
  );
}

/**
 * El texto original tal y como sale del token: un código PKCS#11 que además
 * **parece una clave de traducción**, para que se vea si alguien lo pasa por
 * `t()` alguna vez.
 */
const RAW_DETAIL =
  "CKR_PIN_INCORRECT (0x000000A0) errors.messages.retry.title " +
  "es.gob.afirma.core.AOException: Error en la postfirma PAdES del documento " +
  "adjunto, con un mensaje incrustado en el código y sin ningún .properties " +
  "localizado detrás del que tirar para enseñarlo en otro idioma.";

describe("el aviso de error", () => {
  it("enseña la situación traducida, y no el texto del token", () => {
    renderIn("es", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    expect(screen.getByRole("alert")).toHaveTextContent("Algo ha fallado");
  });

  it("traduce la situación al idioma de la aplicación", () => {
    renderIn("en", <ErrorNotice situation="incorrectPin" technicalDetail={RAW_DETAIL} />);

    expect(screen.getByRole("alert")).toHaveTextContent("The PIN isn't correct");
  });

  it("enseña el texto original crudo: ni traducido ni recortado", () => {
    renderIn("en", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    // `getByText` compara el textContent entero del nodo, así que esto falla
    // tanto si se ha traducido algo como si se ha recortado el final.
    const raw = screen.getByText(RAW_DETAIL);

    expect(raw.textContent).toBe(RAW_DETAIL);
    // La parte del texto que es una clave de traducción sigue siendo texto.
    expect(raw.textContent).toContain("errors.messages.retry.title");
  });

  it("no toca el texto original al cambiar de idioma", () => {
    const spanish = renderIn(
      "es",
      <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />,
    );
    const inSpanish = screen.getByText(RAW_DETAIL).textContent;
    spanish.unmount();
    renderIn("en", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    expect(screen.getByText(RAW_DETAIL).textContent).toBe(inSpanish);
  });

  it("deja el detalle técnico plegado, y no como mensaje", () => {
    renderIn("es", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    const details = screen.getByText(RAW_DETAIL).closest("details");

    expect(details).not.toBeNull();
    expect(details?.open).toBe(false);
    expect(details).toHaveTextContent("Detalle técnico");
  });

  /**
   * Error al firmar (docs/design/panel-de-firma.md § Estados → Error al
   * firmar): el título es siempre el fijo, y la situación clasificada baja a
   * ser la causa, en orden: título, causa, tranquilidad, detalle y «Copiar».
   */
  it("shows the fixed signing-failed title with the situation as the cause", () => {
    renderIn(
      "es",
      <ErrorNotice situation="tokenAbsent" technicalDetail={RAW_DETAIL} documentUnchanged />,
    );

    const alert = screen.getByRole("alert");
    const text = alert.textContent ?? "";
    expect(text.indexOf("No se ha podido firmar")).toBeLessThan(
      text.indexOf("Falta la tarjeta o el certificado"),
    );
    expect(text.indexOf("Falta la tarjeta o el certificado")).toBeLessThan(
      text.indexOf("El documento sigue como estaba"),
    );
    expect(screen.getByRole("button", { name: "Copiar detalle" })).toBeInTheDocument();
  });

  it("leaves the vague cause out of a signing failure it cannot explain", () => {
    renderIn(
      "es",
      <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} documentUnchanged />,
    );

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("No se ha podido firmar");
    expect(alert).not.toHaveTextContent("Algo ha fallado");
    expect(alert).toHaveTextContent("El documento sigue como estaba");
  });

  it("tells an expired certificate with its own title and remedy", () => {
    renderIn("es", <ErrorNotice situation="certificateExpired" technicalDetail={RAW_DETAIL} />);

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Tu certificado ha caducado");
    expect(alert).toHaveTextContent("Renuévalo con su emisora.");
  });

  it("tells a revoked certificate in one line and keeps its technical detail", () => {
    renderIn("es", <ErrorNotice situation="certificateRevoked" technicalDetail={RAW_DETAIL} />);

    const alert = screen.getByRole("alert");
    expect(alert.querySelectorAll("p.rf-prose")).toHaveLength(0);
    expect(alert).toHaveTextContent("Ese certificado no sirve para firmar");
    expect(screen.getByText(RAW_DETAIL)).toBeInTheDocument();
  });

  it("copies the raw technical detail to the clipboard", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    renderIn(
      "es",
      <ErrorNotice situation="tokenAbsent" technicalDetail={RAW_DETAIL} documentUnchanged />,
    );

    await user.click(screen.getByRole("button", { name: "Copiar detalle" }));

    expect(writeText).toHaveBeenCalledWith(RAW_DETAIL);
  });

  it("does not show the fixed title or the copy button outside signing failures", () => {
    renderIn("es", <ErrorNotice situation="tokenAbsent" technicalDetail={RAW_DETAIL} />);

    expect(screen.queryByText("No se ha podido firmar")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Copiar detalle" })).not.toBeInTheDocument();
  });

  it("leaves the remedy and the help link out of a signing failure", () => {
    renderIn(
      "es",
      <ErrorNotice situation="tokenAbsent" technicalDetail={RAW_DETAIL} documentUnchanged />,
    );
    expect(screen.queryByText(/Comprueba la tarjeta y el lector/)).not.toBeInTheDocument();
  });

  it("does not offer help or reload on a signing failure", () => {
    renderIn(
      "es",
      <ErrorNotice
        situation="unknown"
        technicalDetail={RAW_DETAIL}
        onReload={() => {}}
        documentUnchanged
      />,
    );
    expect(screen.queryByRole("button", { name: /Comentarios y ayuda/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Recargar/ })).not.toBeInTheDocument();
  });

  it.each([
    "bridgeFailed",
    "sealMismatch",
    "unknown",
    "renderFailed",
    "settingNotSaved",
    "expiredSession",
  ] as const)("enseña el enlace a Comentarios y ayuda en la situación %s", (situation) => {
    renderIn("es", <ErrorNotice situation={situation} technicalDetail={RAW_DETAIL} />);

    expect(screen.getByRole("button", { name: /Comentarios y ayuda/ })).toBeInTheDocument();
  });

  it.each([
    "incorrectPin",
    "tokenAbsent",
    "certificateExpired",
    "moduleNotFound",
    "keyKindUnsupported",
  ] as const)("no enseña el enlace a Comentarios y ayuda en la situación ajena %s", (situation) => {
    renderIn("es", <ErrorNotice situation={situation} technicalDetail={RAW_DETAIL} />);

    expect(screen.queryByRole("button", { name: /Comentarios y ayuda/ })).not.toBeInTheDocument();
  });

  it("abre el destino discussions al pulsar el enlace de ayuda", async () => {
    const user = userEvent.setup();
    const externalDestinations = inMemoryExternalDestinationOpener();
    renderIn(
      "es",
      <ErrorNotice
        situation="unknown"
        technicalDetail={RAW_DETAIL}
        externalDestinations={externalDestinations}
      />,
    );

    await user.click(screen.getByRole("button", { name: /Comentarios y ayuda/ }));

    expect(externalDestinations.opened).toEqual(["discussions"]);
  });

  it("traduce el enlace de ayuda al idioma de la ventana", () => {
    renderIn("en", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    expect(screen.getByRole("button", { name: /Feedback and help/ })).toBeInTheDocument();
  });

  it("no enseña el botón de recargar si nadie lo pide", () => {
    renderIn("es", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    expect(screen.queryByRole("button", { name: /Recargar/ })).not.toBeInTheDocument();
  });

  it("recarga al pulsar el botón, cuando se pide", async () => {
    const user = userEvent.setup();
    const onReload = vi.fn();
    renderIn(
      "es",
      <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} onReload={onReload} />,
    );

    await user.click(screen.getByRole("button", { name: /Recargar/ }));

    expect(onReload).toHaveBeenCalledOnce();
  });

  it("offers to empty the store only with a lost keyring pin", () => {
    renderIn("es", <ErrorNotice situation="unknown" onEmptyStore={vi.fn()} />);

    expect(screen.queryByRole("button", { name: /Vaciar el almacén/ })).not.toBeInTheDocument();
  });

  it("does not offer to empty the store when nobody wired it up", () => {
    renderIn("es", <ErrorNotice situation="keyringPinMissing" />);

    expect(screen.queryByRole("button", { name: /Vaciar el almacén/ })).not.toBeInTheDocument();
  });

  it("asks to confirm before emptying the store", async () => {
    const user = userEvent.setup();
    const onEmptyStore = vi.fn();
    renderIn("es", <ErrorNotice situation="keyringPinMissing" onEmptyStore={onEmptyStore} />);

    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));

    expect(onEmptyStore).not.toHaveBeenCalled();
    expect(screen.getByText(/Se perderán los certificados instalados/)).toBeInTheDocument();
  });

  it("empties the store only after the confirmation", async () => {
    const user = userEvent.setup();
    const onEmptyStore = vi.fn();
    renderIn("es", <ErrorNotice situation="keyringPinMissing" onEmptyStore={onEmptyStore} />);

    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));
    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));

    expect(onEmptyStore).toHaveBeenCalledOnce();
  });

  it("cancelling the confirmation leaves the store alone", async () => {
    const user = userEvent.setup();
    const onEmptyStore = vi.fn();
    renderIn("es", <ErrorNotice situation="keyringPinMissing" onEmptyStore={onEmptyStore} />);

    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));
    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onEmptyStore).not.toHaveBeenCalled();
    expect(screen.queryByText(/¿Seguro\? Se perderán/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Vaciar el almacén" })).toBeInTheDocument();
  });

  /**
   * Firmar con un certificado instalado también pasa por aquí (criterio 3 del
   * #1062): sin esta excepción a la tarjeta fija de «Error al firmar», quien
   * firma no tenía forma de alcanzar el botón sin volver a Preferencias.
   */
  it("offers to empty the store from a signing failure too", async () => {
    const user = userEvent.setup();
    const onEmptyStore = vi.fn();
    renderIn(
      "es",
      <ErrorNotice situation="keyringPinMissing" onEmptyStore={onEmptyStore} documentUnchanged />,
    );

    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));
    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));

    expect(onEmptyStore).toHaveBeenCalledOnce();
  });

  it("empties the store with Enter while the confirmation is open, with its primary focused", async () => {
    const user = userEvent.setup();
    const onEmptyStore = vi.fn();
    renderIn("es", <ErrorNotice situation="keyringPinMissing" onEmptyStore={onEmptyStore} />);

    await user.keyboard("{Enter}");
    expect(onEmptyStore).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));
    expect(screen.getByRole("button", { name: "Vaciar el almacén" })).toHaveFocus();
    (document.activeElement as HTMLElement).blur();
    await user.keyboard("{Enter}");

    expect(onEmptyStore).toHaveBeenCalledOnce();
  });

  it("cancels the confirmation with Escape, and ignores Escape once it is closed", async () => {
    const user = userEvent.setup();
    const onEmptyStore = vi.fn();
    renderIn("es", <ErrorNotice situation="keyringPinMissing" onEmptyStore={onEmptyStore} />);
    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));

    await user.keyboard("{Escape}");

    expect(screen.queryByText(/¿Seguro\? Se perderán/)).not.toBeInTheDocument();
    await user.keyboard("{Escape}");
    await user.keyboard("{Enter}");
    expect(onEmptyStore).not.toHaveBeenCalled();
  });
});
