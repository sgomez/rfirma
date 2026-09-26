/**
 * Las firmas que ya trae un documento, con quién firmó, cuándo y su estado.
 *
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
  /** El estado de la firma. */
  status: SignatureStatus;
  /** Motivo del original, o `null` si el estado es `valid`. */
  reason: string | null;
}

/** El estado de una firma previa. */
export type SignatureStatus =
  | "valid"
  | "certificateExpired"
  | "certificateNotYetValid"
  | "broken"
  | "unverifiable"
  | "notFullyChecked";

/** El tono del peor aviso, de menor a mayor gravedad. */
export type Tone = "information" | "indeterminate" | "attention";

/** El informe de firmas previas del documento, en el orden en que firmaron. */
export interface PreviousSignaturesReport {
  signatures: readonly PreviousSignature[];
  /** Cuántos avisos deja el informe. */
  warningCount: number;
  /** El tono del peor aviso. */
  tone: Tone;
  /** Si el documento cambió después de la última firma. */
  changedAfterLastSignature: boolean;
}

const INVALID_STATUSES: readonly SignatureStatus[] = [
  "certificateExpired",
  "certificateNotYetValid",
  "broken",
  "unverifiable",
];

/**
 * Las firmas previas no válidas: ni las válidas, ni las que no se han
 * comprobado del todo, que no bloquean la firma.
 */
export function invalidSignatures(
  signatures: readonly PreviousSignature[],
): readonly PreviousSignature[] {
  return signatures.filter((signature) => INVALID_STATUSES.includes(signature.status));
}

/** El informe de un documento sin firmas previas, compartido por escritorio y sede. */
export const NO_PREVIOUS_SIGNATURES: PreviousSignaturesReport = {
  signatures: [],
  warningCount: 0,
  tone: "information",
  changedAfterLastSignature: false,
};
