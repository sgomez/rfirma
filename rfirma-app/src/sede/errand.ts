import type { Catalog } from "../i18n/catalog";
import type { Certificate } from "../signing/certificate";
import type { TokenFailure } from "../signing/token";

/**
 * El trámite que abre una sede electrónica por `afirma://`, en el lado de la
 * interfaz (docs/design/ventana-de-sede.md, ID-268…ID-278).
 *
 * Es un **puerto** como el almacén de certificados o el origen del PDF: quien
 * habla con el canal `wss://`, con el protocolo y con el puente es el backend,
 * y esta ventana no conoce a Tauri (ADR-0017). Aquí solo vive el vocabulario
 * de lo que se enseña y el momento en el que se enseña.
 *
 * Sin React a propósito: los momentos se prueban desde aquí sin montar nada.
 */

/** Qué le pide la sede a quien está delante. */
export type SiteOperation =
  /** Firmar un documento: `sign`, `cosign`. */
  | "sign"
  /** Sólo entregar la identidad: `selectcert`. No hay firma de por medio. */
  | "selectcert";

/**
 * Lo que el PDF dice **de sí mismo**, que es lo único que hay para nombrarlo
 * (ID-270).
 *
 * **La petición no trae nunca el nombre del documento**: el `extraData` con el
 * nombre va en la *respuesta*, y `appname` está roto en el original. Así que no
 * hay nombre de fichero ni ruta que enseñar, y fabricar uno sería inventarlo.
 */
export interface SiteDocument {
  /** El título de los metadatos del PDF, o `null` si no lo trae. */
  title: string | null;
  /** Cuántas páginas tiene. */
  pages: number;
  /** Cuántos bytes ocupa. */
  sizeBytes: number;
  /**
   * Qué firma pide la sede sobre las que el documento ya trae: ninguna, una
   * **cofirma**, o una **contrafirma** sobre todas o sobre las últimas. Se
   * dice antes de consentir.
   */
  round: SignatureRound;
  /**
   * Si alguna de las firmas que ya trae el PDF es de un `/SubFilter` que
   * rFirma no sabe leer (#355). No es un rechazo — el PDF certificado sí
   * invalida con certeza y por eso se rechaza sin preguntar; esto es
   * desconocimiento nuestro (ID-298). Se dice con una frase de información,
   * dentro del mismo consentimiento: no hay un sexto momento (ID-302).
   */
  hasUnregisteredSignatures: boolean;
}

/**
 * Por qué rFirma rechazó la petición, **clasificado** y no redactado en el
 * backend (ADR-0009, ID-29).
 *
 * Son los rechazos del transporte (#316), los que ocurren **antes** de que
 * haya nada que consentir. La lista sale del catálogo, así que una situación
 * nueva se añade en `po/messages.pot` y `tsc` la exige aquí.
 */
export type RefusalSituation = keyof Catalog["sede"]["refusals"];

/**
 * Cómo acabó el trámite. En los tres casos **la sede ya ha recibido su
 * respuesta**: los dos canales van desacompasados a propósito (#316), y esta
 * ventana no es el acuse sino donde vive la precisión que el código `SAF_NN` no
 * puede llevar.
 */
export type SiteOutcome =
  /**
   * Firmado y cancelado llevan el documento —el mismo que se enseñó al
   * consentir— porque es lo único que dice **qué** se acaba de firmar o dejar
   * sin firmar, y en el rechazo no hay ninguno: ocurre antes de que llegue.
   */
  | { kind: "signed"; document: SiteDocument | null }
  | { kind: "cancelled"; document: SiteDocument | null }
  /**
   * El lote entero ya está en la sede. No lleva documento porque nunca hubo
   * ninguno delante: del lote sólo se sabe cuántas firmas llevaba.
   */
  | { kind: "batchSigned"; signs: number }
  /** El fichero que pedía la sede queda escrito donde la persona eligió. */
  | { kind: "saved" }
  /** Los ficheros que la persona eligió ya se han entregado a la sede. */
  | { kind: "loaded"; fileCount: number }
  | {
      kind: "refused";
      situation: RefusalSituation;
      /**
       * El texto crudo, sin traducir ni recortar: es lo único accionable que
       * hay en la pantalla, para llevárselo a quien mantiene la sede.
       */
      detail: string;
    };

/**
 * Qué es lo que la sede pide firmar, según el formato de su petición (#530).
 * `null` es que el momento no lo dice: `selectcert` no firma nada y el lote
 * sólo sabe cuántas firmas lleva.
 */
