//! El puerto de Tauri del trámite de sede: sus órdenes y el evento que lo empuja.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SiteErrandPort } from "./sede/errand";
import { type SiteErrandView, siteErrands } from "./sede/siteErrands";
import {
  NO_PREVIOUS_SIGNATURES,
  type PreviousSignaturesReport,
} from "./signing/previousSignatures";
import type { StoreSecret } from "./signing/secret";
import { stage } from "./tauriStage";
import { pdfjsLoader } from "./viewer/pdfjsLoader";

/** El nombre del evento del trámite de sede. Es `commands::SITE_ERRAND`. */
const SITE_ERRAND = "site-errand";

/**
 * **El trámite de una sede, por sus órdenes y su evento**.
 *
 * Es el puerto que sustituye a `noErrand()` en la ventana de sede, y aquí sólo
 * está la mitad que sabe que debajo hay Tauri: una línea por orden. Lo que hay
 * que pensar —convertir cada momento en el que la ventana espera, y los dos
 * momentos que no vienen del backend— vive en `sede/siteErrands.ts`, que se
 * prueba sin Tauri.
 *
 * `watch` escucha **el evento y no un sondeo**: el trámite empuja cada momento
 * nuevo, y que no llegue ninguno es la respuesta normal. La suscripción se
 * guarda como intención igual que la del arrastre, porque `listen` devuelve una
 * promesa y desmontar deprisa dejaría un oyente vivo para siempre.
 *
 * `sign_with_pin` es **la misma orden** que el recorrido local, y no una gemela
 * de sede: la fase que toca la clave privada no sabe de sedes (ADR-0001).
 *
 * `site_install_certificate` es la misma orden que la ventana principal: abre
 * el selector y pide la contraseña por su propio diálogo, con reintentos.
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
    markArea: (area) => stage(() => invoke<boolean>("site_mark_area", { area })),
    decline: () => invoke<void>("site_decline"),
    beginSigning: (certificate) =>
      stage(() => invoke<StoreSecret>("site_begin_signing", { certificate })),
    signWithPin: (pin) => stage(() => invoke<void>("sign_with_pin", { pin })),
    finishSigning: () => stage(() => invoke<void>("site_finish_signing")),
    saveFile: () => stage(() => invoke<boolean>("site_save_file")),
    loadFiles: () => stage(() => invoke<number | null>("site_load_files")),
    // La contraseña la pide el backend con su propio diálogo; `false` es
    // que se cerró sin elegir, y un rechazo llega tal cual a quien llama.
    installCertificate: () => invoke<boolean>("site_install_certificate"),
    lookAgain: () => invoke<void>("site_look_again"),
    allowSha1Once: () => invoke<void>("site_allow_sha1_once"),
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
    openDocument: async (id) => {
      try {
        return await loader.load(
          new Uint8Array(await invoke<ArrayBuffer>("read_document", { id })),
        );
      } catch {
        return null;
      }
    },
    // Un fallo al pedirlas no es una puerta, igual que en escritorio
    // (journey/usePreviousSignatures.ts): el aviso simplemente no se monta.
    previousSignatures: (document) =>
      invoke<PreviousSignaturesReport>("previous_signatures", { document }).catch(
        () => NO_PREVIOUS_SIGNATURES,
      ),
  });
}
