// Los guiones de firma, cofirma y contrafirma de la sede publicada.

import { createHash } from "node:crypto";
import { gzipSync } from "node:zlib";

import { aCondition, bytesOf, emit, settle, settlingTheError } from "../lib/events.mjs";
import {
  theChallenge,
  theInvoice,
  thePdfOfTheTest,
  theReferenceSignature,
  theXmlDocument,
} from "../lib/fixtures.mjs";
import { withTheDataDeclaredGzipped } from "../lib/patches.mjs";
import { aPublishedScript } from "../lib/script.mjs";

/** La transformación XPath que declara el guion de transformaciones a medida y busca en la firma. */
const THE_DECLARED_TRANSFORM = "http://www.w3.org/TR/1999/REC-xpath-19991116";

const APART = "certificate-and-signature-apart";
const A_ZIP_CONTAINER = "a-zip-container";
const THE_TRANSFORM_DECLARED = "the-transform-declared";

/** Un `sign()` sobre `content`, con el formato y `extraParams` del guion. */
function theSignScript(format, extraParams, content, measuring) {
  theSignScriptWith("SHA256withRSA", format, extraParams, content, measuring);
}

/** Un `sign()` con el algoritmo del guion. */
function theSignScriptWith(algorithm, format, extraParams, content, measuring) {
  AutoScript.sign(
    content.toString("base64"),
    algorithm,
    format,
    extraParams,
    (signature, certificate) => answering(measuring, String(signature), String(certificate)),
    settlingTheError,
  );
}

/** Un `cosign()` sobre `content`, con el formato y `extraParams` del guion. */
function theCosignScript(format, extraParams, content, measuring) {
  AutoScript.cosign(
    content.toString("base64"),
    "SHA256withRSA",
    format,
    extraParams,
    (signature, certificate) => answering(measuring, String(signature), String(certificate)),
    settlingTheError,
  );
}

/** Un `counterSign()` sobre `content`, con cualquiera de las dos grafías de la fachada. */
function theCountersignScript(format, extraParams, content) {
  const countersigning = AutoScript.counterSign ?? AutoScript.countersign;
  if (!countersigning) {
    settle({
      event: "error",
      type: "unsupported",
      message: "la fachada del cliente publicado no exporta la contrafirma",
    });
    return;
  }
  countersigning.call(
    AutoScript,
    content.toString("base64"),
    "SHA256withRSA",
    format,
    extraParams,
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    settlingTheError,
  );
}

/** La respuesta trae el certificado y la firma por separado, cada uno con su contenido. */
function theCertificateAndTheSignatureApart(signature, certificate) {
  const apart = signature.length > 0 && certificate.length > 0 && signature !== certificate;
  return [
    aCondition(
      APART,
      apart,
      apart
        ? "el certificado y la firma llegaron separados"
        : "la respuesta no trajo los dos componentes por separado",
    ),
  ];
}

/** Un contenedor ASiC-S es un ZIP: sus dos primeros bytes son la marca `PK`. */
function theAsicContainer(signature) {
  const bytes = bytesOf(signature);
  const zipped = bytes.length > 2 && bytes[0] === 0x50 && bytes[1] === 0x4b;
  return [
    aCondition(
      A_ZIP_CONTAINER,
      zipped,
      zipped ? "el contenedor empieza por la marca PK" : "lo que volvió no es un contenedor ZIP",
    ),
  ];
}

/** La transformación declarada aparece como `<ds:Transform>` en el XAdES que vuelve. */
function theDeclaredTransform(signature) {
  const applied = bytesOf(signature).toString("utf8").includes(THE_DECLARED_TRANSFORM);
  return [
    aCondition(
      THE_TRANSFORM_DECLARED,
      applied,
      applied
        ? "la firma declara la transformación pedida"
        : "la firma volvió sin la transformación pedida",
    ),
  ];
}

/** Emite lo que `measuring` saque de la respuesta y cierra el trámite con ella. */
function answering(measuring, signature, certificate) {
  for (const condition of measuring ? measuring(signature, certificate) : []) {
    emit({ event: "condition", ...condition });
  }
  settle({ event: "success", result: signature, certificate });
}

