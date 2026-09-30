/** El puerto de la barra de título nativa: el estado que la ventana le manda y las acciones que devuelve. */

export type TitlebarAction = "open" | "status" | "preferences" | "feedback" | "about";

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
}

interface TitlebarState {
  openVisible: boolean;
  warningVisible: boolean;
  labels: TitlebarLabels;
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
