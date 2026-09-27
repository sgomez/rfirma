# Cabecera

La barra única de la ventana: el nombre de la aplicación, los documentos y el
único menú, en una sola fila.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados.
- El panel de estado — es la puerta: el [panel de estado](panel-de-estado.md) se
  abre desde su menú, y el menú avisa cuando hay algo que mirar ahí.
- Preferencias y el primer arranque — con la variante sin documentos.

## Estructura

Una fila de 44 px sobre `--rf-bg`, de izquierda a derecha:

1. **`rFirma`**, la identidad.
2. **El botón partido** «Abrir PDF… ▾».
3. **Las pestañas** de los documentos abiertos, y al final el **«+N ▾»** si no
   caben.
4. Un hueco flexible.
5. **El botón de menú**, al extremo derecho.

Lo que va de 2 a 3 es de los documentos, y su ficha es
[Pestañas de documentos](pestanas-de-documentos.md); esta cuenta la barra, la
identidad y el menú.

**Nada más.** Ni certificado ni estado del documento: el certificado lo dice el
selector del [panel de firma](panel-de-firma.md) y el estado, la ✓ de la
pestaña.

### Las dos variantes

- **Con documentos**: la ventana principal, con o sin documento abierto. Sin
  ninguno, la barra lleva el botón partido y ninguna pestaña.
- **Sin documentos**: Preferencias, el panel de estado y el primer arranque.
  Solo `rFirma` y el menú; ni botón partido ni pestañas, porque esas vistas no
  son de ningún documento. Mismo alto, misma raya, mismo menú.

### Geometría

- Alto 44 px, fondo `--rf-bg`, **borde inferior de 1 px** en
  `--rf-border-subtle`, `align-items: stretch`, relleno `0 6px 0 4px`, sin
  separación entre piezas: cada una trae la suya.
- `rFirma` a 15 px, peso 700, `letter-spacing: .4px`, relleno `0 12px 0 8px`:
  se lee como identidad y no como título.
- El hueco flexible no baja de 24 px, para que la última pestaña no toque el
  menú.
- El botón de menú mide 32×30 px, con `--rf-radius-md`, el icono de tres rayas
  de 18 px y 4 px de aire a su derecha.
- La barra va en `z-index: 10`; con cualquiera de sus tres menús abierto —el del
  botón de menú, los recientes o las pestañas ocultas— sube a **11**.
- El menú flota a 48 px del borde superior y a 10 px del derecho, con 230 px de
  ancho mínimo, 6 px de relleno, 2 px entre entradas, `--rf-radius-md`, borde
  `--rf-border-subtle`, fondo `--rf-bg` y `--rf-shadow-elevated`.
- Cada entrada es `.rf-prose` con relleno de 9 × 10 px y `--rf-radius-sm`. El
  divisor es un `.rf-divider` con 4 px de aire.
- **Toda entrada reserva a su derecha una columna de 14 px**, lleve icono o no,
  para que el texto de las cuatro quede alineado.

### Los iconos

`<svg>` en línea sobre lienzo `0 0 24 24`, trazo en `currentColor`, extremos y
uniones redondeados. No hay biblioteca de iconos.

- Tres rayas del botón: 18 px, trazo 1.5, `d="M4 7h16M4 12h16M4 17h16"`.
- **Enlace externo**, 14 px, trazo 1.8, en `--rf-text-muted`:
  `d="M14 4h6v6"`, `d="M20 4 11 13"`,
  `d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"`.
- **Triángulo de aviso**, 14 px, trazo 1.8, en `--rf-text`:
  `d="M12 4 2.5 20h19z"`, `d="M12 10v4M12 17v.5"`. Es el mismo `path` que
  «Atención» en el [panel de estado](panel-de-estado.md).

## El menú

Cuatro entradas en dos grupos:

- Estado de rFirma
- ────
- Preferencias…
- Comentarios y ayuda — con el icono de enlace externo
- Acerca de rFirma