export type SigningKind = "pdf" | "challenge" | "xml" | "invoice";

/**
 * Tipo de operación de firma solicitada por la sede, tal como lo emite el
 * backend: `commands::SignatureRoundView`, etiquetado por `kind` (ADR-0011).
 */
export type SignatureRound =
  | { kind: "sign" }
  | { kind: "cosign" }
  | { kind: "counter"; target: "tree" | "leafs" };

/** Un elemento del lote local, tal como lo resume el backend: sin ruta ni contenido. */
export interface LocalBatchItem {
  /** El identificador con el que la sede nombra el elemento. */
  id: string;
  /** Qué es lo que se va a firmar. */
  signing: SigningKind;
  /** Si sobre el elemento se pide una firma inicial o una cofirma. */
  round: SignatureRound;
}

/** En qué momento de la secuencia está la ventana. */
export type ErrandStage =
  /**
   * El canal todavía no se ha abierto.
   */
  | { kind: "waiting" }
  /**
   * El navegador no ha conectado en el plazo previsto. El backend decide y
   * publica este momento a través del puerto.
   */
  | { kind: "unreachable" }
  /**
   * **El canal no se ha abierto y ya no va a abrirse** (ID-341). No es el
   * umbral del reloj: aquí rFirma ya lo sabe —no le queda ni un puerto que
   * atar, o la CA local no ha entrado en ningún almacén— y por eso se enseña la
   * pantalla de reparación sin esperar los treinta segundos.
   */
  | { kind: "noChannel"; reason: NoChannelReason }
  /**
   * El corazón del trámite: una **confirmación escrita**, no el selector de
   * certificados (ID-269). Aparece siempre, también con un solo certificado:
   * `headless` y `mandatoryCertSelection` se ignoran los dos (ID-272).
   */
  | {
      kind: "consent";
      /** `null` cuando el documento no viaja, que es el caso de `selectcert`. */
      document: SiteDocument | null;
      /** Cuántas firmas lleva el lote, y `null` cuando lo que se consiente no es un lote. */
      signs: number | null;
      /** Qué se pide firmar, o `null` cuando el momento no lo dice. */
      signing: SigningKind | null;
      /** Los elementos del lote local, o `null` cuando no lo es. */
      items: readonly LocalBatchItem[] | null;
      /** Los que la sede acepta, ya filtrados por el backend. */
      certificates: readonly Certificate[];
      /**
       * Si la sede acotó la lista. Se dice **que** la acotó y nada más: nunca
       * se enumera lo que descartó ni con qué criterio (ID-277).
       */
      narrowed: boolean;
    }
  /**
   * El almacén pide PIN o contraseña. **No tiene pantalla propia** (ID-273): se
   * monta el `PinDialog` del recorrido local, sin una sola diferencia.
   */
  | { kind: "secret"; certificate: Certificate; failure: TokenFailure | null }
  /**
   * Entre que la persona acepta y que la firma vuelve a la sede. Dos momentos,
   * y ninguno es criptográfico: las fases de la trifásica son estado interno
   * del motor y no se cuentan aquí.
   */
  | { kind: "signing"; certificate: Certificate; phase: SigningPhase }
  /**
   * La sede pide guardar un fichero y el diálogo del portal está abierto.
   *
   * Se enseña **el nombre que la sede propone y nunca una ruta** (ADR-0011),
   * y quien confirma es la persona dentro del diálogo: aquí no hay botón que
   * pulsar. `null` es que la sede no propuso ninguno.
   */
  | { kind: "saving"; filename: string | null }
  /** Lo mismo para la carga: uno o varios ficheros, según lo que pida la sede. */
  | { kind: "loading"; multiple: boolean }
  /**
   * La sede pidió comprobar las firmas que ya trae el documento y el validador
   * del original no las da por buenas sin que la persona lo confirme. Dos
   * salidas: seguir, que fija la clave y vuelve a comprobar, o cancelar.
   */
  | { kind: "confirming"; messageCode: string }
  | { kind: "outcome"; outcome: SiteOutcome }
  /**
   * No hay nada que consentir ni nada que elegir, y son **dos situaciones
   * distintas porque la salida es distinta** (ID-278).
   */
  | { kind: "noCertificate"; reason: NoCertificateReason; owned: number };

/**
 * Los dos tramos de la firma, y lo que los separa es hasta dónde se puede
 * parar: mientras rFirma firma, cancelar es limpio —la sede no ha recibido
 * nada—; cuando la respuesta ya va de camino no hay nada que cancelar.
 */
