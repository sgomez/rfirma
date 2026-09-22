// El analizador de PDF firmados de la sede: busca el diccionario de firma por texto, sin validar la firma.

import { constants, inflateSync } from "node:zlib";

/** Si los bytes son un PDF con un diccionario `/Sig` que cubre un `/ByteRange` y trae `/Contents`. */
export function isASignedPdf(bytes) {
  const text = bytes.toString("latin1");
  return (
    text.startsWith("%PDF-") &&
    /\/Type\s*\/Sig\b/.test(text) &&
    /\/ByteRange\s*\[/.test(text) &&
    /\/Contents\s*</.test(text)
  );
}

/** Los `/Rect` de los campos de firma del PDF, cada uno como `[llx, lly, urx, ury]`. */
export function theSignatureRectangles(bytes) {
  const rectangles = [];
  for (const body of theObjectBodies(bytes)) {
    if (!/\/FT\s*\/Sig\b/.test(body)) continue;
    const rect = /\/Rect\s*\[([^\]]*)\]/.exec(body);
    if (rect) rectangles.push(rect[1].trim().split(/\s+/).map(Number));
  }
  return rectangles;
}

function theObjectBodies(bytes) {
  const text = bytes.toString("latin1");
  const bodies = [...text.matchAll(/\bobj\b([\s\S]*?)\bendobj\b/g)].map(([, body]) => body);
  return bodies.concat(bodies.flatMap(theCompressedObjects));
}

/** Los objetos de un `/ObjStm` comprimido, donde la compresión completa guarda los campos. */
function theCompressedObjects(body) {
  const dictionary = /^\s*<<([\s\S]*?)>>\s*stream\r?\n/.exec(body);
  if (!dictionary || !/\/Type\s*\/ObjStm\b/.test(dictionary[1])) return [];
  const count = Number(/\/N\s+(\d+)/.exec(dictionary[1])?.[1]);
  const first = Number(/\/First\s+(\d+)/.exec(dictionary[1])?.[1]);
  const end = body.lastIndexOf("endstream");
  const data = Buffer.from(body.slice(dictionary[0].length, end), "latin1");
  let content;
  try {
    content = inflateSync(data, { finishFlush: constants.Z_SYNC_FLUSH }).toString("latin1");
  } catch {
    return [];
  }
  const offsets = content.slice(0, first).trim().split(/\s+/).map(Number).filter((_, i) => i % 2);
  return offsets
    .slice(0, count)
    .map((offset, i) => content.slice(first + offset, first + (offsets[i + 1] ?? content.length)));
}

/** Si el rectángulo ocupa algo en la página: una firma invisible lo deja en `[0 0 0 0]`. */
export function isAVisibleArea([llx, lly, urx, ury]) {
  return Math.abs(urx - llx) > 0 && Math.abs(ury - lly) > 0;
}
