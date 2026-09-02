/**
 * Los puertos que hablan con Tauri: los tres de firma (#60), los dos del
 * documento (#82), el del arrastre (#83), los dos de la configuración —los
 * ajustes y el idioma—, que comparten fichero debajo, y el de la rúbrica
 * (#128).
 *
 * El del arrastre es el único que no habla con una orden sino con un **evento**
 * de la ventana: soltar un fichero no lo pide la interfaz, le ocurre. Aun así
 * vive aquí por la misma razón que los otros —es donde se importa lo de
 * `@tauri-apps/api`— y la ventana lo sigue viendo como un puerto suyo.
 *
 * Este es el **único** fichero del frontal que sabe que debajo hay Tauri, y por
 * eso es el único que importa `invoke`. Vive en la raíz de `src/` y no dentro
 * de `signing/` justamente por eso: la frontera es una sola para toda la
 * aplicación, y repartirla por módulos sería tener dos. La ventana y sus
 * pruebas siguen hablando con `CertificateStore`, `Layer2Composer`,
 * `SigningBackend`, `DocumentPicker`, `PdfSource`, `DocumentDrops` y
 * `RubricPicker`, y quien elige entre estas implementaciones y los dobles de
 * memoria es `main.tsx`.
 *
 * # Los fallos llegan clasificados, no traducidos
 *
 * Las órdenes rechazan con la forma del ID-29 —una situación nuestra y el texto
 * original crudo al lado—, así que aquí no hay ni una tabla de `CKR_*` ni un
 * `catch` que invente un mensaje: lo que no venga con esa forma —una excepción
 * del propio puente de Tauri, una orden que no existe— cae en `unknown` con su
 * texto tal cual, que es exactamente lo que el ADR-0009 pide. Quien lo decide
 * es `errors/classify.ts`, que no es de Tauri sino del ID-29.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Badge } from "./documents/document";
import type { DocumentDrops, Drop } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import type { RecentDocument, RecentsStore } from "./documents/recents";
import { classify } from "./errors/classify";
import type { ErrorSituation } from "./errors/ErrorNotice";
import { FALLBACK_LANGUAGE, isLanguageTag } from "./i18n/languages";
import type { LanguagePreference } from "./i18n/preference";
import type { PreferencesStore } from "./preferences/preferences";
import { DEFAULT_THEME, isTheme, type Theme } from "./preferences/theme";
import type { Certificate, CertificateStore } from "./signing/certificate";
import type { Destination, DestinationSource, SignedDocumentOpener } from "./signing/destination";
import type { SignedDocument, SigningBackend, SigningOrder, StageResult } from "./signing/flow";
import type { Rubric, RubricPicker, RubricSituation } from "./signing/rubric";
import type { TokenFailure } from "./signing/token";
import type { Layer2Composer, SigningIdentity, VisibleSignature } from "./signing/visibleSignature";
import type { PageSet } from "./viewer/signatureBox";
import { type PdfSource, pdfjsSource } from "./viewer/source";

/** Lo que se enseña cuando falla una etapa de la firma. Ver [`classify`]. */
function failureOf(thrown: unknown): TokenFailure {
  const named = classify(thrown);
  return {
    situation: named.situation as TokenFailure["situation"],
    detail: named.detail,
    attemptsLeft: named.attemptsLeft,
  };
}

/**
 * Envuelve una etapa: sale bien, o sale con una situación clasificada.
 *
 * Recibe **la llamada sin hacer** y no la promesa ya hecha, para que la orden
 * se invoque **dentro** del `try`. Con la promesa por parámetro, un fallo
 * síncrono de `invoke` —una orden que no existe— se escaparía de este `catch`,
 * y el rechazo quedaría suelto entre que se crea y que se espera.
 */
async function stage<T>(call: () => Promise<T>): Promise<StageResult<T>> {
  try {
    return { ok: true, value: await call() };
  } catch (thrown) {
    return { ok: false, failure: failureOf(thrown) };
  }
}

