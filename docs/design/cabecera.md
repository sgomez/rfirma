# Cabecera

La franja superior de la ventana: el nombre de la aplicación y el único menú.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados.
- El panel de estado — es la puerta: el [panel de estado](panel-de-estado.md) se
  abre desde su menú, y el menú avisa cuando hay algo que mirar ahí.

## Estructura

Una fila de 52 px sobre `--rf-surface`:

- **Izquierda**: `rFirma` en `.rf-title`.
- **Derecha**: el botón de menú.

**Nada más.** Ni certificado ni estado del documento: el certificado lo dice el
botón de firmar y el estado, la ✓ de la [pestaña](pestanas-de-documentos.md).

### Geometría

- Alto 52 px, **sin borde inferior**: la raya la pone la tira de pestañas, que va
  justo debajo con el mismo fondo. Relleno asimétrico: 24 px (`--rf-space-md`) a
  la izquierda y 8 px (`--rf-space-xs`) a la derecha, porque el botón trae su
  propio cuadro de 40 px.
- El nombre en `.rf-title` bajado a 15 px con `letter-spacing: .4px`, para que
  se lea como identidad y no como título.
- El botón de menú es un cuadrado de 40×40 px con `--rf-radius-md` y el icono de
  tres rayas de 20 px.
- La cabecera va en `z-index: 5`; con el menú abierto sube a **11** para que el
  menú quede sobre la tira, que es 10.
- El menú flota a 50 px del borde superior y a 8 px del derecho, con 230 px de
  ancho mínimo, 6 px de relleno, 2 px entre entradas, `--rf-radius-md`, borde
  `--rf-border-subtle`, fondo `--rf-bg` y `--rf-shadow-elevated`.
- Cada entrada es `.rf-prose` con relleno de 9 × 10 px y `--rf-radius-sm`. El
  divisor es un `.rf-divider` con 4 px de aire.
- **Toda entrada reserva a su derecha una columna de 14 px**, lleve icono o no,
  para que el texto de las cuatro quede alineado.

### Los iconos

`<svg>` en línea sobre lienzo `0 0 24 24`, trazo en `currentColor`, extremos y
uniones redondeados. No hay biblioteca de iconos.

- Tres rayas del botón: 20 px, trazo 1.5, `d="M4 7h16M4 12h16M4 17h16"`.
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

**Lo que no está en el menú:** abrir un documento (lo hace el «+» de la tira),
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

En el artboard `Main`, palancas «Menú de la cabecera»:

- **Cerrado** (por defecto), en cualquier estado de la ventana.
- **Abierto**: el botón se rellena con `--rf-primary` / `--rf-on-primary`; el
  menú flota anclado a la derecha, por encima de la tira.
- **Con aviso** o **todo en orden**: el triángulo en «Estado de rFirma», o no.
- **Foco**: sin foco, en la primera entrada, o en «Comentarios y ayuda».

## Componentes y tokens

`.rf-title`, `.rf-prose`, `.rf-row`, `.rf-gap-xs`, `.rf-divider`,
`--rf-surface`, `--rf-bg`, `--rf-border-subtle`, `--rf-primary`,
`--rf-on-primary`, `--rf-text`, `--rf-text-muted`, `--rf-shadow-elevated`,
`--rf-focus-ring`, `--rf-radius-md`, `--rf-radius-sm`.

## Decisiones

- **Sin barra de menús clásica.** GNOME la abandonó, Windows 11 no la usa y en
  macOS Tauri registra un menú nativo en la barra del sistema.
- **Sin insignia de estado ni certificado** (25/09/2026). La insignia «Sin
  firmar / Firmado» pasa a la ✓ de cada pestaña, que dice lo mismo por
  documento; el certificado, al botón «Firmar como».
- **El divisor del menú.** Sin él las cuatro entradas se leen del mismo rango y
  «Estado de rFirma» deja de ser lo primero.
- **«Estado de rFirma», no «Estado».** El nombre lo desambigua del estado del
  documento.
- **El aviso es una silueta, no un contador ni un punto de color.** Un número
  obligaría a contar en dos sitios; un punto de color sería el único indicador,
  que la sección 8 del [sistema de diseño](design-system.md) prohíbe.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`.
