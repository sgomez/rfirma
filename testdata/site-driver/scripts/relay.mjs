// Los guiones de la sede publicada por servidor intermedio, que solo usa el banco de conformidad.

import { settle, settlingTheError } from "../lib/events.mjs";
import { theInvoice } from "../lib/fixtures.mjs";
import { aPublishedScript } from "../lib/script.mjs";

/** Un documento que en base64 pasa de `MAX_LONG_GENERAL_URL` y obliga al servidor intermedio. */
function aDocumentTooLongForTheUrl() {
  return Buffer.from("%PDF-1.7\n".concat("d".repeat(3000)), "utf8");
}

/** Una firma en modo servidor intermedio: los servlets son el `XMLHttpRequest` del banco. */
function theRelayScript() {
  AutoScript.setServlets(
    "https://sede.example/afirma-signature-storage/StorageService",
    "https://sede.example/afirma-signature-retriever/RetrieveService",
  );
  AutoScript.sign(
    aDocumentTooLongForTheUrl().toString("base64"),
    "SHA256withRSA",
    "CAdES",
    "mode=explicit",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    settlingTheError,
  );
}

/** La URL larga por servidor intermedio con una cofirma de factura, que no se admite. */
function theRelayRefusedScript() {
  AutoScript.setServlets(
    "https://sede.example/afirma-signature-storage/StorageService",
    "https://sede.example/afirma-signature-retriever/RetrieveService",
  );
  AutoScript.cosign(
    theInvoice().toString("base64"),
    "SHA256withRSA",
    "FacturaE",
    "",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    settlingTheError,
  );
}

export const RELAY_SCRIPTS = {
  relay: aPublishedScript(theRelayScript, { modes: ["relay"], benchOnly: true }),
  relayrefused: aPublishedScript(theRelayRefusedScript, { modes: ["relay"], benchOnly: true }),
};
