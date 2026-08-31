/**
 * La rúbrica: la firma manuscrita escaneada que va **dentro** del recuadro.
 *
 * Elegirla, comprobarla, reescalarla y aplanar su transparencia a blanco es
 * cosa de `rubric::normalize` en Rust (ADR-0012). La interfaz recibe la imagen
 * **ya normalizada** y la enseña: la miniatura no es una vista previa de lo que
 * el usuario eligió, es el fichero que se va a firmar.
 */

/** La rúbrica normalizada, lista para enseñarla y para estamparla. */
export interface Rubric {
  /**
   * La imagen ya normalizada, como `data:` para pintarla en un `<img>`. Es un
   * JPEG, y un JPEG no tiene alfa: la transparencia del PNG original ya viene
   * aplanada a blanco aquí dentro, así que la miniatura sale blanca porque el
   * fichero lo es, no porque el CSS lo pinte (ID-24).
   */
  dataUrl: string;
  width: number;
  height: number;
}

/**
 * Los seis fallos de la rúbrica, con los mismos nombres que
 * `rubric::error::Situation`. Tres hablan de la imagen —el ADR-0012 no cuenta
 * más— y tres del disco. El reescalado no está: se hace en silencio, porque es
 * lo que el usuario habría pedido de todos modos.
 */
export type RubricSituation =
  | "notAnAcceptedImage"
  | "damagedImage"
  | "imageTooLarge"
  | "sourceUnreadable"
  | "storeUnwritable"
  | "storeUnreadable";

/** Un fallo al preparar la rúbrica: situación clasificada y detalle crudo. */
export interface RubricFailure {
  situation: RubricSituation;
  /** El texto original, sin traducir: está para pegarlo en un informe. */
  detail: string;
}

/** Lo que devuelve elegir una rúbrica: la imagen, un fallo, o una cancelación. */
export type RubricChoice = { rubric: Rubric } | { failure: RubricFailure } | null;

/**
 * Por dónde entra la rúbrica. Puerto por lo mismo que el selector de
 * documentos: bajo el arenero el fichero lo entrega el portal, y la
 * normalización ocurre al elegir, con el panel todavía abierto, nunca al
 * firmar.
 */
export interface RubricPicker {
  choose(): Promise<RubricChoice>;
}

/** El selector mientras no hay orden expuesta: se comporta como cancelación. */
export function emptyRubricPicker(): RubricPicker {
  return { choose: async () => null };
}