/** Los certificados de los tokens conectados. No pide el PIN. */
export function tauriCertificateStore(): CertificateStore {
  return {
    list: () => invoke<readonly Certificate[]>("list_certificates"),
  };
}

/**
 * El compositor autoritativo: el mismo `signing::layer2_text` que compone lo
 * que se envía en `layer2Text`.
 *
 * Por eso la vista previa es honesta y no una imitación: es literalmente la
 * cadena que va a acabar estampada.
 */
export function tauriLayer2Composer(): Layer2Composer {
  return {
    compose: async (signature: VisibleSignature, signer: SigningIdentity) => {
      try {
        return await invoke<string>("compose_visible_text", {
          order: previewOrder(signature, signer),
        });
      } catch {
        // La vista previa no es sitio para un aviso de error: si no se puede
        // componer —el token se ha retirado mientras se miraba—, el recuadro
        // se queda en su estado vacío y lo contará el intento de firmar.
        return null;
      }
    },
  };
}

/**
 * La orden que compone la vista previa.
 *
 * Lleva un recuadro degenerado y ningún documento **a propósito**:
 * `compose_visible_text` solo mira las casillas, el motivo y el instante, y
 * darle una posición de mentira es más honesto que darle una de verdad que
 * nadie va a usar.
 */
function previewOrder(signature: VisibleSignature, signer: SigningIdentity): SigningOrder {
  return {
    document: "",
    certificate: signer.certificate,
    placement: {
      page: 1,
      pages: { only: [1] },
      pageCount: 1,
      mediaBox: [0, 0, 0, 0],
      rotation: 0,
      rect: [0, 0, 0, 0],
    },
    fields: signature.fields,
    reason: signature.reason,
    signedAt: signer.signedAt,
    rubric: null,
    language: signer.language,
  };
}

/**
 * Las tres etapas, cada una en su orden.
 *
 * El ciclo a medias **no vive aquí**: entre la prefirma y la postfirma lo
 * guarda el backend, que es quien tiene el sello de sesión. Este objeto no
 * tiene estado, y eso es lo que impide que la ventana pueda alterar el sello
 * que la postfirma exige idéntico (ADR-0016).
 */
export function tauriSigningBackend(): SigningBackend {
  return {
    presign: (order) => stage(() => invoke<void>("begin_signing", { order })),
    sign: (pin) => stage(() => invoke<void>("sign_with_pin", { pin })),
    postsign: () => stage(() => invoke<SignedDocument>("finish_signing")),
    discard: cancelSigning,
  };
}

/**
 * Olvida el ciclo a medias cuando se cancela en el diálogo del PIN.
 *
 * Quien la llama es `useSigning.cancel`, a través del puerto: por eso está
 * enchufada como `discard` arriba y no exportada suelta para que alguien se
 * acuerde de invocarla.
 */
export function cancelSigning(): Promise<void> {
  return invoke<void>("cancel_signing");
}

/**
 * Un documento recién abierto, tal como lo devuelve `open_document`. Es
 * `commands::OpenedDocumentView` de Rust, campo a campo: **un identificador y
 * un nombre, ninguna ruta** (ADR-0011).
 */
interface OpenedDocumentView {
  id: string;
  name: string;
  modified: number | null;
}

/**
 * El portal de ficheros, por la orden que abre el diálogo desde Rust (ID-63).
 *
 * El diálogo no se abre desde aquí: si lo hiciera, el frontal tendría que pedir
 * el permiso del complemento de diálogo y la lista de permisos de la ventana
 * crecería. Lo que cruza es lo que el backend apuntó.
 *
 * Cancelar devuelve `null`, y eso **no es un fallo**: es lo que deja el
 * documento activo, la lista y el visor como estaban (ID-73).
 */
export function tauriDocumentPicker(): DocumentPicker {
  return {
    choose: async () => {
      const opened = await invoke<OpenedDocumentView | null>("open_document");
      return opened === null ? null : recentOf(opened);
    },
  };
}

