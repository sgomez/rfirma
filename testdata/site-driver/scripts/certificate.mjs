// Los guiones de selección de certificado de la sede publicada: filtros, almacén y fijación.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

import { aConditionEvent, bytesOf, emit, settle, settlingTheError } from "../lib/events.mjs";
import { withAReleaseWithoutReset } from "../lib/patches.mjs";
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

const THE_EXPECTED_CERTIFICATE = "the-expected-certificate";
const THE_EXPIRED_HIDDEN_ONLY_WITHOUT_FILTERS = "the-expired-hidden-only-without-filters";
const THE_BATCH_SIGNED_WITH_THE_FILTERED = "the-batch-signed-with-the-filtered-certificate";
const THE_PINNED_WITHOUT_ASKING = "the-pinned-certificate-without-asking";
const THE_PINNED_DESPITE_THE_FILTERS = "the-pinned-certificate-despite-the-filters";
const SIGNED_WITH_THE_PINNED = "signed-with-the-pinned-certificate";
const A_NEW_SELECTION_AFTER_THE_RELEASE = "a-new-selection-after-the-release";

/** Los certificados del kit de la FNMT que montan los almacenes `several` y `expired`. */
const THE_KIT = {
  "active-rsa": "el RSA activo (99999999R)",
  "active-ecc": "el de curva elíptica (99949991H)",
  "pseudonym-rsa": "el de seudónimo (TEST-0000)",
  "expired-rsa": "el RSA caducado (99999999R)",
};

function theKitCertificate(name) {
  return readFileSync(new URL(`../certificates/${name}.der`, import.meta.url));
}

function whichOfTheKit(certificate) {
  const der = bytesOf(certificate);
  return Object.keys(THE_KIT).find((name) => theKitCertificate(name).equals(der)) ?? null;
}

/** Lo que volvió de una operación, dicho para la observación. */
function described(answer) {
  if (answer.error) return `la operación falló: ${answer.error}`;
  const name = whichOfTheKit(answer.certificate);
  return name ? `volvió ${THE_KIT[name]}` : "volvió un certificado que no es del kit";
}

function isTheKit(answer, name) {
  return !answer.error && whichOfTheKit(answer.certificate) === name;
}

/** Una selección que se resuelve con el certificado o con el error, sin cerrar el trámite. */
function aSelection(properties) {
  return new Promise((resolve) => {
    AutoScript.selectCertificate(
      properties.join("\n"),
      (data) => resolve({ certificate: String(data) }),
      (type, message) => resolve({ error: `${type}: ${message}` }),
    );
  });
}

/** Una firma CAdES del reto que se resuelve con el certificado firmante o con el error. */
function aSignature() {
  return new Promise((resolve) => {
    AutoScript.sign(
      Buffer.from("documento de la fijación").toString("base64"),
      "SHA256withRSA",
      "CAdES",
      "",
      (_signature, certificate) => resolve({ certificate: String(certificate) }),
      (type, message) => resolve({ error: `${type}: ${message}` }),
    );
  });
}

/**
 * Sin persona delante, un diálogo que no tenía que salir solo se ve como un trámite que no vuelve:
 * la condición sale no conforme antes de que se agote la espera y se mate al cliente.
 */
function unansweredMeansAsked(condition) {
  const patience = Number(process.env.RFIRMA_BENCH_TIMEOUT_MS ?? "45000");
  setTimeout(
    () => {
      emit(
        aConditionEvent(
          condition,
          false,
          "el trámite no volvió solo: el cliente preguntó donde no tocaba o se quedó mostrando un error",
        ),
      );
      settle({ event: "done" });
    },
    Math.max(patience - 8000, 1000),
  );
}

function settlingThe(answer) {
  settle(
    answer.error
      ? { event: "error", type: answer.error.split(": ")[0], message: answer.error }
      : { event: "success", data: answer.certificate },
  );
}

/** Una selección desatendida con `properties` que tiene que devolver `expected` del kit. */
function theFilteredSelectionScript(properties, expected) {
  return async () => {
    unansweredMeansAsked(THE_EXPECTED_CERTIFICATE);
    const answer = await aSelection(["headless=true", ...properties]);
    emit(aConditionEvent(THE_EXPECTED_CERTIFICATE, isTheKit(answer, expected), described(answer)));
    settlingThe(answer);
  };
}

const aFilteredSelection = (properties, expected) =>
  aPublishedScript(theFilteredSelectionScript(properties, expected), {
    conditions: [THE_EXPECTED_CERTIFICATE],
  });

/** La misma selección desatendida con el almacén que nombra la sede. */
const aSelectionFromTheKeyStore = (keyStore, properties, expected) =>
  aPublishedScript(
    () => {
      AutoScript.setKeyStore(keyStore);
      return theFilteredSelectionScript(properties, expected)();
    },
    { conditions: [THE_EXPECTED_CERTIFICATE] },
  );

