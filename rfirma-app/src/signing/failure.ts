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
      return { situation: "certificateNotYetValid", detail: `notBefore=${status.notBefore}` };
    case "revoked":
      return { situation: "certificateRevoked", detail: `revocado: ${status.reason}` };
    case "unreadable":
      // El detalle es el del decodificador de Rust, tal cual. Fabricarlo aquí
      // —«el DER no es un X.509 legible»— llenaba con prosa nuestra el hueco
      // que el ID-29 reserva al texto original crudo, y dejaba el informe de
      // fallo sin lo único que servía para diagnosticarlo.
      return { situation: "certificateUnreadable", detail: status.detail };
    default:
      // `valid` ya salió por `isUsable`, así que esta rama es inalcanzable.
      // Está para que una sexta variante del estado no se cuele en silencio
      // como «se puede firmar»: `tsc` la exige aquí en cuanto se añada.
      return { situation: "unknown", detail: `estado no clasificado: ${status.kind}` };
  }
}
