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
  | { kind: "notYetValid" }
  | { kind: "revoked"; reason: string }
  | { kind: "unreadable" };

/** Un certificado elegible, con lo justo para pintarlo y para firmar con él. */
export interface Certificate {
  /** El `CKA_LABEL` del objeto dentro del token: identifica la fila. */
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

/** El almacén mientras no hay orden expuesta que hable con el token: vacío. */
export function emptyCertificateStore(): CertificateStore {
  return { list: async () => [] };
}
