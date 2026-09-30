import { describe, expect, it, vi } from "vitest";
import { applyTheme, isTheme, noWindowTheme, THEMES } from "./theme";

/** **Grada A**: un atributo en un elemento, sin backend y sin ventana. */
describe("el tema", () => {
  it("forces the chosen one with the attribute the design tokens read", () => {
    const root = document.createElement("html");

    applyTheme("dark", noWindowTheme, root);

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
    applyTheme("dark", noWindowTheme, root);

    applyTheme("system", noWindowTheme, root);

    expect(root.hasAttribute("data-theme")).toBe(false);
  });

  it("recognises the three themes and nothing else", () => {
    expect(THEMES).toEqual(["system", "light", "dark"]);
    expect(isTheme("light")).toBe(true);
    expect(isTheme("sepia")).toBe(false);
  });
});

describe("el tema de la ventana", () => {
  it.each([
    ["dark", "dark"],
    ["light", "light"],
    ["system", null],
  ] as const)("sets the window theme when applying %s", (theme, expected) => {
    const setTheme = vi.fn();

    applyTheme(theme, setTheme, document.createElement("html"));

    expect(setTheme).toHaveBeenCalledWith(expected);
  });
});
