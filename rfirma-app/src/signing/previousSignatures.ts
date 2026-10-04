//! El vocabulario de una firma previa —quién firmó, cuándo y su estado— y del informe de firmas previas de un documento. Sin React.
/**
 * El backend las lee del PDF con el puente Java, las valida y compone el
 * aviso; aquí solo se enseñan, en el orden cronológico en que llegan.
 */
export interface PreviousSignature {
  /** El nombre del titular. */
  name: string;
  /** El NIF del titular. */
  idNumber: string;
  /** La entidad representada, si el certificado la lleva. */
  organizationIdentifier: string | null;
  /** La autoridad emisora del certificado. */
  issuer: string;
  /** Número de serie del certificado. */
  certificateSerialNumber: string;
  /** Instante de la firma en ISO-8601, o `null` si el puente no lo trajo. */
  signingTime: string | null;
  /** La validez de la firma (ADR-0043). */
  validity: Validity;
  /** Por qué no es válida, o `null` si lo es. */
  validityReason: ValidityReason | null;
  /** La fecha declarada o sellada, o `null` si la firma no trae ninguna. */
  signingDate: SigningDate | null;
  /** Si es la firma que cierra el documento a más firmas. */
  closesDocument: boolean;
  /** Las contrafirmas de esta firma, a cualquier profundidad. */
  countersignatures: readonly PreviousSignature[];
}

/** La validez de una firma: la misma en el aviso, en «Ver firmas», en el resumen y al firmar. */
export type Validity = "valid" | "expired" | "invalid";

/** Por qué una firma está caducada o no es válida. */
export type ValidityReason =
  | { kind: "certificateExpired"; date: string; holder: string | null }
  | { kind: "modifiedAfterSigning" }
  | { kind: "damaged" }
  | { kind: "certificateNotYetValid"; date: string }
  | { kind: "unknownSignatureType" }
  | { kind: "unsupportedAlgorithm" }
  | { kind: "cosignNotAdmitted"; closedBy: string | null };

/** La fecha de una firma: la declara quien firma o la prueba el sello de una TSA. */
export type SigningDate =
  | { kind: "declared"; at: string }
  | { kind: "stamped"; at: string; tsa: string };

/** Lo que se encuentra en el documento entero y no es de ninguna firma. */
export type DocumentFinding =
  | "modifiedAfterLastSignature"
  | "formFilledAfterSigning"
  | "contentAddedOnTop";

/** El tono del peor aviso, de menor a mayor gravedad. */
type Tone = "information" | "attention";

/** El formato de firma del documento, o que no se reconoce. */
export type SignatureFormat = "pades" | "cades" | "xades" | "unrecognized";

/** El informe de firmas previas del documento, en el orden en que firmaron. */
export interface PreviousSignaturesReport {
  signatures: readonly PreviousSignature[];
  /** Cuántos avisos deja el informe. */
  warningCount: number;
  /** El tono del peor aviso. */
  tone: Tone;
  /** Si el documento cambió después de la última firma. */
  changedAfterLastSignature: boolean;
  /** El formato de firma del documento; sin él, PAdES. */
  format?: SignatureFormat;
  /** Los hallazgos del documento, primero en cualquier lista de problemas. */
  findings: readonly DocumentFinding[];
}

/** Un problema que «¿Firmar de todos modos?» enseña: un hallazgo, o una firma que no es válida. */
export type SigningProblem =
  | { kind: "finding"; finding: DocumentFinding }
  | { kind: "signature"; number: number; signature: PreviousSignature };

/**
 * Los problemas del informe, los hallazgos primero y luego cada firma
 * caducada o no válida con su número de orden entre todas. Las válidas no salen.
 */
export function signingProblems(report: PreviousSignaturesReport): readonly SigningProblem[] {
  const findings = report.findings.map((finding): SigningProblem => ({ kind: "finding", finding }));
  const signatures = report.signatures.flatMap((signature, index): SigningProblem[] =>
    signature.validity === "valid" ? [] : [{ kind: "signature", number: index + 1, signature }],
  );
  return [...findings, ...signatures];
}

/** El informe de un documento sin firmas previas, compartido por escritorio y sede. */
export const NO_PREVIOUS_SIGNATURES: PreviousSignaturesReport = {
  signatures: [],
  warningCount: 0,
  tone: "information",
  changedAfterLastSignature: false,
  findings: [],
};
