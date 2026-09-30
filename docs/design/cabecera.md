# Cabecera

La parte de arriba de la ventana: el nombre de la aplicación, los documentos,
el aviso y el único menú. En Windows y macOS es una sola barra; en Linux, la
barra de título nativa de GTK con una tira de pestañas debajo.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados.
- El panel de estado — es la puerta: el [panel de estado](panel-de-estado.md) se
  abre desde su menú, y el botón de aviso lleva a él cuando hay algo que mirar
  ahí.
- Preferencias y el primer arranque — con la variante sin documentos.

## Estructura

### En Windows y macOS

Una fila de 44 px sobre `--rf-bg`, de izquierda a derecha:

1. **`rFirma`**, la identidad.
2. **El botón partido** «Abrir PDF… ▾».
3. **Las pestañas** de los documentos abiertos, y al final el **«+N ▾»** si no
   caben.
4. Un hueco flexible.
5. **El botón de aviso**, solo si hay algo que atender.
6. **El botón de menú**, al extremo derecho. Solo en Windows: en macOS el menú
   es el de la barra del sistema, y el botón de aviso queda en el extremo.

### En Linux

En todo Linux —GNOME, KDE y los demás, sin detectar el escritorio— la barra de
título es la **nativa de GTK**, y la fila de 44 px no existe. Dentro de ella:

1. A la izquierda, **el botón partido** GTK «Abrir PDF… | ▾».
2. En el centro, el título **«rFirma»**.
3. A la derecha, **el botón de aviso** (solo si hay algo que atender), **el ☰**
   y **los botones de ventana del tema**, en el orden que diga el escritorio.

Debajo, ya dentro de la ventana, **una tira con solo las pestañas** y el
«+N ▾», del mismo gris que la barra. **Sin ninguna pestaña la tira no existe**:
ni en el inicio sin documentos, ni en Preferencias, ni en el panel de estado, ni
en el primer arranque.

La barra sigue la **preferencia de tema de rFirma** —claro, oscuro o sistema—,
no solo la del escritorio.

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
  Solo `rFirma`, el aviso y el menú; ni botón partido ni pestañas, porque esas
  vistas no son de ningún documento. Mismo alto, misma raya, mismo menú. En
  Linux, la barra de título GTK sin botón partido y sin tira.

### Geometría

En Windows y macOS:

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
- El botón de aviso mide 32×30 px, con `--rf-radius-md`, 2 px de aire hasta el
  ☰ y el triángulo a 16 px.

En Linux, lo que dibuja el lienzo para imitar la barra de GTK:

- Barra de 47 px en el gris de la headerbar, que el lienzo saca de los tokens:
  8 % de `--rf-text` sobre `--rf-bg` (en claro da el `#ebebeb` de Adwaita). En
  la aplicación ese gris lo pone el tema GTK, no los tokens.
- Botón partido de 34 px de alto, «Abrir PDF…» a 14 px y peso 700, flecha de
  30 px; el aviso y el ☰, de 34×34.
- La tira de pestañas mide 38 px, con el mismo gris y la raya inferior en
  `--rf-border-subtle`. Sin tira, la raya la lleva la barra.
- Los menús son los de GTK: pico hacia el botón, esquinas redondeadas, filas de
  una línea y sin iconos.

### Los iconos

`<svg>` en línea sobre lienzo `0 0 24 24`, trazo en `currentColor`, extremos y
uniones redondeados. No hay biblioteca de iconos.

- Tres rayas del botón: 18 px, trazo 1.5, `d="M4 7h16M4 12h16M4 17h16"`.
- **Enlace externo**, 14 px, trazo 1.8, en `--rf-text-muted`:
  `d="M14 4h6v6"`, `d="M20 4 11 13"`,
  `d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"`.
- **Triángulo de aviso**, 16 px, trazo 1.6, en `--rf-text`:
  `d="M12 4 2.5 20h19z"`, `d="M12 10v4M12 17v.5"`. Es el mismo `path` que
  «Atención» en el [panel de estado](panel-de-estado.md). En Linux el botón
  lleva el icono simbólico de advertencia del tema (`dialog-warning-symbolic`).

## El menú

Cuatro entradas en dos grupos:

- Estado de rFirma
- ────
- Preferencias…
- Comentarios y ayuda — con el icono de enlace externo, salvo en Linux
- Acerca de rFirma