/**
 * La rúbrica ya normalizada, tal como la devuelve `choose_rubric`. Es
 * `commands::RubricView` de Rust, campo a campo: el JPEG en Base64, sin el
 * prefijo `data:` —lo antepone aquí, que es quien sabe que es para un
 * `<img>`— y sus dimensiones.
 */
interface RubricViewPayload {
  base64: string;
  width: number;
  height: number;
}

/**
 * Lo que devuelve elegir una rúbrica, tal cual lo emite Rust. Es
 * `commands::RubricChoiceView`, campo a campo: la imagen adoptada o por qué
 * no se ha podido, nunca las dos.
 */
interface RubricChoiceViewPayload {
  rubric: RubricViewPayload | null;
  failure: { situation: string; detail: string } | null;
}

/**
 * El selector de la rúbrica, por la orden que abre el diálogo del portal
 * desde Rust y adopta lo elegido en `RubricStore` (ID-82).
 *
 * Cancelar el diálogo devuelve `null`, y **no es un fallo**: es lo que deja
 * la rúbrica ya elegida como estaba. Una imagen que no vale tampoco revienta
 * la promesa —viaja como `{ failure }`, con el panel de firma todavía
 * abierto (ADR-0010)—, así que `choose` no necesita `try`/`catch`: las seis
 * situaciones de `RubricSituation` llegan ya clasificadas en la propia
 * respuesta.
 */
function rubricOf(payload: RubricViewPayload): Rubric {
  const { base64, width, height } = payload;
  return { dataUrl: `data:image/jpeg;base64,${base64}`, width, height };
}

export function tauriRubricPicker(): RubricPicker {
  return {
    choose: async () => {
      const outcome = await invoke<RubricChoiceViewPayload | null>("choose_rubric");
      if (outcome === null) return null;
      if (outcome.rubric !== null) return { rubric: rubricOf(outcome.rubric) };
      const failure = outcome.failure;
      if (failure === null) return null;
      return {
        failure: {
          situation: failure.situation as RubricSituation,
          detail: failure.detail,
        },
      };
    },
    stored: async () => {
      const found = await invoke<RubricViewPayload | null>("read_rubric");
      return found === null ? null : rubricOf(found);
    },
  };
}

/** El nombre del evento del arrastre. Es `commands::DOCUMENT_DROPPED`. */
const DOCUMENT_DROPPED = "document-dropped";

/**
 * Lo que llega al soltar, tal cual lo emite Rust. Es
 * `commands::DroppedDocumentView`, campo a campo: **un documento ya abierto o
 * un fallo, y ninguna ruta**.
 */
interface DroppedDocumentView {
  document: OpenedDocumentView | null;
  failure: { situation: string; detail: string } | null;
  ignored: number;
}

/**
 * El arrastre, por el evento nativo de la ventana (ID-67).
 *
 * Quién decide qué se abre de lo soltado está del otro lado: aquí no se mira
 * ninguna ruta porque ninguna llega. Lo que llega es lo mismo que devuelve el
 * diálogo, más el motivo cuando no se ha abierto nada y cuántos ficheros más
 * venían en el gesto.
 *
 * `listen` devuelve una promesa y la suscripción tiene que poder cancelarse
 * antes de que se resuelva —un efecto de React se limpia cuando quiere—, así
 * que se guarda la intención y se aplica cuando llegue: sin eso, desmontar
 * deprisa deja un oyente vivo escuchando para siempre.
 */
export function tauriDocumentDrops(): DocumentDrops {
  return {
    subscribe: (listener) => {
      let listening = true;
      const stopping = listen<DroppedDocumentView>(DOCUMENT_DROPPED, (event) => {
        if (listening) listener(dropOf(event.payload));
      });
      void stopping.then((stop) => {
        if (!listening) stop();
      });
      return () => {
        listening = false;
        void stopping.then((stop) => stop());
      };
    },
  };
}