export type SigningPhase = "signing" | "returning";

/**
 * Por qué no hay canal por el que hablar con la sede.
 *
 * Las dos llegan **medidas** desde el backend, y ninguna de las dos se
 * diagnostica en la ventana: la pantalla de reparación es la misma para las
 * dos, porque rFirma no puede saber si el permiso del navegador se denegó.
 */
export type NoChannelReason =
  /** El canal no se ha podido abrir: sin puertos libres, sin material TLS
   * utilizable, o sin llegar a escuchar. Las tres se reparan igual. */
  | "channelNotOpened"
  /** La CA local no ha entrado en ningún almacén NSS: sin ella no hay canal. */
  | "localCaMissing";

/** Por qué no hay ningún certificado con el que seguir. */
export type NoCertificateReason =
  /** No hay ninguno instalado. Tiene arreglo, y el arreglo no depende de la sede. */
  | "none"
  /** La sede los ha excluido todos. Instalar otro no arregla nada. */
  | "excluded";

/** El trámite vivo, entero. `null` es que no hay ninguno. */
export interface Errand {
  /**
   * El origen de la petición, nombrado **a secas** (ID-271).
   *
   * `null` es que no hay origen válido, y entonces queda una etiqueta serena y
   * no una advertencia. **El `Origin` no se usa para rechazar el saludo**: es
   * falsificable desde cualquier programa local, así que sirve para atribuir y
   * nunca para afirmar.
   *
   * Durante la espera todavía no se sabe: llega con la petición.
   */
  origin: string | null;
  operation: SiteOperation;
  stage: ErrandStage;
}

/** Lo que la ventana de sede necesita del backend. */
export interface SiteErrandPort {
  /**
   * Sigue el trámite vivo. Llama a `onChange` con cada momento nuevo y
   * devuelve cómo dejar de escuchar.
   *
   * Que no llame nunca es la respuesta normal: la mayoría de los arranques de
   * rFirma no vienen de una sede.
   */
  watch(onChange: (errand: Errand | null) => void): () => void;
  /** La persona consiente, con el asa del certificado que ha elegido. */
  consent(certificateId: string): Promise<void>;
  /** El secreto tecleado en el diálogo del almacén. */
  submitSecret(secret: string): Promise<void>;
  /**
   * Abandona el trámite. Libera el `idsession` y la sede recibe `CANCEL` de
   * inmediato, sin esperar a que nadie cierre nada.
   */
  cancel(): Promise<void>;
  /** Cierra la ventana. La sede ya tiene su respuesta. */
  close(): Promise<void>;
  /** Vuelve a mirar el almacén, por si se instaló uno con la ventana abierta. */
  lookAgain(): Promise<void>;
  /** Lleva a instalar un certificado, que es el arreglo de «no tienes ninguno». */
  installCertificate(): Promise<void>;
  /** Instala la CA local, sin la cual el navegador ni llega a preguntar. */
  installLocalCa(): Promise<void>;
}

/**
 * Un trámite que no existe: la ventana no se monta.
 *
 * Es lo que se cablea mientras el canal no llegue hasta aquí, y lo que hace
 * que arrancar rFirma a mano no enseñe nunca esta ventana.
 */
export function noErrand(): SiteErrandPort {
  return {
    watch: () => () => {},
    consent: async () => {},
    submitSecret: async () => {},
    cancel: async () => {},
    close: async () => {},
    lookAgain: async () => {},
    installCertificate: async () => {},
    installLocalCa: async () => {},
  };
}

/**
 * Cuánto se queda el desenlace antes de cerrarse solo (ms).
 *
 * Quince segundos y no cinco: con cinco no daba tiempo a leer, y el caso que lo
 * decide es el rechazo, donde cerrarse sola reproduciría el síntoma que el
 * aviso venía a evitar (ID-274).
 */
export const OUTCOME_CLOSE_MS = 15_000;

/** La dirección que Chrome no deja abrir desde fuera: se copia, no se pulsa. */
export const CHROME_LOCAL_NETWORK_SETTINGS = "chrome://settings/content/loopbackNetwork";

/**
 * Qué palabra lleva el botón principal del consentimiento.
 *
 * `selectcert` no firma nada, así que decir «Firmar» ahí sería mentir sobre lo
 * que se está a punto de hacer.
 */
export function consentActionKey(operation: SiteOperation): "sign" | "identify" {
  return operation === "selectcert" ? "identify" : "sign";
}