/** Una selección sin condición: lo que vuelva, certificado o código, lo juzga el catálogo. */
function theBareSelectionScript(properties, keyStore) {
  if (keyStore) AutoScript.setKeyStore(keyStore);
  aSelection(properties).then(settlingThe);
}

/** En el almacén `expired`: sin filtros vuelve el vigente solo; con uno del emisor, el caducado. */
async function theExpiredCertificatesScript() {
  unansweredMeansAsked(THE_EXPIRED_HIDDEN_ONLY_WITHOUT_FILTERS);
  const unfiltered = await aSelection(["headless=true"]);
  await theChannelClosing();
  const filtered = await aSelection(["headless=true", "filters=issuer.contains:AC FNMT Usuarios"]);
  const held = isTheKit(unfiltered, "active-ecc") && isTheKit(filtered, "expired-rsa");
  emit(
    aConditionEvent(
      THE_EXPIRED_HIDDEN_ONLY_WITHOUT_FILTERS,
      held,
      `sin filtros ${described(unfiltered)}; con el del emisor ${described(filtered)}`,
    ),
  );
  settlingThe(filtered);
}

/** Un lote local de un documento con los filtros de la sede, que tiene que firmar el de seudónimo. */
function theFilteredBatchScript() {
  AutoScript.setLocalBatchProcess(true);
  AutoScript.createBatch("SHA256", "CAdES", "sign", null);
  AutoScript.addDocumentToBatch(
    "uno",
    Buffer.from("documento del lote filtrado").toString("base64"),
  );
  AutoScript.signBatchProcess(
    true,
    null,
    null,
    "headless=true\nfilters=subject.contains:TEST-0000",
    (result, certificate) => {
      const answer = { certificate: String(certificate) };
      emit(
        aConditionEvent(
          THE_BATCH_SIGNED_WITH_THE_FILTERED,
          isTheKit(answer, "pseudonym-rsa"),
          described(answer),
        ),
      );
      settle({
        event: "success",
        result: Buffer.from(JSON.stringify(result), "utf8").toString("base64"),
        certificate: String(certificate),
      });
    },
    settlingTheError,
  );
}

/** Dos selecciones de la persona: en la primera cambia al PKCS#12, y la segunda se deja ver. */
async function theKeyStoreKeptScript() {
  const first = await aSelection([]);
  await theChannelClosing();
  const second = await aSelection([]);
  emit({ event: "success", step: "first", data: first.certificate ?? first.error });
  settlingThe(second);
}

const PINNING_THE_PSEUDONYM = ["headless=true", "filters=subject.contains:TEST-0000"];
const FILTERING_THE_ELLIPTIC = ["headless=true", "filters=subject.contains:99949991H"];

/** Fija el de seudónimo con una selección desatendida y corre `then` con lo que volvió. */
function pinningThePseudonymAnd(condition, then) {
  return async () => {
    unansweredMeansAsked(condition);
    AutoScript.setStickySignatory(true);
    const pinned = await aSelection(PINNING_THE_PSEUDONYM);
    await theChannelClosing();
    const answer = await then();
    const held = isTheKit(pinned, "pseudonym-rsa") && isTheKit(answer, "pseudonym-rsa");
    emit(
      aConditionEvent(
        condition,
        held,
        `al fijar ${described(pinned)}; después ${described(answer)}`,
      ),
    );
    settlingThe(answer);
  };
}

/** Fija el de seudónimo, lo suelta con una operación sin `sticky` y vuelve a fijar sin `resetsticky`. */
async function theReleaseWithoutResetScript() {
  unansweredMeansAsked(A_NEW_SELECTION_AFTER_THE_RELEASE);
  AutoScript.setStickySignatory(true);
  const pinned = await aSelection(PINNING_THE_PSEUDONYM);
  await theChannelClosing();
  AutoScript.setStickySignatory(false, true);
  const unpinned = await aSelection(FILTERING_THE_ELLIPTIC);
  await theChannelClosing();
  AutoScript.setStickySignatory(true);
  const again = await aSelection(FILTERING_THE_ELLIPTIC);
  const held = isTheKit(pinned, "pseudonym-rsa") && isTheKit(again, "active-ecc");
  emit(
    aConditionEvent(
      A_NEW_SELECTION_AFTER_THE_RELEASE,
      held,
      `al fijar ${described(pinned)}; sin sticky ${described(unpinned)}; al volver a fijar ${described(again)}`,
    ),
  );
  settlingThe(again);
}

const PKCS11_OF_SOFTHSM = "PKCS11:/usr/lib/softhsm/libsofthsm2.so";

/** El DER del de seudónimo en Base64, como lo pide `encodedcert:`. */
const theEncodedPseudonym = () => theKitCertificate("pseudonym-rsa").toString("base64");

/** La huella SHA-256 del de seudónimo en hexadecimal, en minúsculas y con un espacio por byte. */
const theSpacedThumbprint = () =>
  createHash("sha256")
    .update(theKitCertificate("pseudonym-rsa"))
    .digest("hex")
    .match(/../g)
    .join(" ");

