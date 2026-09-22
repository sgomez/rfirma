// Los guiones de firma, cofirma y contrafirma de la sede publicada.

import { createHash } from "node:crypto";
import { gzipSync } from "node:zlib";

import {
  signsTheData,
  theCmsSignature,
  theKeyFamilyOf,
  thePublicKeyAlgorithmOf,
  theShapeOf,
} from "../lib/cms.mjs";
import { aCondition, bytesOf, emit, settle, settlingTheError } from "../lib/events.mjs";
import {
  theChallenge,
  theCmsSignatureOfTheSite,
  theInvoice,
  thePdfOfTheTest,
  theReferenceSignature,
  theXmlDocument,
} from "../lib/fixtures.mjs";
import { isASignedPdf } from "../lib/pades.mjs";
import { withTheDataDeclaredGzipped } from "../lib/patches.mjs";
import { aPublishedScript, NOT_YET_DRIVEN } from "../lib/script.mjs";
import { isAXadesSignature, signsTheRoleAndThePlace, theXadesEnvelope } from "../lib/xades.mjs";
import { theZipEntries } from "../lib/zip.mjs";

/** La transformación XPath que declara el guion de transformaciones a medida y busca en la firma. */
const THE_DECLARED_TRANSFORM = "http://www.w3.org/TR/1999/REC-xpath-19991116";

const APART = "certificate-and-signature-apart";
const A_ZIP_CONTAINER = "a-zip-container";
const THE_TRANSFORM_DECLARED = "the-transform-declared";
const A_SINGLE_SIGNER = "a-single-signer";
const TWO_PARALLEL_SIGNERS = "two-parallel-signers";
const THE_SIGNER_COUNTERSIGNED = "the-signer-countersigned";
const EVERY_NODE_COUNTERSIGNED = "every-node-countersigned";
const ONLY_THE_LEAVES_COUNTERSIGNED = "only-the-leaves-countersigned";
const THE_DATA_INSIDE = "the-data-inside";
const THE_DATA_LEFT_OUT = "the-data-left-out";
const THE_HASH_SIGNED_AS_IT_CAME = "the-hash-signed-as-it-came";
const THE_UNCOMPRESSED_DATA_SIGNED = "the-uncompressed-data-signed";
const THE_ALGORITHM_OF_THE_KEY = "the-algorithm-of-the-key";
const A_CADES_SIGNATURE = "a-cades-signature";
const COSIGNED_AS_CADES = "cosigned-as-cades";
const A_XADES_SIGNATURE = "a-xades-signature";
const THE_ENVELOPE_REQUESTED = "the-envelope-requested";
const THE_ROLE_AND_THE_PLACE_SIGNED = "the-role-and-the-place-signed";
const THE_SIGNATURE_INSIDE_THE_PDF = "the-signature-inside-the-pdf";
const THE_DATA_AND_THE_SIGNATURE_INSIDE = "the-data-and-the-signature-inside";

