// La canonicalización XML 1.0 inclusiva de la sede, sobre su propio lector de XML; no es C14N exclusiva ni 1.1.

const XML_NAMESPACE = "http://www.w3.org/XML/1998/namespace";

const ENTITIES = { lt: "<", gt: ">", amp: "&", quot: '"', apos: "'" };

/** La canonicalización XML que la sede no sabe hacer, para que quien la pida no la tome por un fallo. */
export class UnsupportedXml extends Error {}

function decoded(raw) {
  return raw.replace(/&(#x[0-9a-fA-F]+|#[0-9]+|\w+);/g, (_, name) => {
    if (name.startsWith("#x")) return String.fromCodePoint(Number.parseInt(name.slice(2), 16));
    if (name.startsWith("#")) return String.fromCodePoint(Number.parseInt(name.slice(1), 10));
    if (name in ENTITIES) return ENTITIES[name];
    throw new UnsupportedXml(`entidad &${name}; sin declarar`);
  });
}

function theNameParts(qname) {
  const colon = qname.indexOf(":");
  return colon < 0
    ? { prefix: "", local: qname }
    : { prefix: qname.slice(0, colon), local: qname.slice(colon + 1) };
}

function theNamespaceOf(prefix, inScope) {
  if (prefix === "xml") return XML_NAMESPACE;
  const uri = inScope.get(prefix);
  if (uri === undefined && prefix !== "") throw new Error(`prefijo «${prefix}» sin declarar`);
  return uri ?? "";
}

const ATTRIBUTE = /\s*([^\s=/>]+)\s*=\s*(?:"([^"]*)"|'([^']*)')/y;

function anElement(tag, parent) {
  const [, qname] = /^<([^\s/>]+)/.exec(tag);
  const rawAttributes = [];
  ATTRIBUTE.lastIndex = qname.length + 1;
  for (let match = ATTRIBUTE.exec(tag); match; match = ATTRIBUTE.exec(tag)) {
    const raw = match[2] ?? match[3];
    rawAttributes.push([match[1], decoded(raw.replace(/[\t\n]/g, " "))]);
  }
  const inScope = new Map(parent.inScope);
  const declarations = new Map();
  const attributes = [];
  for (const [name, value] of rawAttributes) {
    if (name === "xmlns") declarations.set("", value);
    else if (name.startsWith("xmlns:")) declarations.set(name.slice(6), value);
    else attributes.push({ name, value });
  }
  for (const [prefix, uri] of declarations) inScope.set(prefix, uri);
  const { prefix, local } = theNameParts(qname);
  return {
    type: "element",
    qname,
    local,
    namespace: theNamespaceOf(prefix, inScope),
    inScope,
    attributes: attributes.map(({ name, value }) => {
      const parts = theNameParts(name);
      return {
        name,
        ...parts,
        namespace: parts.prefix ? theNamespaceOf(parts.prefix, inScope) : "",
        value,
      };
    }),
    children: [],
    parent,
  };
}

function theEndOf(text, marker, from) {
  const at = text.indexOf(marker, from);
  if (at < 0) throw new Error(`falta «${marker}»`);
  return at;
}

/** El árbol de un documento XML: elementos con sus espacios de nombres resueltos, textos, comentarios e instrucciones. */
export function parseXml(source) {
  const text = source.replace(/^﻿/, "").replace(/\r\n?/g, "\n");
  const document = { type: "document", children: [], inScope: new Map(), parent: null };
  let current = document;
  let at = 0;
  const append = (node) => current.children.push({ ...node, parent: current });
  while (at < text.length) {
    if (text.startsWith("<?", at)) {
      const end = theEndOf(text, "?>", at);
      const [, target, data = ""] = /^<\?(\S+)\s*([\s\S]*)$/.exec(text.slice(at, end));
      if (target.toLowerCase() !== "xml") append({ type: "pi", target, data });
      at = end + 2;
    } else if (text.startsWith("<!--", at)) {
      const end = theEndOf(text, "-->", at);
      append({ type: "comment", value: text.slice(at + 4, end) });
      at = end + 3;
    } else if (text.startsWith("<![CDATA[", at)) {
      const end = theEndOf(text, "]]>", at);
      append({ type: "text", value: text.slice(at + 9, end) });
      at = end + 3;
    } else if (text.startsWith("<!DOCTYPE", at)) {
      const end = theEndOf(text, ">", at);
      if (text.slice(at, end).includes("["))
        throw new UnsupportedXml("DOCTYPE con subconjunto interno");
      at = end + 1;
    } else if (text.startsWith("</", at)) {
      const end = theEndOf(text, ">", at);
      const qname = text.slice(at + 2, end).trim();
      if (current.type !== "element" || current.qname !== qname) {
        throw new Error(`cierre «${qname}» sin su apertura`);
      }
      current = current.parent;
      at = end + 1;
    } else if (text[at] === "<") {
      const end = theEndOf(text, ">", at);
      const tag = text.slice(at, end + 1);
      const element = anElement(tag, current);
      current.children.push(element);
      if (!tag.endsWith("/>")) current = element;
      at = end + 1;
    } else {
      const end = text.indexOf("<", at) < 0 ? text.length : text.indexOf("<", at);
      append({ type: "text", value: decoded(text.slice(at, end)) });
      at = end;
    }
  }
  if (current !== document) throw new Error(`«${current.qname}» sin cerrar`);
  return document;
}

