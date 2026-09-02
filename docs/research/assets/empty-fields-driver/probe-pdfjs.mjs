// Enumera anotaciones con el mismo pdfjs-dist que usa rfirma (6.3.289).
// Uso: node probe-pdfjs.mjs <fichero.pdf>
import { readFileSync } from "node:fs";

const PDFJS = "/home/sergio/Developer/SideProjects/rfirma/rfirma-app/node_modules/pdfjs-dist/legacy/build/pdf.mjs";
const { getDocument } = await import(PDFJS);

const bytes = new Uint8Array(readFileSync(process.argv[2]));
const doc = await getDocument({ data: bytes }).promise;
console.log("paginas:", doc.numPages);

const campos = await doc.getFieldObjects();
console.log("getFieldObjects():", JSON.stringify(campos, null, 1));

for (let n = 1; n <= doc.numPages; n++) {
  const page = await doc.getPage(n);
  const anns = await page.getAnnotations(); // intent por omision: "display"
  const vistas = anns.map((a) => ({
    id: a.id,
    subtype: a.subtype,
    fieldType: a.fieldType,
    fieldName: a.fieldName,
    rect: a.rect,
    hidden: a.hidden,
    fieldValue: a.fieldValue,
    hasOwnCanvas: a.hasOwnCanvas,
    keys: Object.keys(a).sort().join(","),
  }));
  console.log(`pagina ${n} rotate=${page.rotate} view=${JSON.stringify(page.view)}`);
  console.log(JSON.stringify(vistas, null, 1));
}
