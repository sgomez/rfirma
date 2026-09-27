/**
 * El certificado, en el lado de la interfaz.
 *
 * Es el reflejo de `pkcs11::certificate` del backend: la interfaz **no lee
 * DER**, no calcula caducidades y no habla con el token. Recibe el titular, el
 * DNI y el estado ya decididos, porque quien sabe leer un X.509 es el módulo de
 * Rust y una segunda lectura en TypeScript sería otra verdad sobre lo mismo.
 */

import type { TFunction } from "i18next";

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
type CertificateStoreClass = "card" | "firefox" | "chrome" | "nssdb" | "installed";

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
  /**
   * El `CN` del subject, tal cual: en un certificado de persona física de la
   * FNMT viene en orden «APELLIDO1 APELLIDO2 NOMBRE», no «nombre y apellidos».
   */
  holderName: string;
  /** El `CN` tal y como lo estampa la firma visible: con el identificador enmascarado. */
  stampedSigner: string;
  /** Nombre de pila del RDN `GN`, o vacío si el certificado no lo trae. */
  givenName: string;
  /** Primer apellido del RDN `SN`, o vacío si el certificado no lo trae. */
  surname: string;
  /**
   * El DNI o NIE **en claro**, tal cual viene del RDN `serialNumber`. La
   * máscara del recuadro la aplica Rust al componer `layer2Text` (ID-19); aquí
   * se enseña tal cual, porque el panel dice con qué identidad se firma y no es
   * el recuadro que se estampa en el PDF.
   */
  idNumber: string;
  /** El NIF de la entidad representada, si el certificado la lleva. */
  organizationIdentifier: string | null;
  /** El nombre de la entidad representada, o nada si el certificado no es de representante. */
  entityName: string | null;
  /** La autoridad emisora. */
  issuer: string;
  /** Número de serie del certificado. */
  certificateSerialNumber: string;
  /**
   * Todos los almacenes donde está esta misma copia, por orden de preferencia.
   * No es adorno: el mismo certificado en el perfil de Firefox y en
   * `~/.pki/nssdb` es indistinguible sin él, y quien tiene tres iguales no
   * puede elegir a ciegas.
   */
  stores: readonly CertificateStoreClass[];
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

/** Primera línea de la fila: la entidad si es de representante, el titular si es personal. */
export function certificateHeadline(certificate: Certificate): string {
  return certificate.entityName ?? certificate.holderName;
}

/** Segunda línea de la fila, completa. */
export function certificateSubtitle(certificate: Certificate, t: TFunction): string {
  return certificate.entityName != null
    ? t("panel.certificate.onBehalfOf", {
        holder: certificate.holderName,
        nif: certificate.organizationIdentifier ?? "",
      })
    : t("panel.certificate.personalCapacity", { idNumber: certificate.idNumber });
}

/** Versión corta de la segunda línea, para la caja cerrada. */
export function certificateCompactSubtitle(certificate: Certificate, t: TFunction): string {
  return certificate.entityName != null
    ? t("panel.certificate.onBehalfOfShort", { holder: certificate.holderName })
    : certificateSubtitle(certificate, t);
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

function byHeadlineThenStore(a: Certificate, b: Certificate): number {
  return (
    holderCollator.compare(certificateHeadline(a), certificateHeadline(b)) ||
    holderCollator.compare(a.stores[0] ?? "", b.stores[0] ?? "")
  );
}

/**
 * Agrupa y ordena los certificados para el desplegable: los usables arriba,
 * los que no lo son abajo, y dentro de cada grupo alfabético por primera
 * línea, desempatando por almacén. Es una función pura y sin
 * locale implícito de sistema —el `Intl.Collator` fija «es»— para que el
 * orden no dependa de dónde corre la aplicación.
 */
export function groupCertificates(certificates: readonly Certificate[]): CertificateGroups {
  const available: Certificate[] = [];
  const unusable: Certificate[] = [];
  for (const certificate of certificates) {
    (isUsable(certificate.status) ? available : unusable).push(certificate);
  }
  available.sort(byHeadlineThenStore);
  unusable.sort(byHeadlineThenStore);
  return { available, unusable };
}

/** El que la sede trae puesto: el recordado si se puede usar, y si no el primero utilizable de la lista. */
export function sitePreselection(certificates: readonly Certificate[]): Certificate | null {
  const remembered = certificates.find(
    (certificate) => certificate.remembered && isUsable(certificate.status),
  );
  return remembered ?? groupCertificates(certificates).available[0] ?? null;
}

/** «06/2027»: mes y año de caducidad, sin traducir su formato. */
export function expiryMonthYear(notAfter: number): string {
  const date = new Date(notAfter * 1000);
  return `${String(date.getMonth() + 1).padStart(2, "0")}/${date.getFullYear()}`;
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
   * **El backend abre el selector de ficheros y, con el elegido, pide su
   * contraseña**: esta pantalla nunca la ve ni la teclea. `false` es haber
   * cerrado el selector, o el diálogo de la contraseña, sin elegir ni
   * instalar nada, que no es un fallo: deja la lista como estaba. Rechaza
   * cuando el fichero no se puede abrir o cuando su clave no es RSA ni
   * de curva elíptica.
   */
  install(): Promise<boolean>;
  /** Quita un `.p12` instalado, por el asa de su fila. */
  remove(id: string): Promise<void>;
  /** Vacía el Almacén de rFirma entero, ya confirmado por la persona (ADR-0034). */
  emptyStore(): Promise<void>;
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
    emptyStore: async () => {},
  };
}

/**
 * Los que se instalaron en rFirma desde un `.p12`, que son los únicos que
 * Preferencias enseña y los únicos que se pueden quitar (ID-198).
 *
 * El orden es el mismo del desplegable —alfabético por primera línea— para
 * que la misma persona salga en el mismo sitio en las dos pantallas, y **los
 * caducados no se caen**: que desaparezca no le explica nada a quien lo instaló.
 */
export function installedCertificates(
  certificates: readonly Certificate[],
): readonly Certificate[] {
  return certificates
    .filter((certificate) => certificate.stores.includes("installed"))
    .sort(byHeadlineThenStore);
}
