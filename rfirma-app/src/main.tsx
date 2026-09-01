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
import { inMemoryRecents } from "./documents/recents";
import { createI18n } from "./i18n/i18n";
import { LanguageProvider } from "./i18n/LanguageProvider";
import { inMemoryLanguagePreference } from "./i18n/preference";
import { inMemoryPreferences } from "./preferences/preferences";
import { emptyRubricPicker } from "./signing/rubric";
import {
  tauriCertificateStore,
  tauriDocumentDrops,
  tauriDocumentPicker,
  tauriLayer2Composer,
  tauriPdfSource,
  tauriSigningBackend,
} from "./tauri";

const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en index.html");
}

// Seis puertos hablan ya con el backend: los tres de firma del #60
// —`tauriCertificateStore`, `tauriLayer2Composer` y `tauriSigningBackend`—, los
// dos del documento del #82, `tauriDocumentPicker` y `tauriPdfSource`, y el del
// arrastre del #83, `tauriDocumentDrops`, que es el único que escucha un evento
// de la ventana en vez de llamar a una orden. Los
// que siguen en memoria son los que tocan el disco por sitios que todavía no
// tienen orden expuesta: los recientes, los ajustes y la rúbrica. Cuando la
// tengan, se sustituyen aquí y en ningún otro sitio (ID-75): ni la ventana ni
// sus pruebas conocen a Tauri.
//
// El idioma sale de la preferencia guardada, nunca del navegador (ID-02).
const preference = inMemoryLanguagePreference();
const i18n = createI18n(await preference.read());

const recents = inMemoryRecents();

// El nombre de la carpeta de documentos del usuario lo resuelve el backend
// (`paths::documents_folder`). Bajo el arenero el destino es esa carpeta y solo
// esa, así que la lista tiene una entrada (ADR-0011).
const DOCUMENTS_FOLDER = "Documentos";

const preferences = inMemoryPreferences(
  {
    destination: DOCUMENTS_FOLDER,
    rememberVisibleSignature: true,
    rememberActivity: true,
  },
  () => {
    void recents.clear();
  },
);

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
