/**
 * Dónde se ancla el menú de la aplicación (ADR-0007): el ☰ de la cabecera en
 * Windows, el menú de aplicación nativo en macOS y la barra de título GTK en
 * Linux. Con `"titlebar"` la cabecera HTML se queda en la tira de pestañas.
 */
export type MenuAnchor = "header" | "native" | "titlebar";

/** El anclaje que le toca a la plataforma, leído del `userAgent` del WebView. */
export function menuAnchorFor(userAgent: string): MenuAnchor {
  if (/mac os x|macintosh/i.test(userAgent)) return "native";
  if (/linux/i.test(userAgent) && !/android/i.test(userAgent)) return "titlebar";
  return "header";
}
