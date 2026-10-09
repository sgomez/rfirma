# Sin barra de menús: una sola barra con la identidad, los documentos y el menú

La primera versión de la interfaz dibujaba una barra de menús clásica
(`Archivo · Ver · Ayuda`) dentro de la ventana. Se retira.

Ninguno de los escritorios objetivo la usa hoy: GNOME la retiró de su guía de
estilo en favor de una cabecera con botón de menú, y Windows 11 no la emplea en
sus aplicaciones nuevas.

En Windows y macOS, `rfirma` tiene **una sola barra permanente**, de 44 px, que
es a la vez cabecera y tira de pestañas. De izquierda a derecha: la identidad (`rFirma`), el **botón
partido** de abrir, las **pestañas** de los documentos abiertos y, al extremo
derecho, el botón de menú.

- **El botón partido** reúne abrir y reabrir en un mismo control con dos
  gestos: el segmento principal, «Abrir PDF…», abre el diálogo del sistema
  directamente (Ctrl+O); la flecha despliega «Abiertos recientemente». Sin
  recientes —actividad apagada o lista vacía— la flecha desaparece y queda un
  botón simple.
- **Las pestañas que no caben** se recogen en un «+N ▾» al final, con un menú de
  las ocultas; la activa ocupa siempre el último hueco visible.

La barra no lleva el estado del documento ni el certificado: el documento
firmado lo marca su pestaña con ✓, y el certificado lo dice el selector del
panel de firma. Una insignia de estado se descartó por eso: repetía la pestaña y
confundía «Estado de rFirma» con el estado del documento.

Las vistas que no son de ningún documento —Preferencias, el panel de estado, el
primer arranque— ocupan el cuerpo de la ventana, y **la barra se queda solo con
`rFirma` y el menú**: sin botón de abrir y sin pestañas, porque las pestañas son
de documentos y esas vistas no son de ninguno.

## En Linux, la barra de título nativa de GTK

En todo Linux —sin detectar el escritorio— abrir y el ☰ van en la **barra de
título nativa de GTK**: el botón partido a la izquierda, **«rFirma» de título**
en el centro, y el ☰ y los botones de ventana del tema a la derecha. La barra de
44 px no existe; debajo, dentro de la ventana, queda **una tira con solo las
pestañas** y el «+N ▾», que desaparece cuando no hay ninguna pestaña. La tira no
es región de arrastre: es contenido, no barra de título.

**El aviso es un botón aparte**, a la izquierda del ☰, y no una marca en el
menú, porque los menús nativos no admiten marcas.

La interfaz decide el modo por plataforma —Linux, o Windows y macOS—, y conduce la
barra por un puerto: le manda si se ve abrir, si hay aviso y las etiquetas ya
traducidas, y recibe las acciones, que hacen lo mismo que los botones HTML. La
medición que lo sostiene es la nota de research «Abrir y el menú en la barra de
título nativa de GTK, en Linux» (`docs/research/barra-de-titulo-en-linux.md`).

**rFirma le quita algo a WebKitGTK según la sesión**, al arrancar, antes de
crear ningún webview y salvo que el entorno ya traiga alguna variable que elija
el renderizador; lo decide `desktop/adapters/webkit_renderer.rs` y ningún
manifiesto de empaquetado exporta variables de renderizado.

En una sesión X11 fija `WEBKIT_DISABLE_COMPOSITING_MODE=1`. Un popover de GTK3 en
X11 es una ventana hija recortada dentro de la principal, y con la composición
acelerada lo que queda alrededor del bocadillo no es la página, sino un recuadro
gris opaco. Se midió en Xfce sobre X11, con una Intel y con llvmpipe: el recuadro
sale con cualquier popover, con o sin el CSS propio, y desaparece solo sin
composición acelerada. En Wayland, en el mismo equipo, el popover es una
superficie aparte y no pasa.

En una sesión Wayland fija `WEBKIT_DISABLE_DMABUF_RENDERER=1`. El renderizador
DMA-BUF de WebKitGTK pide a GTK3 un contexto GL sobre la ventana; con el
`egl-wayland` de NVIDIA eso registra sincronización explícita en la superficie,
GTK3 commitea después un buffer de memoria compartida y Mutter corta la conexión
(«Explicit Sync only supported on dmabuf buffers», Error 71). Se midió en el
flatpak, con `org.gnome.Platform` 50 y 51 sobre una RTX 4060 Ti, con la interfaz
entera y con la composición acelerada encendida; en el anfitrión, con la
WebKitGTK del sistema, no fallaba. Es lo mismo que hace Tabularis desde febrero
de 2026. Sin medir en Intel ni AMD.

## Los menús retirados

