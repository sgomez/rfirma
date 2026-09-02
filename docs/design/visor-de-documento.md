# Visor de documento

Columna central. Enseña el PDF y es donde se **coloca** el recuadro de la firma
visible arrastrándolo sobre la página. Responde a «cómo va a quedar».

## Casos de uso que la usan

- Firmar un PDF en local — desde que se abre el documento hasta que se guarda.

## Estructura

Superficie de ancho flexible sobre `--rf-surface`, con `overflow: hidden`:

- **La hoja**, centrada, proporción A4 (1 : 1,414), fondo `--rf-bg` forzando
  `data-theme="light"` y `--rf-shadow-card`. El papel es papel: no cambia con
  el tema.
- **El recuadro de firma**, superpuesto sobre la hoja en posición libre.
- **La barra flotante**, anclada al pie a `--rf-space-md` del borde.

El área reserva 88 px inferiores para que la hoja nunca quede debajo de la
barra.

### Geometría

- La hoja lleva borde de 1 px en `--rf-border-subtle`, `--rf-radius-sm` y
  `--rf-shadow-card`.
- **Zona de soltar del estado vacío**: 520 × 300 px, borde de **2 px**
  discontinuo en `--rf-border-strong` y `--rf-radius-xl` —no `lg`—, con 48 px
  (`--rf-space-lg`) de relleno y 16 px entre sus tres piezas. Dentro, de arriba
  abajo: el icono de subir de 28 px teñido con `--rf-text-muted`, el texto en
  `.rf-title` centrado y la línea de apoyo «Se abrirá el explorador de
  archivos» en `.rf-prose rf-text-muted`. Debajo de la caja, a 24 px, la línea
  de privacidad, también en `.rf-prose rf-text-muted`. Las dos piezas son una
  pila **centrada en el visor**, vertical y horizontalmente: el hueco sobra por
  igual arriba y abajo, no queda todo debajo de la caja.
- Es la única zona de soltar con borde de 2 px y radio `xl`; la de la
  [bandeja](bandeja-de-documentos.md) es de 1 px y radio `md`. La diferencia es
  deliberada: una es la entrada principal de la pantalla vacía y la otra un
  atajo permanente en una columna estrecha.
- **Barra flotante**: píldora con 4 px de relleno, 2 px entre botones, borde de
  1 px en `--rf-border-subtle` y `--rf-shadow-elevated`. Cada botón es un
  **círculo de 32 px** con su icono de 16 px dentro. El divisor entre los dos
  grupos es una línea de 1 × 24 px en `--rf-border-subtle` con 4 px de margen.
- **Asa del recuadro**: pastilla en `--rf-primary` **alineada al borde
  izquierdo** del recuadro (a −2 px, no centrada), con 3 px de relleno
  vertical y 6 px de horizontal, el rótulo a 8 px en peso 700 y la cruz de
  cuatro puntas de 14 px a 4 px del rótulo.
- **Número de página**: pastilla de **56 × 30 px** con `--rf-radius-sm` y el
  número a 13 px en peso 700 — más apretada que un `.rf-input` corriente, que
  mide 44 px de alto y no cabe dentro de una barra de 40. El «de 27» que la
  sigue va en `.rf-body rf-text-muted`, y el porcentaje del zoom ocupa 44 px
  como mínimo, también en peso 700. La barra entera lleva `white-space: nowrap`:
  es una sola línea y el «de 27» no parte nunca.
  **El ancho es la única medida de la barra que no es la del canvas** (ID-44):
  el artboard dibuja un `<span>` de 34 px que solo muestra el número, y aquí es
  un `<input>` en el que se escribe, con su cursor y sitio para tres cifras sin
  que el número baile al pasar de 9 a 100. El alto sí es el suyo, 30 px. Y la
  pastilla va lisa como en el canvas: las flechas de la plataforma se apagan
  con `appearance: textfield` —no caben en 30 px— y de página se cambia con los
  cuatro botones de la barra, que es el gesto que el artboard dibuja. El campo
  sigue siendo `type="number"`, así que el teclado no pierde nada.

