import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./design-system/index.css";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents } from "./documents/recents";
import { createI18n } from "./i18n/i18n";
import { LanguageProvider } from "./i18n/LanguageProvider";
import { inMemoryLanguagePreference } from "./i18n/preference";
import { inMemoryPreferences } from "./preferences/preferences";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";
import { emptyLayer2Composer } from "./signing/visibleSignature";
import { emptyPdfSource } from "./viewer/source";

const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en index.html");
}

// Las cinco dependencias que tocan el disco viven todavía en memoria: quien
// las guarda es el backend —`memory::Configuration`, `memory::State`, el portal
// de ficheros— y no hay ninguna orden expuesta que las lea ni las escriba. La
// del visor es `emptyPdfSource`: el PDF se pintará con `pdfjsSource` en cuanto
// haya por dónde pedirle los bytes al portal.
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
        certificates={emptyCertificateStore()}
        rubrics={emptyRubricPicker()}
        composer={emptyLayer2Composer()}
        signer={unavailableSigningBackend()}
      />
    </LanguageProvider>
  </StrictMode>,
);
