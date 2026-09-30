/** El puerto de la barra de título nativa: el estado que la ventana le manda y las acciones que devuelve. */

export type TitlebarActionName =
  | "open"
  | "status"
  | "preferences"
  | "feedback"
  | "about"
  | "clearRecents";

/** Lo que devuelve la barra: el control pulsado y, en un reciente, su identificador. */
export type TitlebarAction = { action: TitlebarActionName } | { action: "recent"; path: string };

/** Las etiquetas de la barra, ya traducidas por la ventana. */
interface TitlebarLabels {
  open: string;
  openTooltip: string;
  warning: string;
  menu: string;
  status: string;
  preferences: string;
  feedback: string;
  about: string;
  recents: string;
  clearRecents: string;
  notFound: string;
}

/** Lo que pinta cada entrada de «Abiertos recientemente»; `path` es el identificador del reciente. */
export interface TitlebarRecent {
  path: string;
  name: string;
  folder: string;
  signed: boolean;
  found: boolean;
}

interface TitlebarState {
  openVisible: boolean;
  warningVisible: boolean;
  labels: TitlebarLabels;
  recents: readonly TitlebarRecent[];
}

export interface NativeTitlebar {
  show(state: TitlebarState): void;
  /** Devuelve con qué dejar de escuchar. */
  onAction(listener: (action: TitlebarAction) => void): () => void;
}

export interface InMemoryNativeTitlebar extends NativeTitlebar {
  readonly shown: readonly TitlebarState[];
  readonly latest: TitlebarState | null;
  press(action: TitlebarAction): void;
}

export function inMemoryNativeTitlebar(): InMemoryNativeTitlebar {
  const shown: TitlebarState[] = [];
  const listeners = new Set<(action: TitlebarAction) => void>();
  return {
    get shown() {
      return shown;
    },
    get latest() {
      return shown.at(-1) ?? null;
    },
    show: (state) => {
      shown.push(state);
    },
    onAction: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    press: (action) => {
      for (const listener of [...listeners]) listener(action);
    },
  };
}

/** La barra de las plataformas que no la tienen: no enseña nada y no emite nada. */
export function absentNativeTitlebar(): NativeTitlebar {
  return {
    show: () => {},
    onAction: () => () => {},
  };
}
