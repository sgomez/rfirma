//! Las historias de la sede en su momento 4, el desenlace: cada final del trámite y un rechazo por cada acción que cuenta.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeStoryMeta } from "../../.storybook/decorators/sedeWindow";
import { sedeErrand } from "../../.storybook/fixtures/sedeView";
import type { SiteOutcome } from "./errand";
import type { SedeView } from "./SedeView";
import { signedDocument } from "./sedeWindowFixtures";

const meta = { title: "Sede/4 · Desenlace", ...sedeStoryMeta } satisfies Meta<typeof SedeView>;

export default meta;

type Story = StoryObj<typeof meta>;

const outcome = (result: SiteOutcome, origin: string | null = "sede.ejemplo.gob.es"): Story => ({
  args: { errand: sedeErrand({ kind: "outcome", outcome: result }, { origin }) },
});

export const Signed = outcome({ kind: "signed", document: signedDocument });

export const SignedWithoutDocument = outcome({ kind: "signed", document: null });

export const SignedWithoutOrigin = outcome({ kind: "signed", document: signedDocument }, null);

export const Cancelled = outcome({ kind: "cancelled", document: signedDocument });

export const BatchSigned = outcome({ kind: "batchSigned", signs: 3 });

export const Saved = outcome({ kind: "saved" });

export const Loaded = outcome({ kind: "loaded", fileCount: 2 });

export const RefusedContactSite = outcome({
  kind: "refused",
  situation: "missingFormat",
  detail: "SAF_01: falta el parámetro format",
});

export const RefusedWithCause = outcome({
  kind: "refused",
  situation: "sha1",
  detail: "SAF_03: el algoritmo 'SHA1withRSA' es SHA-1: rFirma firma con SHA-2",
});

export const RefusedExplicitXades = outcome({
  kind: "refused",
  situation: "explicitXades",
  detail: "SAF_04: XAdES explícito",
});

export const RefusedInvoiceMultisignature = outcome({
  kind: "refused",
  situation: "invoiceMultisignature",
  detail: "SAF_05: multifirma de factura",
});

export const RefusedUnsupportedCountersignature = outcome({
  kind: "refused",
  situation: "unsupportedCountersignature",
  detail: "SAF_06: contrafirma no admitida",
});

export const RefusedWithoutOrigin = outcome(
  { kind: "refused", situation: "sha1", detail: "SAF_03: SHA-1" },
  null,
);

export const RefusedCloseOther = outcome({
  kind: "refused",
  situation: "portsTaken",
  detail: "SAF_09: puertos ocupados",
});

export const RefusedRetry = outcome({
  kind: "refused",
  situation: "saveCancelled",
  detail: "SAF_12: guardado cancelado",
});

export const RefusedOtherCertificate = outcome({
  kind: "refused",
  situation: "certificateNotFound",
  detail: "SAF_13: certificado no encontrado",
});

export const RefusedUnknown = outcome({
  kind: "refused",
  situation: "unknown",
  detail: "error inesperado",
});

export const RefusedNamedByTheDesk = outcome({
  kind: "refused",
  situation: "incorrectPin",
  detail: "CKR_PIN_INCORRECT",
});
