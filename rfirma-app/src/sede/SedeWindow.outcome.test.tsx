import { composeStories } from "@storybook/react-vite";
import { fireEvent, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { inMemoryExternalDestinationOpener } from "../desktop/externalDestination";
import { elapse } from "../testing/elapse";
import { renderWithCatalog } from "../testing/render";
import {
  type NAMED_BY_THE_DESK,
  OUTCOME_CLOSE_MS,
  type REFUSAL_ACTION_OF,
  type RefusalSituation,
} from "./errand";
import * as outcomeModule from "./SedeOutcome.stories";
import { SedeWindow } from "./SedeWindow";
import { scriptedErrand, scriptedFrom } from "./sedeWindowFixtures";

const RETRY = "Vuelve a la sede e inténtalo de nuevo.";
const CONTACT_SITE = "Contacta con la sede para terminar el trámite.";
const CLOSE_OTHER = "Cierra el otro trámite o la otra aplicación de firma y vuelve a intentarlo.";
const SHA1_CAUSE = "La sede ha pedido una firma con SHA-1, que ya no es segura.";
const INVOICE_MULTISIGNATURE_CAUSE =
  "La sede ha pedido añadir una segunda firma a una factura electrónica, que solo admite una.";
const UNSUPPORTED_COUNTERSIGNATURE_CAUSE =
  "La sede ha pedido firmar sobre la firma de otra persona en un tipo de documento que no lo permite.";
const OTHER_CERTIFICATE = "No se puede firmar con ese certificado. Vuelve a la sede y elige otro.";

const SITE_ACTION: Record<keyof typeof REFUSAL_ACTION_OF, string> = {
  appendedSignaturePage: CONTACT_SITE,
  unsupportedFilter: CONTACT_SITE,
  unsupportedProtocolVersion: CONTACT_SITE,
  missingFormat: CONTACT_SITE,
  unsupportedKeyStore: CONTACT_SITE,
  errandInFlight: CLOSE_OTHER,
  portsTaken: CLOSE_OTHER,
  sha1: SHA1_CAUSE,
  invoiceMultisignature: INVOICE_MULTISIGNATURE_CAUSE,
  unsupportedCountersignature: UNSUPPORTED_COUNTERSIGNATURE_CAUSE,
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

/** Grada A: el momento 4, el desenlace: qué se dice de cada rechazo y su cierre a los quince segundos. Lo que enseña cada historia está en `SedeWindow.presentation.test.tsx`. */

const { Signed, RefusedContactSite, RefusedCloseOther, RefusedUnknown } =
  composeStories(outcomeModule);

describe("4 · outcome", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

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

  it.each([
    {
      situation: "sha1",
      detail: "SAF_03: el algoritmo 'SHA1withRSA' es SHA-1: rFirma firma con SHA-2",
      cause: SHA1_CAUSE,
      note: "Pedid SHA256withRSA o superior.",
    },
    {
      situation: "invoiceMultisignature",
      detail: "SAF_04: FacturaE no admite cofirma ni contrafirma",
      cause: INVOICE_MULTISIGNATURE_CAUSE,
      note: "FacturaE no admite cofirma ni contrafirma; pedid una firma simple (sign).",
    },
    {
      situation: "unsupportedCountersignature",
      detail: "SAF_04: contrafirma fuera de CAdES, CMS y XAdES",
      cause: UNSUPPORTED_COUNTERSIGNATURE_CAUSE,
      note: "La contrafirma solo existe en CAdES, CMS y XAdES; en otro formato, pedid una cofirma (cosign).",
    },
  ] as const)(
    "says why rFirma refuses $situation instead of what to do, with a note for the site and the raw detail",
    ({ situation, detail, cause, note }) => {
      const { port } = scriptedErrand({
        kind: "outcome",
        outcome: { kind: "refused", situation, detail },
      });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(cause)).toBeInTheDocument();
      expect(screen.queryByText(CONTACT_SITE)).not.toBeInTheDocument();
      expect(screen.getByText(note)).toBeInTheDocument();
      expect(screen.getByText(detail)).toBeInTheDocument();
    },
  );

  it("keeps the note for the site inside the technical detail, after the SHA-1 hint", () => {
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "sha1", detail: "CRUDO" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    const cause = screen.getByText(SHA1_CAUSE);
    const hint = screen.getByText(/puedes permitir SHA-1 en Preferencias/);
    const detailLabel = screen.getByText("Detalle técnico para la sede");
    const note = screen.getByText("Pedid SHA256withRSA o superior.");
    expect(cause.compareDocumentPosition(hint)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(hint.compareDocumentPosition(detailLabel)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(detailLabel.compareDocumentPosition(note)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(screen.queryByText(/Para quien mantiene la sede/)).not.toBeInTheDocument();
  });

  it("copies the note for the site along with the raw detail", () => {
    const writeText = vi.fn(async () => {});
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
    const { port } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "sha1", detail: "CRUDO" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    fireEvent.click(screen.getByRole("button", { name: /Copiar/ }));

    expect(writeText).toHaveBeenCalledWith("Pedid SHA256withRSA o superior.\n\nCRUDO");
    vi.unstubAllGlobals();
  });

  it("opens discussions outside from the help link of an unknown refusal", () => {
    const destinations = inMemoryExternalDestinationOpener();
    const { port } = scriptedFrom(RefusedUnknown);
    renderWithCatalog(<SedeWindow errands={port} externalDestinations={destinations} />);

    fireEvent.click(screen.getByRole("button", { name: /Comentarios y ayuda/ }));

    expect(destinations.opened).toEqual(["discussions"]);
  });

  it.each([
    ["taken ports", RefusedCloseOther],
    ["an unknown refusal", RefusedUnknown],
  ])("does not close by itself on %s: the person has to act", async (_, story) => {
    const { port, calls } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} />);

    await elapse(OUTCOME_CLOSE_MS * 2);

    expect(calls.close).not.toHaveBeenCalled();
    expect(screen.queryByText(/se cerrará/i)).not.toBeInTheDocument();
  });

  it("does not close by itself while another errand is in flight", async () => {
    const { port, calls } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "errandInFlight", detail: "63131: en uso" },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await elapse(OUTCOME_CLOSE_MS * 2);

    expect(calls.close).not.toHaveBeenCalled();
  });

  it.each([
    ["a known refusal", RefusedContactSite],
    ["a signature", Signed],
  ])("closes by itself after fifteen seconds on %s, and not before", async (_, story) => {
    const { port, calls } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} />);

    await elapse(OUTCOME_CLOSE_MS - 1_000);
    expect(calls.close).not.toHaveBeenCalled();

    await elapse(1_000);
    expect(calls.close).toHaveBeenCalledOnce();
  });

  it.each([
    ["a signature", Signed],
    ["a refusal that stays open", RefusedCloseOther],
  ])("focuses Cerrar on %s, so Enter closes it", (_, story) => {
    const { port } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();
  });
});
