export const TAB_MIN_WIDTH = 160;
export const TAB_GAP = 2;
/** Lo que ocupa «+N ▾» con su aire, reservado solo cuando alguna pestaña no cabe. */
export const MORE_WIDTH = 60;

/** Las pestañas que se ven, en su orden, y las que quedan en «+N». */
export interface TabLayout<T> {
  visible: T[];
  hidden: T[];
}

/** Reparte las pestañas en el ancho dado; la activa nunca queda oculta. */
export function layOutTabs<T extends { id: string }>(
  width: number,
  tabs: readonly T[],
  activeId: string | null,
): TabLayout<T> {
  if (tabs.length <= capacity(width)) return { visible: [...tabs], hidden: [] };

  const slots = Math.max(1, capacity(width - MORE_WIDTH));
  const active = tabs.find((tab) => tab.id === activeId);
  const leading = tabs.slice(0, slots);
  const visible =
    active === undefined || leading.includes(active)
      ? leading
      : [...tabs.slice(0, slots - 1), active];
  return { visible, hidden: tabs.filter((tab) => !visible.includes(tab)) };
}

function capacity(width: number): number {
  return Math.max(0, Math.floor((width + TAB_GAP) / (TAB_MIN_WIDTH + TAB_GAP)));
}
