/**
 * Los puertos que hablan con Tauri: los dos de firma que quedan (#60, #194),
 * los dos del documento (#82), el del arrastre (#83), los dos de la
 * configuración —los ajustes y el idioma—, que comparten fichero debajo, y el
 * de la rúbrica (#128).
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
 * pruebas siguen hablando con `CertificateStore`, `SigningBackend`,
 * `DocumentPicker`, `PdfSource`, `DocumentDrops` y `RubricPicker`, y quien
 * elige entre estas implementaciones y los dobles de memoria es `main.tsx`.
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
import type { UrlHandlerChoice, UrlHandlers } from "./desktop/urlHandlers";
import type { Badge, DocumentInHand } from "./documents/document";
import type { DocumentDrops, Drop } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import type { RecentDocument, RecentsStore } from "./documents/recents";
import { classify } from "./errors/classify";
import type { ErrorSituation } from "./errors/ErrorNotice";
import { FALLBACK_LANGUAGE, isLanguageTag } from "./i18n/languages";
import type { LanguagePreference } from "./i18n/preference";
import type { PreferencesStore } from "./preferences/preferences";
import { DEFAULT_THEME, isTheme, type Theme } from "./preferences/theme";
import type { SiteErrandPort } from "./sede/errand";
import { type SiteErrandView, siteErrands } from "./sede/siteErrands";
import type { Certificate, CertificateStore } from "./signing/certificate";
import type { Destination, DestinationSource, SignedDocumentOpener } from "./signing/destination";
import type { SignedDocument, SigningBackend, StageResult } from "./signing/flow";
import type { Rubric, RubricPicker, RubricSituation } from "./signing/rubric";
import type { StoreSecret } from "./signing/secret";
import type { StampComposer } from "./signing/stampPreview";
import type { TokenFailure } from "./signing/token";
import type { NewVersion, VersionCheck } from "./updates/newVersion";
import { pdfjsLoader } from "./viewer/pdfjsLoader";
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

/**
 * Los certificados de los tokens conectados, y los dos gestos de Preferencias
 * sobre los `.p12` instalados. Listar no pide el PIN.
 */
export function tauriCertificateStore(): CertificateStore {
  return {
    list: () => invoke<readonly Certificate[]>("list_certificates"),
    install: (password) => invoke<boolean>("install_certificate", { password }),
    remove: (id) => invoke<void>("remove_certificate", { id }),
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
    presign: (order) => stage(() => invoke<StoreSecret>("begin_signing", { order })),
    sign: (pin) => stage(() => invoke<void>("sign_with_pin", { pin })),
    postsign: () => stage(() => invoke<SignedDocument>("finish_signing")),
    padesLowerLeft: (placement) => invoke<[number, number]>("pades_lower_left", { placement }),
    unregisteredSignatures: (document) => invoke<boolean>("unregistered_signatures", { document }),
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
      return opened === null ? null : inHandOf(opened);
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
  /** El resto de PDF del mismo gesto: entran igual en Recientes (ID-306). */
  alsoEntering: OpenedDocumentView[];
  failure: { situation: string; detail: string } | null;
  discarded: number;
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
    pending: async () => {
      const invoked = await invoke<DroppedDocumentView | null>("read_invocation");
      return invoked === null ? null : dropOf(invoked);
    },
  };
}

/** Lo soltado, en el vocabulario de la ventana. */
function dropOf(view: DroppedDocumentView): Drop {
  return {
    document: view.document === null ? null : inHandOf(view.document),
    alsoEntering: view.alsoEntering.map(inHandOf),
    failure:
      view.failure === null
        ? null
        : {
            situation: view.failure.situation as ErrorSituation,
            detail: view.failure.detail,
          },
    discarded: view.discarded,
  };
}

/**
 * Un documento recién abierto, **puesto delante**.
 *
 * Lo comparten el diálogo y el arrastre a propósito: soltar un PDF tiene que
 * dejar exactamente lo mismo que elegirlo, y dos conversiones parecidas es
 * justo por donde dejarían de serlo.
 *
 * Sale con `remembered` en `true` porque los dos caminos son una persona
 * eligiendo un fichero suyo: de eso queda rastro (ID-34). El documento que no
 * se recuerda es el que mandará una sede (ID-286), y entra por otro puerto.
 */
