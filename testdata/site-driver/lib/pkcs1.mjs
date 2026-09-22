// La firma PKCS#1 suelta de la sede: la verifica con la clave del certificado, sin envoltorio CMS.

import { verify, X509Certificate } from "node:crypto";

/** Si `signature` es la firma en SHA-256 de `data` con la clave del certificado DER, tal cual. */
export function isABarePkcs1(data, signature, certificate) {
  try {
    return verify("sha256", data, new X509Certificate(certificate).publicKey, signature);
  } catch {
    return false;
  }
}
