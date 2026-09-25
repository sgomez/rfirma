// El analizador de firmas CMS de la sede: lee DER y BER lo justo para medir firmantes y contenido, y verificarlos.

import { createHash, verify, X509Certificate } from "node:crypto";

const SIGNED_DATA = "1.2.840.113549.1.7.2";
const MESSAGE_DIGEST = "1.2.840.113549.1.9.4";
const COUNTERSIGNATURE = "1.2.840.113549.1.9.6";
const SIGNING_CERTIFICATE = "1.2.840.113549.1.9.16.2.12";
const SIGNING_CERTIFICATE_V2 = "1.2.840.113549.1.9.16.2.47";

const DIGESTS = {
  "1.3.14.3.2.26": "sha1",
  "2.16.840.1.101.3.4.2.1": "sha256",
  "2.16.840.1.101.3.4.2.2": "sha384",
  "2.16.840.1.101.3.4.2.3": "sha512",
};

const KEY_FAMILIES = [
  ["1.2.840.113549.1.1.", "rsa"],
  ["1.2.840.10045.", "ec"],
];

function theLength(bytes, at) {
  const first = bytes[at];
  if (first < 0x80) return { length: first, after: at + 1 };
  if (first === 0x80) return { length: null, after: at + 1 };
  let length = 0;
  for (let i = 1; i <= (first & 0x7f); i++) length = length * 256 + bytes[at + i];
  return { length, after: at + 1 + (first & 0x7f) };
}

/** Un nodo TLV en `at`, con sus hijos si es construido; admite la longitud indefinida de BER. */
function aNode(bytes, at) {
  if (at + 2 > bytes.length) throw new Error("TLV truncado");
  const tag = bytes[at];
  const constructed = (tag & 0x20) !== 0;
  const { length, after } = theLength(bytes, at + 1);
  if (length === null) {
    const children = [];
    let cursor = after;
    while (bytes[cursor] !== 0 || bytes[cursor + 1] !== 0) {
      const child = aNode(bytes, cursor);
      children.push(child);
      cursor = child.end;
      if (cursor + 2 > bytes.length) throw new Error("BER sin fin de contenido");
    }
    return { tag, constructed, children, start: at, end: cursor + 2 };
  }
  const end = after + length;
  if (end > bytes.length) throw new Error("TLV más largo que los datos");
  if (!constructed) return { tag, constructed, value: bytes.subarray(after, end), start: at, end };
  const children = [];
  for (let cursor = after; cursor < end; cursor = children.at(-1).end) {
    children.push(aNode(bytes, cursor));
  }
  return { tag, constructed, children, start: at, end };
}

/** El contenido de un OCTET STRING, primitivo o troceado en BER. */
function theOctets(node) {
  if (!node.constructed) return node.value;
  return Buffer.concat(node.children.map(theOctets));
}

export function theOid(node) {
  const bytes = node.value;
  const parts = [Math.floor(bytes[0] / 40), bytes[0] % 40];
  let value = 0;
  for (const byte of bytes.subarray(1)) {
    value = value * 128 + (byte & 0x7f);
    if ((byte & 0x80) === 0) {
      parts.push(value);
      value = 0;
    }
  }
  return parts.join(".");
}

const isContextTag = (node, number) => (node.tag & 0xdf) === (0x80 | number);

function theAttributes(node) {
  return new Map(
    node.children.map((attribute) => [
      theOid(attribute.children[0]),
      attribute.children[1].children,
    ]),
  );
}

/** Los atributos firmados como SET, que es lo que firma el firmante, y no con su etiqueta implícita. */
function theSignedAttributesAsASet(node, bytes) {
  const set = Buffer.from(bytes.subarray(node.start, node.end));
  set[0] = 0x31;
  return set;
}

function aSigner(node, bytes) {
  const fields = node.children;
  const withSignedAttributes = isContextTag(fields[3], 0);
  const signed = withSignedAttributes ? theAttributes(fields[3]) : new Map();
  const unsignedNode = fields.find((field, index) => index > 3 && isContextTag(field, 1));
  const unsigned = unsignedNode ? theAttributes(unsignedNode) : new Map();
  const digest = signed.get(MESSAGE_DIGEST)?.[0];
  return {
    digestAlgorithm: theOid(fields[2].children[0]),
    signatureAlgorithm: theOid(fields[withSignedAttributes ? 4 : 3].children[0]),
    messageDigest: digest ? theOctets(digest) : null,
    signedAttributes: withSignedAttributes ? theSignedAttributesAsASet(fields[3], bytes) : null,
    signature: theOctets(fields[withSignedAttributes ? 5 : 4]),
    cades: signed.has(SIGNING_CERTIFICATE_V2) || signed.has(SIGNING_CERTIFICATE),
    countersigners: (unsigned.get(COUNTERSIGNATURE) ?? []).map((child) => aSigner(child, bytes)),
  };
}

/** Los firmantes y el contenido de un `SignedData`, o `null` si los bytes no son uno. */
export function theCmsSignature(bytes) {
  try {
    const contentInfo = aNode(bytes, 0);
    if (theOid(contentInfo.children[0]) !== SIGNED_DATA) return null;
    const fields = contentInfo.children[1].children[0].children;
    const encapsulated = fields[2].children;
    const eContent = encapsulated[1]?.children[0];
    return {
      content: eContent ? theOctets(eContent) : null,
      signers: fields.at(-1).children.map((signer) => aSigner(signer, bytes)),
    };
  } catch {
    return null;
  }
}

function aShape(signer) {
  return `[${signer.countersigners.map(aShape).sort().join("")}]`;
}

