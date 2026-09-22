// El analizador de firmas XML de la sede: envoltura, XAdES y atributos firmados, leídos por texto y sin validar.

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