function inHandOf(opened: OpenedDocumentView): DocumentInHand {
  return {
    id: opened.id,
    name: opened.name,
    // Un documento recién abierto se tiene por **no firmado** (ID-71): saber
    // si un PDF ya trae firmas es otro trabajo, y el panel ya declara ese dato
    // como desconocido. Se anota lo que se sabe.
    badge: "Unsigned",
    modified: opened.modified,
    // Dónde cayó su recuadro la última vez lo sabe el backend, que guarda la
    // bandeja por ruta canónica: llega al anotarlo, no al abrirlo.
    placement: null,
    remembered: true,
  } satisfies DocumentInHand;
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
 * El sello de verdad, antes de firmar: `preview_signature` (ID-107).
 *
 * Debajo hay un **ciclo trifásico en seco** con un `PK1` inventado, que
 * devuelve un PDF cuyos bytes visibles están medidos idénticos a los del
 * firmado de verdad. No pide PIN —el certificado elegido es público y se lee
 * sin él— y lo que devuelve **se tira**: firmar de verdad vuelve a prefirmar
 * desde cero.
 *
 * Los bytes viajan como bytes, igual que en `read_document`, y se abren con el
 * mismo `pdf.js` que pinta el original: por eso la vista previa no es una
 * imitación de nada, es el mismo compositor y el mismo lector.
 */
export function tauriStampComposer(): StampComposer {
  const loader = pdfjsLoader();
  return {
    compose: async (order) => {
      try {
        const bytes = await invoke<ArrayBuffer>("preview_signature", { order });
        return { ok: true, pdf: await loader.load(new Uint8Array(bytes)) };
      } catch (thrown) {
        // La vista previa **no es una puerta** (ID-111): el fallo se cuenta y
        // se sigue pudiendo firmar, así que aquí no se relanza nada.
        const named = classify(thrown);
        return {
          ok: false,
          failure: {
            situation: named.situation === "unknown" ? "documentUnreadable" : named.situation,
            detail: named.detail,
          },
        };
      }
    },
  };
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
  notifyNewVersion: boolean;
  theme: Theme;
  /**
   * **La única pregunta al entorno** (ID-184): si Preferencias puede ofrecer
   * «Junto al documento original». La contesta el backend; escribirla no
   * sirve de nada, así que no cruza al revés.
   */
  offersTheOriginalFolder: boolean;
  /**
   * Si el aviso del primer arranque (CA local y permiso de red local, #365)
   * ya se ha descartado. Viaja en los dos sentidos: se lee para decidir si el
   * aviso se monta y se escribe una vez, al pulsar «Entendido».
   */
  trustNoticeSeen: boolean;
  /**
   * Si al arrancar se pregunta quién atiende los enlaces `afirma://`
   * (ID-239). Viaja en los dos sentidos: el banner lo apaga y Preferencias lo
   * vuelve a encender.
   */
  askAboutUrlHandler: boolean;
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
 * elige —bajo el sandbox hay una sola carpeta—, y el backend lo ignora.
 *
 * `offersOriginalFolder` tampoco cruza al escribir: la contesta el backend
 * (ID-184), así que `save` proyecta explícitamente las claves que sí son del
 * contrato de `ConfigurationView`, en vez de mandar `preferences` entero.
 */
export function tauriPreferences(): PreferencesStore {
  return {
    read: async () => {
      const configuration = await readConfiguration();
      return {
        theme: isTheme(configuration.theme) ? configuration.theme : DEFAULT_THEME,
        destination: configuration.destination,
        offersOriginalFolder: configuration.offersTheOriginalFolder,
        rememberVisibleSignature: configuration.rememberVisibleSignature,
        rememberActivity: configuration.rememberActivity,
        notifyNewVersion: configuration.notifyNewVersion,
        trustNoticeSeen: configuration.trustNoticeSeen,
        askAboutUrlHandler: configuration.askAboutUrlHandler,
      };
    },
    save: async (preferences) => {
      const stored = await readConfiguration();
      await writeConfiguration({
        ...stored,
        theme: preferences.theme,
        rememberVisibleSignature: preferences.rememberVisibleSignature,
        rememberActivity: preferences.rememberActivity,
        notifyNewVersion: preferences.notifyNewVersion,
        trustNoticeSeen: preferences.trustNoticeSeen,
        askAboutUrlHandler: preferences.askAboutUrlHandler,
      });
    },
    forgetActivity: () => invoke<void>("forget_activity"),
    chooseFolder: () => invoke<string | null>("choose_destination"),
  };
}

/**
 * Quién atiende los enlaces `afirma://`: `url_handlers` y `choose_url_handler`.
 *
 * Los dos son órdenes y no un evento: la ventana pregunta al arrancar, para el
 * banner y para Preferencias, y escribe cuando la persona elige. Lo que hay
 * debajo —el canal, GIO y el `mimeapps.list` del `$HOME`— no cruza: lo que
 * cruza es lo que se puede saber y lo que se puede elegir (ID-238, ID-240).
 */
export function tauriUrlHandlers(): UrlHandlerChoice {
  return {
    who: () => invoke<UrlHandlers>("url_handlers"),
    choose: (handler) => invoke<void>("choose_url_handler", { handler }),
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

/**
 * Si hay una versión nueva publicada.
 *
 * Aquí no hay ni URL ni caché ni comparación de versiones: todo eso es de
 * `app::version`, que es quien pregunta —como mucho una vez cada 24 h— y quien
 * decide que sin red no se dice nada. La orden contesta `null` en los tres
 * casos en que no hay nada que contar, y `null` es lo que llega a la ventana.
 */
export function tauriVersionCheck(): VersionCheck {
  return {
    latest: async () => await invoke<NewVersion | null>("check_for_new_version"),
  };
}

/** El nombre del evento del trámite de sede. Es `commands::SITE_ERRAND`. */
const SITE_ERRAND = "site-errand";

/**
 * **El trámite de una sede, por sus órdenes y su evento** (ID-336, ID-338).
 *
 * Es el puerto que sustituye a `noErrand()` en la ventana de sede, y aquí sólo
 * está la mitad que sabe que debajo hay Tauri: una línea por orden. Lo que hay
 * que pensar —convertir cada momento en el que la ventana espera, y los dos
 * momentos que no vienen del backend— vive en `sede/siteErrands.ts`, que se
 * prueba sin Tauri (TD-78).
 *
 * `watch` escucha **el evento y no un sondeo**: el trámite empuja cada momento
 * nuevo, y que no llegue ninguno es la respuesta normal. La suscripción se
 * guarda como intención igual que la del arrastre, porque `listen` devuelve una
 * promesa y desmontar deprisa dejaría un oyente vivo para siempre.
 *
 * `sign_with_pin` es **la misma orden** que el recorrido local, y no una gemela
 * de sede: la fase que toca la clave privada no sabe de sedes (ADR-0001).
 *
 * `site_install_certificate` recibe la contraseña del `.p12` y esta pantalla no
 * la pide —no hay dónde: `SedeNoCertificate` tiene un botón y nada más—, así
 * que va vacía. Instala un `.p12` sin contraseña; con una, la orden falla y la
 * pantalla se queda como estaba, igual que al descartar el diálogo.
 */
export function tauriSiteErrands(): SiteErrandPort {
  const loader = pdfjsLoader();

  return siteErrands({
    watch: (onView) => {
      let listening = true;
      const stopping = listen<SiteErrandView>(SITE_ERRAND, (event) => {
        if (listening) onView(event.payload);
      });
      void stopping.then((stop) => {
        if (!listening) stop();
      });
      return () => {
        listening = false;
        void stopping.then((stop) => stop());
      };
    },
    identify: (certificate) => stage(() => invoke<void>("site_identify", { certificate })),
    decline: () => invoke<void>("site_decline"),
    beginSigning: (certificate) =>
      stage(() => invoke<StoreSecret>("site_begin_signing", { certificate })),
    signWithPin: (pin) => stage(() => invoke<void>("sign_with_pin", { pin })),
    finishSigning: () => stage(() => invoke<void>("site_finish_signing")),
    // Un `.p12` con contraseña —el caso normal— rechaza aquí, y desde esta
    // pantalla no hay contraseña que mandar: lo que le queda a la persona es la
    // misma pantalla, no una promesa sin recoger.
    installCertificate: () =>
      invoke<boolean>("site_install_certificate", { password: "" }).catch(() => false),
    lookAgain: () => invoke<void>("site_look_again"),
    installLocalCa: () => invoke<void>("install_local_ca"),
    closeWindow: () => invoke<void>("close_site_window"),
    // Los bytes viajan como bytes, igual que en `read_document` de la ventana
    // principal, y se abren con el mismo `pdf.js`: el tamaño sale de los bytes
    // porque no hay una segunda forma de saberlo —de la ruta del fichero de
    // paso no llega nada (ADR-0011)—. Que no se puedan leer no es un fallo que
    // enseñar: es que no hay tarjeta que pintar.
    describeDocument: async (id) => {
      try {
        const bytes = new Uint8Array(await invoke<ArrayBuffer>("read_document", { id }));
        const pdf = await loader.load(bytes);
        return { title: pdf.title ?? null, pages: pdf.pageCount, sizeBytes: bytes.byteLength };
      } catch {
        return null;
      }
    },
  });
}