Arriba lo de **esta instalación**; abajo el grupo de siempre de la GNOME HIG, en
su orden. No hay barra de menús
([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)).

**Lo que no está en el menú:** abrir un documento (lo hace el botón partido),
guardar (el pie del panel), paginación y zoom (la píldora del
[visor](visor-de-documento.md)), atajos y guía rápida (no existen).

### El aviso

Cuando hay algo que **rFirma puede y debe arreglar**, «Estado de rFirma» lleva
el triángulo a la derecha. Lo encienden dos cosas y solo dos:

- **El certificado de rFirma ausente o a medias** en los navegadores.
- **`Sin configurar`** en `Firma en sedes`.

No lo encienden: que las sedes abran AutoFirma (es una elección), «No aplica»,
`Certificados de firma electrónica: Ninguno` (no lo arregla rFirma) ni una
versión nueva (eso lo dice la franja de la
[ventana principal](ventana-principal.md)).

La fila del panel **informa**; el triángulo **llama**. Por eso hay «Atención»
que no lo encienden.

Sin número ni contador, y la marca es la silueta, no el color. Ocupa la columna
de 14 px, así que aparecer o desaparecer no mueve nada.

### El foco por teclado

La entrada enfocada lleva el anillo del sistema —2 px en `--rf-focus-ring`, 2 px
de desplazamiento— y fondo `--rf-surface`. Forma y color, no color solo.

## Estados

En el artboard `Main`:

- **Con documentos** (cualquier posición de «Estado» salvo la última) o **sin
  documentos** («sin pestañas · Preferencias»), las dos variantes.
- Palancas «Menú de la cabecera»:
  - **Cerrado** (por defecto), en cualquier estado de la ventana.
  - **Abierto**: el botón se rellena con `--rf-primary` / `--rf-on-primary`; el
    menú flota anclado a la derecha, sobre el contenido.
  - **Con aviso** o **todo en orden**: el triángulo en «Estado de rFirma», o no.
  - **Foco**: sin foco, en la primera entrada, o en «Comentarios y ayuda».

La barra la estampa `_cabecera.part` en todos los artboards que pintan la
ventana principal, con o sin documentos.

## Componentes y tokens

`.rf-prose`, `.rf-row`, `.rf-gap-xs`, `.rf-divider`,
`--rf-surface`, `--rf-bg`, `--rf-border-subtle`, `--rf-primary`,
`--rf-on-primary`, `--rf-text`, `--rf-text-muted`, `--rf-shadow-elevated`,
`--rf-focus-ring`, `--rf-radius-md`, `--rf-radius-sm`.

## Decisiones

- **Sin barra de menús clásica.** GNOME la abandonó, Windows 11 no la usa y en
  macOS Tauri registra un menú nativo en la barra del sistema
  ([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)).
- **Una sola barra, no cabecera y tira** (27/09/2026). Se descartó la cabecera
  de 52 px sobre `--rf-surface` con la tira de pestañas de 40 px debajo: eran
  dos franjas para la parte de arriba de la ventana, 92 px que ahora son 44, y
  los 48 que sobran vuelven al visor. El fondo pasa a `--rf-bg`, el del visor,
  porque ya no hay una tira que se funda con la pestaña activa: la activa se
  marca con un subrayado.
- **Sin insignia de estado ni certificado** (25/09/2026). La insignia «Sin
  firmar / Firmado» pasa a la ✓ de cada pestaña, que dice lo mismo por
  documento; el certificado, al selector del [panel de firma](panel-de-firma.md).
- **El divisor del menú.** Sin él las cuatro entradas se leen del mismo rango y
  «Estado de rFirma» deja de ser lo primero.
- **«Estado de rFirma», no «Estado».** El nombre lo desambigua del estado del
  documento.
- **El aviso es una silueta, no un contador ni un punto de color.** Un número
  obligaría a contar en dos sitios; un punto de color sería el único indicador,
  que la sección 8 del [sistema de diseño](design-system.md) prohíbe.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`; la barra única, el 27/09/2026.
