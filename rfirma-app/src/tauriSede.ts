/** El puerto de Tauri del trámite de sede: sus órdenes y el evento que lo empuja (ID-336, ID-338). */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SiteErrandPort } from "./sede/errand";
import { type SiteErrandView, siteErrands } from "./sede/siteErrands";
import type { StoreSecret } from "./signing/secret";
import { stage } from "./tauriStage";
import { pdfjsLoader } from "./viewer/pdfjsLoader";

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
    readErrand: () => invoke<SiteErrandView | null>("read_site_errand"),
    identify: (certificate) => stage(() => invoke<void>("site_identify", { certificate })),
    confirmSignatures: () => stage(() => invoke<void>("site_confirm_signatures")),
    decline: () => invoke<void>("site_decline"),
    beginSigning: (certificate) =>
      stage(() => invoke<StoreSecret>("site_begin_signing", { certificate })),
    signWithPin: (pin) => stage(() => invoke<void>("sign_with_pin", { pin })),
    finishSigning: () => stage(() => invoke<void>("site_finish_signing")),
    saveFile: () => stage(() => invoke<boolean>("site_save_file")),
    loadFiles: () => stage(() => invoke<number | null>("site_load_files")),
    // Un `.p12` con contraseña —el caso normal— rechaza aquí, y desde esta
    // pantalla no hay contraseña que mandar: lo que le queda a la persona es la
    // misma pantalla, no una promesa sin recoger.
    installCertificate: () =>
      invoke<boolean>("site_install_certificate", { password: "" }).catch(() => false),
    lookAgain: () => invoke<void>("site_look_again"),
    installLocalCa: () => invoke<void>("install_local_ca"),
    closeWindow: () => invoke<void>("close_site_window"),
    dismissWarning: () => invoke<void>("site_dismiss_the_warning"),
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
