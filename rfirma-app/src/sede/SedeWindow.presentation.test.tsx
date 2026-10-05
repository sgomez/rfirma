import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import type { ComponentType } from "react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { CHROME_LOCAL_NETWORK_SETTINGS } from "./errand";
import * as confirmModule from "./SedeConfirm.stories";
import * as consentModule from "./SedeConsent.stories";
import * as markingModule from "./SedeMarking.stories";
import * as noCertificateModule from "./SedeNoCertificate.stories";
import * as outcomeModule from "./SedeOutcome.stories";
import * as signingModule from "./SedeSigning.stories";
import * as waitingModule from "./SedeWaiting.stories";

/** Presentación: lo que enseña cada historia de la ventana de sede, una fila por historia. */

type Matcher = string | RegExp;

interface Expectation {
  shows?: Matcher[];
  hides?: Matcher[];
  buttons?: Matcher[];
  noButtons?: Matcher[];
  disabled?: Matcher[];
  noButtonAtAll?: true;
}

const waiting = composeStories(waitingModule);
const consent = composeStories(consentModule);
const confirm = composeStories(confirmModule);
const signing = composeStories(signingModule);
const marking = composeStories(markingModule);
const outcome = composeStories(outcomeModule);
const noCertificate = composeStories(noCertificateModule);

const DOCUMENT_TITLE = "Solicitud de subvención 2026";
const CONTACT_SITE = "Contacta con la sede para terminar el trámite.";
const CLOSE_OTHER = "Cierra el otro trámite o la otra aplicación de firma y vuelve a intentarlo.";
const NO_COMMENTS_HELP = /Comentarios y ayuda/;
const INSTALL = "Instalar un certificado…";
const NO_CERTIFICATE_BUTTONS = [INSTALL, "Volver a buscar", "Cerrar"];