/** La forma del árbol de firmantes: `[]` por firmante, con sus contrafirmas dentro, en orden canónico. */
export function theShapeOf(signature) {
  return signature.signers.map(aShape).sort().join("");
}

/** Si el `messageDigest` del firmante es el resumen de `data` con su propio algoritmo. */
export function signsTheData(signer, data) {
  const algorithm = DIGESTS[signer.digestAlgorithm];
  if (!algorithm || !signer.messageDigest) return false;
  return createHash(algorithm).update(data).digest().equals(signer.messageDigest);
}

/** La familia de clave de un OID de firma o de clave pública: `rsa`, `ec` o `null`. */
export function theKeyFamilyOf(oid) {
  return KEY_FAMILIES.find(([prefix]) => oid.startsWith(prefix))?.[1] ?? null;
}

/** El OID de la clave pública de un certificado X.509 en DER. */
export function thePublicKeyAlgorithmOf(certificate) {
  const tbs = aNode(certificate, 0).children[0].children;
  const offset = isContextTag(tbs[0], 0) ? 1 : 0;
  return theOid(tbs[offset + 5].children[0].children[0]);
}

/** Los certificados que viajan en el `SignedData`, en DER. */
export function theCertificatesIn(bytes) {
  const fields = aNode(bytes, 0).children[1].children[0].children;
  const certificates = fields.find((field, index) => index > 2 && isContextTag(field, 0));
  return (certificates?.children ?? []).map((node) => bytes.subarray(node.start, node.end));
}

const SIGNATURE_DIGESTS = {
  "1.2.840.113549.1.1.5": "sha1",
  "1.2.840.113549.1.1.11": "sha256",
  "1.2.840.113549.1.1.12": "sha384",
  "1.2.840.113549.1.1.13": "sha512",
  "1.2.840.10045.4.1": "sha1",
  "1.2.840.10045.4.3.2": "sha256",
  "1.2.840.10045.4.3.3": "sha384",
  "1.2.840.10045.4.3.4": "sha512",
};

const KEY_ONLY_SIGNATURES = new Set(["1.2.840.113549.1.1.1", "1.2.840.10045.2.1"]);

function theKeyOf(certificate) {
  try {
    return new X509Certificate(certificate).publicKey;
  } catch {
    return null;
  }
}

function theSignatureDigestOf(signer) {
  if (KEY_ONLY_SIGNATURES.has(signer.signatureAlgorithm)) return DIGESTS[signer.digestAlgorithm];
  return SIGNATURE_DIGESTS[signer.signatureAlgorithm];
}

/** Cada firmante con lo que firma: el contenido los de primer nivel, la firma de su padre las contrafirmas. */
function theSignersWithWhatTheySign(signers, content) {
  return signers.flatMap((signer) => [
    { signer, content },
    ...theSignersWithWhatTheySign(signer.countersigners, signer.signature),
  ]);
}

function theUnverifiableAlgorithmOf(signer) {
  const hashesWithItsDigest =
    signer.signedAttributes || KEY_ONLY_SIGNATURES.has(signer.signatureAlgorithm);
  if (hashesWithItsDigest && !DIGESTS[signer.digestAlgorithm]) return signer.digestAlgorithm;
  if (!theSignatureDigestOf(signer)) return signer.signatureAlgorithm;
  return null;
}

function theKeysVerifying(signer, content, keys) {
  const hash = theSignatureDigestOf(signer);
  const signed = signer.signedAttributes ?? content;
  return keys.filter((key) => {
    try {
      return verify(hash, signed, key, signer.signature);
    } catch {
      return false;
    }
  });
}

const aVerification = (verified, reason) => ({ verified, reason });

function aSignerVerdict(signer, signed, keys, returnedKey) {
  const measurableDigest = signer.signedAttributes && DIGESTS[signer.digestAlgorithm];
  if (measurableDigest && !signsTheData(signer, signed)) {
    return aVerification(false, "el messageDigest de un firmante no es el resumen de lo que firma");
  }
  const unverifiable = theUnverifiableAlgorithmOf(signer);
  if (unverifiable) return aVerification(null, `la sede no sabe verificar ${unverifiable}`);
  const verifying = theKeysVerifying(signer, signed, keys);
  if (verifying.length === 0) {
    return aVerification(false, "la firma de un firmante no verifica con ninguna clave");
  }
  return { ...aVerification(true), byTheReturned: verifying.includes(returnedKey) };
}

/** Si cada firmante del CMS, contrafirmas incluidas, verifica con su clave, y alguno con el certificado devuelto. */
export function theCmsVerification(bytes, certificate, detachedContent = null) {
  const cms = theCmsSignature(bytes);
  if (!cms) return aVerification(false, "lo que volvió no es un CMS SignedData");
  const content = cms.content ?? detachedContent;
  if (!content) return aVerification(false, "el CMS no lleva los datos y la sede no los conoce");
  const returnedKey = theKeyOf(certificate);
  if (!returnedKey) return aVerification(false, "el certificado devuelto no es un X.509 DER");
  const keys = [returnedKey, ...theCertificatesIn(bytes).map(theKeyOf).filter(Boolean)];
  const verdicts = theSignersWithWhatTheySign(cms.signers, content).map(
    ({ signer, content: signed }) => aSignerVerdict(signer, signed, keys, returnedKey),
  );
  const failure =
    verdicts.find(({ verified }) => verified === false) ??
    verdicts.find(({ verified }) => verified === null);
  if (failure) return failure;
  if (!verdicts.some(({ byTheReturned }) => byTheReturned)) {
    return aVerification(false, "ningún firmante verifica con el certificado devuelto");
  }
  return aVerification(
    true,
    "cada firmante verifica con su clave, uno con el certificado devuelto",
  );
}
