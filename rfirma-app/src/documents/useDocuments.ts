import { useCallback, useEffect, useState } from "react";
import type { DocumentPicker } from "./picker";
import type { RecentDocument, RecentsStore } from "./recents";

export interface Documents {
  /** Los recientes, el más reciente primero. */
  recents: RecentDocument[];
  /** El documento que se está firmando, o `null` si no hay ninguno. */
  active: RecentDocument | null;
  /** Abre un documento por el portal. Ver [`DocumentPicker`]. */
  open: () => Promise<void>;
  /** Cambia de documento desde una fila de la bandeja. */
  select: (document: RecentDocument) => void;
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
  const [active, setActive] = useState<RecentDocument | null>(null);

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

  const open = useCallback(async () => {
    const chosen = await picker.choose();
    if (chosen === null) return;
    // El documento se abre igual; lo que la preferencia decide es si queda
    // rastro de haberlo abierto.
    if (remember) {
      await store.record(chosen);
      setRecents(await store.list());
    }
    setActive(chosen);
  }, [picker, store, remember]);

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

  return { recents, active, open, select: setActive, forget, forgetAll };
}
