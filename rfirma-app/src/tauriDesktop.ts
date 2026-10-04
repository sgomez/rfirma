//! Los puertos de Tauri del escritorio: los destinos externos, la versión del binario y la comprobación de versión nueva.

import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import type { ExternalDestinationOpener } from "./desktop/externalDestination";
import type { Installation, NewVersion, VersionCheck } from "./updates/newVersion";

export function tauriExternalDestinationOpener(): ExternalDestinationOpener {
  return {
    open: (destination) => invoke<void>("open_external_destination", { target: destination }),
  };
}

/**
 * Si hay una versión nueva publicada.
 *
 * Aquí no hay ni URL ni caché ni comparación de versiones: todo eso es de
 * `app::version`, que es quien pregunta —siempre— y quien decide que sin red
 * no se dice nada. La orden contesta `null` en los tres
 * casos en que no hay nada que contar, y `null` es lo que llega a la ventana.
 */
export function tauriVersionCheck(): VersionCheck {
  return {
    latest: async () => await invoke<NewVersion | null>("check_for_new_version"),
    install: async () => await invoke<Installation>("install_new_version"),
  };
}

/** La versión del binario en ejecución, la de `Cargo.toml`. */
export async function tauriAppVersion(): Promise<string> {
  return await getVersion();
}
