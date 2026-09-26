/**
 * Las firmas que ya trae un documento, con quién firmó y cuándo.
 *
 * El backend las lee del PDF con el puente Java y las traduce; aquí solo se
 * enseñan, en el orden cronológico en que llegan.
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
}

/** El informe de firmas previas del documento, en el orden en que firmaron. */
export interface PreviousSignaturesReport {
  signatures: readonly PreviousSignature[];
}
