// El analizador de PDF firmados de la sede: busca el diccionario de firma por texto, sin validar la firma.

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
  for (const [, body] of bytes.toString("latin1").matchAll(/\bobj\b([\s\S]*?)\bendobj\b/g)) {
    if (!/\/FT\s*\/Sig\b/.test(body)) continue;
    const rect = /\/Rect\s*\[([^\]]*)\]/.exec(body);
    if (rect) rectangles.push(rect[1].trim().split(/\s+/).map(Number));
  }
  return rectangles;
}

/** Si el rectángulo ocupa algo en la página: una firma invisible lo deja en `[0 0 0 0]`. */
export function isAVisibleArea([llx, lly, urx, ury]) {
  return Math.abs(urx - llx) > 0 && Math.abs(ury - lly) > 0;
}
