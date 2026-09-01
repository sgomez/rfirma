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
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents } from "./documents/recents";
import { createI18n } from "./i18n/i18n";
import { LanguageProvider } from "./i18n/LanguageProvider";
import { inMemoryLanguagePreference } from "./i18n/preference";
import { inMemoryPreferences } from "./preferences/preferences";
import { emptyRubricPicker } from "./signing/rubric";
import { tauriCertificateStore, tauriLayer2Composer, tauriSigningBackend } from "./signing/tauri";
import { emptyPdfSource } from "./viewer/source";

const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en index.html");
}

// Los tres puertos de firma ya están enchufados a las órdenes del #60:
// `tauriCertificateStore`, `tauriLayer2Composer` y `tauriSigningBackend`. Los
// que siguen en memoria son los que tocan el disco por sitios que todavía no
// tienen orden expuesta —los recientes, los ajustes, el portal de ficheros y la
// rúbrica—, más la del visor, `emptyPdfSource`: el PDF se pintará con
// `pdfjsSource` en cuanto haya por dónde pedirle los bytes al portal.
// Cuando la haya, se sustituyen aquí y en ningún otro sitio: ni la ventana ni
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
        picker={inMemoryDocumentPicker()}
        pdfs={emptyPdfSource()}
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
