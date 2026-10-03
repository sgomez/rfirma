//! Dónde se ancla el menú de la aplicación, por plataforma: cabecera o barra de título GTK.

/**
 * Dónde se ancla el menú de la aplicación (ADR-0007): el ☰ de la cabecera en
 * Windows y macOS, y la barra de título GTK en Linux. Con `"titlebar"` la cabecera HTML se queda en la tira de pestañas.
 */
export type MenuAnchor = "header" | "titlebar";

/** El anclaje que le toca a la plataforma, leído del `userAgent` del WebView. */
export function menuAnchorFor(userAgent: string): MenuAnchor {
  if (/linux/i.test(userAgent) && !/android/i.test(userAgent)) return "titlebar";
  return "header";
}
