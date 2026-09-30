/**
 * Si hay una versión nueva publicada: el puerto que lo pregunta, el que la instala y su doble.
 *
 * Es **la única conexión saliente** de rFirma, y quien la hace es Rust:
 * pregunta a las Releases del repositorio y compara con la versión que
 * corre. La ventana no sabe nada de eso —ni URL, ni caché, ni comparación de
 * versiones—: pregunta por el puerto y, si le contestan, lo cuenta.
 *
 * Donde la respuesta dice que es instalable, la ventana ofrece instalarla; donde no,
 * la franja lleva a *Acerca de*, que es donde están las órdenes de alta del repositorio.
 *
 * Sin red no hay respuesta y **no pasa nada**: `null` es «no hay nada que
 * decir», no un error. La franja, sencillamente, no se monta.
 */
export interface NewVersion {
  /** La versión publicada, tal como la etiqueta la Release: `0.4.1`. */
  version: string;
  /** Si la aplicación puede instalarla por sí misma. */
  installable: boolean;
}

/** Lo que pasó al instalar la versión anunciada; `installed` cierra la aplicación. */
export type Installation =
  | "installed"
  | "noUpdate"
  | "networkFailure"
  | "invalidSignature"
  | "notAvailable";

/** Quien sabe si hay una versión nueva y sabe instalarla. Ver [`NewVersion`]. */
export interface VersionCheck {
  /** La versión publicada si es más nueva que la que corre; `null` si no. */
  latest(): Promise<NewVersion | null>;
  /** Descarga, verifica e instala la versión anunciada. */
  install(): Promise<Installation>;
}

/**
 * La comprobación sin red: contesta lo que se le diga, y por omisión que no
 * hay nada, y cuenta las instalaciones pedidas. Es el doble de las pruebas;
 * quien pregunta de verdad es `tauriVersionCheck`.
 */
export function inMemoryVersionCheck(
  published: NewVersion | null = null,
  installation: Installation = "installed",
): VersionCheck & { installCalls: number } {
  const check = {
    installCalls: 0,
    latest: async () => published,
    install: async () => {
      check.installCalls += 1;
      return installation;
    },
  };
  return check;
}
