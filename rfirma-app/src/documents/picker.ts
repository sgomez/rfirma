import type { RecentDocument } from "./recents";

/**
 * Por dónde entra un documento en la aplicación.
 *
 * La entrada pasa **siempre por el portal** del sistema: bajo el arenero es la
 * única forma de leer un fichero de fuera, y saltárselo es lo que prohíbe el
 * ADR-0004. De ahí que esto sea un puerto y no un `<input type="file">`: el
 * WebView no puede abrir el explorador del sistema por su cuenta, y un campo
 * de fichero en el HTML sería justamente el segundo camino que no debe haber.
 *
 * Devuelve el documento ya canonicalizado y con sus metadatos cacheados,
 * porque eso lo sabe quien tocó el disco —`memory::recents::RecentDocument`—
 * y no la interfaz.
 */
export interface DocumentPicker {
  /** Abre el explorador del sistema. `null` si se cancela. */
  choose(): Promise<RecentDocument | null>;
}

/**
 * El selector sin portal: entrega los documentos que se le den, en orden, y
 * luego se comporta como una cancelación. Es el doble de las pruebas; quien
 * habla con el portal de verdad es `tauriDocumentPicker`.
 */
export function inMemoryDocumentPicker(documents: readonly RecentDocument[] = []): DocumentPicker {
  const pending = documents.slice();
  return {
    choose: async () => pending.shift() ?? null,
  };
}
