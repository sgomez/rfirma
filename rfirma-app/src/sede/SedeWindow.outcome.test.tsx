import { fireEvent, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { inMemoryExternalDestinationOpener } from "../desktop/externalDestination";
import es from "../i18n/locales/es";
import { renderWithCatalog } from "../testing/render";
import { NAMED_BY_THE_DESK, OUTCOME_CLOSE_MS, type RefusalSituation } from "./errand";
import { SedeWindow } from "./SedeWindow";
import { elapse, scriptedErrand, signedDocument } from "./sedeWindowFixtures";

/** Grada A: el momento 4, el desenlace, y su cierre a los quince segundos (TD-63). */

describe("4 · outcome", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("still shows what was signed: the outcome is where you check it was that document", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "signed", document: signedDocument },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Solicitud de subvención 2026")).toBeInTheDocument();
    expect(screen.getByText("27 páginas · 2,4 MB")).toBeInTheDocument();
  });

  it("shows no document in a refusal, because there never was one", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "missingFormat", detail: "format=" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByText("Solicitud de subvención 2026")).not.toBeInTheDocument();
  });

  it("says rFirma keeps no copy, which is the one thing you cannot deduce", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "signed", document: signedDocument },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Firmado y enviado")).toBeInTheDocument();
    expect(screen.getByText("rFirma no guarda copia.")).toBeInTheDocument();
  });

  it("confirms a plain save with no document row: the person just chose where", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "saved" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Guardado")).toBeInTheDocument();
    expect(screen.queryByText("Solicitud de subvención 2026")).not.toBeInTheDocument();
  });

  it("says how many files were delivered when the load ends there", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "loaded", fileCount: 2 },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Cargado")).toBeInTheDocument();
    expect(
      screen.getByText("Se han enviado 2 ficheros a sede.ejemplo.gob.es."),
    ).toBeInTheDocument();
  });

  it("confirms the batch is at the site, with how many signatures it carried", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "batchSigned", signs: 3 },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Lote firmado y enviado")).toBeInTheDocument();
    expect(
      screen.getByText("Las 3 firmas del lote ya están en sede.ejemplo.gob.es."),
    ).toBeInTheDocument();
  });

  it("gives a batch refusal its own phrase and leaves the raw detail copiable", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "batchPresignerUnreachable",
        detail: "presigner: connection refused",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(
      screen.getByText("El servicio de sede.ejemplo.gob.es que prepara el lote no ha contestado."),
    ).toBeInTheDocument();
    expect(screen.getByText("presigner: connection refused")).toBeInTheDocument();
  });

  it.each([
    [
      "triphaseServerUrlMissing",
      "sede.ejemplo.gob.es ha pedido firmar con su servicio de firma, pero no ha dicho dónde está.",
    ],
    ["triphaseServerException", "El servicio de firma de sede.ejemplo.gob.es ha dado un error."],
    ["triphaseServerUnreachable", "El servicio de firma de sede.ejemplo.gob.es no ha contestado."],
    [
      "triphaseServerUnexpectedAnswer",
      "El servicio de firma de sede.ejemplo.gob.es ha contestado algo que rFirma no sabe leer.",
    ],
  ] as const)("gives the triphase server refusal %s its own phrase", (situation, phrase) => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation, detail: "CRUDO" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(phrase)).toBeInTheDocument();
  });

  it.each([
    ...Object.entries(es.sede.refusals).map(([situation, phrase]) => ({
      situation: situation as RefusalSituation,
      text: phrase.replaceAll("{{origin}}", "sede.ejemplo.gob.es"),
    })),
    ...NAMED_BY_THE_DESK.map((situation) => ({
      situation,
      text: es.errors.situations[situation].title,
    })),
  ])("tells the $situation refusal with its own sentence", ({ situation, text }) => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation, detail: "CRUDO" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(text)).toBeInTheDocument();
  });

  it("classifies a cancelled save as its own refusal, with its own phrase", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "saveCancelled",
        detail: "el dialogo de guardado se cerro sin elegir nada",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(
      screen.getByText(
        "Has cerrado el diálogo de guardado sin elegir dónde guardar el fichero que pedía sede.ejemplo.gob.es.",
      ),
    ).toBeInTheDocument();
  });

  it.each([
    {
      situation: "explicitXades",
      detail: "SAF_06: mode=explicit con XAdES (firma de la huella SHA-1)",
      sentence:
        "sede.ejemplo.gob.es ha pedido un tipo de firma antiguo que ya no es seguro: esa firma podría hacerse pasar por la de otro documento distinto del tuyo. Por eso rFirma no la hace. No es un fallo tuyo; si necesitas terminar el trámite, contacta con sede.ejemplo.gob.es.",
      note: "Quitad mode=explicit o usad CAdES explícita, que firma el documento sin incluirlo.",
    },
    {
      situation: "invoiceMultisignature",
      detail: "SAF_04: FacturaE no admite cofirma ni contrafirma",
      sentence:
        "sede.ejemplo.gob.es ha pedido añadir una firma a una factura electrónica que ya está firmada. Una factura electrónica solo admite una firma, así que rFirma no puede hacerlo. No es un fallo tuyo; si necesitas terminar el trámite, contacta con sede.ejemplo.gob.es.",
      note: "FacturaE no admite cofirma ni contrafirma; pedid una firma simple (sign).",
    },
    {
      situation: "unsupportedCountersignature",
      detail: "SAF_04: contrafirma fuera de CAdES, CMS y XAdES",
      sentence:
        "sede.ejemplo.gob.es ha pedido firmar sobre la firma de otra persona en un tipo de documento que no lo permite, así que rFirma no puede hacerlo. No es un fallo tuyo; si necesitas terminar el trámite, contacta con sede.ejemplo.gob.es.",
      note: "La contrafirma solo existe en CAdES, CMS y XAdES; en otro formato, pedid una cofirma (cosign).",
    },
  ] as const)(
    "explains why rFirma refuses $situation, with a note for the site and the raw detail",
    ({ situation, detail, sentence, note }) => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "refused", situation, detail },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(sentence)).toBeInTheDocument();
      expect(screen.getByText(note)).toBeInTheDocument();
      expect(screen.getByText(detail)).toBeInTheDocument();
    },
  );

  it("adds no note for the site to a refusal that has none", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "unsupportedFilter", detail: "CRUDO" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByText(/pedid/)).not.toBeInTheDocument();
  });

  it("says another application holds the ports instead of blaming the browser", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "portsTaken",
        detail: "63131: Address already in use (os error 98)",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(
      screen.getByText(/ofrece unos puertos que otra aplicación ya está usando/),
    ).toBeInTheDocument();
    expect(screen.getByText("63131: Address already in use (os error 98)")).toBeInTheDocument();
    expect(screen.queryByText("La petición no ha llegado")).not.toBeInTheDocument();
  });

  it("adds nothing to a cancellation: the title already says it", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "cancelled", document: signedDocument },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("Has cancelado la firma")).toBeInTheDocument();
    expect(screen.queryByText(/no se ha firmado nada/i)).not.toBeInTheDocument();
  });

  it("states a refusal without blaming anyone, and leaves the raw detail copiable", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "appendedSignaturePage",
        detail: "signaturePages=append",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("rFirma ha rechazado la petición")).toBeInTheDocument();
    expect(
      screen.getByText(
        "sede.ejemplo.gob.es pide colocar la firma en una página añadida al final, y rFirma no hace eso.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("signaturePages=append")).toBeInTheDocument();
    expect(screen.queryByText(/el fallo es de/i)).not.toBeInTheDocument();
  });

  it("shows the help link on unknown refusal and opens discussions outside", async () => {
    const destinations = inMemoryExternalDestinationOpener();
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "unknown",
        detail: "error inesperado del transporte",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} externalDestinations={destinations} />);

    const helpButton = screen.getByRole("button", { name: /Comentarios y ayuda/ });
    expect(helpButton).toBeInTheDocument();

    fireEvent.click(helpButton);
    expect(destinations.opened).toEqual(["discussions"]);
  });

  it("does not show the help link on known refusals", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "appendedSignaturePage",
        detail: "signaturePages=append",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByRole("button", { name: /Comentarios y ayuda/ })).not.toBeInTheDocument();
  });

  it("titles taken ports as a channel that could not open, not as a refusal", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "portsTaken", detail: "63131: en uso" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(
      screen.getByText("rFirma no ha podido abrir el canal con la página"),
    ).toBeInTheDocument();
    expect(screen.queryByText("rFirma ha rechazado la petición")).not.toBeInTheDocument();
  });

  it("does not close by itself on taken ports: the person has to close the other application", async () => {
    const { port, calls } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "portsTaken", detail: "63131: en uso" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await elapse(OUTCOME_CLOSE_MS * 2);
    expect(calls.close).not.toHaveBeenCalled();
    expect(screen.queryByText(/se cerrará sola/i)).not.toBeInTheDocument();
  });

  it("does not close by itself on unknown refusal", async () => {
    const { port, calls } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "unknown",
        detail: "error desconocido",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await elapse(OUTCOME_CLOSE_MS * 2);
    expect(calls.close).not.toHaveBeenCalled();
    expect(screen.queryByText(/se cerrará en/i)).not.toBeInTheDocument();
  });

  it("closes by itself after fifteen seconds on known refusals, and not before", async () => {
    const { port, calls } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "appendedSignaturePage",
        detail: "signaturePages=append",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await elapse(OUTCOME_CLOSE_MS - 1_000);
    expect(calls.close).not.toHaveBeenCalled();

    await elapse(1_000);
    expect(calls.close).toHaveBeenCalledOnce();
  });

  it("closes by itself after fifteen seconds, and not before", async () => {
    const { port, calls } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "signed", document: signedDocument },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await elapse(OUTCOME_CLOSE_MS - 1_000);
    expect(calls.close).not.toHaveBeenCalled();

    await elapse(1_000);
    expect(calls.close).toHaveBeenCalledOnce();
  });
});
