//! El cableado de la ventana principal (`index.html`): quién implementa cada puerto.

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
import { StrictMode, useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { App, type AppHandle } from "./App";
import { RenderErrorBoundary } from "./errors/RenderErrorBoundary";
import { createI18n } from "./i18n/i18n";
import { LanguageProvider } from "./i18n/LanguageProvider";
import { SetupWizard } from "./setup/SetupWizard";
import { menuAnchorFor } from "./shell/menuAnchor";
import { absentNativeTitlebar } from "./shell/nativeTitlebar";
import { visibleSignatureFrom } from "./signing/visibleSignature";
import {
  tauriAppVersion,
  tauriCertificateStore,
  tauriDestinations,
  tauriDocumentDrops,
  tauriDocumentPicker,
  tauriExternalDestinationOpener,
  tauriLanguagePreference,
  tauriNativeTitlebar,
  tauriPdfSource,
  tauriPreferences,
  tauriRecents,
  tauriRubricPicker,
  tauriSignedDocumentOpener,
  tauriSigningBackend,
  tauriStampComposer,
  tauriStatusPort,
  tauriVersionCheck,
  tauriVisibleSignatureMemory,
  tauriWindowTheme,
} from "./tauri";

const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en index.html");
}

// `SetupWizard` usa los mismos casos de uso que el panel de estado
// (`tauriStatusPort`); `setupWizardSeen` viaja con el resto de ajustes y se lee
// antes de pintar para que el asistente no llegue a montarse desde el segundo
// arranque. `RootView` guarda ese estado para que «Terminar» quite el asistente
// sin recargar la ventana.
//
// `SedeWindow` no se monta aquí: tiene su propio punto de entrada,
// `sede/main.tsx`, y la ventana la crea `app::startup` solo cuando hay trámite.
//
// El idioma sale de la preferencia guardada, nunca del navegador.
const preference = tauriLanguagePreference();
const i18n = createI18n(await preference.read());

const recents = tauriRecents();

const preferences = tauriPreferences();
const windowTheme = tauriWindowTheme();
const initialPreferences = await preferences.read();
const initialSignature = visibleSignatureFrom(await tauriVisibleSignatureMemory().read());
const appVersion = await tauriAppVersion();
const statusPort = tauriStatusPort();
// Fuera del árbol, como `errands` en `sede/main.tsx`: lo usa también el
// `RenderErrorBoundary` que envuelve a `RootView`, y crear uno nuevo en cada
// pintada de `RootView` lo habría dejado sin compartir.
const externalDestinations = tauriExternalDestinationOpener();
const titlebar =
  menuAnchorFor(navigator.userAgent) === "titlebar"
    ? tauriNativeTitlebar()
    : absentNativeTitlebar();

function RootView() {
  const [setupWizardSeen, setSetupWizardSeen] = useState(initialPreferences.setupWizardSeen);
  const appHandle = useRef<AppHandle | null>(null);

  const finishWizard = useCallback(() => {
    void preferences
      .read()
      .then((current) => preferences.save({ ...current, setupWizardSeen: true }));
    setSetupWizardSeen(true);
  }, []);

  useEffect(() => {
    if (setupWizardSeen) return;
    return titlebar.onAction((action) => {
      if (action.action === "status" || action.action === "preferences") finishWizard();
    });
  }, [setupWizardSeen, finishWizard]);

  return (
    <>
      <SetupWizard
        seen={setupWizardSeen}
        preferences={preferences}
        statusPort={statusPort}
        onFinish={finishWizard}
        onOpenStatus={() => {
          finishWizard();
          appHandle.current?.openStatus();
        }}
        onOpenPreferences={() => {
          finishWizard();
          appHandle.current?.openPreferences();
        }}
        onOpenHelp={() => void externalDestinations.open("discussions")}
        onOpenAbout={() => appHandle.current?.openAbout()}
      />
      <App
        recents={recents}
        picker={tauriDocumentPicker()}
        drops={tauriDocumentDrops()}
        pdfs={tauriPdfSource()}
        preferences={preferences}
        windowTheme={windowTheme}
        destinations={tauriDestinations()}
        certificates={tauriCertificateStore()}
        rubrics={tauriRubricPicker()}
        stamps={tauriStampComposer()}
        signer={tauriSigningBackend()}
        opener={tauriSignedDocumentOpener()}
        initialSignature={initialSignature}
        versions={tauriVersionCheck()}
        version={appVersion}
        externalDestinations={externalDestinations}
        status={statusPort}
        covered={!setupWizardSeen}
        titlebar={titlebar}
        onReady={(handle) => {
          appHandle.current = handle;
        }}
      />
    </>
  );
}

createRoot(root).render(
  <StrictMode>
    <LanguageProvider i18n={i18n} preference={preference}>
      <RenderErrorBoundary externalDestinations={externalDestinations}>
        <RootView />
      </RenderErrorBoundary>
    </LanguageProvider>
  </StrictMode>,
);