Los menús que se han eliminado no se han movido a otro sitio: **no hacían
falta**. Abrir un documento ya tiene el botón partido y la zona de soltar;
guardar tiene la caja «Guardar en» del pie del panel de firma; y paginación y
zoom viven en la barra flotante del visor, que es exactamente lo que un menú
*Ver* habría contenido.

**Criterio de pertenencia al menú:** el menú contiene lo que habla **de la
aplicación**; nunca lo que habla **del documento**. Abrir, guardar, paginar y
ampliar son del documento y ya tienen su sitio en el recorrido; estado, ajustes,
ayuda e identidad son de la aplicación y no tienen otro. Un tope numérico
inventado se incumple el día que estorba, y el criterio ya excluye lo que el
tope querría excluir. Cuando el grupo de aplicación crezca de verdad, la
respuesta no es una barra de menús sino una página más dentro de Preferencias o
de Estado.

## Considered Options

- **Cabecera de 52 px y, debajo, una tira de pestañas de 40 px.** Fue el diseño
  hasta el 27/09/2026. Dos franjas para una sola cosa —la parte de arriba de la
  ventana— se comían 92 px de alto; con la barra única son 44, y esos 48 px
  vuelven al visor, que es la región principal.
- **Un «+» al final de la tira que desplegaba abrir y los recientes juntos.**
  Abrir costaba dos clics —desplegar y elegir «Abrir un PDF…»— cuando es el
  gesto más frecuente, y no tenía atajo propio. El botón partido deja abrir a un
  clic o a Ctrl+O y aparta reabrir a la flecha, dentro del mismo control.
- **Flechas ‹ › para desplazar la tira cuando las pestañas no caben.**
  Desplazar obliga a buscar pasando de una en una; el menú «+N» enseña todas las
  ocultas de golpe y la activa no se pierde nunca.

- **En Linux, la ventana sin decorar y la barra HTML como barra de título.**
  Arrastrar y redimensionar funcionan, pero se pierden la sombra y las esquinas
  del tema, el redimensionado cambia de cursor solo a ciegas, el doble clic no
  sigue el ajuste del escritorio y los botones de ventana dibujados en HTML
  nunca serían los del tema.
- **La barra de título GTK solo en GNOME, detectando el escritorio.** En KDE y
  los demás la barra de GTK funciona y no desentona más que cualquier
  aplicación GTK; detectar el escritorio añadía un tercer modo de cabecera a
  cambio de nada.
- **Los menús de la barra como `gtk::Menu` en X11.** Un menú abre su propia
  ventana emergente y no tiene recuadro, pero pierde el bocadillo y la flecha,
  su posición y su aspecto los decide el tema, y en Xubuntu desentonaba con la
  barra. Dejaba además dos implementaciones de los mismos menús.
- **`WEBKIT_DISABLE_DMABUF_RENDERER=1` también en X11.** Quita el recuadro, y
  es el remedio que Tauri documenta para otros fallos de pintado en Linux
  ([tauri#9394](https://github.com/tauri-apps/tauri/issues/9394)), pero en la
  WebKitGTK 2.52 del anfitrión deja vacíos los modos de transporte y el webview
  se cae al entrar en composición acelerada
  ([block/buzz#3654](https://github.com/block/buzz/issues/3654)): quita la
  composición de rebote y con un cuelgue latente. En el flatpak ese cuelgue no
  se reproduce.
- **`WEBKIT_DMABUF_RENDERER_FORCE_SHM=1` en X11.** Mantiene la composición con
  búferes en memoria compartida, y el recuadro sigue.
- **La política de aceleración `Never` en el webview ya creado.** Es API y no
  variable, pero llega cuando la página ya ha empezado a componer, y en Xfce
  sobre una Intel la ventana se quedó en negro.
- **Apagar la composición también en Wayland.** Era el paliativo del #22 en el
  manifiesto flatpak, y desde la WebKitGTK que trae el runtime 50 en octubre de
  2026 ya no evita el Error 71; apagar el renderizador DMA-BUF sí, y conserva la
  composición.
- **Decidir el renderizador en el manifiesto flatpak.** Dos sitios decidiendo lo
  mismo, y el manifiesto anulaba a este código sin que ninguno lo dijera.

## Consequences

- Sin barra de menús se pierden los aceleradores de teclado visibles. No hay
  todavía una lista de atajos decidida; cuando la haya, hará falta un sitio
  donde consultarlos. Ctrl+O es el primero, y lo anuncia el `title` de «Abrir
  PDF…».
- Las pestañas comparten fila con la identidad, el botón partido y el menú, así
  que caben menos que en una tira propia: el desborde a «+N» llega antes.
- En X11 el visor pinta sin composición acelerada, por software: al cambiar de
  página de un PDF pesado se ve el repintado, por debajo de medio segundo en Xfce
  con una Intel.
