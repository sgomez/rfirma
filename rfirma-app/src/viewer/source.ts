import type { RecentDocument } from "../documents/recents";
import type { PdfDocument } from "./pdf";
import { pdfjsLoader } from "./pdfjsLoader";

/**
 * De qué documento se pinta el PDF.
 *
 * Es un puerto por lo mismo que lo es el selector: bajo el arenero los bytes
 * los entrega el **portal**, no una ruta que el WebView pueda abrir. La
 * aplicación nunca conoce la ruta original de un documento, así que aquí no hay
 * ni una URL: entra el documento de la bandeja y sale el PDF ya abierto.
 */
export interface PdfSource {
  /** El documento abierto, o `null` si no se puede pintar. */
  open(document: RecentDocument): Promise<PdfDocument | null>;
}

/** Lee los bytes de un documento. Lo aporta el backend, por el portal. */
export type ReadDocument = (document: RecentDocument) => Promise<Uint8Array>;

/** El origen de verdad: los bytes del portal, abiertos con `pdf.js`. */
export function pdfjsSource(read: ReadDocument): PdfSource {
  const loader = pdfjsLoader();
  return { open: async (document) => loader.load(await read(document)) };
}

/**
 * El origen mientras no hay orden expuesta que lea los bytes: no abre nada, y
 * el visor se queda en su estado vacío. Sirve de doble en las pruebas y de
 * relleno en `main.tsx`, igual que `inMemoryDocumentPicker`.
 */
export function emptyPdfSource(): PdfSource {
  return { open: async () => null };
}
