import type { DocumentInHand } from "./document";

/**
 * Por dónde entra un documento en la aplicación.
 *
 * La entrada pasa **siempre por el portal** del sistema: bajo el sandbox es la
 * única forma de leer un fichero de fuera, y saltárselo es lo que prohíbe el
 * ADR-0004. De ahí que esto sea un puerto y no un `<input type="file">`: el
 * WebView no puede abrir el explorador del sistema por su cuenta, y un campo
 * de fichero en el HTML sería justamente el segundo camino que no debe haber.
 *
 * Devuelve el documento ya canonicalizado y con sus metadatos, porque eso lo
 * sabe quien tocó el disco y no la interfaz. Y devuelve un documento **en la
 * mano**, no una fila: quien decide si se anota es la bandeja, y lo que sale
 * del diálogo se recuerda porque lo eligió una persona (ID-287).
 */
export interface DocumentPicker {
  /** Abre el explorador del sistema. `null` si se cancela. */
  choose(): Promise<DocumentInHand | null>;
}

/**
 * El selector sin portal: entrega los documentos que se le den, en orden, y
 * luego se comporta como una cancelación. Es el doble de las pruebas; quien
 * habla con el portal de verdad es `tauriDocumentPicker`.
 */
export function inMemoryDocumentPicker(documents: readonly DocumentInHand[] = []): DocumentPicker {
  const pending = documents.slice();
  return {
    choose: async () => pending.shift() ?? null,
  };
}
