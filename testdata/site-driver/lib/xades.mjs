// El analizador de firmas XML de la sede: envoltura, XAdES y atributos leídos por texto, y la verificación de cada Signature.

import { createHash, verify, X509Certificate } from "node:crypto";

import { canonicalize, parseXml, theElementsUnder, UnsupportedXml } from "./c14n.mjs";

const XMLDSIG = "http://www.w3.org/2000/09/xmldsig#";
const XADES = "http://uri.etsi.org/01903";
const ENVELOPED_TRANSFORM = `${XMLDSIG}enveloped-signature`;

const aTag = (local) => new RegExp(`<(/?)((?:[\\w.-]+:)?${local})(?=[\\s>/])`, "g");

function theRootElement(xml) {
  const body = xml.replace(/<\?[\s\S]*?\?>|<!--[\s\S]*?-->|<!DOCTYPE[^>]*>/g, "");
  return /<(?:[\w.-]+:)?([\w.-]+)/.exec(body)?.[1] ?? null;
}

/** Dónde empieza y acaba el elemento cuya etiqueta abre en `start`, contando anidados del mismo nombre. */
function theElementAt(xml, start) {
  const [, qname] = /^<([\w.:-]+)/.exec(xml.slice(start));
  const local = qname.split(":").at(-1);
  if (/^<[^>]*\/>/.test(xml.slice(start)))
    return { local, start, end: xml.indexOf(">", start) + 1 };
  const tags = aTag(local.replace(/[.]/g, "\\."));
  tags.lastIndex = start;
  let depth = 0;
  for (let match = tags.exec(xml); match; match = tags.exec(xml)) {
    if (match[2] !== qname) continue;
    const selfClosing = xml[xml.indexOf(">", match.index) - 1] === "/";
    if (match[1]) depth--;
    else if (!selfClosing) depth++;
    if (depth === 0) return { local, start, end: xml.indexOf(">", match.index) + 1 };
  }
  return { local, start, end: xml.length };
}

function theElementWithId(xml, id) {
  const at = xml.search(
    new RegExp(`\\s(?:Id|ID|id)="${id.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}"`),
  );
  return at < 0 ? null : theElementAt(xml, xml.lastIndexOf("<", at));
}

function theFirstSignature(xml) {
  const match = aTag("Signature").exec(xml);
  return match ? theElementAt(xml, match.index) : null;
}

function theReferences(xml) {
  return [...xml.matchAll(/<(?:[\w.-]+:)?Reference\b([^>]*)>/g)].map((match) => ({
    uri: /\bURI="([^"]*)"/.exec(match[1])?.[1] ?? null,
    type: /\bType="([^"]*)"/.exec(match[1])?.[1] ?? "",
  }));
}

const holdsTheDocument = (xml, element, documentRoot) =>
  new RegExp(`<(?:[\\w.-]+:)?${documentRoot}[\\s>/]`).test(xml.slice(element.start, element.end));

const inside = (element, outer) => element.start >= outer.start && element.end <= outer.end;

/** La envoltura de una firma XML: `enveloped`, `enveloping`, `detached`, `externally-detached` o `null`. */
export function theXadesEnvelope(xml, documentRoot, externalUri = null) {
  const signature = theFirstSignature(xml);
  if (!signature) return null;
  const root = theRootElement(xml);
  const references = theReferences(xml).filter(
    (reference) => !reference.type.includes("SignedProperties"),
  );
  if (externalUri && references.some((reference) => reference.uri === externalUri)) {
    return "externally-detached";
  }
  if (
    root === documentRoot &&
    references.some((reference) => reference.uri === "") &&
    xml.includes(ENVELOPED_TRANSFORM)
  ) {
    return "enveloped";
  }
  const targets = references
    .filter((reference) => reference.uri?.startsWith("#"))
    .map((reference) => theElementWithId(xml, reference.uri.slice(1)))
    .filter((target) => target && holdsTheDocument(xml, target, documentRoot));
  if (
    root === "Signature" &&
    targets.some((target) => target.local === "Object" && inside(target, signature))
  ) {
    return "enveloping";
  }
  if (
    root !== "Signature" &&
    root !== documentRoot &&
    targets.some((target) => !inside(target, signature))
  ) {
    return "detached";
  }
  return null;
}

