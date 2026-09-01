/**
 * El tema de la ventana: lo que el usuario elige ver.
 *
 * `system` **no es «claro»**: es no forzar nada y dejar que mande
 * `prefers-color-scheme`, que es lo que hacía la ventana antes de que el ajuste
 * existiera. Por eso son tres valores y no un interruptor: un booleano no
 * puede decir «lo que diga el sistema».
 *
 * Las etiquetas son las de `memory::Theme` del backend, que es como se
 * persiste. Si cambia una, cambia en los dos sitios.
 */
export const THEMES = ["system", "light", "dark"] as const;

export type Theme = (typeof THEMES)[number];

/** El tema mientras no hay nada elegido. */
export const DEFAULT_THEME: Theme = "system";

/** Si `value` es uno de los tres temas. */
export function isTheme(value: string): value is Theme {
  return (THEMES as readonly string[]).includes(value);
}

/**
 * Pone el tema en el documento.
 *
 * Es un atributo en `<html>` y no una clase porque así lo declaran los tokens
 * del bundle (`tokens/color.css`): `[data-theme="light|dark"]` redefine los
 * roles de color sobre cualquier elemento. `system` **quita** el atributo en
 * vez de escribir uno tercero: la media query del bundle es
 * `:root:not([data-theme="light"])`, así que lo que devuelve el mando al
 * sistema operativo es la ausencia del atributo, no un valor más.
 */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  if (theme === "system") {
    root.removeAttribute("data-theme");
    return;
  }
  root.setAttribute("data-theme", theme);
}