En Linux es el menú GTK estándar, con las dos secciones y sin icono de enlace
externo, y **F10** lo abre. En macOS estas entradas viven en el menú de la
aplicación de la barra del sistema.

Arriba lo de **esta instalación**; abajo el grupo de siempre de la GNOME HIG, en
su orden. No hay barra de menús
([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)).

**Lo que no está en el menú:** abrir un documento (lo hace el botón partido),
guardar (el pie del panel), paginación y zoom (la píldora del
[visor](visor-de-documento.md)), atajos y guía rápida (no existen).

### El aviso

Cuando hay algo que **rFirma puede y debe arreglar**, aparece un **botón de
aviso** propio a la izquierda del ☰ —en macOS, en el extremo derecho de la
barra—. Lo encienden dos cosas y solo dos:

- **El certificado de rFirma ausente o a medias** en los navegadores.
- **`Sin configurar`** en `Firma en sedes`.

No lo encienden: que las sedes abran AutoFirma (es una elección), «No aplica»,
`Certificados de firma electrónica: Ninguno` (no lo arregla rFirma) ni una
versión nueva (eso lo dice la franja de la
[ventana principal](ventana-principal.md)).

La fila del panel **informa**; el botón **llama**. Por eso hay «Atención» que no
lo encienden.

- **Solo existe cuando hay problema.** Con todo en orden no hay botón, ni hueco.
- **Lleva al panel «Estado de rFirma»**, y dentro del panel se oculta: ya estás
  donde te llevaría.
- Icono de advertencia en **el color del texto**, nunca en un color de alerta;
  sin número ni contador. La marca es la silueta.
- Nombre accesible y tooltip: **«Estado de rFirma: requiere atención»**.
- Ni el ☰ ni la entrada «Estado de rFirma» llevan marca.

Es igual en los tres escritorios; solo cambia el icono (el del tema en Linux).

### El foco por teclado

La entrada enfocada lleva el anillo del sistema —2 px en `--rf-focus-ring`, 2 px
de desplazamiento— y fondo `--rf-surface`. Forma y color, no color solo.

## Estados

En el artboard `Main`, y la palanca «Escritorio» en todos los que estampan
`_cabecera.part`:

- **Escritorio**: Linux (por defecto), Windows o macOS.

- **Con documentos** (cualquier posición de «Estado» salvo la última) o **sin
  documentos** («sin pestañas · Preferencias»), las dos variantes.
- Palancas «Menú de la cabecera»:
  - **Cerrado** (por defecto), en cualquier estado de la ventana.
  - **Abierto**: en Windows el botón se rellena con `--rf-primary` /
    `--rf-on-primary` y el menú flota anclado a la derecha, sobre el contenido;
    en Linux, el menú GTK bajo el ☰; en macOS, el menú de la aplicación en la
    barra del sistema.
  - **Aviso**: con algo que revisar, el botón de aviso; con todo en orden, nada.
  - **Foco** (Windows): sin foco, en la primera entrada, o en «Comentarios y
    ayuda».
- «Abiertos recientemente: desplegados» enseña en Linux el menú GTK de los
  recientes.

La barra la estampa `_cabecera.part` en todos los artboards que pintan la
ventana principal, con o sin documentos; lleva las dos cabeceras, la de GTK y la
de 44 px.

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
- **En Linux, la barra de título nativa de GTK** (30/09/2026). Se descartó la
  barra HTML sobre una ventana sin decorar haciendo de barra de título: los
  botones de ventana dibujados en HTML nunca serían los del tema. La medición y
  el prototipo están en `docs/research/barra-de-titulo-en-linux.md` y en la rama
  `prototype/linux-native-titlebar`.
- **Todo Linux, no solo GNOME.** Se descartó montar la barra GTK solo en GNOME
  detectando el escritorio: en KDE la headerbar de GTK funciona como en
  cualquier aplicación GTK, y detectarlo añadía un tercer modo de cabecera a
  cambio de nada.
- **El menú GTK, sin icono de enlace externo.** Es el menú estándar, sin
  iconos; en Windows «Comentarios y ayuda» lo conserva.
- **El aviso, un botón y no una marca** (30/09/2026). Se descartó el triángulo
  en el ☰ y en la entrada «Estado de rFirma»: el menú GTK estándar va sin
  iconos, y un botón propio lleva al panel de un clic y se comporta igual en los
  tres escritorios.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`; la barra única, el 27/09/2026;
la barra de título en Linux y el botón de aviso, el 30/09/2026.
