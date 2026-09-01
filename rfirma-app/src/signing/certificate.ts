/**
 * El certificado, en el lado de la interfaz.
 *
 * Es el reflejo de `pkcs11::certificate` del backend: la interfaz **no lee
 * DER**, no calcula caducidades y no habla con el token. Recibe el titular, el
 * DNI y el estado ya decididos, porque quien sabe leer un X.509 es el módulo de
 * Rust y una segunda lectura en TypeScript sería otra verdad sobre lo mismo.
 */

/**
 * En qué estado está el certificado, decidido **antes** de pedir el PIN.
 *
 * Los cinco valores son las cinco variantes de `CertificateStatus` en Rust, con
 * los mismos nombres. `revoked` no lo produce el módulo PKCS#11 —comprobar la
 * revocación es hablar con el OCSP— pero tiene sitio aquí para que ese
 * resultado no acabe disfrazado de fallo del token.
 */
export type CertificateStatus =
  | { kind: "valid" }
  /** `notAfter` en segundos desde la época, como lo da el backend. */
  | { kind: "expired"; notAfter: number }
  /** `notBefore` en segundos desde la época. */
  | { kind: "notYetValid"; notBefore: number }
  | { kind: "revoked"; reason: string }
  /**
   * Por qué el DER no se pudo leer, **en las palabras del decodificador**.
   *
   * Cruza con su carga desde `pkcs11::certificate` igual que `expired` y
   * `revoked`: sin ella, `refusalFor` acababa fabricando la prosa del detalle
   * justo en el hueco que el ID-29 reserva al texto original crudo, y el
   * informe de fallo perdía lo único que servía para diagnosticarlo.
   */
  | { kind: "unreadable"; detail: string };

/**
 * De qué clase es el almacén de donde salió el certificado.
 *
 * Cruza la frontera como **clase en inglés** y nunca como texto ya escrito ni
 * como ruta: el rótulo lo pone el catálogo de esta ventana, igual que hace con
 * la `situation` de un fallo. Un nombre compuesto en Rust se saltaría los
 * catálogos y saldría en castellano en la versión en inglés.
 */
export type CertificateStoreClass = "card" | "firefox" | "chrome" | "nssdb";

/** Un certificado elegible, con lo justo para pintarlo y para firmar con él. */
export interface Certificate {
  /**
   * El **asa** que acuñó el backend al listar, sin significado aquí.
   *
   * Es lo que identifica la fila, y no la etiqueta: las etiquetas se repiten
   * —dos claves con el mismo `CKA_LABEL` en un perfil de Firefox, dos
   * `FNMT-GEMELO-99999999R` en el token de pruebas— así que buscando por
   * etiqueta se firmaba siempre con el primero de los dos. La referencia
   * entera no puede cruzar: lleva la ruta del módulo y el `configdir` del
   * perfil (ADR-0011).
   */
  id: string;
  /** El `CKA_LABEL` del objeto dentro del token. Se enseña, no identifica. */
  label: string;
  /** Nombre y apellidos del titular. */
  holderName: string;
  /**
   * El DNI o NIE **en claro**, tal cual viene del RDN `serialNumber`. La
   * máscara del recuadro la aplica Rust al componer `layer2Text` (ID-19); aquí
   * se enseña tal cual, porque el panel dice con qué identidad se firma y no es
   * el recuadro que se estampa en el PDF.
   */
  idNumber: string;
  /** La autoridad emisora. */
  issuer: string;
  /**
   * Dónde estaba. No es adorno: el mismo certificado en el perfil de Firefox y
   * en `~/.pki/nssdb` es indistinguible sin él, y quien tiene tres iguales no
   * puede elegir a ciegas.
   */
  store: CertificateStoreClass;
  status: CertificateStatus;
}

/**
 * Si se puede firmar con él. Lo mira el recorrido **antes** de abrir el diálogo
 * del PIN: pedir el secreto que desbloquea la clave para luego fallar por una
 * fecha que ya se conocía es hacer teclear un PIN para nada.
 */
export function isUsable(status: CertificateStatus): boolean {
  return status.kind === "valid";
}

/**
 * De dónde salen los certificados del token. Es un puerto por lo mismo que lo
 * son el selector de documentos y el origen del PDF: quien habla con PKCS#11 es
 * el backend, y la ventana no conoce a Tauri.
 */
export interface CertificateStore {
  /** Los certificados que hay ahora mismo en los tokens conectados. */
  list(): Promise<readonly Certificate[]>;
}

/**
 * Un almacén vacío: ni token ni orden de por medio.
 *
 * Desde el #60 quien habla con PKCS#11 es `tauriCertificateStore`; esto queda
 * como doble para pintar la ventana sin backend, que es el estado «Sin
 * certificado» de la ficha.
 */
export function emptyCertificateStore(): CertificateStore {
  return { list: async () => [] };
}
