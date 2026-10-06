//! Los datos de ejemplo que comparten las historias de sede: las firmas previas y un PDF en blanco.

import type { PdfDocument } from "../../../viewer/pdf";
import type {
  PreviousSignature,
  PreviousSignaturesReport,
} from "../../../signing/previousSignatures";

/** Una firma previa válida de la misma persona que firma. */
export function previousSignature(overrides: Partial<PreviousSignature> = {}): PreviousSignature {
  return {
    name: "Ada Lovelace Byron",
    idNumber: "99999999R",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1",
    signingTime: "2024-01-01T10:00:00Z",
    validity: "valid",
    validityReason: null,
    signingDate: null,
    closesDocument: false,
    countersignatures: [],
    ...overrides,
  };
}

/** El informe de firmas previas; sin avisos salvo que se digan. */
export function previousSignaturesReport(
  signatures: readonly PreviousSignature[],
  overrides: Partial<Omit<PreviousSignaturesReport, "signatures">> = {},
): PreviousSignaturesReport {
  return {
    signatures,
    warningCount: 0,
    tone: "information",
    changedAfterLastSignature: false,
    findings: [],
    ...overrides,
  };
}

/** Un PDF de tres páginas cuyas pintadas no terminan: el visor se enseña con la hoja en blanco. */
export const blankPdf: PdfDocument = {
  pageCount: 3,
  getPage: (number) =>
    Promise.resolve({
      number,
      rotate: 0,
      view: [0, 0, 595, 842],
      getViewport: ({ scale }) => ({
        width: 595 * scale,
        height: 842 * scale,
        convertToPdfPoint: (x, y) => [x / scale, 842 - y / scale],
        convertToViewportPoint: (x, y) => [x * scale, (842 - y) * scale],
      }),
      render: () => ({ promise: new Promise<void>(() => {}), cancel: () => {} }),
    }),
};
