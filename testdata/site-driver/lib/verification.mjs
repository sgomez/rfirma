// La condición transversal `the-signature-verifies`: la firma devuelta verifica con la clave de su certificado, no solo tiene la forma.

import { theCmsVerification } from "./cms.mjs";
import { aMeasuredCondition, bytesOf } from "./events.mjs";
import { thePadesVerification } from "./pades.mjs";
import { theXadesVerification } from "./xades.mjs";

export const THE_SIGNATURE_VERIFIES = "the-signature-verifies";

const VERIFYING = {
  cms: theCmsVerification,
  pdf: (signature, certificate) => thePadesVerification(signature, certificate),
  xml: (signature, certificate) => theXadesVerification(signature.toString("utf8"), certificate),
};

function theVerification(format, signature, certificate, data) {
  try {
    return VERIFYING[format](bytesOf(signature), bytesOf(certificate), data);
  } catch (error) {
    return { verified: false, reason: `la firma no se pudo leer: ${error?.message}` };
  }
}

/** La medida de un guion: la firma `cms`, `pdf` o `xml` verifica con el certificado; `data` son los datos de una firma explícita. */
export const theSignatureVerifies =
  (format, data = () => null) =>
  (signature, certificate) => {
    const { verified, reason } = theVerification(format, signature, certificate, data());
    return [aMeasuredCondition(THE_SIGNATURE_VERIFIES, verified, reason)];
  };