/** Lo soltado, en el vocabulario de la ventana. */
function dropOf(view: DroppedDocumentView): Drop {
  return {
    document: view.document === null ? null : recentOf(view.document),
    failure:
      view.failure === null
        ? null
        : {
            situation: view.failure.situation as ErrorSituation,
            detail: view.failure.detail,
          },
    ignored: view.ignored,
  };
}

/**
 * Un documento recién abierto, como fila de la bandeja.
 *
 * Lo comparten el diálogo y el arrastre a propósito: soltar un PDF tiene que
 * dejar exactamente lo mismo que elegirlo, y dos conversiones parecidas es
 * justo por donde dejarían de serlo.
 */
function recentOf(opened: OpenedDocumentView): RecentDocument {
  return {
    id: opened.id,
    name: opened.name,
    // Un documento recién abierto se anota como **no firmado** (ID-71): saber
    // si un PDF ya trae firmas es otro trabajo, y el panel ya declara ese dato
    // como desconocido. Se anota lo que se sabe.
    badge: "Unsigned",
    modified: opened.modified,
    lastUsed: Math.floor(Date.now() / 1000),
    // Lo acaba de conceder el portal, así que responde.
    available: true,
    // Dónde cayó su recuadro la última vez lo sabe el backend, que guarda la
    // bandeja por ruta canónica: llega al anotarlo, no al abrirlo.
    placement: null,
  } satisfies RecentDocument;
}

/**
 * Una fila de la bandeja tal cual la devuelve Rust. Es
 * `commands::RecentDocumentView`, campo a campo: **un identificador opaco y un
 * nombre, ninguna ruta** (ADR-0011).
 *
 * `available` viene recalculado contra el disco de ahora mismo y no se persiste
 * nunca: una fila que no responde llega con `false` y **revive** cuando la ruta
 * reaparece.
 */
interface RecentDocumentView {
  id: string;
  name: string;
  badge: Badge;
  modified: number | null;
  lastUsed: number;
  available: boolean;
  placement: { rect: [number, number, number, number]; pages: PageSet } | null;
}

/**
 * La bandeja en el disco (ID-75).
 *
 * Tres de las cuatro operaciones son órdenes propias; la cuarta, «Vaciar la
 * lista», **ya era** `forget_activity` y no se duplica: vaciar la bandeja y
 * olvidar la actividad son la misma promesa (ID-34).
 *
 * Lo que cruza en las tres es el **identificador opaco** que acuñó el backend
 * al abrir (ID-62). La deduplicación de la bandeja sigue siendo por la ruta
 * canónica, que solo Rust conoce y que no sale de allí.
 */
export function tauriRecents(): RecentsStore {
  return {
    list: async () => (await invoke<RecentDocumentView[]>("list_recents")).map(rowOf),
    record: async (document) =>
      rowOf(
        await invoke<RecentDocumentView>("record_recent", {
          id: document.id,
          placement: document.placement && {
            rect: [
              document.placement.rect.x0,
              document.placement.rect.y0,
              document.placement.rect.x1,
              document.placement.rect.y1,
            ],
            pages: document.placement.pages,
          },
        }),
      ),
    forget: (id) => invoke<void>("forget_recent", { id }),
    clear: () => invoke<void>("forget_activity"),
  };
}

/** Una fila de la bandeja, en el vocabulario de la ventana. */
function rowOf(view: RecentDocumentView): RecentDocument {
  const [x0, y0, x1, y1] = view.placement?.rect ?? [0, 0, 0, 0];
  return {
    id: view.id,
    name: view.name,
    badge: view.badge,
    modified: view.modified,
    lastUsed: view.lastUsed,
    available: view.available,
    placement: view.placement && { rect: { x0, y0, x1, y1 }, pages: view.placement.pages },
  } satisfies RecentDocument;
}

/**
 * El PDF que se pinta: los bytes del portal, abiertos con `pdf.js` (ID-76).
 *
 * Los bytes viajan **como bytes** y no como una lista de números en JSON
 * (ID-66): `read_document` contesta con la respuesta binaria del puente de
 * Tauri, que aquí llega como un `ArrayBuffer`.
 */
