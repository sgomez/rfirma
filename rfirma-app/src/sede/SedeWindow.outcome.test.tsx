import { fireEvent, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { inMemoryExternalDestinationOpener } from "../desktop/externalDestination";
import { renderWithCatalog } from "../testing/render";
import {
  type NAMED_BY_THE_DESK,
  OUTCOME_CLOSE_MS,
  type REFUSAL_ACTION_OF,
  type RefusalSituation,
} from "./errand";
import { SedeWindow } from "./SedeWindow";
import { elapse, scriptedErrand, signedDocument } from "./sedeWindowFixtures";

const RETRY = "Vuelve a la sede e inténtalo de nuevo.";
const CONTACT_SITE = "Contacta con la sede para terminar el trámite.";
const CLOSE_OTHER = "Cierra el otro trámite o la otra aplicación de firma y vuelve a intentarlo.";
const OTHER_CERTIFICATE = "No se puede firmar con ese certificado. Vuelve a la sede y elige otro.";

const SITE_ACTION: Record<keyof typeof REFUSAL_ACTION_OF, string> = {
  appendedSignaturePage: CONTACT_SITE,
  unsupportedFilter: CONTACT_SITE,
  unsupportedProtocolVersion: CONTACT_SITE,
  missingFormat: CONTACT_SITE,
  unsupportedKeyStore: CONTACT_SITE,
  errandInFlight: CLOSE_OTHER,
  portsTaken: CLOSE_OTHER,
  sha1: CONTACT_SITE,
  explicitXades: CONTACT_SITE,
  invoiceMultisignature: CONTACT_SITE,
  unsupportedCountersignature: CONTACT_SITE,
  saveCancelled: RETRY,
  loadCancelled: RETRY,
  cannotSaveData: RETRY,
  cannotLoadData: RETRY,
  batchPresignerUnreachable: RETRY,
  batchPostsignerUnreachable: RETRY,
  batchInvalidPresignResponse: RETRY,
  batchInvalidPostsignResponse: RETRY,
  batchSigningFailed: RETRY,
  triphaseServerUrlMissing: CONTACT_SITE,
  triphaseServerException: RETRY,
  triphaseServerUnreachable: RETRY,
  triphaseServerUnexpectedAnswer: RETRY,
  certificateNotFound: OTHER_CERTIFICATE,
  folderMissing: RETRY,
  unwritable: RETRY,
  invalidSignature: CONTACT_SITE,
  confirmationNeeded: CONTACT_SITE,
  localBatchSign: CONTACT_SITE,
  siteErrandNotLive: RETRY,
  pdfHasUnregisteredSignatures: CONTACT_SITE,
  secretOnTheReaderKeypad: OTHER_CERTIFICATE,
  userCancelled: RETRY,
  promptFailed: RETRY,
  unknown: RETRY,
};

const DESK_TITLE: Record<(typeof NAMED_BY_THE_DESK)[number], string> = {
  incorrectPin: "El PIN no es correcto",
  pinLocked: "La tarjeta está bloqueada",
  tokenAbsent: "Falta la tarjeta o el certificado",
  expiredSession: "Algo ha fallado",
  moduleNotFound: "No se ha podido cargar el módulo de la tarjeta",
  pkcs12Unreadable: "Ese fichero no sirve como certificado",
  incorrectPkcs12Password: "La contraseña no es correcta",
  pkcs12NoPrivateKey: "Ese fichero no sirve como certificado",
  keyKindUnsupported: "Ese certificado no sirve para firmar",
  mechanismNotOffered: "Ese certificado no sirve para firmar",
  notAPdf: "Ese fichero no es un PDF",
  documentEncrypted: "El PDF está protegido",
  documentCertified: "Firmarlo invalidaría la certificación del autor",
  documentUnreadable: "No se ha podido leer el documento",
  boxOutOfPage: "La firma visible se sale del documento",
  pageOutOfDocument: "La firma visible se sale del documento",
  sealMismatch: "Algo ha fallado",
  bridgeFailed: "Algo ha fallado",
  notAFolder: "La carpeta de destino no está disponible",
  folderUnreadable: "La carpeta de destino no está disponible",
  folderUnwritable: "La carpeta de destino no está disponible",
  noFreeName: "Hay demasiados documentos con ese nombre",
  removalNotSupported: "No se puede quitar solo este certificado",
  noKeyring: "El llavero del escritorio no responde",
  keyringPinMissing: "El llavero ha perdido el PIN del Almacén",
};

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

    expect(screen.getByText("Firmado y enviado")).toBeInTheDocument();
    expect(
      screen.getByText("Las 3 firmas del lote ya están en sede.ejemplo.gob.es."),
    ).toBeInTheDocument();
  });

  it.each([
    ...Object.entries(SITE_ACTION).map(([situation, action]) => ({
      situation: situation as RefusalSituation,
      text: action,
    })),
    ...Object.entries(DESK_TITLE).map(([situation, title]) => ({
      situation: situation as RefusalSituation,
      text: title,
    })),
  ])("tells the $situation refusal in its sentence", ({ situation, text }) => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation, detail: "CRUDO" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(text)).toBeInTheDocument();
  });

  it("tells a refusal without naming the site, even when the request brings no origin", () => {
    const { port } = scriptedErrand(
      { kind: "outcome", outcome: { kind: "refused", situation: "sha1", detail: "CRUDO" } },
      { origin: null },
    );
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(CONTACT_SITE)).toBeInTheDocument();
    expect(screen.queryByText(/La petición/)).not.toBeInTheDocument();
  });

  it.each([
    {
      situation: "sha1",
      detail: "SAF_03: el algoritmo 'SHA1withRSA' es SHA-1: rFirma firma con SHA-2",
      cause: "La sede ha pedido una firma con SHA-1, que ya no es segura.",
      note: "Pedid SHA256withRSA o superior.",
    },
    {
      situation: "explicitXades",
      detail: "SAF_06: mode=explicit con XAdES (firma de la huella SHA-1)",
      cause:
        "La sede ha pedido un tipo de firma antiguo que podría hacerse pasar por la de otro documento.",
      note: "Quitad mode=explicit o usad CAdES explícita, que firma el documento sin incluirlo.",
    },
    {
      situation: "invoiceMultisignature",
      detail: "SAF_04: FacturaE no admite cofirma ni contrafirma",
      cause:
        "La sede ha pedido añadir una segunda firma a una factura electrónica, que solo admite una.",
      note: "FacturaE no admite cofirma ni contrafirma; pedid una firma simple (sign).",
    },
    {
      situation: "unsupportedCountersignature",
      detail: "SAF_04: contrafirma fuera de CAdES, CMS y XAdES",
      cause:
        "La sede ha pedido firmar sobre la firma de otra persona en un tipo de documento que no lo permite.",
      note: "La contrafirma solo existe en CAdES, CMS y XAdES; en otro formato, pedid una cofirma (cosign).",
    },
  ] as const)(
    "says why rFirma refuses $situation before what to do, with a note for the site and the raw detail",
    ({ situation, detail, cause, note }) => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "refused", situation, detail },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      const causeLine = screen.getByText(cause);
      const actionLine = screen.getByText(CONTACT_SITE);
      expect(causeLine.compareDocumentPosition(actionLine)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
      expect(screen.getByText(note)).toBeInTheDocument();
      expect(screen.getByText(detail)).toBeInTheDocument();
    },
  );

  it("adds neither a cause nor a note for the site to a refusal that has none", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "unsupportedFilter", detail: "CRUDO" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByText(/La sede ha pedido/)).not.toBeInTheDocument();
    expect(screen.queryByText(/pedid/)).not.toBeInTheDocument();
  });

  it("asks to close the other application on taken ports, under the common refusal title", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "portsTaken",
        detail: "63131: Address already in use (os error 98)",
      },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText("No se ha completado la petición")).toBeInTheDocument();
    expect(screen.getByText(CLOSE_OTHER)).toBeInTheDocument();
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

    expect(screen.getByText("No se ha completado la petición")).toBeInTheDocument();
    expect(screen.getByText(CONTACT_SITE)).toBeInTheDocument();
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

  it.each(["portsTaken", "errandInFlight"] as const)(
    "does not close by itself on %s: the person has to close the other one",
    async (situation) => {
      const { port, calls } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "refused", situation, detail: "63131: en uso" },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapse(OUTCOME_CLOSE_MS * 2);
      expect(calls.close).not.toHaveBeenCalled();
      expect(screen.queryByText(/se cerrará sola/i)).not.toBeInTheDocument();
    },
  );

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

  it("focuses Cerrar on a signature, so Enter closes without waiting", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "signed", document: signedDocument },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();
  });

  it("focuses Cerrar on a refusal that stays open, so Enter closes it", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "portsTaken", detail: "" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();
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
