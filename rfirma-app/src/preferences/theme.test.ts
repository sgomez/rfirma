import { describe, expect, it } from "vitest";
import { applyTheme, isTheme, THEMES } from "./theme";

/** **Grada A**: un atributo en un elemento, sin backend y sin ventana. */
describe("el tema", () => {
  it("forces the chosen one with the attribute the design tokens read", () => {
    const root = document.createElement("html");

    applyTheme("dark", root);

    expect(root.getAttribute("data-theme")).toBe("dark");
  });

  /**
   * `system` **quita** el atributo en vez de escribir un tercer valor: la
   * media query del bundle es `:root:not([data-theme="light"])`, así que lo
   * que devuelve el mando al escritorio es la ausencia del atributo. Escribir
   * `data-theme="system"` dejaría la ventana clavada en claro dentro de un
   * escritorio oscuro.
   */
  it("gives the choice back to the desktop by removing the attribute", () => {
    const root = document.createElement("html");
    applyTheme("dark", root);

    applyTheme("system", root);

    expect(root.hasAttribute("data-theme")).toBe(false);
  });

  it("recognises the three themes and nothing else", () => {
    expect(THEMES).toEqual(["system", "light", "dark"]);
    expect(isTheme("light")).toBe(true);
    expect(isTheme("sepia")).toBe(false);
  });
});
