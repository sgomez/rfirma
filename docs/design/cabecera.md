# Cabecera

La franja superior de la ventana: identidad, estado del documento y el único
menú de la aplicación.

## Casos de uso que la usan

- Firmar un PDF en local — presente en los diez estados.
- El panel de estado y el menú de la cabecera
  ([#659](https://github.com/sgomez/rfirma/issues/659)) — es la puerta: el
  [panel de estado](panel-de-estado.md) se abre desde su menú, y el menú avisa
  cuando hay algo que mirar ahí.

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
  `--rf-radius-sm`. El divisor es un `.rf-divider` con 4 px de aire arriba y
  abajo.
- **Toda entrada reserva a su derecha una columna de 14 px**, lleve icono o no.
  Es lo que mantiene alineado el texto de las cuatro: una columna que sólo
  aparece cuando hay algo dentro desplaza las demás entradas.

### Los iconos

Las tres rayas del botón son un `<svg>` **en línea**, copiado del artboard
(ID-53): 20×20 px sobre lienzo `0 0 24 24`, trazo de 1.5 en `currentColor`,
extremos y uniones redondeados, `d="M4 7h16M4 12h16M4 17h16"`. No hay
biblioteca de iconos ni icono de fuente, y el `☰` de texto que hubo antes ya
no está: un glifo tipográfico cambia de forma con la fuente instalada.

Los dos iconos del menú salen de la misma cantera y con la misma receta, a 14 px
y trazo 1.8:

- **Enlace externo**, en `--rf-text-muted`, sobre «Comentarios y ayuda»:
  `d="M14 4h6v6"`, `d="M20 4 11 13"`,
  `d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"`.
- **Triángulo de aviso**, en `--rf-text`, sobre «Estado de rFirma»:
  `d="M12 4 2.5 20h19z"`, `d="M12 10v4M12 17v.5"`. Es **el mismo `path`** que
  dibuja «Atención» en el [panel de estado](panel-de-estado.md).

## El menú

Botón de 40 px que despliega un menú con **cuatro entradas en dos grupos**,
separados por un divisor:

- Estado de rFirma
- ────
- Preferencias…
- Comentarios y ayuda — con el icono de enlace externo
- Acerca de rFirma

**El divisor no es adorno.** Sin él las cuatro se leen como una lista del mismo
rango y «Estado de rFirma» deja de ser lo primero: arriba va lo que habla de
**esta instalación**, y abajo el grupo de siempre —preferencias, ayuda, acerca
de— en su orden.

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

### El aviso

Cuando hay algo que **rFirma puede y debe arreglar**, «Estado de rFirma» lleva
el triángulo a la derecha. Dice «entra a mirar», y nada más.

**Lo encienden dos cosas, y solo dos**
([#661](https://github.com/sgomez/rfirma/issues/661)):

- **El certificado de rFirma ausente o a medias** —`0 de 3 almacenes` o
  `2 de 3`—, que es una instalación sin terminar.
- **`Sin configurar`** en `Firma en sedes`: nadie atiende los enlaces de las
  sedes, así que una firma que empiece en una no va a llegar a ninguna parte.

**No lo enciende nada más.** En concreto:

- **Que las sedes abran AutoFirma no es una avería, es una elección legítima.**
  La fila del [panel](panel-de-estado.md) sigue diciendo «Atención» cuando entras
  a mirar —hay algo que se puede cambiar—, pero no sale a buscarte por ello.
- **«No aplica» no llama a nadie**: ni el certificado apagado porque firma
  AutoFirma, ni `No se puede consultar` dentro del flatpak. No saber quién firma,
  o saber que el certificado no hace falta, no es tener algo que reparar.
- **`Certificados de firma electrónica: Ninguno` tampoco.** Instalar un certificado propio es
  cosa de la FNMT o de quien lo emita, no de rFirma; el panel dice `Cómo
  instalar` y ahí se acaba lo que puede hacer.
- **Una versión nueva tampoco.** Eso lo dice la franja de la
  [ventana principal](ventana-principal.md), con su interruptor
  `Avisarme cuando haya una versión nueva`, y no hay ningún otro aviso del
  escritorio: el triángulo no es un segundo canal para lo mismo.

**El veredicto de la fila y el disparo del triángulo dejan de ser la misma
regla**: la fila **informa** de lo que hay, el triángulo **llama** para que
vengas. Por eso hay «Atención» que no lo encienden: que haya un gesto disponible
no es lo mismo que haga falta darlo.

La marca es sobria a propósito:

- **Sin número ni contador.** Contar aquí obligaría a mantener dos fuentes de
  verdad sobre lo mismo, y la verdad está en la tabla del
  [panel](panel-de-estado.md).
- **La marca es la silueta, no el color**: el triángulo va en `--rf-text`, igual
  que el texto de la entrada.
- Ocupa la **misma columna de 14 px** que el icono de enlace externo, así que
  aparecer o desaparecer no mueve nada.

### El foco por teclado

Cada entrada enfocada lleva **dos indicadores**: el anillo del sistema —2 px en
`--rf-focus-ring` con 2 px de desplazamiento, los tokens de la sección 8 del
[sistema de diseño](design-system.md)— y el fondo en `--rf-surface`. Forma y
color, no color solo.

## Estados

- **Sin documento**: solo el nombre y el botón de menú.
- **Con documento sin firmar**: insignia `Sin firmar`.
- **Documento firmado**: insignia `Firmado` en `--rf-primary`.
- **Menú abierto**: el botón se rellena con `--rf-primary`; el menú flota con
  `--rf-shadow-elevated` anclado a la derecha.
- **Menú con aviso**: «Estado de rFirma» con el triángulo, y solo por lo que
  rFirma puede arreglar —certificado ausente o a medias, o nadie atendiendo los
  enlaces de las sedes—. Es independiente de todo lo demás: el documento puede
  estar firmado y la instalación coja, y al revés, el panel puede tener una fila
  en «Atención» sin que el menú diga nada.

El menú **arranca cerrado**. El artboard «1 · Vacío · menú abierto» lo dibuja
desplegado para enseñar sus cuatro entradas, pero eso es una posibilidad y no el
estado inicial: abrir la aplicación con un menú encima del documento no es lo
que el canvas pide.

## Componentes y tokens

`.rf-title`, `.rf-prose`, `.rf-row`, `.rf-gap-xs`, `.rf-divider`, `.rf-badge`,
`.rf-badge--primary`, `--rf-surface`, `--rf-border-subtle`, `--rf-primary`,
`--rf-on-primary`, `--rf-text`, `--rf-text-muted`, `--rf-shadow-elevated`,
`--rf-focus-ring`, `--rf-radius-md`, `--rf-radius-sm`.

## Decisiones

La barra de menús clásica (`Archivo · Ver · Ayuda`) se dibujó primero y se
retiró: GNOME la abandonó, Windows 11 no la usa y en macOS Tauri registra un
menú nativo en la barra del sistema, de modo que dibujarla dentro de la ventana
la duplicaría. Fundirla con la barra de aplicación devolvió además 30 px de
alto al [panel de firma](panel-de-firma.md), que iba justo.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «1 · Vacío · menú abierto».

**Las cuatro entradas del menú** —decididas en el
[#656](https://github.com/sgomez/rfirma/issues/656) y dibujadas el 17/09/2026 en
el [#659](https://github.com/sgomez/rfirma/issues/659)— se dibujaron aquí y no
en la ficha de [Preferencias](preferencias.md) ni en la del
[panel de estado](panel-de-estado.md) por una razón material: el menú es de la
cabecera, y la cabecera es la misma esté delante una vista del cuerpo o
ninguna. Con Preferencias o Estado de rFirma abiertos la cabecera se queda
intacta arriba, así que el menú que la abre se ve y se usa igual, y
documentarlo dos veces más sería repetir la misma fila.

**«Estado de rFirma», y no «Estado» a secas.** En esta misma franja vive la
insignia del documento, así que «Estado» se leería como estado del documento.

**El aviso es una silueta y no un contador**, ni un punto de color. Un número
obligaría a contar en dos sitios —el menú y la tabla del panel— y a decidir qué
entra en la cuenta; un punto de color sería el único indicador, que es
exactamente lo que la sección 8 del [sistema de diseño](design-system.md)
prohíbe. El triángulo reutiliza el `path` de «Atención» del
[panel de estado](panel-de-estado.md): dos dibujos distintos para lo mismo serían
dos vocabularios.
