import { useCallback, useEffect, useRef, useState } from "react";
import type { Placement } from "../viewer/signatureBox";
import type { DocumentInHand } from "./document";
import type { DocumentPicker } from "./picker";
import { place as placedIn, type RecentDocument, type RecentsStore, taken } from "./recents";

export interface Documents {
  /** Los recientes, el más reciente primero. */
  recents: RecentDocument[];
  /** Los documentos abiertos, uno por pestaña, en el orden en que se abrieron. */
  tabs: DocumentInHand[];
  /** El documento de la pestaña activa, o `null` si no hay ninguno abierto. */
  active: DocumentInHand | null;
  /** Abre un documento por el portal en una pestaña nueva, y la activa. */
  open: () => Promise<void>;
  /** Abre en una pestaña, y la activa, un documento que ya viene abierto. */
  accept: (document: DocumentInHand) => Promise<void>;
  /** Abre en una pestaña un documento que ya viene abierto, sin activarla. */
  enter: (document: DocumentInHand) => Promise<void>;
  /** Abre un reciente, o activa su pestaña si ya está abierto. */
  select: (row: RecentDocument) => void;
  /** Activa la pestaña de un documento abierto. */
  activate: (id: string) => void;
  /** Cierra una pestaña; si era la activa, pasa a la de al lado. */
  close: (id: string) => void;
  /** Vuelve a leer del disco el documento activo, sin cambiar de pestaña. */
  reopen: () => void;
  /** Apunta dónde ha caído el recuadro del documento activo. */
  place: (placement: Placement | null) => Promise<void>;
  /** Vacía los recientes; lo abierto sigue abierto, pero deja de recordarse. */
  clearRecents: () => Promise<void>;
  /** Vacía los recientes y cierra todas las pestañas. */
  forgetAll: () => Promise<void>;
}

/** El estado de los documentos abiertos y de los recientes. */
export function useDocuments(
  store: RecentsStore,
  picker: DocumentPicker,
  remember = true,
): Documents {
  const [recents, setRecents] = useState<RecentDocument[]>([]);
  const [tabs, setTabs] = useState<DocumentInHand[]>([]);
  const [active, setActive] = useState<DocumentInHand | null>(null);
  const tabsNow = useRef<DocumentInHand[]>([]);
  const activeNow = useRef<DocumentInHand | null>(null);

  const commitTabs = useCallback((next: DocumentInHand[]) => {
    tabsNow.current = next;
    setTabs(next);
  }, []);

  const commitActive = useCallback((next: DocumentInHand | null) => {
    activeNow.current = next;
    setActive(next);
  }, []);

  useEffect(() => {
    let current = true;
    store.list().then((entries) => {
      if (current) setRecents(entries);
    });
    return () => {
      current = false;
    };
  }, [store]);

  const show = useCallback(
    (document: DocumentInHand, front: boolean) => {
      const existing = tabsNow.current.find((tab) => tab.id === document.id);
      if (existing === undefined) commitTabs([...tabsNow.current, document]);
      if (front && activeNow.current?.id !== document.id) commitActive(existing ?? document);
    },
    [commitTabs, commitActive],
  );

  const noted = useCallback(
    async (document: DocumentInHand) => {
      if (!remember || !document.remembered) return document;
      const row = await store.record(document);
      setRecents(await store.list());
      return taken(row);
    },
    [store, remember],
  );

  const accept = useCallback(
    async (document: DocumentInHand) => show(await noted(document), true),
    [noted, show],
  );

  const enter = useCallback(
    async (document: DocumentInHand) => show(await noted(document), false),
    [noted, show],
  );

  const open = useCallback(async () => {
    const chosen = await picker.choose();
    if (chosen === null) return;
    await accept(chosen);
  }, [picker, accept]);

  const select = useCallback((row: RecentDocument) => show(taken(row), true), [show]);

  const activate = useCallback(
    (id: string) => {
      const tab = tabsNow.current.find((one) => one.id === id);
      if (tab !== undefined) show(tab, true);
    },
    [show],
  );

  const close = useCallback(
    (id: string) => {
      const index = tabsNow.current.findIndex((tab) => tab.id === id);
      if (index < 0) return;
      const rest = tabsNow.current.filter((tab) => tab.id !== id);
      commitTabs(rest);
      if (activeNow.current?.id === id) commitActive(rest[index] ?? rest[index - 1] ?? null);
    },
    [commitTabs, commitActive],
  );

  const clearRecents = useCallback(async () => {
    await store.clear();
    setRecents(await store.list());
    commitTabs(tabsNow.current.map((tab) => ({ ...tab, remembered: false })));
  }, [store, commitTabs]);

  const forgetAll = useCallback(async () => {
    await store.clear();
    setRecents(await store.list());
    commitTabs([]);
    commitActive(null);
  }, [store, commitTabs, commitActive]);

  // El documento activo no cambia al colocar el recuadro: cambiarlo volvería a
  // abrir el PDF en cada arrastre. Lo que cambia es su pestaña y su fila.
  const place = useCallback(
    async (placement: Placement | null) => {
      const tab = tabsNow.current.find((one) => one.id === activeNow.current?.id);
      if (tab === undefined) return;
      const placed = { ...tab, placement };
      commitTabs(tabsNow.current.map((one) => (one.id === tab.id ? placed : one)));
      if (!remember || !tab.remembered) return;
      setRecents((current) => placedIn(current, tab.id, placement));
      await store.record(placed);
    },
    [store, remember, commitTabs],
  );

  const reopen = useCallback(() => {
    const current = activeNow.current;
    if (current === null) return;
    const tab = tabsNow.current.find((one) => one.id === current.id);
    commitActive({ ...(tab ?? current) });
  }, [commitActive]);

  return {
    recents,
    tabs,
    active,
    open,
    accept,
    enter,
    select,
    activate,
    close,
    reopen,
    place,
    clearRecents,
    forgetAll,
  };
}