/** Todos los elementos del árbol bajo `node`, en orden de documento. */
export function* theElementsUnder(node) {
  for (const child of node.children) {
    if (child.type !== "element") continue;
    yield child;
    yield* theElementsUnder(child);
  }
}

const inText = (value) =>
  value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\r/g, "&#xD;");

const inAttribute = (value) =>
  value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/"/g, "&quot;")
    .replace(/\t/g, "&#x9;")
    .replace(/\n/g, "&#xA;")
    .replace(/\r/g, "&#xD;");

const byCodePoints = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

function theXmlAttributesInherited(element) {
  const inherited = new Map();
  for (let ancestor = element.parent; ancestor?.type === "element"; ancestor = ancestor.parent) {
    for (const attribute of ancestor.attributes) {
      if (attribute.prefix === "xml" && !inherited.has(attribute.name)) {
        inherited.set(attribute.name, attribute);
      }
    }
  }
  const own = new Set(element.attributes.map((attribute) => attribute.name));
  return [...inherited.values()].filter((attribute) => !own.has(attribute.name));
}

function theStartTag(element, rendered, apex) {
  const declarations = [];
  const next = new Map(rendered);
  const defaultNamespace = element.inScope.get("") ?? "";
  if (defaultNamespace !== (rendered.get("") ?? "")) {
    declarations.push(["", defaultNamespace]);
    next.set("", defaultNamespace);
  }
  for (const [prefix, uri] of element.inScope) {
    if (prefix === "" || prefix === "xml" || rendered.get(prefix) === uri) continue;
    declarations.push([prefix, uri]);
    next.set(prefix, uri);
  }
  declarations.sort(([a], [b]) => byCodePoints(a, b));
  const attributes = [
    ...element.attributes,
    ...(apex ? theXmlAttributesInherited(element) : []),
  ].sort((a, b) => byCodePoints(a.namespace, b.namespace) || byCodePoints(a.local, b.local));
  const tag =
    `<${element.qname}` +
    declarations
      .map(([prefix, uri]) => ` ${prefix ? `xmlns:${prefix}` : "xmlns"}="${inAttribute(uri)}"`)
      .join("") +
    attributes.map((attribute) => ` ${attribute.name}="${inAttribute(attribute.value)}"`).join("") +
    ">";
  return { tag, next };
}

function aProcessingInstruction(node) {
  return `<?${node.target}${node.data ? ` ${node.data}` : ""}?>`;
}

function theCanonicalElement(element, rendered, options, apex = false) {
  const { tag, next } = theStartTag(element, rendered, apex);
  const inside = element.children.map((child) => theCanonicalChild(child, next, options)).join("");
  return `${tag}${inside}</${element.qname}>`;
}

function theCanonicalChild(node, rendered, options) {
  if (node.type === "element") {
    return options.excluded.has(node) ? "" : theCanonicalElement(node, rendered, options);
  }
  if (node.type === "text") return inText(node.value);
  if (node.type === "comment") return options.withComments ? `<!--${node.value}-->` : "";
  return aProcessingInstruction(node);
}

function theCanonicalDocument(document, options) {
  const root = document.children.findIndex((node) => node.type === "element");
  return document.children
    .map((node, index) => {
      if (node.type === "element") {
        return options.excluded.has(node)
          ? ""
          : theCanonicalElement(node, new Map(), options, true);
      }
      if (node.type === "text" || (node.type === "comment" && !options.withComments)) return "";
      const serialized =
        node.type === "comment" ? `<!--${node.value}-->` : aProcessingInstruction(node);
      return index < root ? `${serialized}\n` : `\n${serialized}`;
    })
    .join("");
}

/** La forma canónica (C14N 1.0 inclusiva) del documento o del subárbol `node`, sin los elementos de `excluded`. */
export function canonicalize(node, { excluded = new Set(), withComments = false } = {}) {
  const options = { excluded, withComments };
  if (node.type === "document") return theCanonicalDocument(node, options);
  return theCanonicalElement(node, new Map(), options, true);
}
