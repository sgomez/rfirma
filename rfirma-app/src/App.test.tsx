import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

// Grada A (no necesita nada, carril rápido). Su papel no es probar la interfaz
// —todavía no hay— sino que la cadena de TypeScript esté enchufada de verdad:
// que vitest resuelva JSX, jsdom y testing-library. Sin esto, `just test`
// pasaría en verde con la cadena rota.
describe("App", () => {
  it("renderiza la ventana vacía", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "rfirma" })).toBeInTheDocument();
  });
});
