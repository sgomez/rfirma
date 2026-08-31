# Sin barra de menús: una cabecera única, con anclaje distinto por plataforma

La primera versión de la interfaz dibujaba una barra de menús clásica
(`Archivo · Ver · Ayuda`) dentro de la ventana. Se retira.

Ninguno de los tres escritorios objetivo la usa hoy: GNOME la retiró de su guía
de estilo en favor de una cabecera con botón de menú, Windows 11 no la emplea en
sus aplicaciones nuevas, y en macOS el menú de aplicación vive en la barra del
sistema —que es donde Tauri lo registra—, de modo que dibujarlo además dentro de
la ventana lo duplicaría.

`rfirma` tiene **una sola cabecera**: identidad a la izquierda, estado del
documento y un botón de menú a la derecha, con dos entradas, `Preferencias…` y
`Acerca de rFirma`.

Los menús que se han eliminado no se han movido a otro sitio: **no hacían
falta**. Abrir un documento ya tiene la zona de soltar de la bandeja; guardar
tiene la fila «Se guardará en» del panel de firma; y paginación y zoom viven en
la barra flotante del visor, que es exactamente lo que un menú *Ver* habría
contenido.

En **macOS**, `Preferencias` y `Acerca de` deben registrarse en el menú de
aplicación nativo (`tauri::menu`), con `Cmd+,` para preferencias, y no en el
botón de la cabecera. Misma acción, dos anclajes según la plataforma.

## Consequences

- La capa de interfaz necesita saber en qué plataforma corre para decidir dónde
  se ancla cada acción. Las acciones se definen una vez; el anclaje es una
  decisión de plataforma.
- En macOS el botón de menú de la cabecera se queda sin contenido propio. Habrá
  que ocultarlo, no dejarlo vacío.
- Sin barra de menús se pierden los aceleradores de teclado visibles. No hay
  todavía una lista de atajos decidida; cuando la haya, hará falta un sitio
  donde consultarlos, y ese sitio no puede ser el menú que acabamos de reducir
  a dos entradas.
- El hito v0.1 es solo Linux, así que la parte de macOS no bloquea nada. Se
  registra ahora porque el momento de saberlo es antes de escribir el código de
  menús, no después.
