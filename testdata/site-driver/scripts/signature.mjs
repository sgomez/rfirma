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
  aCertifiedPdf,
  aPasswordProtectedPdf,
  aPdfWithAnUnregisteredSignature,
  theChallenge,
  theCmsSignatureOfTheSite,
  theEd25519KeyStore,
  theInvoice,
  thePdfOfTheTest,
  theReferenceSignature,
  theXmlDocument,
} from "../lib/fixtures.mjs";
import { isASignedPdf, isAVisibleArea, theSignatureRectangles } from "../lib/pades.mjs";
import { withTheDataDeclaredGzipped } from "../lib/patches.mjs";
import { isABarePkcs1 } from "../lib/pkcs1.mjs";
import { aPublishedScript } from "../lib/script.mjs";
import { isAXadesSignature, signsTheRoleAndThePlace, theXadesEnvelope } from "../lib/xades.mjs";
import { theZipEntries } from "../lib/zip.mjs";
import { servletServing } from "./batch.mjs";

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
const COSIGNED_AS_CMS = "cosigned-as-cms";
const A_XADES_SIGNATURE = "a-xades-signature";
const THE_ENVELOPE_REQUESTED = "the-envelope-requested";
const THE_ROLE_AND_THE_PLACE_SIGNED = "the-role-and-the-place-signed";
const THE_SIGNATURE_INSIDE_THE_PDF = "the-signature-inside-the-pdf";
const THE_DATA_AND_THE_SIGNATURE_INSIDE = "the-data-and-the-signature-inside";
const A_BARE_PKCS1 = "a-bare-pkcs1";
const THROUGH_THE_TRIPHASE_SERVER = "through-the-triphase-server";
const THE_PRESIGNATURE_SIGNED_WITH_THE_KEY = "the-presignature-signed-with-the-key";
const THE_SERVER_SIGNATURE_AS_IT_CAME = "the-server-signature-as-it-came";
const WITHOUT_A_VISIBLE_SIGNATURE = "without-a-visible-signature";
const WHERE_THE_REQUEST_SAYS = "the-signature-where-the-request-says";
const THE_DEFAULT_ENVELOPE = "the-default-envelope";
const THE_SHA1_OF_THE_DATA_SIGNED = "the-sha1-of-the-data-signed";
const THE_ENVELOPE_THE_POLICY_DEMANDS = "the-envelope-the-policy-demands";
const THE_AGE_POLICY_INSIDE = "the-age-policy-inside";

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
  theCosignScriptWith("SHA256withRSA", format, extraParams, content, measuring);
}

