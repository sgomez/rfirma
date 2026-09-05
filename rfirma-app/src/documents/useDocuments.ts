import { useCallback, useEffect, useState } from "react";
import type { Placement } from "../viewer/signatureBox";
import type { DocumentInHand } from "./document";
import type { DocumentPicker } from "./picker";
import { place as placedIn, type RecentDocument, type RecentsStore, taken } from "./recents";

export interface Documents {
  /** Los recientes, el más reciente primero. */
  recents: RecentDocument[];
  /**
   * El documento que se está firmando, o `null` si no hay ninguno.
   *
   * **No es una fila de la bandeja** (ID-287): puede no tener ninguna detrás,
   * y entonces de él no queda rastro en ninguna parte.
   */
  active: DocumentInHand | null;
  /** Abre un documento por el portal. Ver [`DocumentPicker`]. */
  open: () => Promise<void>;
  /**
   * Recibe un documento que **ya viene abierto**: el que se ha soltado en la
   * ventana.
   *
   * Es la mitad de [`open`] que no habla con el portal, y existe para que
   * arrastrar acabe exactamente donde acaba el diálogo —anotado en la bandeja
   * si toca, y activo— en vez de por un camino paralelo que se pareciera.
   */
  accept: (document: DocumentInHand) => Promise<void>;
  /** Cambia de documento desde una fila de la bandeja. Ver `taken` en `recents.ts`. */
  select: (row: RecentDocument) => void;
  /**
   * **Vuelve a leer del disco el documento que hay delante**, sin cambiar de
   * documento.
   *
   * Es lo que hay debajo de «Volver a firmar» (ID-80): abrir el mismo documento
   * otra vez, porque entre una firma y la siguiente el usuario ha podido
   * modificarlo fuera. No hay ningún mecanismo nuevo para el recuadro que ya no
   * quepa —si el documento releído tiene menos páginas avisa el aviso del
   * ID-22— porque lo que se repone es la fila de la bandeja, igual que al
   * seleccionarlo.
   *
   * Sin documento delante no hace nada.
   */
  reopen: () => void;
  /**
   * Apunta dónde ha caído el recuadro **del documento que hay delante**.
   *
   * Se guarda en su fila y no en un ajuste global porque la posición es de este
   * documento y de esta página (ID-74): el siguiente documento arranca con el
   * recuadro donde toque, no donde lo dejó el anterior (ID-22).
   *
   * Sin documento activo no hace nada; con «Recordar mi actividad» apagado
   * tampoco, y de un documento que no se recuerda tampoco: en los dos últimos
   * casos no hay fila donde apuntarlo (ID-286).
   */
  place: (placement: Placement | null) => Promise<void>;
  /** Quita una fila de la lista. Ver `forget` en `recents.ts`. */
  forget: (id: string) => Promise<void>;
  /**
   * Vacía la bandeja entera. Es lo que disparan «Vaciar la lista» y apagar
   * «Recordar mi actividad» en Preferencias.
   */
  forgetAll: () => Promise<void>;
}

/**
 * El estado de la bandeja: qué documentos hay y cuál se está firmando.
 *
 * Las dos dependencias entran por parámetro y no se construyen aquí para que
 * la bandeja se pueda probar entera sin backend, que es lo que pide la grada A
 * de este sub-issue.
 *
 * `remember` es «Recordar mi actividad» (ID-34), y aquí es donde manda: con la
 * preferencia apagada, abrir un documento **no** lo apunta en la bandeja. Sin
 * este dueño, apagarla solo purgaba una vez y el siguiente documento volvía a
 * quedarse, con lo que el estado «Vacía … o con «Recordar mi actividad»
 * apagado» de `bandeja-de-documentos.md` era inalcanzable.
 */
export function useDocuments(
  store: RecentsStore,
  picker: DocumentPicker,
  remember = true,
): Documents {
  const [recents, setRecents] = useState<RecentDocument[]>([]);
  const [active, setActive] = useState<DocumentInHand | null>(null);

  // Los recientes se pintan con lo cacheado: esta es la única lectura, y no
  // abre ningún PDF (ADR-0010). Revalidar —comparar el `mtime`— es cosa del
  // documento que se selecciona, y de quien sabe leer PDFs.
  useEffect(() => {
    let current = true;
    store.list().then((entries) => {
      if (current) setRecents(entries);
    });
    return () => {
      current = false;
    };
  }, [store]);

  const accept = useCallback(
    async (document: DocumentInHand) => {
      // El documento se abre igual; lo que se decide aquí es si queda rastro
      // de haberlo abierto, y son dos cosas distintas las que pueden decir que
      // no: la preferencia «Recordar mi actividad» (ID-34) y el documento
      // mismo, que puede venir de donde no se recuerda nada (ID-286).
      if (!remember || !document.remembered) {
        setActive(document);
        return;
      }
      // Lo que vuelve de anotarlo es la fila que el almacén ya tenía de este
      // documento: su insignia cacheada y dónde había caído su recuadro. Es
      // así como un documento que ya estuvo abierto repone su página y su
      // posición sin que la ventana guarde nada por su cuenta.
      const noted = await store.record(document);
      setRecents(await store.list());
      setActive(taken(noted));
    },
    [store, remember],
  );

  const open = useCallback(async () => {
    const chosen = await picker.choose();
    if (chosen === null) return;
    await accept(chosen);
  }, [picker, accept]);

  const forget = useCallback(
    async (id: string) => {
      await store.forget(id);
      setRecents(await store.list());
      setActive((current) => (current?.id === id ? null : current));
    },
    [store],
  );

  const forgetAll = useCallback(async () => {
    await store.clear();
    setRecents(await store.list());
    setActive(null);
  }, [store]);

  const place = useCallback(
    async (placement: Placement | null) => {
      // Sin fila detrás no hay dónde apuntarlo, y de un documento que no se
      // recuerda no la hay nunca (ID-286): el recuadro se queda donde la
      // ventana lo tiene puesto y se pierde al cerrarlo.
      if (!remember || active === null || !active.remembered) return;
      // La fila se actualiza, el documento activo **no**: cambiarlo volvería a
      // disparar la apertura del PDF en cada arrastre del recuadro, y lo que ha
      // cambiado no es qué documento hay delante sino dónde cae su firma.
      setRecents((current) => placedIn(current, active.id, placement));
      await store.record({ ...active, placement });
    },
    [store, remember, active],
  );

  // El documento activo se repone **desde su fila**, y no desde la copia que
  // hay en `active`: el arrastre del recuadro actualiza la fila y a propósito
  // no toca el documento activo, así que reponer la copia devolvería el
  // recuadro a donde estaba al abrirlo y perdería lo último arrastrado.
  const reopen = useCallback(() => {
    setActive((current) => {
      if (current === null) return null;
      const row = recents.find((entry) => entry.id === current.id);
      return row === undefined ? { ...current } : taken(row);
    });
  }, [recents]);

  const select = useCallback((row: RecentDocument) => setActive(taken(row)), []);

  return { recents, active, open, accept, select, reopen, place, forget, forgetAll };
}
