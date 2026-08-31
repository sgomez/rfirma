import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./design-system/index.css";
import { createI18n } from "./i18n/i18n";
import { LanguageProvider } from "./i18n/LanguageProvider";
import { inMemoryLanguagePreference } from "./i18n/preference";

const root = document.getElementById("root");
if (!root) {
  throw new Error("no existe #root en index.html");
}

// El idioma sale de la preferencia guardada, nunca del navegador (ID-02). La
// implementación que habla con `memory::Configuration` llega con el diálogo de
// Preferencias; hasta entonces la preferencia vive en memoria y se cambia el
// mismo día en este único sitio.
const preference = inMemoryLanguagePreference();
const i18n = createI18n(await preference.read());

createRoot(root).render(
  <StrictMode>
    <LanguageProvider i18n={i18n} preference={preference}>
      <App />
    </LanguageProvider>
  </StrictMode>,
);
