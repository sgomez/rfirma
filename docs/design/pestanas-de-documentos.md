# Pestañas de documentos

La tira bajo la cabecera, su menú «+» y los recientes del estado vacío.
Responde a una sola pregunta: **qué documento se firma**, y es el único sitio
donde se abre, se cambia o se cierra uno.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados.

## Estructura

Una tira de 40 px, de izquierda a derecha:

1. **Pestañas**, una por documento abierto: icono de PDF, nombre, ✓ si está
   firmado y ✕ para cerrar.
2. **Flechas ‹ ›**, solo cuando las pestañas no caben.
3. **«+»**, que despliega el menú de abrir.

Sin documento abierto, la tira solo lleva el «+» y el
[visor](visor-de-documento.md) enseña la zona de soltar con los recientes
debajo.

### Geometría

- Tira: 40 px, `--rf-surface`, borde inferior de 1 px en `--rf-border-subtle`,
  relleno lateral de 8 px, 2 px entre piezas, `align-items: flex-end`,
  `z-index: 10`.
- **Todo lo que vive en la tira es una caja de 33 px** con borde transparente de
  1 px arriba y a los lados y `margin-bottom: -1px`: pestañas, flechas y «+»
  comparten línea base y centro.
- **Pestaña**: 240 px de ancho fijo (168 px con muchas), relleno `0 6px 0 12px`,
  8 px entre piezas, radio `--rf-radius-md` arriba, 13 px. Nombre con elipsis y
  entero en el `title`.
  - **Activa**: borde `--rf-border-subtle`, fondo `--rf-bg` —tapa la raya
    inferior y se funde con el visor— y peso 600.
  - **Bajo el ratón**: fondo `--rf-border-subtle`.
  - **Resto**: texto en `--rf-text-muted`, sin fondo.
  - La ✓ es de 13 px en `--rf-text-muted`. El ✕ ocupa siempre su hueco de
    20 px, visible solo en la activa y bajo el ratón, para que la pestaña no
    cambie de ancho.
- **Flechas y «+»**: botón interior de 28 px con `--rf-radius-md` e icono de
  16 px.
- **Con desbordamiento**, aparecen ‹ y ›, la lista se desplaza para que la
  activa quede a la vista y el «+» va tras la flecha derecha.

## El menú del «+»

Se abre hacia abajo anclado al «+», 340 px de ancho, `z-index: 8`,
`--rf-radius-lg`, borde `--rf-border-subtle`, fondo `--rf-bg`,
`--rf-shadow-elevated`, 6 px de relleno. De arriba abajo:

1. **«Abrir un PDF…»** con icono de carpeta, fondo `--rf-surface`, peso 600.
2. Divisor y el rótulo **RECIENTES** (`.rf-label` en versalitas,
   `letter-spacing: .6px`).
3. **Seis filas de dos líneas**: nombre a 13 px con la ✓ si está firmado, y
   debajo la carpeta a 12 px en `--rf-text-muted`; a la derecha, cuándo se abrió
   («hoy», «ayer», «12 sep»).
4. Divisor y **«Vaciar la lista»** en `--rf-text-muted`.

Se alinea por la izquierda del «+»; cuando el «+» queda al borde derecho, por la
derecha, para no salirse de la ventana.

**«Abrir un PDF…»** abre el explorador del sistema **en la última carpeta
usada**; en el flatpak, que no la conoce, en la carpeta de destino de
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

## Estados

En el artboard `Main`:

- **Vacío** (`estado: vacío`): la tira solo lleva el «+». En el visor, la zona
  de soltar de 520 × 170 px y debajo, al mismo ancho, RECIENTES con «Vaciar la
  lista» a la derecha y las mismas filas que el menú, ninguna «Abierto».
- **Con documentos**: la activa y las demás; la segunda pestaña enseña el
  estado bajo el ratón.
- **Firmado**: la pestaña pasa a `…-firmado.pdf` con su ✓.
- **Menú «+» abierto** (palanca «Menú +»).
- **Desbordada** (palanca «Contenido: extremo»): doce pestañas de 168 px con
  flechas.
- Con «Recordar mi actividad» apagado en [Preferencias](preferencias.md) no hay
  recientes: el menú se queda en «Abrir un PDF…» y el estado vacío en la zona
  de soltar.

## Componentes y tokens

`.rf-label`, `.rf-divider`, `.rf-row`, `.rf-stack`, `.rf-btn--ghost`,
`--rf-surface`, `--rf-bg`, `--rf-border-subtle`, `--rf-text-muted`,
`--rf-radius-md`, `--rf-radius-lg`, `--rf-shadow-elevated`.

## Decisiones

- **Pestañas en lugar de la bandeja lateral** de 300 px. La bandeja era una
  columna fija que le quitaba al visor el ancho que necesita para colocar la
  firma visible; las pestañas ocupan 40 px de alto y dicen lo mismo.
- **Los recientes van en el menú del «+» y en el estado vacío**, no en un botón
  propio. Hubo un botón «Recientes» en la tira y se quitó: abrir y reabrir son
  el mismo gesto.
- **«Abierto» es texto, no un icono.** Un punto o una marca de pestaña se
  confunden con «sin guardar» o con «firmado».
- **Se pierde la insignia de tres valores** (`Firmado`, `Sin firmar`,
  `No disponible`): la ✓ dice lo primero, su ausencia lo segundo, y «No se
  encuentra» lo tercero.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`, el 25/09/2026.