/** El documento de referencia se llama `documento`, y así se busca dentro de la firma XML. */
const THE_DOCUMENT_ROOT = "documento";
const THE_EXTERNAL_URI = "https://sede.example/documento.xml";
const THE_ROLE = "emisor";
const THE_CITY = "Madrid";

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
function theCountersignScript(format, extraParams, content, measuring) {
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
    (signature, certificate) => answering(measuring, String(signature), String(certificate)),
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

/** Una condición sobre el CMS que volvió; si no es un `SignedData`, no se cumple. */
function onTheCms(name, holding, describing) {
  return (signature) => {
    const cms = theCmsSignature(bytesOf(signature));
    if (!cms) return [aCondition(name, false, "lo que volvió no es un CMS SignedData")];
    return [aCondition(name, holding(cms), describing(cms))];
  };
}

const theShape = (cms) => `firmantes con la forma ${theShapeOf(cms)}`;

const withTheShape = (name, shape) => onTheCms(name, (cms) => theShapeOf(cms) === shape, theShape);

const theDataInside = (data) =>
  onTheCms(
    THE_DATA_INSIDE,
    (cms) => cms.content?.equals(data()) ?? false,
    (cms) =>
      cms.content ? `el CMS lleva ${cms.content.length} bytes dentro` : "el CMS no lleva los datos",
  );

const theDataLeftOut = onTheCms(
  THE_DATA_LEFT_OUT,
  (cms) => cms.content === null,
  (cms) =>
    cms.content ? `el CMS lleva ${cms.content.length} bytes dentro` : "el CMS no lleva los datos",
);

const signingTheData = (name, data) =>
  onTheCms(
    name,
    (cms) => cms.signers.length === 1 && signsTheData(cms.signers[0], data()),
    (cms) =>
      cms.signers.length === 1 && signsTheData(cms.signers[0], data())
        ? "el messageDigest firmado es el resumen de lo esperado"
        : "el messageDigest firmado no es el resumen de lo esperado",
  );

const allCades = onTheCms(
  A_CADES_SIGNATURE,
  (cms) => cms.signers.length > 0 && cms.signers.every((signer) => signer.cades),
  (cms) =>
    cms.signers.every((signer) => signer.cades)
      ? "un CMS cuyos firmantes llevan signingCertificate: CAdES"
      : "un CMS con algún firmante sin signingCertificate: no es CAdES",
);

const cosignedAsCades = onTheCms(
  COSIGNED_AS_CADES,
  (cms) =>
    theShapeOf(cms) === "[][]" &&
    cms.signers.some((signer) => signer.cades) &&
    cms.signers.some((signer) => !signer.cades),
  (cms) =>
    `${theShape(cms)}; ${cms.signers.filter((signer) => signer.cades).length} con signingCertificate`,
);

/** El algoritmo del firmante es de la familia de la clave del certificado que volvió. */
function theAlgorithmOfTheKey(signature, certificate) {
  const cms = theCmsSignature(bytesOf(signature));
  let key = null;
  try {
    key = theKeyFamilyOf(thePublicKeyAlgorithmOf(bytesOf(certificate)));
  } catch {}
  const signed =
    cms?.signers.length === 1 ? theKeyFamilyOf(cms.signers[0].signatureAlgorithm) : null;
  return [
    aCondition(
      THE_ALGORITHM_OF_THE_KEY,
      key !== null && signed === key,
      `clave ${key ?? "desconocida"}; firma ${cms?.signers[0]?.signatureAlgorithm ?? "ilegible"}`,
    ),
  ];
}

const inXml = (signature) => bytesOf(signature).toString("utf8");

function aXadesSignature(signature) {
  const xades = isAXadesSignature(inXml(signature));
  return [
    aCondition(
      A_XADES_SIGNATURE,
      xades,
      xades ? "una Signature de XMLDSig con QualifyingProperties" : "no es una firma XAdES",
    ),
  ];
}

const theEnvelope = (expected) => (signature) => {
  const envelope = theXadesEnvelope(inXml(signature), THE_DOCUMENT_ROOT, THE_EXTERNAL_URI);
  return [
    aCondition(
      THE_ENVELOPE_REQUESTED,
      envelope === expected,
      `envoltura leída: ${envelope ?? "ninguna"}`,
    ),
  ];
};

function theRoleAndThePlace(signature) {
  const signed = signsTheRoleAndThePlace(inXml(signature), THE_ROLE, THE_CITY);
  return [
    aCondition(
      THE_ROLE_AND_THE_PLACE_SIGNED,
      signed,
      signed
        ? `la firma declara el cargo ${THE_ROLE} y la ciudad ${THE_CITY}`
        : "la firma no declara el cargo y la ciudad pedidos",
    ),
  ];
}

function theSignatureInsideThePdf(signature) {
  const signed = isASignedPdf(bytesOf(signature));
  return [
    aCondition(
      THE_SIGNATURE_INSIDE_THE_PDF,
      signed,
      signed
        ? "un PDF con su diccionario /Sig y su /ByteRange"
        : "no es un PDF con la firma dentro",
    ),
  ];
}

/** El ZIP trae la firma desprendida en `META-INF/signature.p7s` y el documento aparte, tal cual. */
function theDataAndTheSignatureInside(signature) {
  const entries = theZipEntries(bytesOf(signature));
  const cms = entries?.has("META-INF/signature.p7s")
    ? theCmsSignature(entries.get("META-INF/signature.p7s"))
    : null;
  const data = [...(entries ?? [])].find(
    ([name, content]) =>
      !name.startsWith("META-INF/") && name !== "mimetype" && content.equals(theChallenge()),
  );
  const inside = cms !== null && cms.content === null && data !== undefined;
  return [
    aCondition(
      THE_DATA_AND_THE_SIGNATURE_INSIDE,
      inside,
      entries
        ? `entradas: ${[...entries.keys()].join(", ")}`
        : "lo que volvió no es un ZIP legible",
    ),
  ];
}

const measuringAll =
  (...measurings) =>
  (signature, certificate) =>
    measurings.flatMap((measuring) => measuring(signature, certificate));

/** Emite lo que `measuring` saque de la respuesta y cierra el trámite con ella. */
function answering(measuring, signature, certificate) {
  try {
    for (const condition of measuring ? measuring(signature, certificate) : []) {
      emit({ event: "condition", ...condition });
    }
  } catch (error) {
    process.stderr.write(`la sede no pudo medir la firma: ${error?.message}\n`);
  }
  settle({ event: "success", result: signature, certificate });
}

/** Un `sign()` en CAdES con un `tsaURL` de sintaxis inválida (BUG-23). */
function theSignWithABrokenTsaUrlScript() {
  theSignScript("CAdES", "mode=explicit\ntsaURL=http://tsa invalida", theChallenge());
}

const signing = (format, extraParams, content, measuring) => () =>
  theSignScript(format, extraParams, content(), measuring);

const cosigning = (format, extraParams, content, measuring) => () =>
  theCosignScript(format, extraParams, content(), measuring);

const theChallengeHash = () => createHash("sha256").update(theChallenge()).digest();

const theHashSignedLeavingTheDataOut = measuringAll(
  signingTheData(THE_HASH_SIGNED_AS_IT_CAME, theChallenge),
  theDataLeftOut,
);

const countersigning = (target, content, measuring) => () =>
  theCountersignScript("CAdES", `target=${target}`, content(), measuring);

const theCadesImplicitSignature = () => theReferenceSignature("cades-implicit.p7s");
const theCountersignedCadesSignature = () =>
  theReferenceSignature("cades-implicit.countersign-tree.p7s");

export const SIGNATURE_SCRIPTS = {
  signcades: aPublishedScript(
    signing(
      "CAdES",
      "mode=explicit",
      theChallenge,
      measuringAll(
        theCertificateAndTheSignatureApart,
        withTheShape(A_SINGLE_SIGNER, "[]"),
        theDataLeftOut,
        theAlgorithmOfTheKey,
      ),
    ),
    { conditions: [APART, A_SINGLE_SIGNER, THE_DATA_LEFT_OUT, THE_ALGORITHM_OF_THE_KEY] },
  ),
  signcadesimplicit: aPublishedScript(
    signing("CAdES", "mode=implicit", theChallenge, theDataInside(theChallenge)),
    { ...NOT_YET_DRIVEN, conditions: [THE_DATA_INSIDE] },
  ),
  signcadesagepolicy: aPublishedScript(
    signing("CAdES", "expPolicy=FirmaAGE", theChallenge, theDataInside(theChallenge)),
    { ...NOT_YET_DRIVEN, conditions: [THE_DATA_INSIDE] },
  ),
  signgzip: aPublishedScript(
    signing(
      "CAdES",
      "mode=explicit",
      () => gzipSync(theChallenge()),
      signingTheData(THE_UNCOMPRESSED_DATA_SIGNED, theChallenge),
    ),
    {
      conditions: [THE_UNCOMPRESSED_DATA_SIGNED],
      patch: withTheDataDeclaredGzipped,
    },
  ),
  signcadesasics: aPublishedScript(
    signing(
      "CAdES-ASiC-S",
      "",
      theChallenge,
      measuringAll(theAsicContainer, theDataAndTheSignatureInside),
    ),
    { conditions: [A_ZIP_CONTAINER, THE_DATA_AND_THE_SIGNATURE_INSIDE] },
  ),
  signauto: aPublishedScript(signing("auto", "", theChallenge, allCades), {
    conditions: [A_CADES_SIGNATURE],
  }),
  signxades: aPublishedScript(signing("XAdES", "", theXmlDocument), { benchOnly: true }),
  signxadesauto: aPublishedScript(signing("auto", "", theXmlDocument, aXadesSignature), {
    conditions: [A_XADES_SIGNATURE],
  }),
  signxadesenveloping: aPublishedScript(
    signing("XAdES Enveloping", "", theXmlDocument, theEnvelope("enveloping")),
    { conditions: [THE_ENVELOPE_REQUESTED] },
  ),
  signxadesenveloped: aPublishedScript(
    signing("XAdES Enveloped", "", theXmlDocument, theEnvelope("enveloped")),
    { ...NOT_YET_DRIVEN, conditions: [THE_ENVELOPE_REQUESTED] },
  ),
  signxadesdetached: aPublishedScript(
    signing("XAdES Detached", "", theXmlDocument, theEnvelope("detached")),
    { ...NOT_YET_DRIVEN, conditions: [THE_ENVELOPE_REQUESTED] },
  ),
  signxadesexternallydetached: aPublishedScript(
    signing(
      "XAdES Externally Detached",
      `uri=${THE_EXTERNAL_URI}`,
      theXmlDocument,
      theEnvelope("externally-detached"),
    ),
    { ...NOT_YET_DRIVEN, conditions: [THE_ENVELOPE_REQUESTED] },
  ),
  signxadeswithatransform: aPublishedScript(
    signing(
      "XAdES Enveloping",
      `xmlTransforms=1\nxmlTransform0Type=${THE_DECLARED_TRANSFORM}\nxmlTransform0Body=/*`,
      theXmlDocument,
      theDeclaredTransform,
    ),
    { conditions: [THE_TRANSFORM_DECLARED] },
  ),
  signpades: aPublishedScript(signing("PAdES", "", thePdfOfTheTest, theSignatureInsideThePdf), {
    conditions: [THE_SIGNATURE_INSIDE_THE_PDF],
  }),
  signpadesoveranonpdf: aPublishedScript(signing("PAdES", "", theChallenge)),
  signpadeschecking: aPublishedScript(signing("PAdES", "checkSignatures=true", thePdfOfTheTest)),
  signpadesvisible: aPublishedScript(signing("PAdES", "visibleSignature=want", thePdfOfTheTest)),
  signfacturae: aPublishedScript(signing("FacturaE", "", theInvoice), { benchOnly: true }),
  signfacturaewitharole: aPublishedScript(
    signing(
      "FacturaE",
      `signerClaimedRoles=${THE_ROLE}\nsignatureProductionCity=${THE_CITY}`,
      theInvoice,
      theRoleAndThePlace,
    ),
    { conditions: [THE_ROLE_AND_THE_PLACE_SIGNED] },
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
    signing(
      "CAdES",
      "precalculatedHashAlgorithm=SHA-256",
      theChallengeHash,
      theHashSignedLeavingTheDataOut,
    ),
    { conditions: [THE_HASH_SIGNED_AS_IT_CAME, THE_DATA_LEFT_OUT] },
  ),
  signcadeswithaprecalculatedhashinimplicitmode: aPublishedScript(
    signing(
      "CAdES",
      "precalculatedHashAlgorithm=SHA-256\nmode=implicit",
      theChallengeHash,
      theHashSignedLeavingTheDataOut,
    ),
    { ...NOT_YET_DRIVEN, conditions: [THE_HASH_SIGNED_AS_IT_CAME, THE_DATA_LEFT_OUT] },
  ),
  signwithanunknownformat: aPublishedScript(signing("NoSuchFormat", "", theChallenge)),
  signwithoutaformat: aPublishedScript(signing(null, "", theChallenge)),
  signwithbrokentsa: aPublishedScript(theSignWithABrokenTsaUrlScript),
  cosigncades: aPublishedScript(
    cosigning("CAdES", "", theCadesImplicitSignature, withTheShape(TWO_PARALLEL_SIGNERS, "[][]")),
    { conditions: [TWO_PARALLEL_SIGNERS] },
  ),
  cosignauto: aPublishedScript(
    cosigning("auto", "", theCadesImplicitSignature, withTheShape(TWO_PARALLEL_SIGNERS, "[][]")),
    { conditions: [TWO_PARALLEL_SIGNERS] },
  ),
  cosignautooveracmssignature: aPublishedScript(
    cosigning("auto", "", theCmsSignatureOfTheSite, cosignedAsCades),
    { ...NOT_YET_DRIVEN, conditions: [COSIGNED_AS_CADES] },
  ),
  cosignautowithoutasignature: aPublishedScript(cosigning("auto", "", theXmlDocument)),
  cosignpadeschecking: aPublishedScript(
    cosigning("PAdES", "checkSignatures=true", thePdfOfTheTest),
  ),
  cosignfacturae: aPublishedScript(cosigning("FacturaE", "", theInvoice), { benchOnly: true }),
  countersigncades: aPublishedScript(
    countersigning(
      "tree",
      theCadesImplicitSignature,
      withTheShape(THE_SIGNER_COUNTERSIGNED, "[[]]"),
    ),
    { conditions: [THE_SIGNER_COUNTERSIGNED] },
  ),
  countersigncadestree: aPublishedScript(
    countersigning(
      "tree",
      theCountersignedCadesSignature,
      withTheShape(EVERY_NODE_COUNTERSIGNED, "[[[]][]]"),
    ),
    { ...NOT_YET_DRIVEN, conditions: [EVERY_NODE_COUNTERSIGNED] },
  ),
  countersigncadesanothertarget: aPublishedScript(
    countersigning(
      "signers",
      theCountersignedCadesSignature,
      withTheShape(ONLY_THE_LEAVES_COUNTERSIGNED, "[[[]]]"),
    ),
    { ...NOT_YET_DRIVEN, conditions: [ONLY_THE_LEAVES_COUNTERSIGNED] },
  ),
};