export function tauriPdfSource(): PdfSource {
  return pdfjsSource(async (document) => {
    const bytes = await invoke<ArrayBuffer>("read_document", { id: document.id });
    return new Uint8Array(bytes);
  });
}

/**
 * La configuración tal como cruza: es `commands::ConfigurationView`, con el
 * destino **por su nombre** y sin una sola ruta (ADR-0011).
 */
interface ConfigurationView {
  language: string;
  destination: string;
  rememberVisibleSignature: boolean;
  rememberActivity: boolean;
  theme: Theme;
}

function readConfiguration(): Promise<ConfigurationView> {
  return invoke<ConfigurationView>("read_configuration");
}

function writeConfiguration(configuration: ConfigurationView): Promise<void> {
  return invoke<void>("write_configuration", { configuration });
}

/**
 * Los ajustes, guardados en el disco por `memory::Memory`.
 *
 * Cada escritura **relee** antes de escribir en vez de recordar lo último que
 * mandó: el idioma va por su propio puerto y se guarda en la misma
 * configuración, así que una copia local aquí se quedaría atrás en cuanto
 * alguien cambiara el idioma y devolvería el anterior en la escritura
 * siguiente.
 *
 * El destino que se manda es el que se leyó: la ventana lo enseña y no lo
 * elige —bajo el arenero hay una sola carpeta—, y el backend lo ignora.
 */
export function tauriPreferences(): PreferencesStore {
  return {
    read: async () => {
      const configuration = await readConfiguration();
      return {
        theme: isTheme(configuration.theme) ? configuration.theme : DEFAULT_THEME,
        destination: configuration.destination,
        rememberVisibleSignature: configuration.rememberVisibleSignature,
        rememberActivity: configuration.rememberActivity,
      };
    },
    save: async (preferences) => {
      const stored = await readConfiguration();
      await writeConfiguration({ ...stored, ...preferences });
    },
    forgetActivity: () => invoke<void>("forget_activity"),
    chooseFolder: () => invoke<string | null>("choose_destination"),
  };
}

/**
 * Dónde caerá el documento que hay delante: `preview_destination`.
 *
 * Lo compone el backend con la misma carpeta comprobada y el mismo
 * `landing_for` con los que va a escribir después, así que el pie enseña lo que
 * va a ocurrir y no una promesa parecida (ID-63, ID-67).
 */
export function tauriDestinations(): DestinationSource {
  return {
    previewFor: (documentId) => invoke<Destination>("preview_destination", { id: documentId }),
  };
}

/**
 * Abrir el PDF firmado y su carpeta: `open_signed_document` y
 * `open_signed_folder`.
 *
 * **No se les manda ninguna ruta**, porque la ventana no tiene ninguna
 * (ADR-0011): lo que abren es el fichero que dejó la última postfirma, que es
 * justo el que el resumen tiene delante. El complemento `opener` se llama desde
 * Rust por lo mismo que el del diálogo (ID-63, ID-85), y debajo es el portal
 * `OpenURI`.
 */
export function tauriSignedDocumentOpener(): SignedDocumentOpener {
  return {
    openDocument: () => invoke<void>("open_signed_document"),
    openFolder: () => invoke<void>("open_signed_folder"),
  };
}

/**
 * El idioma, en la misma configuración que los demás ajustes.
 *
 * Es un puerto aparte porque el idioma se lee **antes** de que haya ventana
 * —`createI18n` lo necesita para el primer pintado— y los ajustes solo se leen
 * al montar la aplicación. Debajo es el mismo fichero.
 */
export function tauriLanguagePreference(): LanguagePreference {
  return {
    read: async () => {
      const { language } = await readConfiguration();
      return isLanguageTag(language) ? language : FALLBACK_LANGUAGE;
    },
    save: async (language) => {
      const stored = await readConfiguration();
      await writeConfiguration({ ...stored, language });
    },
  };
}
