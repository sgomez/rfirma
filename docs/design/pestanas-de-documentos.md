# Pestañas de documentos

Lo que la [cabecera](cabecera.md) lleva de los documentos: el botón partido de
abrir con sus recientes, las pestañas y el menú de las que no caben, más los
recientes del estado vacío. Responde a una sola pregunta: **qué documento se
firma**, y es el único sitio donde se abre, se cambia o se cierra uno.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados.

## Estructura

Dentro de la barra de 44 px, tras `rFirma`:

1. **El botón partido**: «Abrir PDF…», que abre el diálogo del sistema, y una
   flecha ▾ que despliega «Abiertos recientemente».
2. **Pestañas**, una por documento abierto: icono de PDF, nombre, ✓ si está
   firmado y ✕ para cerrar.
3. **«+N ▾»**, solo cuando las pestañas no caben: N es cuántas quedan fuera, y
   despliega el menú de las ocultas.

Sin documento abierto, la barra lleva el botón partido y ninguna pestaña, y el
[visor](visor-de-documento.md) enseña la zona de soltar con los recientes
debajo. En las vistas que no son de documentos no está nada de esto: la barra se
queda con `rFirma` y el menú.

### Geometría

- **Botón partido**: 32 px de alto, borde de 1 px en `--rf-border-strong`,
  `--rf-radius-md`, fondo `--rf-bg`, 12 px de aire hasta la primera pestaña.
  - **«Abrir PDF…»**: `.rf-btn--ghost` a 13 px y peso 600, icono de carpeta de
    15 px (trazo 1.6), relleno `0 10px 0 9px`, 7 px entre icono y texto. `title`
    «Ctrl+O».
  - Un filete de 1 × 16 px en `--rf-border-subtle` separa los dos segmentos.
  - **Flecha**: 30 px de ancho, chevron de 14 px (trazo 2), `title` «Abiertos
    recientemente». Desplegada, fondo `--rf-surface` con un borde interior de
    1 px en `--rf-border-subtle`, y el chevron gira 180°.
  - **Sin recientes** no hay filete ni flecha, y «Abrir PDF…» redondea sus
    cuatro esquinas: queda un botón simple.
- **Pestaña**: todo el alto de la barra, `flex: 1 1 200px` entre 160 y 200 px,
  relleno `0 4px 0 10px`, 8 px entre piezas, 13 px, 2 px entre pestañas. Nombre
  con elipsis y entero en el `title`.
  - Un **subrayado de 2 px** con `margin-bottom: -1px`, que pisa la raya de la
    barra.
  - **Activa**: subrayado `--rf-text`, texto `--rf-text`, peso 600.
  - **Bajo el ratón**: texto `--rf-text`, sin subrayado ni fondo.
  - **Resto**: texto en `--rf-text-muted`, subrayado transparente.
  - La ✓ es de 13 px en `--rf-text-muted`. El ✕ ocupa siempre su hueco de
    20 px, visible solo en la activa y bajo el ratón, para que la pestaña no
    cambie de ancho.
- **«+N ▾»**: `.rf-btn--ghost` de 30 px de alto, 13 px y peso 600, relleno
  `0 6px 0 8px`, chevron de 14 px, 4 px de aire tras la última pestaña.
  Desplegado, el mismo fondo y borde interior que la flecha del botón partido.
- **Con desbordamiento**, se ven tantas pestañas como caben (cuatro a 1180 px),
  y **la activa ocupa siempre el último hueco visible**: nunca queda en el menú.

## Los menús

Los dos se abren hacia abajo, con 4 px de separación, 360 px de ancho,
`--rf-radius-lg`, borde `--rf-border-subtle`, fondo `--rf-bg`,
`--rf-shadow-elevated`, 6 px de relleno y 2 px entre filas. Mientras uno está
abierto, la barra sube a `z-index: 11`.

### «Abiertos recientemente», desde la flecha

Alineado por la izquierda del botón partido. De arriba abajo:

1. El rótulo **ABIERTOS RECIENTEMENTE** (`.rf-label` en versalitas,
   `letter-spacing: .6px`).
2. **Las filas de los recientes**, de dos líneas: nombre a 13 px con la ✓ si
   está firmado, y debajo la carpeta a 12 px en `--rf-text-muted`; a la
   derecha, cuándo se abrió («hoy», «ayer», «12 sep»). Relleno de 7 × 10 px.
3. Divisor y **«Vaciar la lista»** en `--rf-text-muted`.

