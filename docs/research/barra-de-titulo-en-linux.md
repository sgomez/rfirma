# Abrir y el menú en la barra de título nativa de GTK, en Linux: medición

Qué hace falta para que, **en Linux**, la barra de título **nativa** de GTK (la
`GtkHeaderBar`) lleve los dos controles de la aplicación: el botón partido
«Abrir PDF…», con su flecha de recientes, a la izquierda, y el ☰ a la derecha,
junto a los botones de ventana. El título «rFirma» se queda en la headerbar, y
por eso sale de la barra de la interfaz. Las **pestañas** siguen dentro de la
ventana, en una tira HTML con las pestañas y el «+N ▾». En Windows y macOS no
cambia nada: sigue la barra única del ADR-0007.

Es **todo Linux, no solo GNOME**: la headerbar de GTK en KDE se ve como en
GNOME, igual que cualquier aplicación GTK con headerbar propia, y no hace falta
detectar el escritorio.

Se mide contra el código de las crates, contra las fuentes de GTK 3 y contra un
PoC ejecutado a mano, no contra la documentación de lo que debería pasar.

Entorno: Ubuntu 26.04, GNOME Shell 50.1 sobre Wayland, GTK 3 (`gtk` 0.18.2).
PoC sobre **Tauri 2.12.0, tao 0.37.1, wry 0.57.0**; el repositorio fija hoy
Tauri 2.11.5, tao 0.35.3, wry 0.55.1. Además, una máquina virtual con KDE
Plasma. En el equipo GNOME, `button-layout` es `':minimize,maximize,close'`
(el que impone Ubuntu) y el portal dice `color-scheme = 1` (preferencia oscura).

## Veredicto

**Viable con la API que Tauri ya expone, y medido de punta a punta sobre Tauri
2.12.** `WebviewWindow::gtk_window()` da la ventana GTK; se crea una
`HeaderBar` con el título, el botón partido y el ☰, y se pone con
`set_titlebar` antes de mostrar la ventana. Medido a mano en GNOME Wayland, en
GNOME con `GDK_BACKEND=x11` y en KDE:

* La ventana se mueve arrastrando la headerbar.
* Los botones responden y sus clics llegan a la interfaz.
* Los desplegables son los menús estándar de GTK (`GMenu` en un
  `GtkMenuButton`): tienen el estilo del tema y se cierran al pulsar fuera.
* Sin documento, la headerbar se queda con el título, el ☰ y los botones de
  ventana, como pide el ADR-0007.

**Hay que subir antes a Tauri 2.12.** Con 2.11 / tao 0.35, en Wayland la barra
la pone tao y se come los clics de lo que se le añade (sección 2). Con 2.12 esa
barra no existe y el camino es el mismo en Wayland y en X11.

El enganche con React son dos cables: del botón GTK a la interfaz, un **evento
de Tauri**; de la interfaz a GTK, un **comando** con los recientes, la
visibilidad según la vista y las etiquetas ya traducidas. El catálogo sigue
siendo uno, el de i18next.

Lo que cuesta:

1. **Dos modos de cabecera en la interfaz**: en Linux, la tira de pestañas sola;
   en Windows y macOS, la barra de 44 px de hoy.
2. **Más alto**: headerbar más tira de pestañas ocupan más que la barra única.
3. **Dos temas que alinear**: el de GTK en la headerbar y los tokens del diseño
   en el cuerpo (sección 5).

La barra HTML haciendo de barra de título, con la ventana sin decorar, **está
descartada**: unos botones de ventana dibujados en HTML no son los del
escritorio. Está en «Opciones descartadas».

## 1. Dónde se engancha

**El punto de enganche** es `WebviewWindow::gtk_window()`, que devuelve la
`gtk::ApplicationWindow`. La ventana principal ya se crea desde código
(`rfirma-app/src-tauri/src/lib.rs`, `open_the_main_window`), así que el
enganche va justo después de `build()`.

**La headerbar se crea y se pone antes de mostrar la ventana.** GTK avisa y
desrealiza la ventana si se llama a `set_titlebar` sobre una ventana ya
realizada (`gtk/gtkwindow.c` de gtk-3-24, L4245). El PoC construye la ventana
con `.visible(false)`, pone la headerbar y la muestra; su registro confirma
que en ese momento la ventana **no está realizada**, en Wayland y en X11
(medido).

**Con Tauri 2.12 no hay barra que reutilizar.** tao 0.37 solo pone una barra
propia —un `EventBox` vacío— cuando la ventana va **sin** decorar en Wayland
(`tao-0.37.1/src/platform_impl/linux/window.rs:192-197`). Una ventana decorada
se queda con la barra por defecto de GTK, que es interna:
`gtk_window_get_titlebar` la oculta y devuelve `NULL` (`gtkwindow.c`
L4294-4305). El PoC lo confirma: `titlebar()` es `None` en los dos backends.