/** Un `sign()` en CAdES con un `tsaURL` de sintaxis inválida (BUG-23). */
function theSignWithABrokenTsaUrlScript() {
  theSignScript("CAdES", "mode=explicit\ntsaURL=http://tsa invalida", theChallenge());
}

const signing = (format, extraParams, content, measuring) => () =>
  theSignScript(format, extraParams, content(), measuring);

const cosigning = (format, extraParams, content) => () =>
  theCosignScript(format, extraParams, content());

const theCadesImplicitSignature = () => theReferenceSignature("cades-implicit.p7s");

export const SIGNATURE_SCRIPTS = {
  signcades: aPublishedScript(
    signing("CAdES", "mode=explicit", theChallenge, theCertificateAndTheSignatureApart),
    { conditions: [APART] },
  ),
  signgzip: aPublishedScript(
    signing("CAdES", "mode=explicit", () => gzipSync(theChallenge())),
    {
      patch: withTheDataDeclaredGzipped,
    },
  ),
  signcadesasics: aPublishedScript(signing("CAdES-ASiC-S", "", theChallenge, theAsicContainer), {
    conditions: [A_ZIP_CONTAINER],
  }),
  signauto: aPublishedScript(signing("auto", "", theChallenge)),
  signxades: aPublishedScript(signing("XAdES", "", theXmlDocument), { benchOnly: true }),
  signxadesauto: aPublishedScript(signing("auto", "", theXmlDocument)),
  signxadesenveloping: aPublishedScript(signing("XAdES Enveloping", "", theXmlDocument)),
  signxadeswithatransform: aPublishedScript(
    signing(
      "XAdES Enveloping",
      `xmlTransforms=1\nxmlTransform0Type=${THE_DECLARED_TRANSFORM}\nxmlTransform0Body=/*`,
      theXmlDocument,
      theDeclaredTransform,
    ),
    { conditions: [THE_TRANSFORM_DECLARED] },
  ),
  signpades: aPublishedScript(signing("PAdES", "", thePdfOfTheTest)),
  signpadesoveranonpdf: aPublishedScript(signing("PAdES", "", theChallenge)),
  signpadeschecking: aPublishedScript(signing("PAdES", "checkSignatures=true", thePdfOfTheTest)),
  signpadesvisible: aPublishedScript(signing("PAdES", "visibleSignature=want", thePdfOfTheTest)),
  signfacturae: aPublishedScript(signing("FacturaE", "", theInvoice), { benchOnly: true }),
  signfacturaewitharole: aPublishedScript(
    signing("FacturaE", "signerClaimedRoles=emisor\nsignatureProductionCity=Madrid", theInvoice),
  ),
  signfacturaewithaforbiddenparam: aPublishedScript(
    signing("FacturaE", "tsaURL=http://tsa.example/tsa", theInvoice),
  ),
  signcadeswithadigestonlyalgorithm: aPublishedScript(() =>
    theSignScriptWith("SHA256", "CAdES", "mode=explicit", theChallenge()),
  ),
  signcadeswithanunsupportedalgorithm: aPublishedScript(() =>
    theSignScriptWith("MD5withRSA", "CAdES", "mode=explicit", theChallenge()),
  ),
  signcadeswithaprecalculatedhash: aPublishedScript(
    signing("CAdES", "precalculatedHashAlgorithm=SHA-256", () =>
      createHash("sha256").update(theChallenge()).digest(),
    ),
  ),
  signwithanunknownformat: aPublishedScript(signing("NoSuchFormat", "", theChallenge)),
  signwithoutaformat: aPublishedScript(signing(null, "", theChallenge)),
  signwithbrokentsa: aPublishedScript(theSignWithABrokenTsaUrlScript),
  cosigncades: aPublishedScript(cosigning("CAdES", "", theCadesImplicitSignature)),
  cosignauto: aPublishedScript(cosigning("auto", "", theCadesImplicitSignature)),
  cosignautowithoutasignature: aPublishedScript(cosigning("auto", "", theXmlDocument)),
  cosignpadeschecking: aPublishedScript(
    cosigning("PAdES", "checkSignatures=true", thePdfOfTheTest),
  ),
  cosignfacturae: aPublishedScript(cosigning("FacturaE", "", theInvoice), { benchOnly: true }),
  countersigncades: aPublishedScript(() =>
    theCountersignScript("CAdES", "target=tree", theCadesImplicitSignature()),
  ),
};
