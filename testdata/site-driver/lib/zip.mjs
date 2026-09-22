// El lector de ZIP de la sede: las entradas de un contenedor y sus bytes, sin cifrado ni ZIP64.

import { inflateRawSync } from "node:zlib";

const END_OF_CENTRAL_DIRECTORY = 0x06054b50;
const CENTRAL_ENTRY = 0x02014b50;
const LOCAL_HEADER = 0x04034b50;

function theEndOfTheCentralDirectory(bytes) {
  for (let at = bytes.length - 22; at >= Math.max(0, bytes.length - 22 - 0xffff); at--) {
    if (bytes.readUInt32LE(at) === END_OF_CENTRAL_DIRECTORY) return at;
  }
  return -1;
}

function theContentOf(bytes, method, localAt, compressedSize) {
  if (bytes.readUInt32LE(localAt) !== LOCAL_HEADER) throw new Error("cabecera local ausente");
  const start = localAt + 30 + bytes.readUInt16LE(localAt + 26) + bytes.readUInt16LE(localAt + 28);
  const stored = bytes.subarray(start, start + compressedSize);
  if (method === 0) return stored;
  if (method === 8) return inflateRawSync(stored);
  throw new Error(`método de compresión ${method} no admitido`);
}

/** Las entradas de un ZIP, nombre a bytes, o `null` si los bytes no son un ZIP legible. */
export function theZipEntries(bytes) {
  try {
    const end = theEndOfTheCentralDirectory(bytes);
    if (end < 0) return null;
    const entries = new Map();
    let at = bytes.readUInt32LE(end + 16);
    for (let count = bytes.readUInt16LE(end + 10); count > 0; count--) {
      if (bytes.readUInt32LE(at) !== CENTRAL_ENTRY) return null;
      const nameLength = bytes.readUInt16LE(at + 28);
      const name = bytes.subarray(at + 46, at + 46 + nameLength).toString("utf8");
      const content = theContentOf(
        bytes,
        bytes.readUInt16LE(at + 10),
        bytes.readUInt32LE(at + 42),
        bytes.readUInt32LE(at + 20),
      );
      entries.set(name, content);
      at += 46 + nameLength + bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32);
    }
    return entries;
  } catch {
    return null;
  }
}
