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

## La barra flotante

Una sola pieza, en píldora elevada, con dos grupos separados por un divisor:

```
« ‹ [3] de 27 › »  │  − 100 % + ⤢
```

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

- **Vacío** (sin documento): en lugar de la hoja, una zona de soltar de
  520 × 300 con borde discontinuo, «Arrastra un PDF o pulsa para abrirlo» y,
  debajo, «Solo PDF. El documento no sale de tu ordenador en ningún momento».
  La barra flotante no aparece.
- **Documento cargado, recuadro sin seleccionar**: sin tiradores ni asa.
- **Configurando**: recuadro seleccionado, con tiradores y asa.
- **Atenuado**: bajo cualquier diálogo, la hoja baja a `opacity: .45`.
- **Firmado**: el recuadro pierde tiradores y asa; ya no se mueve.

## Componentes y tokens

Maquetación propia con `var(--rf-*)`. `--rf-shadow-elevated` en la barra,
`--rf-radius-pill`, `--rf-border-subtle`, `--rf-space-md` de separación al
borde inferior.

## Lo que esta pantalla deja abierto

**Dos arrastres conviven en el visor**: mover el documento (pan) y mover el
recuadro. Lo natural es que arrastrar sobre el recuadro lo mueva a él y sobre
el resto de la página desplace el documento, pero con zoom alto el recuadro
puede ocupar casi todo el visor y el pan desaparece. La salida habitual es
reservar el pan a la barra espaciadora o al botón central del ratón. Es
comportamiento, no aspecto, y se decide al implementar.

**Con muchas páginas** puede costar encontrar la correcta. La siguiente vuelta
sería una tira de miniaturas, que es otra columna: no se ha metido.

## Decisiones

La paginación empezó como una pastilla por página bajo la hoja. Se cambió por
la barra flotante al comprobar que con 27 páginas ya no cabe, y de paso dejó de
colgar del documento para pertenecer al visor, que es a lo que pertenece.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards «1 · Vacío» y «5 · Configurando la firma
visible».
