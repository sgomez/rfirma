// Tabla: para cada campo /Sig de cada PDF del banco, lo que pdf.js expone
// (fieldValue, hasAppearance, annotationFlags) frente a la verdad (/V presente).
import { readFileSync } from "node:fs";

const PDFJS = "/home/sergio/Developer/SideProjects/rfirma/rfirma-app/node_modules/pdfjs-dist/legacy/build/pdf.mjs";
const { getDocument } = await import(PDFJS);

const ficheros = ["empty-fields.pdf", "signed-field.pdf", "signed-invisible.pdf"];
console.log("fichero               campo            fieldValue hasAppearance flags");
for (const f of ficheros) {
  const doc = await getDocument({ data: new Uint8Array(readFileSync(f)) }).promise;
  for (let n = 1; n <= doc.numPages; n++) {
    for (const a of await (await doc.getPage(n)).getAnnotations()) {
      if (a.fieldType !== "Sig") continue;
      console.log(
        `${f.padEnd(21)} ${String(a.fieldName).padEnd(16)} ${String(a.fieldValue).padEnd(10)} ` +
        `${String(a.hasAppearance).padEnd(13)} ${a.annotationFlags}`);
    }
  }
}
