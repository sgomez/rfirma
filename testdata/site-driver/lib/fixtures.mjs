// Los documentos que firman los guiones: los del banco de referencia y los congelados del lote.

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

/** Un PDF 1.4 de una página, armado aquí para que el carril PAdES no dependa de la prueba Rust. */
function aOnePagePdf() {
  const content = "BT /F1 24 Tf 72 750 Td (rfirma: suite de conformidad) Tj ET\n";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] " +
      "/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    `<< /Length ${content.length} >>\nstream\n${content}endstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
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
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefAt}\n%%EOF\n`;
  return Buffer.from(pdf, "latin1");
}

/** Una firma congelada del banco de referencia, la que reciben las multifirmas. */
export function theReferenceSignature(name) {
  return readFileSync(join(here, "../reference", name));
}
