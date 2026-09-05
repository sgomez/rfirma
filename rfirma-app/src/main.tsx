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
import {
  tauriCertificateStore,
  tauriDestinations,
  tauriDocumentDrops,
  tauriDocumentPicker,
  tauriLanguagePreference,
  tauriPdfSource,
  tauriPreferences,
  tauriRecents,
  tauriRubricPicker,
  tauriSignedDocumentOpener,
  tauriSigningBackend,
  tauriStampComposer,
  tauriVersionCheck,
} from "./tauri";
import { TrustNotice } from "./trust/TrustNotice";

const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en index.html");
}

// Once puertos hablan ya con el backend: los dos de firma del #60 que quedan
// —`tauriCertificateStore` y `tauriSigningBackend`—, los
// dos del documento del #82, `tauriDocumentPicker` y `tauriPdfSource`, el del
// arrastre del #83, `tauriDocumentDrops`, que es el único que escucha un evento
// de la ventana en vez de llamar a una orden, los dos de la configuración,
// `tauriPreferences` y `tauriLanguagePreference`, que debajo son el mismo
// fichero, el de la bandeja del #126, `tauriRecents`, que es el que la hace
// sobrevivir al reinicio (ID-75), el de la rúbrica del #128, `tauriRubricPicker`,
// el del destino del #130, `tauriDestinations`, que es quien sabe con qué
// nombre y en qué carpeta va a caer lo firmado (ID-63), y el del resumen del
// #131, `tauriSignedDocumentOpener`, que bajo el sandbox es lo único que lleva
// al usuario hasta el fichero que acaba de firmar (ID-79).
// y el de la versión del #271, `tauriVersionCheck`, que es la única conexión
// saliente de la aplicación y sólo sirve para poner una franja bajo la cabecera
// (ID-181).
// La sustitución ocurre solo en este fichero: ni la ventana ni sus pruebas
// conocen a Tauri.
//
// `TrustNotice` (#365) no es un puerto: no habla con Tauri, así que no hay
// nada que doblar ni que cablear aquí más que montarlo. Lo que sí cruza a
// Tauri es si ya se descartó: `trustNoticeSeen` viaja en la misma
// configuración que el resto de ajustes, y se lee aquí, antes de pintar,
// para que el aviso no llegue a montarse en el segundo arranque en adelante.
//
// El idioma sale de la preferencia guardada, nunca del navegador (ID-02).
const preference = tauriLanguagePreference();
const i18n = createI18n(await preference.read());

const recents = tauriRecents();

const preferences = tauriPreferences();
const initialPreferences = await preferences.read();

createRoot(root).render(
  <StrictMode>
    <LanguageProvider i18n={i18n} preference={preference}>
      {/* Sin condición salvo el descarte ya persistido: se explica antes de
          que el navegador pregunte, no como reacción a un fallo (ID-231,
          #365), y solo en el primer arranque. */}
      <TrustNotice
        seen={initialPreferences.trustNoticeSeen}
        onAcknowledge={() => {
          void preferences.save({ ...initialPreferences, trustNoticeSeen: true });
        }}
      />
      <App
        recents={recents}
        picker={tauriDocumentPicker()}
        drops={tauriDocumentDrops()}
        pdfs={tauriPdfSource()}
        preferences={preferences}
        destinations={tauriDestinations()}
        certificates={tauriCertificateStore()}
        rubrics={tauriRubricPicker()}
        stamps={tauriStampComposer()}
        signer={tauriSigningBackend()}
        opener={tauriSignedDocumentOpener()}
        versions={tauriVersionCheck()}
      />
    </LanguageProvider>
  </StrictMode>,
);
