import type { Certificate } from "./certificate";
import type { PreviousSignature } from "./previousSignatures";

/**
 * «Ya lo firmaste tú», con qué certificado: el mismo con el que se firmó antes,
 * u otro certificado del mismo titular y entidad representada.
 */
export type SameSignerNotice = "sameCertificate" | "otherCertificate";

/**
 * Si el certificado elegido ya firmó el documento, y con cuál certificado
 * (ID-404): el NIF del titular y la entidad representada tienen que coincidir
 * con alguna firma previa; «ninguna» entidad cuenta como valor. `null` sin
 * certificado elegido y sin coincidencia.
 */
export function sameSignerNotice(
  certificate: Certificate | null,
  signatures: readonly PreviousSignature[],
): SameSignerNotice | null {
  if (certificate === null) {
    return null;
  }
  const bySameHolder = signatures.filter(
    (signature) =>
      signature.idNumber === certificate.idNumber &&
      signature.organizationIdentifier === certificate.organizationIdentifier,
  );
  if (bySameHolder.length === 0) {
    return null;
  }
  const withThisCertificate = bySameHolder.some(
    (signature) =>
      signature.issuer === certificate.issuer &&
      signature.certificateSerialNumber === certificate.certificateSerialNumber,
  );
  return withThisCertificate ? "sameCertificate" : "otherCertificate";
}