export const CERTIFICATE_SCRIPTS = {
  selectcert: aPublishedScript(() => theSelectionScript(""), {
    conditions: [A_CERTIFICATE_ALONE],
  }),
  selectcertheadless: aPublishedScript(() => theSelectionScript("headless=true"), {
    conditions: [A_CERTIFICATE_ALONE],
  }),
  sticky: aPublishedScript(theStickyScript),
  filtersand: aFilteredSelection(
    ["filters=issuer.contains:Ceres;subject.contains:IDCES-"],
    "active-rsa",
  ),
  filtersindexed: aFilteredSelection(
    ["filters.1=subject.contains:NINGUN-TITULAR", "filters.2=subject.contains:99949991H"],
    "active-ecc",
  ),
  filtersingular: aFilteredSelection(
    ["filter=subject.contains:99949991H", "filters=subject.contains:TEST-0000"],
    "active-ecc",
  ),
  filtersoverindexed: aFilteredSelection(
    ["filters=subject.contains:99949991H", "filters.1=subject.contains:TEST-0000"],
    "active-ecc",
  ),
  filtersfromone: aFilteredSelection(
    ["filters.0=subject.contains:TEST-0000", "filters.1=subject.contains:99949991H"],
    "active-ecc",
  ),
  filtersexcludeall: aPublishedScript(() =>
    theBareSelectionScript(["headless=true", "filters=subject.contains:NINGUN-TITULAR"]),
  ),
  filtersunknown: aFilteredSelection(
    ["filters=subject.contains:99949991H;nosuchfilter:x"],
    "active-ecc",
  ),
  filtersunknownalone: aFilteredSelection(["filters=nosuchfilter:x"], "active-ecc"),
  filtersexpired: aPublishedScript(theExpiredCertificatesScript, {
    conditions: [THE_EXPIRED_HIDDEN_ONLY_WITHOUT_FILTERS],
  }),
  filtersubject: aFilteredSelection(["filters=subject.contains:test-0000"], "pseudonym-rsa"),
  filterissuer: aFilteredSelection(["filters=issuer.contains:ac usuarios g2"], "active-ecc"),
  filterthumbprint: aFilteredSelection(
    [`filters=thumbprint:SHA-256:${theSpacedThumbprint()}`],
    "pseudonym-rsa",
  ),
  filterencodedcert: aFilteredSelection(
    [`filters=encodedcert:${theEncodedPseudonym()}`],
    "pseudonym-rsa",
  ),
  filterrfc2254: aFilteredSelection(
    ["filters=subject.rfc2254:(serialnumber=IDCES-99949991H);issuer.rfc2254:(cn=AC USUARIOS G2)"],
    "active-ecc",
  ),
  filterkeyusage: aFilteredSelection(
    ["filters=keyusage.nonrepudiation:true;keyusage.keyencipherment:false"],
    "active-ecc",
  ),
  filterpolicyid: aFilteredSelection(
    ["filters=policyid:1.3.6.1.4.1.5734.3.20.1.0,0.4.0.194112.1.0"],
    "active-ecc",
  ),
  filterqualified: aFilteredSelection(
    ["filters=qualified:113710BDF09DC4F465C12741829122AB"],
    "pseudonym-rsa",
  ),
  batchfiltered: aPublishedScript(theFilteredBatchScript, {
    conditions: [THE_BATCH_SIGNED_WITH_THE_FILTERED],
  }),
  keystorepkcs11: aPublishedScript(() => theBareSelectionScript([], PKCS11_OF_SOFTHSM)),
  keystoreunknown: aSelectionFromTheKeyStore("NINGUNO", [], "active-rsa"),
  keystoreforeign: aPublishedScript(() => theBareSelectionScript(["headless=true"], "WINDOWS")),
  keystorekept: aPublishedScript(theKeyStoreKeptScript),
  stickyselect: aPublishedScript(
    pinningThePseudonymAnd(THE_PINNED_WITHOUT_ASKING, () => aSelection([])),
    { conditions: [THE_PINNED_WITHOUT_ASKING] },
  ),
  stickyfilters: aPublishedScript(
    pinningThePseudonymAnd(THE_PINNED_DESPITE_THE_FILTERS, () =>
      aSelection(FILTERING_THE_ELLIPTIC),
    ),
    { conditions: [THE_PINNED_DESPITE_THE_FILTERS] },
  ),
  stickysigns: aPublishedScript(pinningThePseudonymAnd(SIGNED_WITH_THE_PINNED, aSignature), {
    conditions: [SIGNED_WITH_THE_PINNED],
  }),
  stickyreleased: aPublishedScript(theReleaseWithoutResetScript, {
    conditions: [A_NEW_SELECTION_AFTER_THE_RELEASE],
    patch: withAReleaseWithoutReset,
  }),
};
