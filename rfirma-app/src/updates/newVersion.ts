/**
 * Si hay una versión nueva publicada: el puerto que lo pregunta y su doble.
 *
 * Es **la única conexión saliente** de rFirma, y quien la hace es Rust
 * (`app/version.rs`): pregunta a las Releases del repositorio como mucho una
 * vez cada 24 h, compara con la versión que corre y contesta. La ventana no
 * sabe nada de eso —ni URL, ni caché, ni comparación de versiones—: pregunta
 * por el puerto y, si le contestan, lo cuenta en la franja de notificación.
 *
 * **Es un aviso y nada más** (ID-181): no hay descarga ni instalación detrás,
 * porque el mecanismo se autoliquidaría en cuanto el paquete lo actualizara el
 * sistema. La acción de la franja lleva a *Acerca de*, que es donde están las
 * órdenes de alta del repositorio, y el `opener:deny-open-url` del ID-85 sigue
 * denegado.
 *
 * Sin red no hay respuesta y **no pasa nada**: `null` es «no hay nada que
 * decir», no un error. La franja, sencillamente, no se monta.
 */
export interface NewVersion {
  /** La versión publicada, tal como la etiqueta la Release: `0.4.1`. */
  version: string;
}

/** Quien sabe si hay una versión nueva. Ver [`NewVersion`]. */
export interface VersionCheck {
  /** La versión publicada si es más nueva que la que corre; `null` si no. */
  latest(): Promise<NewVersion | null>;
}

/**
 * La comprobación sin red: contesta lo que se le diga, y por omisión que no
 * hay nada. Es el doble de las pruebas; quien pregunta de verdad es
 * `tauriVersionCheck`.
 */
export function inMemoryVersionCheck(published: NewVersion | null = null): VersionCheck {
  return {
    latest: async () => published,
  };
}