No lleva «Abrir un PDF…»: abrir es el segmento principal del mismo botón.

### Las pestañas ocultas, desde «+N»

Alineado por la derecha de «+N». Una fila por pestaña que no cabe, de **una
línea**: el nombre a 13 px con elipsis y la ✓ si está firmado, relleno de
7 × 10 px. Elegir una la trae a la barra como activa.

### Abrir

**«Abrir PDF…»** —o Ctrl+O— abre el explorador del sistema **en la última
carpeta usada**; en el flatpak, que no la conoce, en la carpeta de destino de
Preferencias ([ADR-0011](../adr/0011-destino-del-documento-firmado.md)).

## Los recientes

- **Diez como máximo**, con desalojo por último uso. Se cachean nombre, estado,
  `mtime` y fecha de último uso para pintar la fila sin abrir el fichero
  ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).
- **Ya abierto en una pestaña**: la fecha se cambia por «Abierto» en cursiva, y
  pulsarlo lleva a su pestaña (`title` «Ir a su pestaña»).
- **No se encuentra**: el fichero ya no está en su ruta. La fila sale al 45 %,
  con «No se encuentra» en lugar de la carpeta, y no se abre. No se purga sola:
  un USB desmontado no es un fichero borrado, y la fila revive cuando vuelve.
- La ✓ dice que el PDF ya lleva firmas, sean de quien sean; quién y cuándo lo
  cuenta el aviso de firmas previas del [panel](panel-de-firma.md).
- **Sin recientes** —«Recordar mi actividad» apagado en
  [Preferencias](preferencias.md), o la lista vacía— desaparece la flecha del
  botón partido y, en el estado vacío, la lista bajo la zona de soltar.

## Estados

En el artboard `Main`:

- **Vacío** (`estado: vacío`): el botón partido y ninguna pestaña. En el visor,
  la zona de soltar de 520 × 170 px y debajo, al mismo ancho, ABIERTOS
  RECIENTEMENTE con «Vaciar la lista» a la derecha y las mismas filas que el
  menú, ninguna «Abierto».
- **Con documentos**: la activa y las demás; la segunda pestaña enseña el
  estado bajo el ratón.
- **Firmado**: la pestaña pasa a `…-firmado.pdf` con su ✓.
- Palanca «Abiertos recientemente»: **plegados**, **desplegados** o **sin
  recientes** (lista vacía o actividad apagada).
- **Desbordada** (palanca «Contenido: extremo»): doce documentos, cuatro
  pestañas a la vista y «+8 ▾»; palanca «Desborde de pestañas» para abrir su
  menú.

## Componentes y tokens

`.rf-label`, `.rf-divider`, `.rf-row`, `.rf-stack`, `.rf-btn--ghost`,
`--rf-surface`, `--rf-bg`, `--rf-border-subtle`, `--rf-border-strong`,
`--rf-text`, `--rf-text-muted`, `--rf-radius-md`, `--rf-radius-lg`,
`--rf-shadow-elevated`, `--rf-duration-fast`.

## Decisiones

- **Pestañas en lugar de la bandeja lateral** de 300 px. La bandeja era una
  columna fija que le quitaba al visor el ancho que necesita para colocar la
  firma visible.
- **Las pestañas van en la barra de la cabecera**, no en una tira propia
  (27/09/2026). Se descartó la tira de 40 px bajo una cabecera de 52 px: dos
  franjas para la parte de arriba de la ventana
  ([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)). Con la barra
  sobre `--rf-bg`, la activa ya no se funde con el visor por el fondo, y se marca
  con un subrayado de 2 px y peso 600.
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
- **«+N» en lugar de las flechas ‹ ›** (27/09/2026). Desplazar la tira obliga a
  buscar pasando de una en una; el menú enseña todas las ocultas de golpe, y
  como la activa siempre está a la vista no hace falta desplazar para
  encontrarla.
- **«Abiertos recientemente», no «Recientes»**, en el menú y en el estado vacío:
  un rótulo solo, igual en los dos sitios.
- **«Abierto» es texto, no un icono.** Un punto o una marca de pestaña se
  confunden con «sin guardar» o con «firmado».
- **Se pierde la insignia de tres valores** (`Firmado`, `Sin firmar`,
  `No disponible`): la ✓ dice lo primero, su ausencia lo segundo, y «No se
  encuentra» lo tercero.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`; la barra única, el 27/09/2026.