const rows: [string, ComponentType, Expectation][] = [
  [
    "1 · old web client",
    waiting.OldWebClient,
    {
      shows: ["Esta página está desactualizada", /pulsa continuar para seguir/i],
      buttons: ["Continuar"],
    },
  ],
  ["1 · waiting", waiting.Waiting, { shows: ["Conectando con la sede"] }],
  [
    "1 · unreachable",
    waiting.Unreachable,
    {
      shows: [
        "La petición no ha llegado",
        /franja bajo la barra de direcciones/,
        /vuelve a la sede y pulsa Reintentar/,
      ],
      buttons: ["Chrome", "Firefox", "Instalar"],
      noButtons: ["Reintentar"],
    },
  ],
  [
    "1b · no channel",
    waiting.NoChannel,
    {
      shows: ["La petición no ha llegado", CHROME_LOCAL_NETWORK_SETTINGS],
      buttons: [/Copiar/],
    },
  ],
  [
    "2 · consent to a PDF",
    consent.Consent,
    {
      shows: [
        "sede.ejemplo.gob.es pide tu firma de un documento PDF.",
        DOCUMENT_TITLE,
        "27 páginas · 2,4 MB",
      ],
      buttons: ["Firmar"],
    },
  ],
  [
    "2 · challenge",
    consent.SignChallenge,
    { shows: ["sede.ejemplo.gob.es pide tu firma de un reto de autenticación."] },
  ],
  [
    "2 · XML",
    consent.SignXml,
    { shows: ["sede.ejemplo.gob.es pide tu firma de un documento XML."] },
  ],
  [
    "2 · invoice",
    consent.SignInvoice,
    { shows: ["sede.ejemplo.gob.es pide tu firma de una factura electrónica."] },
  ],
  ["2 · untitled PDF", consent.UntitledDocument, { shows: ["Un PDF sin título"] }],
  [
    "2 · narrowed by the site",
    consent.NarrowedBySite,
    {
      shows: ["sede.ejemplo.gob.es ha limitado los certificados válidos."],
      hides: [/criterio|descartad/i],
    },
  ],
  [
    "2 · SHA-1 allowed",
    consent.Sha1Allowed,
    {
      shows: ["Esta sede pide SHA-1, un algoritmo obsoleto. Lo tienes permitido en Preferencias."],
    },
  ],
  [
    "2 · cosign",
    consent.Cosign,
    { shows: ["Ya viene firmado: la tuya será una cofirma junto a las firmas que ya tiene."] },
  ],
  [
    "2 · cosign over two previous signatures, counted only by the notice",
    consent.PreviousSignaturesWithProblem,
    { shows: [/Junto a 2 firmas/], hides: [/firmas? anteriores?/] },
  ],
  [
    "2 · countersign over the tree",
    consent.CountersignTree,
    {
      shows: [
        "Ya viene firmado: la tuya será una contrafirma sobre todas las firmas que ya tiene.",
      ],
      hides: [/cofirma/],
    },
  ],
  [
    "2 · countersign over the leafs",
    consent.CountersignLeafs,
    {
      shows: ["Ya viene firmado: la tuya será una contrafirma sobre las últimas firmas que tiene."],
      hides: [/cofirma/],
    },
  ],
  [
    "2 · previous signature by the same certificate",
    consent.PreviousSignatureBySameCertificate,
    { shows: ["Ya lo firmaste tú con este certificado"] },
  ],
  [
    "2 · batch",
    consent.Batch,
    {
      shows: [
        "Lote de 3 firmas",
        "Los documentos se quedan en la sede: rFirma firma sin descargarlos.",
      ],
    },
  ],
  [
    "2 · local batch",
    consent.LocalBatch,
    {
      shows: [
        "Lote de 5 firmas",
        "001 — un documento PDF (firma)",
        "002 — un reto de autenticación (cofirma)",
        "003 — un documento XML (firma)",
        "004 — una factura electrónica (contrafirma de todas las firmas)",
        "005 — un documento PDF (contrafirma de las últimas firmas)",
      ],
    },
  ],
  [
    "2 · identity data",
    consent.IdentityData,
    {
      shows: ["sede.ejemplo.gob.es pide tus datos de identidad.", /Se enviarán tu nombre, tu NIF/],
      buttons: ["Enviar mis datos"],
      noButtons: ["Firmar", "Identificarse"],
    },
  ],
  [
    "2 · identity data without origin",
    consent.IdentityDataWithoutOrigin,
    { shows: ["Una página sin identificar pide tus datos de identidad."] },
  ],
  [
    "2 · without origin",
    consent.WithoutOrigin,
    { shows: ["Una página sin identificar pide tu firma."] },
  ],
  [
    "2 · terminal order",
    consent.TerminalOrder,
    {
      hides: [/pide tu firma/, /sede/i, /Solicitud de subvención/, /Junto a \d+ firmas?/],
      buttons: [/^Firmar/],
    },
  ],
  [
    "2 · terminal order with previous signatures",
    consent.TerminalOrderWithPreviousSignatures,
    { shows: ["Junto a 1 firma"] },
  ],
  [
    "2b · shadow attack suspect",
    confirm.ShadowAttackSuspect,
    {
      shows: [/se ha modificado después de la última firma/i],
      buttons: ["Continuar", "Cancelar"],
    },
  ],
  [
    "2b · modified form",
    confirm.ModifiedForm,
    { shows: [/formulario cuyos campos se han cambiado después de firmarlo/i] },
  ],
  [
    "2b · certified PDF",
    confirm.CertifiedPdf,
    { shows: [/está certificado y no admite nuevas firmas/i] },
  ],
  ["2b · unknown message", confirm.UnknownMessage, { shows: [/unknownValidatorMessage/] }],
  [
    "1c · unreadable document",
    marking.UnreadableDocument,
    { shows: [/No se ha podido abrir el documento/], disabled: ["Continuar"] },
  ],
  [
    "3 · signing",
    signing.Signing,
    {
      shows: ["Firmando…", "Con ADA LOVELACE BYRON · 99999999R."],
      hides: [/prefirma|posfirma/i],
    },
  ],
  [
    "3 · returning",
    signing.Returning,
    { shows: ["Enviando la firma a sede.ejemplo.gob.es"], noButtons: ["Cancelar"] },
  ],
  [
    "3 · saving a named file",
    signing.SavingNamedFile,
    { shows: ["Guardando informe.pdf"], hides: [/\//] },
  ],
  ["3 · saving an unnamed file", signing.SavingUnnamedFile, { shows: ["Guardando el fichero"] }],
  [
    "3 · saving into an unwritable destination",
    signing.SavingUnwritableDestination,
    {
      shows: [
        "No se ha podido guardar en el destino elegido. Elige otro en el diálogo del sistema.",
      ],
      noButtonAtAll: true,
    },
  ],
  [
    "3 · loading one file",
    signing.LoadingOneFile,
    { shows: ["Cargando un fichero"], noButtonAtAll: true },
  ],
  [
    "3 · loading several files",
    signing.LoadingSeveralFiles,
    { shows: ["Cargando varios ficheros"], noButtonAtAll: true },
  ],
  [
    "4 · signed",
    outcome.Signed,
    {
      shows: [
        "Firmado y enviado",
        "rFirma no guarda copia.",
        DOCUMENT_TITLE,
        "27 páginas · 2,4 MB",
      ],
    },
  ],
  [
    "4 · signed without document",
    outcome.SignedWithoutDocument,
    { shows: ["Firmado y enviado"], hides: [DOCUMENT_TITLE] },
  ],
  [
    "4 · cancelled",
    outcome.Cancelled,
    { shows: ["Has cancelado la firma"], hides: [/no se ha firmado nada/i] },
  ],
  [
    "4 · batch signed",
    outcome.BatchSigned,
    { shows: ["Firmado y enviado", "Las 3 firmas del lote ya están en sede.ejemplo.gob.es."] },
  ],
  ["4 · saved", outcome.Saved, { shows: ["Guardado"], hides: [DOCUMENT_TITLE] }],
  [
    "4 · loaded",
    outcome.Loaded,
    { shows: ["Cargado", "Se han enviado 2 ficheros a sede.ejemplo.gob.es."] },
  ],
  [
    "4 · refusal the site must fix",
    outcome.RefusedContactSite,
    {
      shows: ["No se ha completado la petición", CONTACT_SITE, "SAF_01: falta el parámetro format"],
      hides: [DOCUMENT_TITLE, /La sede ha pedido/, /pedid/, /el fallo es de/i],
      noButtons: [NO_COMMENTS_HELP],
    },
  ],
  [
    "4 · SHA-1 refusal tells how to allow it",
    outcome.RefusedWithCause,
    {
      shows: [
        "Si confías en esta sede, puedes permitir SHA-1 en Preferencias → Firma y volver a firmar desde la sede.",
        "Pedid SHA256withRSA o superior.",
      ],
    },
  ],
  [
    "4 · SHA-1 in XML refusal does not",
    outcome.RefusedSha1InXml,
    {
      shows: ["Pedid SHA256withRSA o superior."],
      hides: [/Preferencias/],
    },
  ],
  [
    "4 · refusal without origin",
    outcome.RefusedWithoutOrigin,
    { shows: [/La sede ha pedido una firma con SHA-1/], hides: [/La petición/, CONTACT_SITE] },
  ],
  [
    "4 · refusal that asks to close the other errand",
    outcome.RefusedCloseOther,
    {
      shows: ["No se ha completado la petición", CLOSE_OTHER, "SAF_09: puertos ocupados"],
      hides: ["La petición no ha llegado"],
    },
  ],
  ["4 · unknown refusal", outcome.RefusedUnknown, { buttons: [NO_COMMENTS_HELP] }],
  [
    "5 · none installed",
    noCertificate.NoneInstalled,
    {
      shows: ["No tienes ningún certificado"],
      buttons: NO_CERTIFICATE_BUTTONS,
      noButtons: ["Cerrar la ventana"],
    },
  ],
  [
    "5 · excluded by the site",
    noCertificate.ExcludedBySite,
    {
      shows: ["sede.ejemplo.gob.es no acepta ninguno de tus 2 certificados"],
      buttons: NO_CERTIFICATE_BUTTONS,
      hides: [/ADA LOVELACE/, /criterio/i],
    },
  ],
  [
    "5 · the only certificate excluded",
    noCertificate.ExcludedOnlyCertificate,
    { shows: ["sede.ejemplo.gob.es no acepta tu certificado"] },
  ],
  [
    "5 · many certificates excluded",
    noCertificate.ExcludedManyCertificates,
    { shows: ["sede.ejemplo.gob.es no acepta ninguno de tus 3 certificados"] },
  ],
  [
    "5 · terminal order, none installed",
    noCertificate.TerminalNoneInstalled,
    {
      shows: ["No tienes ningún certificado"],
      hides: [/Necesitas uno instalado/, /FNMT/],
      buttons: [INSTALL],
    },
  ],
  [
    "5 · terminal order, excluded by the filter",
    noCertificate.TerminalExcludedByFilter,
    {
      shows: ["Tu --filter no deja ninguno de tus 2 certificados"],
      hides: [/sede/i],
      noButtons: [INSTALL],
    },
  ],
];

describe("what each story of the site window shows", () => {
  it.each(rows)("%s", (_label, Story, expected) => {
    renderWithCatalog(<Story />);

    for (const text of expected.shows ?? []) expect(screen.getByText(text)).toBeInTheDocument();
    for (const text of expected.hides ?? []) expect(screen.queryByText(text)).toBeNull();
    for (const name of expected.buttons ?? [])
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    for (const name of expected.noButtons ?? [])
      expect(screen.queryByRole("button", { name })).toBeNull();
    for (const name of expected.disabled ?? [])
      expect(screen.getByRole("button", { name })).toBeDisabled();
    if (expected.noButtonAtAll) expect(screen.queryByRole("button")).toBeNull();
  });
});
