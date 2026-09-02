// El bundle del sistema de diseño va **antes** que cualquier componente: los
// `import` de ES se evalúan en orden, y el CSS de cada pantalla baja a propósito
// medidas de las clases `rf-*` (`.viewer__step` sobre `.rf-btn`, por ejemplo).
// Con el mismo peso de selector gana el último que se emite, así que emitir el
// bundle después anularía en silencio media transcripción.
import "./design-system/index.css";
// Y justo después, lo que el bundle no trae y toda la pantalla necesita: el
// modelo de caja, el margen del documento y la colocación del velo. Va detrás
// del bundle porque son ajustes sobre él (ver `app.css`).
import "./app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { createI18n } from "./i18n/i18n";
import { LanguageProvider } from "./i18n/LanguageProvider";
import { emptyRubricPicker } from "./signing/rubric";
import {
  tauriCertificateStore,
  tauriDocumentDrops,
  tauriDocumentPicker,
  tauriLanguagePreference,
  tauriLayer2Composer,
  tauriPdfSource,
  tauriPreferences,
  tauriRecents,
  tauriSigningBackend,
} from "./tauri";

const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en index.html");
}

// Nueve puertos hablan ya con el backend: los tres de firma del #60
// —`tauriCertificateStore`, `tauriLayer2Composer` y `tauriSigningBackend`—, los
// dos del documento del #82, `tauriDocumentPicker` y `tauriPdfSource`, el del
// arrastre del #83, `tauriDocumentDrops`, que es el único que escucha un evento
// de la ventana en vez de llamar a una orden, y los dos de la configuración,
// `tauriPreferences` y `tauriLanguagePreference`, que debajo son el mismo
// fichero, y el de la bandeja del #126, `tauriRecents`, que es el que la hace
// sobrevivir al reinicio (ID-75). El único que sigue en memoria es la rúbrica,
// que toca el disco por un sitio que todavía no tiene orden expuesta; cuando la
// tenga, se sustituye aquí y en ningún otro sitio: ni la ventana ni sus pruebas
// conocen a Tauri.
//
// El idioma sale de la preferencia guardada, nunca del navegador (ID-02).
const preference = tauriLanguagePreference();
const i18n = createI18n(await preference.read());

const recents = tauriRecents();

const preferences = tauriPreferences();

// El nombre de la carpeta de documentos del usuario lo resuelve el backend
// (`paths::documents_folder`) y llega con los ajustes; esta lista es lo que el
// desplegable ofrece mientras se leen. Bajo el arenero el destino es esa
// carpeta y solo esa, así que tiene una entrada (ADR-0011).
const DOCUMENTS_FOLDER = "Documentos";

createRoot(root).render(
  <StrictMode>
    <LanguageProvider i18n={i18n} preference={preference}>
      <App
        recents={recents}
        picker={tauriDocumentPicker()}
        drops={tauriDocumentDrops()}
        pdfs={tauriPdfSource()}
        preferences={preferences}
        destinations={[DOCUMENTS_FOLDER]}
        certificates={tauriCertificateStore()}
        rubrics={emptyRubricPicker()}
        composer={tauriLayer2Composer()}
        signer={tauriSigningBackend()}
      />
    </LanguageProvider>
  </StrictMode>,
);
