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
  "CKR_PIN_INCORRECT (0x000000A0) errors.situations.unknown.title " +
  "es.gob.afirma.core.AOException: Error en la postfirma PAdES del documento " +
  "adjunto, con un mensaje incrustado en el código y sin ningún .properties " +
  "localizado detrás del que tirar para enseñarlo en otro idioma.";

describe("el aviso de error", () => {
  it("enseña la situación traducida, y no el texto del token", () => {
    renderIn("es", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    expect(screen.getByRole("alert")).toHaveTextContent("No se ha podido completar la operación");
  });

  it("traduce la situación al idioma de la aplicación", () => {
    renderIn("en", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    expect(screen.getByRole("alert")).toHaveTextContent("We couldn't complete the operation");
  });

  it("enseña el texto original crudo: ni traducido ni recortado", () => {
    renderIn("en", <ErrorNotice situation="unknown" technicalDetail={RAW_DETAIL} />);

    // `getByText` compara el textContent entero del nodo, así que esto falla
    // tanto si se ha traducido algo como si se ha recortado el final.
    const raw = screen.getByText(RAW_DETAIL);

    expect(raw.textContent).toBe(RAW_DETAIL);
    // La parte del texto que es una clave de traducción sigue siendo texto.
    expect(raw.textContent).toContain("errors.situations.unknown.title");
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
  it.each(["bridgeFailed", "sealMismatch", "unknown", "renderFailed"] as const)(
    "enseña el enlace a Comentarios y ayuda en la situación %s",
    (situation) => {
      renderIn("es", <ErrorNotice situation={situation} technicalDetail={RAW_DETAIL} />);

      expect(screen.getByRole("button", { name: /Comentarios y ayuda/ })).toBeInTheDocument();
    },
  );

  it.each([
    "incorrectPin",
    "tokenAbsent",
    "certificateExpired",
    "moduleNotFound",
    "keyNotRsa",
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
});
