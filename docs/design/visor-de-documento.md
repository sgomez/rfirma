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
  de privacidad, también en `.rf-prose rf-text-muted`.
- Es la única zona de soltar con borde de 2 px y radio `xl`; la de la
  [bandeja](bandeja-de-documentos.md) es de 1 px y radio `md`. La diferencia es
  deliberada: una es la entrada principal de la pantalla vacía y la otra un
  atajo permanente en una columna estrecha.
- **Barra flotante**: píldora con 4 px de relleno, 2 px entre botones, borde de
  1 px en `--rf-border-subtle` y `--rf-shadow-elevated`. Cada botón es un
  **círculo de 32 px** con su icono de 16 px dentro. El divisor entre los dos
  grupos es una línea de 1 × 24 px en `--rf-border-subtle` con 4 px de margen.
- **Asa del recuadro**: pastilla en `--rf-primary` sobre el borde superior, con
  la cruz de cuatro puntas de 14 px a 4 px del rótulo.

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

Rectángulo con borde `--rf-border-strong` sobre `--rf-surface`. Al estar
seleccionado muestra cuatro tiradores en las esquinas y un asa etiquetada
«Arrastra para colocar» sobre el borde superior.

La posición es **libre**: no hay rejilla de nueve posiciones. Dónde va la firma
se decide mirando el documento —normalmente bajo el nombre de la persona— y no
eligiendo una casilla abstracta. Ver
[ADR-0006](../adr/0006-firma-visible-se-configura-sobre-el-documento.md).

Qué se ve dentro del recuadro lo controla el
[panel de firma](panel-de-firma.md).

**En v0.1 el recuadro se mueve, no se redimensiona.** Nace con una proporción
fija —un tercio del ancho de la página, alto de rúbrica— y se coloca
arrastrándolo, que es lo que decide dónde va la firma. Los cuatro tiradores de
las esquinas son de la vuelta siguiente: redimensionar es un segundo gesto y
cuatro dianas más sobre el mismo elemento, y cambiar el tamaño sin ver todavía
el contenido —lo pone el panel de firma— es decidir a ciegas.

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

## Estados

- **Vacío** (sin documento): en lugar de la hoja, la zona de soltar de
  520 × 300 con su icono, «Arrastra un PDF o pulsa para abrirlo» y «Se abrirá
  el explorador de archivos»; debajo, «Solo PDF. El documento no sale de tu
  ordenador en ningún momento». La barra flotante no aparece, y el
  [panel de firma](panel-de-firma.md) tampoco está montado.
- **Documento cargado, recuadro sin seleccionar**: sin tiradores ni asa.
- **Configurando**: recuadro seleccionado, con tiradores y asa.
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

**La firma va en la página que estás mirando.** Al cambiar de página el
recuadro va contigo, conservando su sitio sobre el papel; si la página nueva es
más pequeña y el recuadro no cabe, vuelve a su posición por omisión —abajo a la
izquierda— en vez de quedarse a medias fuera.

## Sin tira de miniaturas

**No la hay, y no es una deuda.** La barra flotante lleva el número de página
**editable** más primera y última: con 400 páginas se escribe el número y se
llega en un gesto, que es menos que arrastrar una tira. Y una tira de
miniaturas es **una cuarta columna**, justo lo que el ID-25 fija en tres. Si
algún día se demuestra que hace falta, entra como panel superpuesto sobre el
visor y no como región nueva.

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

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards «1 · Vacío» y «5 · Configurando la firma
visible».
