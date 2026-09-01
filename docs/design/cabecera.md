# Cabecera

La franja superior de la ventana: identidad, estado del documento y el único
menú de la aplicación.

## Casos de uso que la usan

- Firmar un PDF en local — presente en los diez estados.

## Estructura

Una sola fila de 56 px sobre `--rf-surface`, separada del cuerpo por
`--rf-border-subtle`:

- **Izquierda**: `rFirma` en `.rf-title`.
- **Derecha**: la insignia de estado del documento y el botón de menú.

La insignia usa dos valores y solo dos: **`Sin firmar`** y **`Firmado`**
(en `.rf-badge--primary`). No aparece cuando no hay documento abierto.

### Geometría

- Alto 56 px. Relleno **asimétrico**: 24 px (`--rf-space-md`) a la izquierda y
  16 px (`--rf-space-sm`) a la derecha, porque a la derecha el botón ya trae su
  propio cuadro de 40 px. Separación entre piezas, 16 px.
- El nombre va en `.rf-title` pero **bajado a 15 px** con `letter-spacing:
  .4px`. Los 20 px plenos de `.rf-title` compiten con el título del panel en
  una franja de 56 px; el tracking abierto es lo que lo devuelve a leerse como
  identidad.
- El botón de menú es un **cuadrado de 40×40 px** con `--rf-radius-md` y el
  icono de tres rayas de 20 px. Nada de relleno propio.
- El menú desplegado flota a 52 px del borde superior de la ventana y a 16 px
  del derecho, con 230 px de ancho mínimo, 6 px de relleno, 2 px entre
  entradas, `--rf-radius-md`, borde `--rf-border-subtle`, fondo `--rf-bg` y
  `--rf-shadow-elevated`.
- Cada entrada es `.rf-prose` con 9 px de relleno vertical, 10 px horizontal y
  `--rf-radius-sm`.

### Los iconos

Las tres rayas del botón son un `<svg>` **en línea**, copiado del artboard
(ID-53): 20×20 px sobre lienzo `0 0 24 24`, trazo de 1.5 en `currentColor`,
extremos y uniones redondeados, `d="M4 7h16M4 12h16M4 17h16"`. No hay
biblioteca de iconos ni icono de fuente, y el `☰` de texto que hubo antes ya
no está: un glifo tipográfico cambia de forma con la fuente instalada.

## El menú

Botón de 40 px que despliega un menú con **dos entradas**:

- Preferencias…
- Acerca de rFirma

No hay barra de menús clásica. Ver
[ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md), que además fija
cómo se ancla esto en macOS.

**Lo que deliberadamente no está en el menú:**

- *Archivo* — abrir tiene la zona de soltar de la bandeja; guardar tiene la
  fila «Se guardará en» del panel de firma. Un menú que los repite es un
  segundo camino para lo mismo.
- *Ver* — paginación y zoom viven en la barra flotante del
  [visor](visor-de-documento.md).
- *Atajos de teclado* y *Guía rápida* — no existen todavía; un menú no es
  sitio para prometerlas.

## Estados

- **Sin documento**: solo el nombre y el botón de menú.
- **Con documento sin firmar**: insignia `Sin firmar`.
- **Documento firmado**: insignia `Firmado` en `--rf-primary`.
- **Menú abierto**: el botón se rellena con `--rf-primary`; el menú flota con
  `--rf-shadow-elevated` anclado a la derecha.

El menú **arranca cerrado**. El artboard «1 · Vacío · menú abierto» lo dibuja
desplegado para enseñar sus dos entradas, pero eso es una posibilidad y no el
estado inicial: abrir la aplicación con un menú encima del documento no es lo
que el canvas pide.

## Componentes y tokens

`.rf-title`, `.rf-badge`, `.rf-badge--primary`, `--rf-surface`,
`--rf-border-subtle`, `--rf-primary`, `--rf-on-primary`,
`--rf-shadow-elevated`, `--rf-radius-md`.

## Decisiones

La barra de menús clásica (`Archivo · Ver · Ayuda`) se dibujó primero y se
retiró: GNOME la abandonó, Windows 11 no la usa y en macOS Tauri registra un
menú nativo en la barra del sistema, de modo que dibujarla dentro de la ventana
la duplicaría. Fundirla con la barra de aplicación devolvió además 30 px de
alto al [panel de firma](panel-de-firma.md), que iba justo.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «1 · Vacío · menú abierto».
