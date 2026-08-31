import type { ErrorSituation } from "../errors/ErrorNotice";
import type { Certificate } from "./certificate";
import { isUsable } from "./certificate";

/**
 * Un fallo del recorrido de firma, con la forma del ID-29: una **situación**
 * nuestra, que el catálogo traduce, y el texto original **crudo** al lado.
 *
 * Lo cumplen por igual los fallos del token (`pkcs11::error`), los del puente y
 * los que decide la propia ventana, como firmar con un certificado caducado:
 * quien los enseña es siempre `ErrorNotice`, y no hay dos formas de contar un
 * error.
 */
export interface SigningFailure {
  situation: ErrorSituation;
  /** El texto original, sin traducir ni recortar. Nunca vacío. */
  detail: string;
}

/**
 * Por qué **no** se puede firmar con este certificado, decidido antes de pedir
 * el PIN.
 *
 * Es la comprobación del ADR: el estado del certificado se conoce leyendo su
 * DER, sin tocar la tarjeta y sin red. Hacer teclear el PIN para luego fallar
 * por una fecha que ya se sabía es pedir el secreto que desbloquea la clave
 * para nada.
 */
export function refusalFor(certificate: Certificate | null): SigningFailure | null {
  if (certificate === null) {
    return { situation: "certificateNotFound", detail: "no hay certificado elegido" };
  }
  if (isUsable(certificate.status)) return null;

  const status = certificate.status;
  switch (status.kind) {
    case "expired":
      return { situation: "certificateExpired", detail: `notAfter=${status.notAfter}` };
    case "notYetValid":
      return { situation: "certificateNotYetValid", detail: "notBefore en el futuro" };
    case "revoked":
      return { situation: "certificateRevoked", detail: `revocado: ${status.reason}` };
    default:
      return { situation: "certificateUnreadable", detail: "el DER no es un X.509 legible" };
  }
}
