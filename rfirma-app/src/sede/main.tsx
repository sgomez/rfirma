// El mismo orden que en `main.tsx`, y por la misma razón: el bundle del sistema
// de diseño va antes que cualquier componente, y los ajustes sobre él después.
import "../design-system/index.css";
import "../app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import { applyTheme } from "../preferences/theme";
import { tauriLanguagePreference, tauriPreferences, tauriSiteErrands } from "../tauri";
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
 *
 * El **tema** sale de esa misma configuración, y hay que aplicarlo aquí a mano:
 * los tokens de color cuelgan de `<html>`, así que quien los pone tiene que
 * salir del árbol de React, y quien lo hacía —`App`— es de la ventana
 * principal y aquí no se monta. Sin esto la ventana de sede ignora el ajuste y
 * se queda con lo que diga `prefers-color-scheme`, que es otra cosa: `system`
 * **no es** el valor elegido, es la ausencia de elección.
 */
const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en sede.html");
}

const preference = tauriLanguagePreference();
// Las dos lecturas van a la vez: son la misma configuración en el disco y
// encadenarlas sólo retrasaría el primer pintado.
const [language, settings] = await Promise.all([preference.read(), tauriPreferences().read()]);

// Antes de montar nada: aplicarlo después dejaría ver un parpadeo del tema que
// no es.
applyTheme(settings.theme);

const i18n = createI18n(language);

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
