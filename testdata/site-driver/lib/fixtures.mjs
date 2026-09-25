// Los documentos que firman los guiones: los del banco de referencia y los congelados del lote.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = join(dirname(fileURLToPath(import.meta.url)), "..");

export function theFrozen(fixture) {
  return readFileSync(join(here, fixture), "utf8").trim();
}

/** El reto de 64 bytes del banco de referencia, el mismo que firman los CAdES. */
export function theChallenge() {
  return readFileSync(join(here, "../reference/challenge.bin"));
}

/** El XML del banco de referencia, el que firman los guiones XAdES de `sign`. */
export function theXmlDocument() {
  return readFileSync(join(here, "../reference/document.xml"));
}

/** La factura de referencia, la que firma el guion FacturaE de `sign`. */
export function theInvoice() {
  return readFileSync(join(here, "../reference/invoice.xml"));
}

/** El PDF que firma `format=PAdES`: el que deje la prueba Rust en disco, o uno de una página. */
export function thePdfOfTheTest() {
  return process.env.RFIRMA_BENCH_PDF ? readFileSync(process.env.RFIRMA_BENCH_PDF) : aOnePagePdf();
}

const THE_PAGE_CONTENT = "BT /F1 24 Tf 72 750 Td (rfirma: suite de conformidad) Tj ET\n";

/** Los cinco objetos de un PDF de una página; `catalogue` añade entradas al catálogo. */
function theObjectsOfOnePage({ catalogue = "", content = THE_PAGE_CONTENT } = {}) {
  return [
    `<< /Type /Catalog /Pages 2 0 R ${catalogue}>>`,
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] " +
      "/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    `<< /Length ${content.length} >>\nstream\n${content}endstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
}

/** Un PDF 1.4 con `objects` numerados desde 1 y `trailer` añadido a su tráiler. */
function aPdfOf(objects, trailer = "") {
  let pdf = "%PDF-1.4\n";
  const offsets = [];
  objects.forEach((body, index) => {
    offsets.push(pdf.length);
    pdf += `${index + 1} 0 obj\n${body}\nendobj\n`;
  });
  const xrefAt = pdf.length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets) {
    pdf += `${String(offset).padStart(10, "0")} 00000 n \n`;
  }
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R ${trailer}>>\nstartxref\n${xrefAt}\n%%EOF\n`;
  return Buffer.from(pdf, "latin1");
}

/** Un PDF 1.4 de una página, armado aquí para que el carril PAdES no dependa de la prueba Rust. */
function aOnePagePdf() {
  return aPdfOf(theObjectsOfOnePage());
}

/** Un PDF certificado sin cambios permitidos: su DocMDP declara `P 1`. */
export function aCertifiedPdf() {
  return aPdfOf([
    ...theObjectsOfOnePage({ catalogue: "/Perms << /DocMDP 6 0 R >> " }),
    "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /Contents <00> " +
      "/ByteRange [0 0 0 0] /Reference [<< /Type /SigRef /TransformMethod /DocMDP " +
      "/TransformParams << /Type /TransformParams /P 1 /V /1.2 >> >>] >>",
  ]);
}

/** Un PDF con una firma de un subfiltro que el original no registra, `adbe.x509.rsa_sha1`. */
export function aPdfWithAnUnregisteredSignature() {
  return aPdfOf([
    ...theObjectsOfOnePage({ catalogue: "/AcroForm << /Fields [7 0 R] /SigFlags 3 >> " }),
    "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.x509.rsa_sha1 /Contents <00> >>",
    "<< /FT /Sig /T (Firma1) /V 6 0 R /Type /Annot /Subtype /Widget /Rect [0 0 0 0] /P 3 0 R >>",
  ]);
}

const THE_PASSWORD_PADDING = Buffer.from(
  "28bf4e5e4e758a4164004e56fffa01082e2e00b6d0683e802f0ca9fe6453697a",
  "hex",
);
const THE_PDF_ID = Buffer.from("7266697266697266697266697266696d", "hex");

function rc4(key, data) {
  const state = Array.from({ length: 256 }, (_, index) => index);
  for (let i = 0, j = 0; i < 256; i++) {
    j = (j + state[i] + key[i % key.length]) & 255;
    [state[i], state[j]] = [state[j], state[i]];
  }
  const out = Buffer.alloc(data.length);
  for (let n = 0, i = 0, j = 0; n < data.length; n++) {
    i = (i + 1) & 255;
    j = (j + state[i]) & 255;
    [state[i], state[j]] = [state[j], state[i]];
    out[n] = data[n] ^ state[(state[i] + state[j]) & 255];
  }
  return out;
}

const md5 = (...parts) => createHash("md5").update(Buffer.concat(parts)).digest();
const padded = (password) =>
  Buffer.concat([Buffer.from(password, "latin1"), THE_PASSWORD_PADDING]).subarray(0, 32);
const asHex = (bytes) => `<${bytes.toString("hex")}>`;

/** Un PDF cifrado con RC4 de 40 bits (revisión 2), con `1234` de contraseña de usuario y de propietario. */
export function aPasswordProtectedPdf() {
  const permissions = Buffer.alloc(4);
  permissions.writeInt32LE(-4);
  const owner = rc4(md5(padded("1234")).subarray(0, 5), padded("1234"));
  const key = md5(padded("1234"), owner, permissions, THE_PDF_ID).subarray(0, 5);
  const user = rc4(key, THE_PASSWORD_PADDING);
  const objectKey = md5(key, Buffer.from([4, 0, 0, 0, 0])).subarray(0, 10);
  const content = rc4(objectKey, Buffer.from(THE_PAGE_CONTENT, "latin1")).toString("latin1");
  return aPdfOf(
    [
      ...theObjectsOfOnePage({ content }),
      `<< /Filter /Standard /V 1 /R 2 /Length 40 /P -4 /O ${asHex(owner)} /U ${asHex(user)} >>`,
    ],
    `/Encrypt 6 0 R /ID [${asHex(THE_PDF_ID)} ${asHex(THE_PDF_ID)}] `,
  );
}

/** La firma CMS sin atributos CAdES, hecha con OpenSSL y un certificado propio, junto a la sede. */
export function theCmsSignatureOfTheSite() {
  return readFileSync(join(here, "cms-implicit.p7s"));
}

/** Una firma congelada del banco de referencia, la que reciben las multifirmas. */
export function theReferenceSignature(name) {
  return readFileSync(join(here, "../reference", name));
}