**El orden de los botones de ventana es el del escritorio**:
`set_show_close_button(true)` y sin `set_decoration_layout`, la headerbar
sigue a `gtk-decoration-layout`. En KDE con `GDK_BACKEND=x11` aparece el icono
de la aplicación en la esquina: GTK pinta el icono cuando la disposición que
le llega incluye `icon`. Que KDE entregue ese valor a las aplicaciones X11 y no
a las de Wayland está **sin medir**. No se corrige: forzar una disposición
sería ignorar la configuración de quien usa el escritorio.

## 2. Por qué antes hay que subir a Tauri 2.12

**Con tao 0.35.3, en Wayland la barra de título no es de GNOME ni de GTK: es de
tao.** Cada ventana recibe una `GtkHeaderBar` metida en un `GtkEventBox`
(`tao-0.35.3/src/platform_impl/linux/wayland/header.rs:6-21`, llamada desde
`window.rs:91`), con dos rarezas: fija
`decoration_layout("menu:minimize,maximize,close")` (`:9`), ignorando el orden
de botones del escritorio, y pone `event_box.set_above_child(true)` (`:14`).

Medido con la primera versión del PoC, sobre 2.11: con botones GTK añadidos a
esa headerbar, **ni esos botones ni el de maximizar de tao responden** al
clic. La causa es el `above_child=true`: la ventana de entrada del `EventBox`
queda por encima de los botones. Con `set_above_child(false)` responden. Es el
fallo de [tao#1046](https://github.com/tauri-apps/tao/issues/1046) y
[tauri#13440](https://github.com/tauri-apps/tauri/issues/13440).

**Tauri 2.12.0 (2026-09-26)** pide `tao ^0.37`; tao 0.36.0 retiró esa headerbar
([CHANGELOG, 0.36.0](https://github.com/tauri-apps/tao/blob/dev/CHANGELOG.md),
[tao#1218](https://github.com/tauri-apps/tao/pull/1218)), y en tao 0.37.1 ya
no existe `wayland/header.rs`. Con 2.12 no hay parche de `above_child` que
mantener.

**La subida cambia por sí sola la ventana de hoy en Wayland**: la barra de tao,
con «rFirma» y su orden de botones fijo, pasa a ser la barra por defecto de
GTK, con el orden del escritorio. En KDE Wayland pasa a la decoración de KWin
(CHANGELOG de tao 0.36). Es un salto de versión menor, pero no invisible.

## 3. Del GTK a React y de React a GTK

**Los controles son acciones y menús de GIO.** El PoC registra un
`gio::SimpleActionGroup` con prefijo `hdr` en la ventana (`open`, `recent` con
parámetro de cadena, `preferences`, `about`). El segmento principal lleva
`set_action_name("hdr.open")`, y la flecha y el ☰ son `GtkMenuButton` con
`set_menu_model` de un `gio::Menu`. GTK 3 dibuja un `GtkPopoverMenu` a partir
del modelo: estilo del tema, cierre al pulsar fuera y navegación por teclado.
La primera versión del PoC montaba el `Popover` a mano, con una caja y
`ModelButton` sueltos: salía sin estilo y no se cerraba al pulsar fuera
(medido).

**Clic → interfaz: un evento de Tauri.** Cada acción hace
`app.emit("header", { action, path })` (`tauri::Emitter`). La interfaz escucha
con `listen("header", …)` y llama a lo mismo que hoy llaman los botones HTML.
El permiso de escuchar ya va en `core:default`.

**Interfaz → GTK: un comando.** `set_header_state({ open_visible, recents,
labels })` es síncrono, así que corre en el hilo principal y puede tocar
widgets GTK; los widgets se guardan en un `thread_local!`, porque no son
`Send`. El comando hace tres cosas:

* **Recientes.** Rehace el `gio::Menu` de la flecha y la oculta si la lista
  está vacía (ADR-0007: «sin recientes la flecha desaparece»). Medido con el
  botón «vaciar recientes» del PoC.
* **Visible u oculto.** En las vistas que no son de ningún documento, el
  ADR-0007 quita el botón de abrir: `split.set_visible(false)`. Medido con el
  botón «vista sin documento» del PoC.
* **Etiquetas**, en la sección 4.

**Ctrl+O.** Hoy lo atiende la interfaz (`rfirma-app/src/App.useOpenShortcut.ts`,
un `keydown` en `window`) mientras el foco esté en el WebView. Los botones GTK
llevan `set_focus_on_click(false)` para no quitárselo. Registrar Ctrl+O como
acelerador GTK no conviene: GTK procesa los aceleradores de la ventana antes de
pasar la tecla al widget con foco (`gtk_window_key_press_event`, `gtkwindow.c`
L8294-8306), así que se lo robaría también a un campo de texto de la interfaz.

**Desactivado durante una firma.** Si la interfaz necesita deshabilitar abrir
en vez de ocultarlo, el mismo comando lo cubre con `set_enabled` en la acción
`hdr.open`.

## 4. i18n de las etiquetas GTK

Las etiquetas salen del `.po` a través de i18next, en el frontend (ADR-0009).
Para no abrir un segundo catálogo en Rust, **la interfaz manda las etiquetas ya
traducidas** en el mismo comando (`labels: { open, recent_title, preferences,
about }`), al arrancar y en cada cambio de idioma. Los menús se reconstruyen
con los textos nuevos. Medido con el botón «English» del PoC.

Mientras la interfaz no ha arrancado, la headerbar enseña el texto por defecto
que ponga Rust. Mejor dejar los controles invisibles hasta el primer
`set_header_state`. El `title` que anuncia «Ctrl+O» pasa a ser el `tooltip` del
botón GTK, y también viaja traducido.

## 5. El tema

Cada mitad decide su tema por su cuenta:

* **La headerbar la pinta GTK**, y tao le fija el modo oscuro según el portal:
  al crear la ventana lee `org.freedesktop.appearance color-scheme`
  (`tao-0.37.1/src/platform_impl/linux/portal.rs:17`) y llama a
  `set_gtk_application_prefer_dark_theme(theme == Dark)` (`window.rs:211-221`).
  Aquí el portal devuelve `1`, así que GTK usa la variante oscura del tema. En
  el flatpak, GTK usa Adwaita salvo que esté instalada la extensión
  `org.gtk.Gtk3theme.<tema>` del tema del anfitrión.
* **El cuerpo lo pinta el CSS**, que sigue a `prefers-color-scheme` o al
  atributo `data-theme` de la preferencia de tema
  (`rfirma-app/src/preferences/theme.ts`, `applyTheme`).

**Cómo alinearlos:** que la preferencia de tema de rFirma mande también en GTK,
con `WebviewWindow::set_theme(Some(Light|Dark))` para claro u oscuro forzado y
`None` para «sistema». Llega a `set_gtk_application_prefer_dark_theme`
(`tao-0.37.1/src/platform_impl/linux/event_loop.rs:969`). El ajuste es de la
aplicación entera (`GtkSettings`), así que la ventana de sede hereda lo mismo.

**Lo que no se alinea es el color exacto**: la headerbar lleva el gris del tema
GTK y la tira de pestañas, los tokens del sistema de diseño.

## 6. Qué da la barra nativa

GTK sigue dando todo esto, y medido a mano en los tres entornos:

* Arrastrar la ventana desde la headerbar.
* Los botones de ventana del tema, en el orden del escritorio.
* La sombra, las esquinas redondeadas y los bordes de redimensionado.

Por código, además: doble clic y clic central y secundario según
`action-*-click-titlebar` (`gtkwindow.c` L1385-1460), el menú de ventana y el
estado inactivo de la headerbar.

En la tira HTML de pestañas **no** hace falta `data-tauri-drag-region`: es
contenido, no barra de título.

## Opciones descartadas

**Solo en GNOME, detectando el escritorio.** Era el planteamiento inicial:
leer `XDG_CURRENT_DESKTOP` (una lista separada por dos puntos,
[Desktop Entry Spec](https://specifications.freedesktop.org/desktop-entry-spec/latest/recognized-keys.html);
aquí `ubuntu:GNOME`, y el flatpak la transmite) y montar la headerbar solo si
algún elemento es `GNOME`. Se descarta porque en KDE la headerbar de GTK
funciona y no desentona más que cualquier aplicación GTK, y detectar el
escritorio añade un tercer modo de cabecera a cambio de nada.

**Ventana sin decorar y la barra HTML como barra de título**
(`.decorations(false)` + `data-tauri-drag-region`). Medida entera sobre 2.11:
arrastrar, doble clic, redimensionar y el foco funcionan, pero pierde lo que la
barra nativa da gratis:

* **Sin sombra y con esquinas rectas.**
* **Redimensionar a ciegas**: solo desde una franja de 5 px **dentro** del
  contenido (`tauri-runtime-wry-2.11.4/src/undecorated_resizing.rs:506`) y sin
  cambiar el cursor (el `FIXME` de
  `tao-0.35.3/src/platform_impl/linux/event_loop.rs:548`).
* **El doble clic fijo a maximizar**, ignorando el ajuste
  (`tauri-2.11.5/src/window/scripts/drag.js:103`).
* **El menú de ventana a mano**, con `gdk::Window::show_window_menu`.
* **Los botones de ventana dibujados en HTML**, que nunca serán los del tema.
* **Hay que decidirlo en el constructor**: quitar la decoración con la ventana
  abierta produce un salto visible.

**El WebView dentro de la headerbar.** Tauri mete cada WebView en el
`default_vbox` de la ventana (`tauri-runtime-wry-2.11.4/src/lib.rs:5236-5237`),
y reubicarlo a mano rompe el manejador de redimensionado, que hace
`webview.parent().parent()` con un `downcast().unwrap()`
(`undecorated_resizing.rs:546`).

**Pestañas en GTK.** Obligaría a duplicar en GTK el estado de React (cierre, ✓
de firmado, desbordamiento «+N ▾»). Se quedan en HTML.

## Precedentes e issues

* [tao#1046](https://github.com/tauri-apps/tao/issues/1046),
  [tauri#13440](https://github.com/tauri-apps/tauri/issues/13440),
  [tauri#11856](https://github.com/tauri-apps/tauri/issues/11856): los botones
  de la barra de Wayland de tao que no responden.
  [tao#1122](https://github.com/tauri-apps/tao/issues/1122): desactivar CSD
  para que el compositor ponga SSD.
* El arreglo es [tao#1218](https://github.com/tauri-apps/tao/pull/1218), en tao
  0.36.0, que llegó a Tauri por
  [tauri#15307](https://github.com/tauri-apps/tauri/pull/15307).
* No se ha encontrado ninguna aplicación Tauri que meta controles propios en la
  headerbar nativa; las que tocan la barra de Wayland, encontradas por
  búsqueda y sin revisar su código a fondo, solo corrigen `above_child` o suben
  a 2.12.

## Cómo se midió (PoC)

El PoC vive en la rama `prototype/linux-native-titlebar`, en
`prototype/linux-native-titlebar/`: una app Tauri mínima con Tauri fijado a `=2.12.0` y páginas HTML planas.
`POC_VARIANT` elige la variante:

| Variante | Qué hace |
| --- | --- |
| `native` | **El diseño de esta nota.** Headerbar GTK con título «rFirma», botón partido «Abrir PDF…» + flecha de recientes y ☰ con «Preferencias…» y «Acerca de rFirma», todo con `GMenu` y acciones GIO. El WebView (`dist/native.html`) lleva la tira de pestañas y tres botones de prueba: vista sin documento, vaciar recientes e idioma inglés. El registro enseña los eventos `header`, Ctrl+O y `prefers-color-scheme` |
| `baseline` | Ventana decorada, sin tocar |
| `html`, `html-runtime` | Las opciones descartadas: sin decorar desde el constructor, y en tiempo de ejecución |

Desde ese directorio de la rama, en la sesión real:

```
PATH="$HOME/.cargo/bin:$PATH" cargo build
POC_VARIANT=native ./target/debug/titlebar-poc
POC_VARIANT=native GDK_BACKEND=x11 ./target/debug/titlebar-poc
```

Para otra máquina, `cargo build --release` da un binario de unos 12 MB con la
interfaz dentro; necesita `libwebkit2gtk-4.1` y `libgtk-3` instalados.

Las interacciones de la variante `native` sobre 2.12 son **manuales**, en la
sesión real y en la máquina virtual KDE. Las mediciones sobre 2.11 (el fallo
de `above_child` y la opción sin decorar) se hicieron en un
`gnome-shell --headless --wayland --unsafe-mode --virtual-monitor` anidado,
con capturas por `org.gnome.Shell.Screenshot` y ratón virtual por
`org.gnome.Shell.Eval`.

**Avisos para quien lo repita:**

* **Esa sesión anidada acabó colgando la sesión gráfica del anfitrión.** No se
  recomienda repetirla fuera de una máquina virtual.
* El shell anidado carga las extensiones del usuario, e instaló actualizaciones
  pendientes de esas extensiones en su perfil.
* En la sesión real, las capturas sin interacción no funcionan:
  `org.gnome.Shell.Screenshot` rechaza a quien llama (`AccessDenied`) y el
  portal `Screenshot` con `interactive=false` pide permiso en pantalla.

## Sin medir

* Si WebKitGTK deriva `prefers-color-scheme` del `prefer-dark` de GTK, y
  `set_theme` como mando único del tema.
* El bundle flatpak `me.sgomez.rfirma` con la headerbar, y su tema GTK por
  defecto (Adwaita) frente al del anfitrión.
* Si el foco vuelve siempre al WebView tras cerrar un menú de la headerbar, de
  modo que Ctrl+O siga funcionando.
* Por qué KDE entrega `icon` en la disposición de botones a X11 y no a Wayland.
* Gestores de ventanas en mosaico (sway, i3), donde la headerbar aparece
  aunque el escritorio no dibuje barras.
