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
  /** `notAfter` en segundos desde la época: cuándo deja de estar en vigor. */
  | { kind: "valid"; notAfter: number }
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
 * `installed` es el `.p12` que se metió en rFirma desde Preferencias (ID-192):
 * es la única clase que se puede **quitar**, y por eso la lista de esa pantalla
 * se queda exactamente con ella (ID-198).
 *
 * Cruza la frontera como **clase en inglés** y nunca como texto ya escrito ni
 * como ruta: el rótulo lo pone el catálogo de esta ventana, igual que hace con
 * la `situation` de un fallo. Un nombre compuesto en Rust se saltaría los
 * catálogos y saldría en castellano en la versión en inglés.
 */
export type CertificateStoreClass = "card" | "firefox" | "chrome" | "nssdb" | "installed";

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
  /**
   * Si es **el que se usó la última vez**, y por tanto el que viene ya puesto
   * en el desplegable al arrancar (#110).
   *
   * Lo decide el backend y no esta ventana, porque lo que se recordó son
   * coordenadas del token —módulo, etiqueta, `CKA_ID`, perfil— y ninguna de
   * ellas puede cruzar la frontera (ADR-0011). Aquí solo llega marcada la fila.
   *
   * Con el certificado recordado fuera del token no viene marcada **ninguna**,
   * y entonces el panel arranca en «Sin certificado» sin decir nada: no es un
   * error, es que no está (ADR-0010).
   */
  remembered: boolean;
}

/**
 * Si se puede firmar con él. Lo mira el recorrido **antes** de abrir el diálogo
 * del PIN: pedir el secreto que desbloquea la clave para luego fallar por una
 * fecha que ya se conocía es hacer teclear un PIN para nada.
 */
export function isUsable(status: CertificateStatus): boolean {
  return status.kind === "valid";
}

/** Los certificados, ya separados en los dos grupos que enseña el desplegable. */
export interface CertificateGroups {
  /** Los que se pueden usar para firmar, arriba. */
  readonly available: readonly Certificate[];
  /**
   * Caducados, todavía no válidos, no leídos o —el día que se empiece a
   * comprobar la revocación (#194)— revocados: cualquier motivo por el que no
   * se puede firmar con ellos cae en el mismo grupo, abajo.
   */
  readonly unusable: readonly Certificate[];
}

/** Alfabético en castellano, con acentos y «ñ» donde toca. */
const holderCollator = new Intl.Collator("es", { sensitivity: "base" });

function byHolderThenStore(a: Certificate, b: Certificate): number {
  return (
    holderCollator.compare(a.holderName, b.holderName) || holderCollator.compare(a.store, b.store)
  );
}

/**
 * Agrupa y ordena los certificados para el desplegable: los usables arriba,
 * los que no lo son abajo, y dentro de cada grupo alfabético por titular,
 * desempatando por almacén (ID-197). Es una función pura y sin locale
 * implícito de sistema —el `Intl.Collator` fija «es»— para que el orden no
 * dependa de dónde corre la aplicación.
 */
export function groupCertificates(certificates: readonly Certificate[]): CertificateGroups {
  const available: Certificate[] = [];
  const unusable: Certificate[] = [];
  for (const certificate of certificates) {
    (isUsable(certificate.status) ? available : unusable).push(certificate);
  }
  available.sort(byHolderThenStore);
  unusable.sort(byHolderThenStore);
  return { available, unusable };
}

/**
 * De dónde salen los certificados del token. Es un puerto por lo mismo que lo
 * son el selector de documentos y el origen del PDF: quien habla con PKCS#11 es
 * el backend, y la ventana no conoce a Tauri.
 */
export interface CertificateStore {
  /** Los certificados que hay ahora mismo en los tokens conectados. */
  list(): Promise<readonly Certificate[]>;
  /**
   * Mete un `.p12` en rFirma y responde si quedó instalado alguno.
   *
   * **Quien abre el selector de ficheros es el backend**, igual que con la
   * rúbrica y con el destino (ID-63), así que la contraseña del fichero viaja
   * antes de que exista fichero elegido. `false` es haber cerrado el selector
   * sin elegir nada, que no es un fallo: deja la lista como estaba. Rechaza
   * cuando el fichero no se puede abrir o cuando su clave no es RSA (ID-197).
   */
  install(password: string): Promise<boolean>;
  /** Quita un `.p12` instalado, por el asa de su fila. */
  remove(id: string): Promise<void>;
}

/**
 * Un almacén vacío: ni token ni orden de por medio.
 *
 * Desde el #60 quien habla con PKCS#11 es `tauriCertificateStore`; esto queda
 * como doble para pintar la ventana sin backend, que es el estado «Sin
 * certificado» de la ficha.
 */
export function emptyCertificateStore(): CertificateStore {
  return {
    list: async () => [],
    install: async () => false,
    remove: async () => {},
  };
}

/**
 * Los que se instalaron en rFirma desde un `.p12`, que son los únicos que
 * Preferencias enseña y los únicos que se pueden quitar (ID-198).
 *
 * El orden es el mismo del desplegable —alfabético por titular— para que la
 * misma persona salga en el mismo sitio en las dos pantallas, y **los
 * caducados no se caen**: que desaparezca no le explica nada a quien lo instaló.
 */
export function installedCertificates(
  certificates: readonly Certificate[],
): readonly Certificate[] {
  return certificates
    .filter((certificate) => certificate.store === "installed")
    .sort(byHolderThenStore);
}