/** Si el XML es una firma XAdES: una `Signature` de XMLDSig con sus `QualifyingProperties`. */
export function isAXadesSignature(xml) {
  return (
    xml.includes(XMLDSIG) &&
    xml.includes(XADES) &&
    aTag("Signature").test(xml) &&
    /<(?:[\w.-]+:)?QualifyingProperties[\s>]/.test(xml)
  );
}

const theTextsOf = (xml, local) =>
  [...xml.matchAll(new RegExp(`<(?:[\\w.-]+:)?${local}(?:\\s[^>]*)?>([^<]*)<`, "g"))].map((match) =>
    match[1].trim(),
  );

/** Si la firma declara el cargo `role` en `ClaimedRole` y la ciudad `city` en su lugar de producción. */
export function signsTheRoleAndThePlace(xml, role, city) {
  const place =
    /<(?:[\w.-]+:)?SignatureProductionPlace(?:V2)?[\s>][\s\S]*?<\/(?:[\w.-]+:)?SignatureProductionPlace(?:V2)?>/.exec(
      xml,
    );
  return (
    theTextsOf(xml, "ClaimedRole").includes(role) &&
    !!place &&
    theTextsOf(place[0], "City").includes(city)
  );
}

const C14N = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
const C14N_WITH_COMMENTS = `${C14N}#WithComments`;
const XPATH_FILTER = "http://www.w3.org/TR/1999/REC-xpath-19991116";
const WITHOUT_THE_SIGNATURES = /^not\(ancestor-or-self::(\w+):Signature\)$/;

const XML_DIGESTS = {
  [`${XMLDSIG}sha1`]: "sha1",
  "http://www.w3.org/2001/04/xmlenc#sha256": "sha256",
  "http://www.w3.org/2001/04/xmldsig-more#sha384": "sha384",
  "http://www.w3.org/2001/04/xmlenc#sha512": "sha512",
};

const XML_SIGNATURES = {
  [`${XMLDSIG}rsa-sha1`]: { hash: "sha1" },
  "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256": { hash: "sha256" },
  "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384": { hash: "sha384" },
  "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512": { hash: "sha512" },
  "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha1": { hash: "sha1", ec: true },
  "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha256": { hash: "sha256", ec: true },
  "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha384": { hash: "sha384", ec: true },
  "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha512": { hash: "sha512", ec: true },
};

const aDsigChild = (element, local) =>
  element?.children.find(
    (child) => child.type === "element" && child.namespace === XMLDSIG && child.local === local,
  );

const aDsigDescendant = (element, local) =>
  [...theElementsUnder(element)].find(
    (child) => child.namespace === XMLDSIG && child.local === local,
  );

const theAttribute = (element, name) =>
  element?.attributes.find((attribute) => attribute.name === name)?.value ?? null;

const theTextOf = (element) =>
  (element?.children ?? []).map((child) => (child.type === "text" ? child.value : "")).join("");

const inBase64 = (element) => Buffer.from(theTextOf(element).replace(/\s/g, ""), "base64");

const theElementChildren = (element) =>
  (element?.children ?? []).filter((child) => child.type === "element");

function unsupported(what) {
  throw new UnsupportedXml(what);
}

function theCanonicalizationOf(algorithm) {
  if (algorithm === C14N) return { withComments: false };
  if (algorithm === C14N_WITH_COMMENTS) return { withComments: true };
  return unsupported(`canonicalización ${algorithm}`);
}

function theReferencedNode(document, uri, elements) {
  if (uri === "") return document;
  if (!uri?.startsWith("#")) return unsupported(`referencia externa ${uri}`);
  const id = uri.slice(1);
  const target = elements.find((element) =>
    element.attributes.some(
      (attribute) => /^(?:Id|ID|id)$/.test(attribute.name) && attribute.value === id,
    ),
  );
  if (!target) throw new Error(`ningún elemento con Id ${id}`);
  return target;
}

function theExclusionsOf(transforms, signature, signatures) {
  const excluded = new Set();
  for (const transform of transforms) {
    const algorithm = theAttribute(transform, "Algorithm");
    if (algorithm === ENVELOPED_TRANSFORM) {
      excluded.add(signature);
    } else if (algorithm === XPATH_FILTER) {
      const expression = theTextOf(aDsigChild(transform, "XPath")).trim();
      const filter = WITHOUT_THE_SIGNATURES.exec(expression);
      if (!filter || aDsigChild(transform, "XPath").inScope.get(filter[1]) !== XMLDSIG) {
        unsupported(`XPath ${expression}`);
      }
      for (const other of signatures) excluded.add(other);
    } else if (algorithm !== C14N && algorithm !== C14N_WITH_COMMENTS) {
      unsupported(`transformación ${algorithm}`);
    }
  }
  return excluded;
}

