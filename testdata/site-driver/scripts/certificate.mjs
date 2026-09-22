// Los guiones de selección de certificado de la sede publicada.

import { aConditionEvent, bytesOf, emit, settle, settlingTheError } from "../lib/events.mjs";
import { aPublishedScript } from "../lib/script.mjs";

const A_CERTIFICATE_ALONE = "a-certificate-alone";

/** Una selección que emite si volvió un único certificado codificado y cierra el trámite. */
function theSelectionScript(extraParams) {
  AutoScript.selectCertificate(
    extraParams,
    (data) => {
      const certificate = bytesOf(data);
      const alone =
        !String(data).includes("|") && certificate.length > 0 && certificate[0] === 0x30;
      emit(
        aConditionEvent(
          A_CERTIFICATE_ALONE,
          alone,
          alone
            ? "volvió un único certificado codificado"
            : "la respuesta no fue un certificado suelto",
        ),
      );
      settle({ event: "success", data: String(data) });
    },
    settlingTheError,
  );
}

/** Una selección de certificado del cliente publicado, resuelta cuando conteste el trámite. */
function selecting(step) {
  return new Promise((resolve) => {
    AutoScript.selectCertificate(
      "",
      (data) => {
        emit({ event: "success", step, data: String(data) });
        resolve();
      },
      (type, message) =>
        settle({ event: "error", step, type: String(type), message: String(message) }),
    );
  });
}

/** La espera a que el cliente publicado procese el cierre del canal antes de reutilizarlo. */
function theChannelClosing() {
  return new Promise((resolve) => setTimeout(resolve, 750));
}

/** Tres selecciones: dos con el certificado fijado con `setStickySignatory` y una tras soltarlo. */
function theStickyScript() {
  AutoScript.setStickySignatory(true);
  return selecting("stuck")
    .then(theChannelClosing)
    .then(() => selecting("stuck-again"))
    .then(theChannelClosing)
    .then(() => {
      AutoScript.setStickySignatory(false);
      return selecting("released");
    })
    .then(() => settle({ event: "done" }));
}

export const CERTIFICATE_SCRIPTS = {
  selectcert: aPublishedScript(() => theSelectionScript(""), {
    conditions: [A_CERTIFICATE_ALONE],
  }),
  selectcertheadless: aPublishedScript(() => theSelectionScript("headless=true"), {
    conditions: [A_CERTIFICATE_ALONE],
  }),
  sticky: aPublishedScript(theStickyScript),
};
