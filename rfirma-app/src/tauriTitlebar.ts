/** El puerto de Tauri de la barra de título nativa: `apply_titlebar_state` y el evento `titlebar-action`. */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { NativeTitlebar, TitlebarAction } from "./shell/nativeTitlebar";

const TITLEBAR_ACTION = "titlebar-action";

export function tauriNativeTitlebar(): NativeTitlebar {
  return {
    show: (state) => {
      void invoke("apply_titlebar_state", { state });
    },
    onAction: (listener) => {
      let listening = true;
      const stopping = listen<TitlebarAction>(TITLEBAR_ACTION, (event) => {
        if (listening) listener(event.payload);
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