### Dentro del recuadro va el sello de verdad (enmienda al ID-44)

El ID-44 decidió dejar el recuadro **vacío** con un argumento correcto para lo
que se sabía entonces: maquetar el sello en HTML sería una imitación local del
compositor autoritativo, y prometería un encuadre que iText no va a respetar. El
sondeo [#115](https://github.com/sgomez/rfirma/issues/115) desactivó la premisa,
no la conclusión: **no hay que maquetar nada**. Un ciclo trifásico en seco con un
`PK1` inventado produce bytes visibles **idénticos** a los del firmado de verdad,
y `pdf.js` de fábrica los pinta. El sello que se ve lo dibuja quien lo dibujará,
no una copia nuestra. Medido en
[prefirma en seco con pdf.js](../research/prefirma-en-seco-pdfjs.md).

Desde v0.3, entonces, **dentro del recuadro va el sello**, con la condición que
sostiene la promesa: **o es el sello de verdad, o no hay recuadro**. No hay
estado intermedio en el que se enseñe una aproximación.

- **Sin certificado no hay recuadro.** El bloque entero de firma visible del
  [panel de firma](panel-de-firma.md) está apagado hasta que hay con qué firmar,
  y la hoja se ve limpia. Es la pieza que hace innecesario decidir qué enseñar
  «mientras tanto»: no hay mientras tanto. Se descartaron por el camino las tres
  respuestas a esa pregunta —recuadro vacío, la rúbrica sola, un texto de
  ejemplo atenuado—, y con ellas la pregunta.
- **El recuadro no se recalcula durante el gesto.** Cuesta 0,15 s en un PDF
  normal, y 1,9 s con 507 MB de RSS en un escaneado de 37 MB: recalcular por
  fotograma está descartado. Mientras se arrastra o se redimensiona, la vista
  anterior **se congela y se atenúa** — sigue sirviendo para medir el bulto, y el
  atenuado dice que aún no es la definitiva.
- **Al soltar se recalcula sola**, salvo en documentos grandes: por encima del
  umbral medido pasa a pedirse con un botón, «Ver cómo queda». Un documento de
  2,4 MB va solo; el escaneado de 37 MB del sondeo, no.
- **Si no se puede dibujar, se dice y se firma igual.** La vista previa **no es
  una puerta**: el recuadro enseña que no ha podido componerse, y sobre si se
  puede firmar manda el botón de firmar, no este recuadro. Los tres fallos
  posibles —documento con contraseña, PDF/A, el puente— **siguen sin medir**.
- **Por debajo del tamaño mínimo** el nombre y la fecha ya no caben y queda la
  rúbrica sola. Ese punto es justamente donde se paran los tiradores: en vez de
  recortar el texto en silencio, el gesto se detiene.
- **Con varias páginas se pide una sola prefirma.** El widget replicado es
  idéntico en todas, así que dibujar uno es dibujarlos todos.

El recuadro sigue siendo **translúcido**, y por la misma razón de antes: opaco
escondería el trozo de documento sobre el que se está decidiendo.

Dos cosas que esto le exige a la implementación y que no se ven en pantalla: hay
que **empaquetar `standard_fonts` de `pdfjs-dist`** y pasarle
`standardFontDataUrl` a `getDocument`, o aceptar por escrito que el corte de
línea es aproximado; y la vista previa **depende de que `annotationMode` siga en
su valor por omisión**, que hoy no vigila ninguna guardia.
## La barra flotante

Una sola pieza, en píldora elevada, con dos grupos separados por un divisor:

```
« ‹ [3] de 27 › »  │  − 100 % + ⤢
```

Los siete botones son `<svg>` **en línea** copiados del artboard (ID-53), no
los glifos tipográficos que insinúa el esquema de arriba: dobles y simples
chevrones para las páginas, menos y más para el zoom, y las cuatro esquinas
para «ajustar a la ventana». Todos sobre lienzo `0 0 24 24` con trazo de 1.5.

- **Páginas**: primera, anterior, número editable, total, siguiente, última.
  Ocupa lo mismo con 4 páginas que con 400 — por eso no hay una pastilla por
  página.
- **Zoom**: alejar, porcentaje, acercar, ajustar a la ventana.

El zoom no es un extra: para colocar el recuadro con precisión hay que
acercarse, así que forma parte de colocar la firma. Por eso va en la misma
barra y no en un menú *Ver*.

## El recuadro de firma

Rectángulo con borde de 2 px en `--rf-border-strong` y `--rf-radius-sm`, sobre
un `--rf-surface` **translúcido** (ver arriba). Al estar
seleccionado muestra cuatro tiradores en las esquinas y un asa etiquetada
«Arrastra para colocar» sobre el borde superior.

La posición es **libre**: no hay rejilla de nueve posiciones. Dónde va la firma
se decide mirando el documento —normalmente bajo el nombre de la persona— y no
eligiendo una casilla abstracta. Ver
[ADR-0006](../adr/0006-firma-visible-se-configura-sobre-el-documento.md).

Qué **dirá** el recuadro lo eligen las casillas del
[panel de firma](panel-de-firma.md), pero se lee **aquí**, dentro del recuadro:
el sello que se ve es el que se va a estampar. Ver «Dentro del recuadro va el
sello de verdad» más arriba.

**Desde v0.3 el recuadro se redimensiona por los tiradores.** En v0.1 solo se
movía; nacía con una proporción fija y esa era toda la geometría disponible.
Ahora los cuatro tiradores de las esquinas funcionan, con `Mayús` para mantener
la proporción, y hay un **tamaño mínimo**: aquel por debajo del cual el nombre y
la fecha ya no caben dentro y el sello deja de decir nada. Los tiradores no
bajan de ahí. No hay medidas escritas en el panel; ver
[panel de firma](panel-de-firma.md).

**Los tiradores son cromo, no papel.** Miden 10 px **en pantalla** al 50 %, al
100 % y al 300 %: no escalan con la hoja, porque son la diana del gesto y no
parte del documento. El recuadro sí escala, porque es la hoja.

**El recuadro se guarda en espacio de usuario PDF, no en píxeles de pantalla.**
Los píxeles se derivan en cada pintada, así que el zoom es puramente visual:
acercarse no mueve la firma. Guardado en píxeles, el recuadro se queda clavado
en la pantalla al cambiar el zoom y se desplaza sobre el documento sin que
nadie lo toque.

Convertir ese rectángulo a los `extraParams` de posición de PAdES **no es solo
invertir la matriz del viewport de `pdf.js`**: iText le aplica además una
transformación según la `/Rotate` de la página, así que hay que entregarle la
inversa. La fórmula, la tabla por rotación y las trampas —entre ellas que un
recuadro fuera de página se recorta en silencio— están medidas en
[Del recuadro dibujado con pdf.js a los extraParams de posición de PAdES](../research/coordenadas-recuadro-pades.md).
Léelo antes de implementar esta pantalla: el fallo no da excepción, coloca la
firma en el sitio equivocado.

## La pastilla bajo la hoja

Centrada bajo la página, `--rf-radius-pill`, fondo `--rf-surface`, borde
`--rf-border-strong` y `--rf-shadow-elevated`. Un texto y un botón, y solo tiene
tres caras:

| Cuándo | Texto | Botón |
| --- | --- | --- |
| Nada colocado | «Aún no has colocado la firma» | `Sellar esta página`, primario — o `Colocar el sello aquí` con `Todas las páginas` |
| Colocado, y esta página no está en el conjunto | «Esta página no se sella» | `Sellar esta página`, secundario |
| Colocado, esta página está en el conjunto, opción `Estas páginas` | «Esta página se sella» | `Quitar el sello`, fantasma |

Con `Solo 1 página` o `Todas las páginas` y la página ya sellada, **no hay
pastilla**: no queda nada que ofrecer ahí.

**El botón cambia de texto con la opción.** Con `Todas las páginas` dice
«Colocar el sello aquí» y no «Sellar esta página», porque el conjunto ya está
completo y lo único que falta es el rectángulo: decir «esta» prometería una
página cuando se sellan las 27.

Es el único camino para elegir páginas que no pasa por teclear, y el que hace
que las páginas se elijan **mirándolas**. Pulsar reescribe el campo del panel.

## En qué páginas se dibuja el recuadro

**En todas las del conjunto, idéntico, y en ninguna más.** El widget se replica:
mismo `/Rect`, mismo contenido. De ahí dos consecuencias que la pantalla tiene
que respetar:

- **La página donde se arrastró el recuadro no se dibuja distinta de las demás.**
  Dibujarla distinta inventaría una diferencia que el PDF no tiene. El «ancla»
  sobrevive solo como el número del pie de `Solo 1 página` en el panel.
- **Fuera del conjunto no se dibuja nada.** Ni un recuadro a trazos: un fantasma
  insinúa que ahí hay algo, que es exactamente la mentira que este diseño existe
  para evitar. Lo dice la pastilla, con palabras.

## Estados

- **Vacío** (sin documento): en lugar de la hoja, la zona de soltar de
  520 × 300 con su icono, «Arrastra un PDF o pulsa para abrirlo» y «Se abrirá
  el explorador de archivos»; debajo, «Solo PDF. El documento no sale de tu
  ordenador en ningún momento». La barra flotante no aparece, y el
  [panel de firma](panel-de-firma.md) tampoco está montado.
- **Sin certificado**: la hoja se ve limpia, sin recuadro y sin pastilla debajo.
  El bloque de firma visible del panel está apagado, así que no hay nada que
  colocar todavía.
- **Documento cargado, sin colocar**: hay certificado, pero ninguna página
  sellada, así que no hay recuadro en ninguna. Bajo la hoja, la pastilla «Aún no
  has colocado la firma» con su botón.
- **Documento cargado, recuadro sin seleccionar**: sin tiradores ni asa.
- **Configurando**: recuadro seleccionado, con tiradores y asa.
- **Página fuera del conjunto**: la hoja se ve **en blanco**, sin recuadro ni
  fantasma, y bajo ella la pastilla «Esta página no se sella · Sellar esta
  página».
- **Moviendo o redimensionando**: el contenido del recuadro se atenúa y se
  queda congelado en la última vista calculada; el borde y los tiradores no.
- **Recalculando**: igual de atenuado, con la etiqueta del asa diciendo
  «Calculando…».
- **Sin vista previa**: el recuadro conserva su borde y sus tiradores, y dentro
  lleva un icono de aviso en lugar del sello. Firmar sigue disponible.
- **Atenuado**: bajo cualquier diálogo, la hoja baja a `opacity: .45`.
- **Firmado**: el recuadro pierde tiradores y asa; ya no se mueve.

## Componentes y tokens

Maquetación propia con `var(--rf-*)`. `--rf-shadow-elevated` en la barra,
`--rf-radius-pill`, `--rf-border-subtle`, `--rf-space-md` de separación al
borde inferior.

## Cómo conviven los dos arrastres

**No hay pan por arrastre, y por eso no hay conflicto.** El documento se
desplaza con la barra de desplazamiento y con la rueda —lo que el WebView ya
hace solo—, así que el arrastre del ratón es **siempre** del recuadro. Esto
resuelve lo que la ficha dejaba abierto: con el zoom al 300 % el recuadro puede
ocupar casi todo el visor y da igual, porque desplazar el documento no depende
de que quede superficie libre donde agarrarlo. Un pan por arrastre habría hecho
falta reservarlo a la barra espaciadora o al botón central, y eso es un gesto
que hay que descubrir; una barra de desplazamiento se ve.

El recuadro se puede mover también **con las flechas** —diez veces más rápido
con `Shift`—, que es el camino de quien no usa ratón y de quien quiere ajustar
un punto exacto. Es el mismo camino: pasa por la misma guardia de página.

**Soltar el recuadro fuera de la página no se acepta.** Aparece el aviso «El
recuadro se ha quedado fuera de la página, así que sigue donde estaba» y el
recuadro vuelve a donde estaba. Es la mitad de interfaz del ID-22; la
autoritativa está en el backend, justo antes de firmar, porque iText recortaría
en silencio y la firma saldría válida igual con la rúbrica encogida.

**La firma NO va en la página que estás mirando, y el recuadro no te sigue.**
Esto cambió en v0.3 ([#152](https://github.com/sgomez/rfirma/issues/152)): el
recuadro nace de un arrastre y ese arrastre lo fija a una página concreta.
Cambiar de página no se lo lleva consigo — se queda donde se puso. Para llevarlo
a la página que tienes delante, se vuelve a arrastrar o se usa la pastilla.

**Las páginas donde el recuadro no cabe no se bloquean.** Se avisan una sola
vez, en el [diálogo de páginas sin sello](dialogo-paginas-sin-sello.md), justo
antes de firmar.

## Sin tira de miniaturas

**No la hay, y no es una deuda.** La barra flotante lleva el número de página
**editable** más primera y última: con 400 páginas se escribe el número y se
llega en un gesto, que es menos que arrastrar una tira. Y una tira de
miniaturas es **una cuarta columna**, justo lo que el ID-25 fija en tres. Si
algún día se demuestra que hace falta, entra como panel superpuesto sobre el
visor y no como región nueva.

**v0.3 lo volvió a mirar y lo confirmó.** Al diseñar el multipágina se prototipó
una tira de miniaturas bajo la hoja donde se marcaban las páginas a sellar, con
el recuadro pintado sobre cada una. Se descartó por lo mismo de siempre —a las
200 páginas es un desplazador que hay que recorrer— y el conjunto se escribe en
el panel, en formato de impresión. Lo que la tira hacía gratis, enseñar que el
recuadro cae en el mismo sitio en todas, lo dice ahora una frase.

## Decisiones

Las pintadas de `pdf.js` pasan por una **cola que cancela la anterior** al
cambiar el zoom o la página. Sin ella dos `RenderTask` escriben sobre el mismo
lienzo y queda una mezcla de dos escalas. Y el arrastre del recuadro **no pasa
por el estado** hasta que se suelta: durante el gesto solo cambia el
`transform` del elemento. Las dos cosas son mecánica, no aspecto, pero se
apuntan aquí porque son lo que hace que esta pantalla se sienta como se ve.

La paginación empezó como una pastilla por página bajo la hoja. Se cambió por
la barra flotante al comprobar que con 27 páginas ya no cabe, y de paso dejó de
colgar del documento para pertenecer al visor, que es a lo que pertenece.

Lo que va **dentro** del recuadro se dibujó primero en dos artboards de trabajo
aparte, y **se fundió en «5 · Colocando la firma visible»** en cuanto se decidió:
dos sitios donde mirar la misma pantalla son dos fuentes de verdad. Vive en su
palanca «Vista previa». El acuerdo que lo simplificó todo —sin certificado, el
bloque apagado— llegó mirándolos: con el recuadro fuera de escena, las tres
alternativas de qué enseñar dentro dejaron de ser una decisión.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards «1 · Vacío» y «5 · Colocando la firma
visible». Los tiradores, la pastilla bajo la hoja y el comportamiento al cambiar
de página se validaron el 02/09/2026 con el
[#155](https://github.com/sgomez/rfirma/issues/155), en el artboard «5», que se
puede pulsar: la palanca «zoom» recorre 50 %, 100 % y 300 %, y la palanca
«tamaño» enseña el mínimo útil.
