import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import { createI18n } from "./i18n/i18n";
import { LanguageProvider } from "./i18n/LanguageProvider";
import { inMemoryLanguagePreference } from "./i18n/preference";

// Grada A (no necesita nada, carril rápido). Su papel no es probar la interfaz
// —todavía no hay— sino que la cadena de TypeScript esté enchufada de verdad:
// que vitest resuelva JSX, jsdom y testing-library. Sin esto, `just test`
// pasaría en verde con la cadena rota.
describe("App", () => {
  it("renders the empty window", () => {
    const preference = inMemoryLanguagePreference();
    render(
      <LanguageProvider i18n={createI18n()} preference={preference}>
        <App />
      </LanguageProvider>,
    );

    expect(screen.getByRole("heading", { name: "rfirma" })).toBeInTheDocument();
  });
});
