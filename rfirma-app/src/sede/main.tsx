// El mismo orden que en `main.tsx`, y por la misma razón: el bundle del sistema
// de diseño va antes que cualquier componente, y los ajustes sobre él después.
import "../design-system/index.css";
import "../app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import { tauriLanguagePreference, tauriSiteErrands } from "../tauri";
import { SedeWindow } from "./SedeWindow";

/**
 * **El cableado de la ventana de sede** (ID-335): su propio montaje, separado
 * del de la ventana principal.
 *
 * Es un punto de entrada aparte —`sede.html`— y no una rama de `main.tsx`
 * porque lo que se quiere es justamente que **no cargue el árbol de la ventana
 * principal**: aquí no hay bandeja, ni visor, ni ajustes, ni aviso del primer
 * arranque. Esta ventana la crea `app::startup` sólo cuando hay trámite
 * (ID-334), así que arrancar rFirma a mano no ejecuta ni una línea de esto.
 *
 * El único puerto es `SiteErrandPort`, y aquí se cablea el **de verdad**
 * (`tauriSiteErrands`) en lugar del doble `noErrand`, que se queda donde
 * estaba: es lo que siguen usando las pruebas de la ventana (TD-63, TD-78).
 *
 * El idioma sale de la preferencia guardada, nunca del navegador (ID-02), y de
 * la misma configuración que lee la ventana principal.
 */
const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en sede.html");
}

const preference = tauriLanguagePreference();
const i18n = createI18n(await preference.read());

// Fuera del árbol: `SedeWindow` se resuscribe cuando el puerto cambia de
// identidad, y uno nuevo en cada pintada lo suscribiría en bucle.
const errands = tauriSiteErrands();

createRoot(root).render(
  <StrictMode>
    <LanguageProvider i18n={i18n} preference={preference}>
      <SedeWindow errands={errands} />
    </LanguageProvider>
  </StrictMode>,
);