function theReferenceHolds(reference, { document, elements, signature, signatures }) {
  const hash = XML_DIGESTS[theAttribute(aDsigChild(reference, "DigestMethod"), "Algorithm")];
  if (!hash) unsupported("algoritmo de resumen de una Reference");
  const transforms = theElementChildren(aDsigChild(reference, "Transforms"));
  const target = theReferencedNode(document, theAttribute(reference, "URI"), elements);
  const canonical = canonicalize(target, {
    excluded: theExclusionsOf(transforms, signature, signatures),
  });
  return createHash(hash)
    .update(canonical, "utf8")
    .digest()
    .equals(inBase64(aDsigChild(reference, "DigestValue")));
}

function theKeyOf(certificate) {
  try {
    return new X509Certificate(certificate).publicKey;
  } catch {
    return null;
  }
}

function aSignatureVerdict(signature, returnedKey, context) {
  const signedInfo = aDsigChild(signature, "SignedInfo");
  const method = aDsigChild(signedInfo, "CanonicalizationMethod");
  const canonical = canonicalize(
    signedInfo,
    theCanonicalizationOf(theAttribute(method, "Algorithm")),
  );
  const algorithm = theAttribute(aDsigChild(signedInfo, "SignatureMethod"), "Algorithm");
  const { hash, ec } = XML_SIGNATURES[algorithm] ?? unsupported(`firma ${algorithm}`);
  const value = inBase64(aDsigChild(signature, "SignatureValue"));
  const verifiesWith = (key) =>
    !!key &&
    verify(
      hash,
      Buffer.from(canonical, "utf8"),
      ec ? { key, dsaEncoding: "ieee-p1363" } : key,
      value,
    );
  const own = aDsigDescendant(aDsigChild(signature, "KeyInfo") ?? signature, "X509Certificate");
  const byTheReturned = verifiesWith(returnedKey);
  return {
    byTheReturned,
    verified: byTheReturned || verifiesWith(own ? theKeyOf(inBase64(own)) : null),
    referencesHold: theElementChildren(signedInfo)
      .filter((child) => child.local === "Reference")
      .every((reference) => theReferenceHolds(reference, { ...context, signature })),
  };
}

const aVerification = (verified, reason) => ({ verified, reason });

function theVerdictsOf(signatures, returnedKey, context) {
  try {
    return signatures.map((signature) => aSignatureVerdict(signature, returnedKey, context));
  } catch (error) {
    if (error instanceof UnsupportedXml) {
      return aVerification(null, `la sede no sabe verificar: ${error.message}`);
    }
    return aVerification(false, `firma ilegible: ${error.message}`);
  }
}

/** Si cada `Signature` verifica con su clave y sus Reference, y alguna con el certificado devuelto. */
export function theXadesVerification(xml, certificate) {
  let document;
  try {
    document = parseXml(xml);
  } catch (error) {
    return aVerification(
      error instanceof UnsupportedXml ? null : false,
      `XML ilegible: ${error.message}`,
    );
  }
  const elements = [...theElementsUnder(document)];
  const signatures = elements.filter(
    (element) => element.namespace === XMLDSIG && element.local === "Signature",
  );
  if (signatures.length === 0) return aVerification(false, "el XML no trae ninguna Signature");
  const returnedKey = theKeyOf(certificate);
  if (!returnedKey) return aVerification(false, "el certificado devuelto no es un X.509 DER");
  const verdicts = theVerdictsOf(signatures, returnedKey, { document, elements, signatures });
  if (!Array.isArray(verdicts)) return verdicts;
  if (!verdicts.every(({ referencesHold }) => referencesHold)) {
    return aVerification(false, "el resumen de alguna Reference no es el de lo que referencia");
  }
  if (!verdicts.every(({ verified }) => verified)) {
    return aVerification(false, "algún SignedInfo no verifica con la clave de su firmante");
  }
  if (!verdicts.some(({ byTheReturned }) => byTheReturned)) {
    return aVerification(false, "ninguna Signature verifica con el certificado devuelto");
  }
  return aVerification(
    true,
    "cada Signature verifica con sus Reference, una con el certificado devuelto",
  );
}
