/**
 * Dónde se ancla el menú de dos entradas.
 *
 * El ADR-0007 define **una** acción, `Preferencias…` y `Acerca de rFirma`, con
 * dos anclajes según la plataforma: el botón ☰ de la cabecera en GNOME y en
 * Windows, y el menú de aplicación nativo (`tauri::menu`, con `Cmd+,`) en
 * macOS. En macOS el botón de la cabecera **se oculta**, no se deja vacío.
 */
export type MenuAnchor = "header" | "native";

/**
 * El anclaje que le toca a la plataforma en la que corre la ventana.
 *
 * Se decide con el `userAgent` del WebView y no con un complemento de Tauri
 * porque es una decisión de la capa de interfaz y no hay ninguna otra que
 * necesite saber el sistema operativo: el único sitio del backend con un
 * `cfg!` es `paths.rs` (ADR-0010), y esto no vive ahí.
 *
 * El hito v0.1 es solo Linux, así que hoy esto siempre contesta `"header"`.
 * Está escrito ahora porque el momento de saberlo es antes de escribir el
 * código de menús, no después.
 */
export function menuAnchorFor(userAgent: string): MenuAnchor {
  return /mac os x|macintosh/i.test(userAgent) ? "native" : "header";
}
