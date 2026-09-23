/** Los puertos de Tauri del documento: el portal, el arrastre, la bandeja y el visor (#82, #83). */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Badge, DocumentInHand } from "./documents/document";
import type { DocumentDrops, Drop } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import type { RecentDocument, RecentsStore } from "./documents/recents";
import type { ErrorSituation } from "./errors/ErrorNotice";
import type { PageSet } from "./viewer/signatureBox";
import { type PdfSource, pdfjsSource } from "./viewer/source";

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