/** Un `cosign()` con el algoritmo del guion. */
function theCosignScriptWith(algorithm, format, extraParams, content, measuring) {
  AutoScript.cosign(
    content.toString("base64"),
    algorithm,
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

const cosignedAsCms = onTheCms(
  COSIGNED_AS_CMS,
  (cms) => theShapeOf(cms) === "[][]" && cms.signers.every((signer) => !signer.cades),
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

const theEnvelope =
  (expected, name = THE_ENVELOPE_REQUESTED) =>
  (signature) => {
    const envelope = theXadesEnvelope(inXml(signature), THE_DOCUMENT_ROOT, THE_EXTERNAL_URI);
    return [aCondition(name, envelope === expected, `envoltura leída: ${envelope ?? "ninguna"}`)];
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

/** La firma NONE es el PKCS#1 de los datos, sin envoltorio, verificable con la clave del certificado. */
function aBarePkcs1(signature, certificate) {
  const bare = isABarePkcs1(theChallenge(), bytesOf(signature), bytesOf(certificate));
  return [
    aCondition(
      A_BARE_PKCS1,
      bare,
      bare
        ? `un PKCS#1 de ${bytesOf(signature).length} bytes que la clave del certificado verifica`
        : "lo que volvió no es un PKCS#1 de los datos con la clave del certificado",
    ),
  ];
}

/** El área que piden los guiones de firma visible colocada: página 1, `[llx, lly, urx, ury]`. */
const THE_AREA = [100, 100, 300, 200];

const theAreaParams = [
  "signaturePage=1",
  `signaturePositionOnPageLowerLeftX=${THE_AREA[0]}`,
  `signaturePositionOnPageLowerLeftY=${THE_AREA[1]}`,
  `signaturePositionOnPageUpperRightX=${THE_AREA[2]}`,
  `signaturePositionOnPageUpperRightY=${THE_AREA[3]}`,
].join("\n");

const theAreasOf = (signature) => theSignatureRectangles(bytesOf(signature));

const describingTheAreas = (areas) =>
  areas.length > 0
    ? `campos de firma con /Rect ${areas.map((area) => `[${area.join(" ")}]`).join(", ")}`
    : "ningún campo de firma con /Rect";

function withoutAVisibleSignature(signature) {
  const areas = theAreasOf(signature);
  const invisible = areas.length > 0 && !areas.some(isAVisibleArea);
  return [aCondition(WITHOUT_A_VISIBLE_SIGNATURE, invisible, describingTheAreas(areas))];
}

function whereTheRequestSays(signature) {
  const areas = theAreasOf(signature);
  const placed = areas.some((area) => area.every((at, i) => Math.abs(at - THE_AREA[i]) < 1));
  return [aCondition(WHERE_THE_REQUEST_SAYS, placed, describingTheAreas(areas))];
}

/** Un XAdES explícito firma el SHA-1 de los datos, declarado como `hash/sha1`. */
function theSha1OfTheData(signature) {
  const xml = inXml(signature);
  const sha1 = createHash("sha1").update(theXmlDocument()).digest("base64");
  const signed = xml.includes("hash/sha1") && xml.includes(sha1);
  return [
    aCondition(
      THE_SHA1_OF_THE_DATA_SIGNED,
      signed,
      signed
        ? "la firma lleva el SHA-1 de los datos con mimeType hash/sha1"
        : "la firma no lleva el SHA-1 de los datos declarado como hash/sha1",
    ),
  ];
}

/** La política de la AGE que expande `expPolicy=FirmaAGE`, en `policy.properties` de AutoFirma. */
const THE_AGE_POLICY_OID = "2.16.724.1.3.1.1.2.1.9";
const THE_AGE_POLICY_HASH = "G7roucf600+f03r/o0bAOQ6WAs0=";
const THE_AGE_POLICY_OID_IN_DER = Buffer.from("060a60855401030101020109", "hex");

const theAgePolicy = (held) => [
  aCondition(
    THE_AGE_POLICY_INSIDE,
    held,
    held
      ? `la firma declara la política ${THE_AGE_POLICY_OID} con su huella`
      : `la firma no declara la política ${THE_AGE_POLICY_OID} con su huella`,
  ),
];

const theAgePolicyInTheXml = (signature) =>
  theAgePolicy(
    inXml(signature).includes(THE_AGE_POLICY_OID) && inXml(signature).includes(THE_AGE_POLICY_HASH),
  );

const theAgePolicyInTheCms = (signature) =>
  theAgePolicy(
    bytesOf(signature).includes(THE_AGE_POLICY_OID_IN_DER) &&
      bytesOf(signature).includes(Buffer.from(THE_AGE_POLICY_HASH, "base64")),
  );

/** En un PAdES el CMS va en hexadecimal dentro de `/Contents`. */
function theAgePolicyInThePdf(signature) {
  const pdf = bytesOf(signature).toString("latin1").toLowerCase();
  return theAgePolicy(
    pdf.includes(THE_AGE_POLICY_OID_IN_DER.toString("hex")) &&
      pdf.includes(Buffer.from(THE_AGE_POLICY_HASH, "base64").toString("hex")),
  );
}

/** El PKCS#1 que el servidor trifásico de la sede pide firmar como prefirma. */
const THE_PRESIGNATURE = Buffer.from("prefirma que pide el servidor trifasico de la sede");

/** Lo que llegó al servidor trifásico en cada fase, para medirlo al cerrar el trámite. */
const whatTheTriphaseServerReceived = { pre: null, post: null };

/** El Base64 URL-safe con relleno que lee el `Base64` de AutoFirma. */
const inUrlSafeBase64 = (bytes) => bytes.toString("base64").replace(/\+/g, "-").replace(/\//g, "_");

const theTriphaseResult = () => theReferenceSignature("cades-implicit.p7s");

function theTriphasePresignature() {
  const xml =
    '<xml>\n <firmas format="CAdES">\n  <firma Id="1">\n' +
    `   <param n="PRE">${THE_PRESIGNATURE.toString("base64")}</param>\n` +
    "  </firma>\n </firmas>\n</xml>";
  return inUrlSafeBase64(Buffer.from(xml, "utf8"));
}

/** El servidor trifásico falso: `op=pre` devuelve la prefirma, `op=post` la firma congelada. */
function theTriphaseServer(query) {
  const received = Object.fromEntries(
    ["op", "cop", "format", "doc", "cert", "session"].map((name) => [name, query.get(name)]),
  );
  emit({ event: "triphase", op: String(received.op), cop: String(received.cop) });
  if (received.op === "pre") {
    whatTheTriphaseServerReceived.pre = received;
    return { status: 200, body: theTriphasePresignature() };
  }
  if (received.op === "post") {
    whatTheTriphaseServerReceived.post = received;
    return { status: 200, body: `OK NEWID=${inUrlSafeBase64(theTriphaseResult())}` };
  }
  return { status: 400, body: "ERR-01: operación trifásica desconocida" };
}

/** El PKCS#1 que la postfirma trae en `session` verifica la prefirma con el certificado de `cert`. */
function thePresignatureSignedWithTheKey(post) {
  if (!post?.session || !post.cert) return false;
  const session = bytesOf(post.session).toString("utf8");
  const pk1 = /<param n="PK1">([^<]+)<\/param>/.exec(session)?.[1];
  const certificate = bytesOf(post.cert.split(",")[0]);
  return !!pk1 && isABarePkcs1(THE_PRESIGNATURE, bytesOf(pk1), certificate);
}

const theTriphaseConditions = (cop, content) => (signature) => {
  const { pre, post } = whatTheTriphaseServerReceived;
  const through =
    pre?.cop === cop && post?.cop === cop && !!pre.doc && bytesOf(pre.doc).equals(content());
  const signedWithTheKey = thePresignatureSignedWithTheKey(post);
  const asItCame = bytesOf(signature).equals(theTriphaseResult());
  return [
    aCondition(
      THROUGH_THE_TRIPHASE_SERVER,
      through,
      through
        ? `la prefirma y la postfirma llegaron al serverUrl con cop=${cop} y los datos en doc`
        : `el serverUrl no recibió la prefirma y la postfirma con cop=${cop} y los datos`,
    ),
    aCondition(
      THE_PRESIGNATURE_SIGNED_WITH_THE_KEY,
      signedWithTheKey,
      signedWithTheKey
        ? "la postfirma trae el PK1 de la prefirma, verificable con el certificado"
        : "la postfirma no trae un PK1 de la prefirma que verifique el certificado",
    ),
    aCondition(
      THE_SERVER_SIGNATURE_AS_IT_CAME,
      asItCame,
      asItCame
        ? "la sede recibió la firma que devolvió la postfirma"
        : "la sede recibió algo distinto de la firma que devolvió la postfirma",
    ),
  ];
};

const THE_TRIPHASE_CONDITIONS = [
  THROUGH_THE_TRIPHASE_SERVER,
  THE_PRESIGNATURE_SIGNED_WITH_THE_KEY,
  THE_SERVER_SIGNATURE_AS_IT_CAME,
];

const THE_OPERATIONS = {
  sign: theSignScript,
  cosign: theCosignScript,
  countersign: theCountersignScript,
};

/** Una operación `CAdEStri` cuyo `serverUrl` es el servidor trifásico falso de la sede. */
const triphasing = (cop, content) => async () => {
  const serverUrl = await servletServing(theTriphaseServer);
  THE_OPERATIONS[cop](
    "CAdEStri",
    `serverUrl=${serverUrl}`,
    content(),
    theTriphaseConditions(cop, content),
  );
};

/** El error con el que el servidor trifásico falso contesta a la prefirma: una excepción suya. */
const A_PRESIGNATURE_FAILURE =
  "ERR-14:prefirma:java.io.IOException: la sede no entrega el documento";

/** Una firma `CAdEStri` cuyo servidor trifásico falla al preparar la prefirma. */
async function theFailingTriphaseServerScript() {
  const serverUrl = await servletServing(() => ({ status: 200, body: A_PRESIGNATURE_FAILURE }));
  theSignScript("CAdEStri", `serverUrl=${serverUrl}`, theChallenge());
}

/** Una cofirma SHA-512 de una firma separada cuya única huella es SHA-256: no hay qué cofirmar. */
function theCosignWithoutTheDataScript() {
  theCosignScriptWith("SHA512withRSA", "CAdES", "", theReferenceSignature("cades-explicit.p7s"));
}

/** Una firma con el almacén PKCS#12 de la sede, cuya única clave es Ed25519. */
function theSignWithAnUnsupportedKeyTypeScript() {
  AutoScript.setKeyStore(`PKCS12:${theEd25519KeyStore()}`);
  theSignScript("CAdES", "", theChallenge());
}

/** Un `signAndSaveToFile()` de un PAdES con `visibleSignature=want`. */
function theSignAndSaveOfAWantedVisibleSignatureScript() {
  AutoScript.signAndSaveToFile(
    "sign",
    thePdfOfTheTest().toString("base64"),
    "SHA256withRSA",
    "PAdES",
    "visibleSignature=want",
    "documento-firmado.pdf",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    settlingTheError,
  );
}

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
    { conditions: [THE_DATA_INSIDE] },
  ),
  signcadesagepolicy: aPublishedScript(
    signing(
      "CAdES",
      "expPolicy=FirmaAGE",
      theChallenge,
      measuringAll(theDataInside(theChallenge), theAgePolicyInTheCms),
    ),
    { conditions: [THE_DATA_INSIDE, THE_AGE_POLICY_INSIDE] },
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
  signxades: aPublishedScript(
    signing("XAdES", "", theXmlDocument, theEnvelope("enveloping", THE_DEFAULT_ENVELOPE)),
    { conditions: [THE_DEFAULT_ENVELOPE] },
  ),
  signxadesauto: aPublishedScript(signing("auto", "", theXmlDocument, aXadesSignature), {
    conditions: [A_XADES_SIGNATURE],
  }),
  signxadesenveloping: aPublishedScript(
    signing("XAdES Enveloping", "", theXmlDocument, theEnvelope("enveloping")),
    { conditions: [THE_ENVELOPE_REQUESTED] },
  ),
  signxadesenveloped: aPublishedScript(
    signing("XAdES Enveloped", "", theXmlDocument, theEnvelope("enveloped")),
    { conditions: [THE_ENVELOPE_REQUESTED] },
  ),
  signxadesdetached: aPublishedScript(
    signing("XAdES Detached", "", theXmlDocument, theEnvelope("detached")),
    { conditions: [THE_ENVELOPE_REQUESTED] },
  ),
  signxadesexternallydetached: aPublishedScript(
    signing(
      "XAdES",
      `format=XAdES Externally Detached\nuri=${THE_EXTERNAL_URI}`,
      theXmlDocument,
      theEnvelope("externally-detached"),
    ),
    { conditions: [THE_ENVELOPE_REQUESTED] },
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
    { conditions: [THE_HASH_SIGNED_AS_IT_CAME, THE_DATA_LEFT_OUT] },
  ),
  signwithanunknownformat: aPublishedScript(signing("NoSuchFormat", "", theChallenge)),
  signwithoutaformat: aPublishedScript(signing(null, "", theChallenge)),
  signwithbrokentsa: aPublishedScript(theSignWithABrokenTsaUrlScript),
  signwithanunknownpolicy: aPublishedScript(
    signing("CAdES", "expPolicy=NoSuchPolicy", theChallenge),
  ),
  signxadesenvelopedoveranonxml: aPublishedScript(
    signing("XAdES", "format=XAdES Enveloped", theChallenge),
  ),
  signooxmloveranonooxml: aPublishedScript(signing("OOXML", "", theChallenge)),
  signfacturaeoverasignedinvoice: aPublishedScript(
    signing("FacturaE", "", () => theReferenceSignature("facturae.xsig")),
  ),
  signfacturaeoveranoninvoice: aPublishedScript(signing("FacturaE", "", theXmlDocument)),
  signcadestriwithafailingserver: aPublishedScript(theFailingTriphaseServerScript),
  signwithanunsupportedkeytype: aPublishedScript(theSignWithAnUnsupportedKeyTypeScript),
  signpadescertifiedheadless: aPublishedScript(signing("PAdES", "headless=true", aCertifiedPdf)),
  signpadesunregisteredheadless: aPublishedScript(
    signing("PAdES", "headless=true", aPdfWithAnUnregisteredSignature),
  ),
  signpadesprotectedheadless: aPublishedScript(
    signing("PAdES", "headless=true", aPasswordProtectedPdf),
  ),
  signpadescertified: aPublishedScript(
    signing("PAdES", "", aCertifiedPdf, theSignatureInsideThePdf),
    { conditions: [THE_SIGNATURE_INSIDE_THE_PDF] },
  ),
  signpadescertifiedallowed: aPublishedScript(
    signing(
      "PAdES",
      "headless=true\nallowSigningCertifiedPdfs=true",
      aCertifiedPdf,
      theSignatureInsideThePdf,
    ),
    { conditions: [THE_SIGNATURE_INSIDE_THE_PDF] },
  ),
  signpadesunregisteredallowed: aPublishedScript(
    signing(
      "PAdES",
      "headless=true\nallowCosigningUnregisteredSignatures=true",
      aPdfWithAnUnregisteredSignature,
      theSignatureInsideThePdf,
    ),
    { conditions: [THE_SIGNATURE_INSIDE_THE_PDF] },
  ),
  signpadesprotectedwithitspassword: aPublishedScript(
    signing("PAdES", "headless=true\nuserPassword=1234", aPasswordProtectedPdf),
  ),
  cosigncades: aPublishedScript(
    cosigning("CAdES", "", theCadesImplicitSignature, withTheShape(TWO_PARALLEL_SIGNERS, "[][]")),
    { conditions: [TWO_PARALLEL_SIGNERS] },
  ),
  cosignauto: aPublishedScript(
    cosigning("auto", "", theCadesImplicitSignature, withTheShape(TWO_PARALLEL_SIGNERS, "[][]")),
    { conditions: [TWO_PARALLEL_SIGNERS] },
  ),
  cosignautooveracmssignature: aPublishedScript(
    cosigning("auto", "", theCmsSignatureOfTheSite, cosignedAsCms),
    { conditions: [COSIGNED_AS_CMS] },
  ),
  cosignautowithoutasignature: aPublishedScript(cosigning("auto", "", theXmlDocument)),
  cosignpadeschecking: aPublishedScript(
    cosigning("PAdES", "checkSignatures=true", thePdfOfTheTest),
  ),
  cosignfacturae: aPublishedScript(cosigning("FacturaE", "", theInvoice)),
  cosignxadesoveranonsignature: aPublishedScript(cosigning("XAdES", "", theChallenge)),
  cosigncadeswithoutthedata: aPublishedScript(theCosignWithoutTheDataScript),
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
    { conditions: [EVERY_NODE_COUNTERSIGNED] },
  ),
  countersigncadesanothertarget: aPublishedScript(
    countersigning(
      "signers",
      theCountersignedCadesSignature,
      withTheShape(ONLY_THE_LEAVES_COUNTERSIGNED, "[[[]]]"),
    ),
    { conditions: [ONLY_THE_LEAVES_COUNTERSIGNED] },
  ),
  signnone: aPublishedScript(signing("NONE", "", theChallenge, aBarePkcs1), {
    conditions: [A_BARE_PKCS1],
  }),
  signcadestri: aPublishedScript(triphasing("sign", theChallenge), {
    conditions: THE_TRIPHASE_CONDITIONS,
  }),
  cosigncadestri: aPublishedScript(triphasing("cosign", theCadesImplicitSignature), {
    conditions: THE_TRIPHASE_CONDITIONS,
  }),
  countersigncadestri: aPublishedScript(triphasing("countersign", theCadesImplicitSignature), {
    conditions: THE_TRIPHASE_CONDITIONS,
  }),
  signcadestriwithoutserverurl: aPublishedScript(signing("CAdEStri", "", theChallenge)),
  signpadesoptional: aPublishedScript(
    signing("PAdES", "visibleSignature=optional", thePdfOfTheTest, withoutAVisibleSignature),
    { conditions: [WITHOUT_A_VISIBLE_SIGNATURE] },
  ),
  signpadeswantedwithanarea: aPublishedScript(
    signing(
      "PAdES",
      `visibleSignature=want\n${theAreaParams}`,
      thePdfOfTheTest,
      whereTheRequestSays,
    ),
    { conditions: [WHERE_THE_REQUEST_SAYS] },
  ),
  signpadesplaced: aPublishedScript(
    signing("PAdES", theAreaParams, thePdfOfTheTest, whereTheRequestSays),
    { conditions: [WHERE_THE_REQUEST_SAYS] },
  ),
  signandsavepadesvisible: aPublishedScript(theSignAndSaveOfAWantedVisibleSignatureScript),
  signcadeswithoutmode: aPublishedScript(signing("CAdES", "", theChallenge, theDataLeftOut), {
    conditions: [THE_DATA_LEFT_OUT],
  }),
  signxadesexplicit: aPublishedScript(
    signing("XAdES", "mode=explicit", theXmlDocument, theSha1OfTheData),
    { conditions: [THE_SHA1_OF_THE_DATA_SIGNED] },
  ),
  signxadesagepolicy: aPublishedScript(
    signing(
      "XAdES",
      "format=XAdES Enveloping\nexpPolicy=FirmaAGE",
      theXmlDocument,
      measuringAll(theEnvelope("detached", THE_ENVELOPE_THE_POLICY_DEMANDS), theAgePolicyInTheXml),
    ),
    { conditions: [THE_ENVELOPE_THE_POLICY_DEMANDS, THE_AGE_POLICY_INSIDE] },
  ),
  signpadesagepolicy: aPublishedScript(
    signing("PAdES", "expPolicy=FirmaAGE", thePdfOfTheTest, theAgePolicyInThePdf),
    { conditions: [THE_AGE_POLICY_INSIDE] },
  ),
  countersignpades: aPublishedScript(() => theCountersignScript("PAdES", "", thePdfOfTheTest())),
  countersignfacturae: aPublishedScript(() => theCountersignScript("FacturaE", "", theInvoice())),
};
