# Pestañas de documentos

Lo que la [cabecera](cabecera.md) lleva de los documentos: el botón partido de
abrir con sus recientes, las pestañas y el menú de las que no caben, más los
recientes del estado vacío. Responde a una sola pregunta: **qué documento se
firma**, y es el único sitio donde se abre, se cambia o se cierra uno.

**La verdad del dibujo es el código y sus historias:**
`documents/DocumentTabs.stories.tsx` («Ventana principal/3 · Pestañas») y
`documents/RecentRows.stories.tsx` («Ventana principal/4 · Recientes»). Esta
ficha cuenta el flujo y el porqué; los textos salen del catálogo (`tabs.*` y
`recents.*` en `po/messages.pot`) y no se copian aquí.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados.

## Flujo entre estados

| Estado | Historia | Qué se ve |
| --- | --- | --- |
| Sin pestañas | `NoTabs` | El botón partido y ninguna pestaña; el visor enseña la zona de soltar con los recientes debajo |
| Con documentos | `Several` | La activa y las demás |
| Firmado | `SignedTabActive` | La marca de firmado en la pestaña |
| Firmando | `SigningLocked` | Las demás pestañas no se activan hasta que acabe la firma, y su `title` lo dice (`tabs.lockedWhileSigning`) |
| Sin botón de abrir | `WithoutOpenButton` | Linux: el botón partido está en la barra de título GTK |
| Sin recientes | `NoRecents` | Desaparecen la flecha del botón partido y, en el estado vacío, la lista |
| Desbordada | `ManyTabs` | Las que caben y el menú de las ocultas (`tabs.more`) |
| Los recientes | `Section`, `SingleRecent`, `Rows` | La sección del estado vacío y las filas con cada situación: abierta ya, firmada, no encontrada |

Los menús —los recientes y las ocultas— son de `Popover` y se abren a mano en
Storybook: arrancan cerrados. El menú que el ☰ abre es de la
[cabecera](cabecera.md).

## Estructura

**Windows y macOS**, dentro de la barra, tras la identidad:

1. **El botón partido**: abrir PDF (`tabs.openPdf`, con su atajo en
   `tabs.openPdfShortcut`), que abre el diálogo del sistema, y una flecha que
   despliega los recientes.
2. **Pestañas**, una por documento abierto: icono de PDF, nombre, marca si está
   firmado (`badges.signed`) y cerrar (`tabs.close`).
3. **El menú de las ocultas**, solo cuando las pestañas no caben: dice cuántas
   quedan fuera.

Sin documento abierto, la barra lleva el botón partido y ninguna pestaña, y el
[visor](visor-de-documento.md) enseña la zona de soltar con los recientes
debajo. En las vistas que no son de documentos no está nada de esto.

**En Linux** las piezas se reparten entre dos franjas: el botón partido va en la
barra de título GTK, a la izquierda; las pestañas y el menú de las ocultas, en
una tira propia dentro de la ventana, del mismo gris que la barra, que lleva
**solo** eso. **Sin ninguna pestaña la tira no existe**: en el inicio sin
documentos queda la barra de título con el botón partido; en Preferencias, el
panel de estado y el primer arranque, la barra sin botón partido y sin tira.

**Con desbordamiento**, se ven tantas pestañas como caben y **la activa ocupa
siempre el último hueco visible**: nunca queda en el menú. El reparto lo calcula
`documents/tabLayout.ts`.

Una pestaña es todo el alto de la barra, con el nombre recortado con elipsis y
entero en el `title`. La activa se marca con un subrayado de 2 px que pisa la
raya de la barra y con peso 600; el resto, en `--rf-text-muted`. El botón de
cerrar ocupa siempre su hueco, visible solo en la activa y bajo el ratón, para
que la pestaña no cambie de ancho.

## Los menús

