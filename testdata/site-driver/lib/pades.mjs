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