Los dos se abren hacia abajo, sobre un `Popover`, con la superficie elevada del
sistema de diseño. Mientras uno está abierto, la barra sube una capa
([capas](ventana-principal.md#capas)).

### Los recientes, desde la flecha

Alineado por la izquierda del botón partido: el rótulo (`recents.heading`), las
filas y, tras un divisor, el vaciado de la lista (`recents.clear`). Cada fila
son dos líneas: el nombre, con la marca si está firmado, y debajo la carpeta; a
la derecha, cuándo se abrió. No lleva «Abrir un PDF…»: abrir es el segmento
principal del mismo botón.

**En Linux es un popover propio**, al estilo del de GNOME Text Editor, de ancho
fijo y con el relleno, la separación y las esquinas de libadwaita. Una fila de
dos líneas por reciente, sin fecha y sin la indicación de «abierto»: el nombre
**recortado por el centro** para que se vean el principio y el final y, debajo,
la **ubicación** —la ruta de la carpeta, con `~/` en lugar del directorio
personal— más pequeña, atenuada y **recortada por el final**. Bajo el portal no
hay ubicación ([ADR-0011](../adr/0011-destino-del-documento-firmado.md)) y la
segunda línea queda vacía. Sin buscador y sin forma de quitar un reciente suelto.

**Con muchos recientes** la lista llega a diez, pero en GTK 3 el popover no
puede salir de la ventana. El rótulo queda fijo arriba y el vaciado fijo abajo;
entre los dos, las filas miden como mucho lo que quepa dentro de la ventana y se
desplazan en vertical. Desplazamiento horizontal, nunca: el ancho es fijo y los
textos se recortan.

### Las pestañas ocultas

Alineado por la derecha de su botón. Una fila por pestaña que no cabe, de **una
línea**, con elipsis y la marca de firmado. Elegir una la trae a la barra como
activa.

### Abrir

**Abrir PDF** —o Ctrl+O— abre el explorador del sistema **en la última carpeta
usada**; en el flatpak, que no la conoce, en la carpeta de destino de
Preferencias ([ADR-0011](../adr/0011-destino-del-documento-firmado.md)).

## Los recientes

- **Diez como máximo**, con desalojo por último uso. Se cachean nombre, estado,
  `mtime` y fecha de último uso para pintar la fila sin abrir el fichero
  ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).
- **Ya abierto en una pestaña**: la fecha se cambia por `recents.open`, en
  cursiva, y pulsarlo lleva a su pestaña (`recents.goToTab`).
- **No se encuentra** (`recents.missing`): el fichero ya no está en su ruta. La
  fila sale atenuada, con ese texto en lugar de la carpeta, y no se abre. No se
  purga sola: un USB desmontado no es un fichero borrado, y la fila revive cuando
  vuelve.
- La marca dice que el PDF ya lleva firmas, sean de quien sean; quién y cuándo lo
  cuenta el aviso de firmas previas del [panel](panel-de-firma.md).
- **Sin recientes** —«Recordar mi actividad» apagado en
  [Preferencias](preferencias.md), o la lista vacía— desaparece la flecha del
  botón partido y, en el estado vacío, la lista bajo la zona de soltar. En Linux
  igual: el botón GTK se queda con el de abrir. Sin recientes tampoco hay
  separador: abrir redondea sus cuatro esquinas y queda un botón simple.
- La fecha de cada fila es «hoy», «ayer» (`recents.today`, `recents.yesterday`) o
  el día y el mes, en el idioma de la interfaz.

## Componentes y tokens

`Button`, `Popover` y los iconos del sistema de diseño; `.rf-label`,
`.rf-divider`; `--rf-surface`, `--rf-bg`, `--rf-border-subtle`,
`--rf-border-strong`, `--rf-text`, `--rf-text-muted`, `--rf-radius-md`,
`--rf-radius-lg`, `--rf-shadow-elevated`, `--rf-duration-fast`. La geometría la
dan `DocumentTabs.css` y las historias.

## Decisiones

- **Pestañas en lugar de la bandeja lateral** de 300 px. La bandeja era una
  columna fija que le quitaba al visor el ancho que necesita para colocar la
  firma visible.
- **Las pestañas van en la barra de la cabecera**, no en una tira propia
  (27/09/2026). Se descartó la tira bajo una cabecera alta: dos franjas para la
  parte de arriba de la ventana
  ([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)). Con la barra
  sobre `--rf-bg`, la activa ya no se funde con el visor por el fondo, y se marca
  con un subrayado y peso 600.
- **Abrir es el segmento principal y reabrir la flecha, en el mismo control**
  (27/09/2026). Antes, abrir y los recientes compartían el menú de un «+» al
  final de la tira, con el argumento de que abrir y reabrir son el mismo gesto.
  Lo son en intención, pero no en frecuencia: abrir un fichero nuevo es lo
  habitual, y detrás de un menú costaba dos clics y no tenía atajo. Con el botón
  partido, abrir es un clic o Ctrl+O, y reabrir sigue a un gesto de distancia
  en el mismo sitio. Hubo también un botón «Recientes» suelto en la tira, y se
  quitó por lo mismo: separaba en dos controles lo que es una sola intención.
  El botón partido se retiró del pie del panel porque un clic en el segmento
  equivocado firmaba con un certificado que no se quería; aquí el mismo error
  abre el diálogo del sistema, que se cierra sin consecuencias.
- **El menú de las ocultas en lugar de las flechas ‹ ›** (27/09/2026).
  Desplazar la tira obliga a buscar pasando de una en una; el menú enseña todas
  las ocultas de golpe, y como la activa siempre está a la vista no hace falta
  desplazar para encontrarla.
- **En Linux, la tira solo con las pestañas, y solo si hay alguna**
  (30/09/2026). El botón partido sube a la barra de título GTK, que es donde
  GTK pone sus controles; lo que queda debajo son los documentos abiertos, y
  sin ninguno una tira vacía sería una franja muerta.
- **En Linux, recientes en un popover propio de dos líneas y sin fecha**
  (01/10/2026). Se descartó el menú GTK estándar de una línea, con la carpeta
  tras el nombre: GTK 3 lo dibuja sin relleno ni separación entre filas, los
  nombres largos no se recortan y ensanchan el menú, y el nombre de la carpeta a
  solas no distingue dos carpetas homónimas. La ruta con `~/` sí las distingue,
  y el ancho fijo con los recortes mantiene el popover en su sitio. La fecha
  sigue fuera: no decide cuál se reabre.
- **El rótulo es el mismo en el menú y en el estado vacío** (`recents.heading`):
  un rótulo solo, igual en los dos sitios.
- **«Abierto» es texto, no un icono.** Un punto o una marca de pestaña se
  confunden con «sin guardar» o con «firmado».
- **Se pierde la insignia de tres valores** (firmado, sin firmar y no
  disponible): la marca dice lo primero, su ausencia lo segundo, y «no se
  encuentra» lo tercero.
- **El cierre de una pestaña es un hermano de la pestaña, no un hijo.** Un botón
  dentro de otro botón no es HTML válido, así que cada pestaña es un contenedor
  con dos botones. axe lo señala dentro del `tablist` y la historia apaga esa
  regla con un comentario; arreglarlo es cambiar el marcado, y esta tanda no
  cambia ninguna pantalla.
