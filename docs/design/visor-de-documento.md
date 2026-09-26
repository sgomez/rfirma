# Visor de documento

La región izquierda, bajo las pestañas. Enseña el PDF y es donde se coloca la
firma visible. Responde a «cómo va a quedar». Sin documento, es la zona de
soltar.

## Casos de uso que la usan

- Firmar un PDF en local — desde que se abre el documento hasta que se guarda.

## Estructura

Superficie flexible sobre `--rf-bg`, con `overflow: hidden`:

- **La hoja**, centrada, proporción A4 (1 : 1,414), fondo `--rf-bg` forzando
  `data-theme="light"`: el papel no cambia con el tema.
- **La firma visible**, sobre la hoja, solo si está encendida en el
  [panel](panel-de-firma.md) y la página está en su conjunto.
- **El aviso de la vista previa**, flotando sobre la píldora, solo cuando hay
  algo que decir.
- **La píldora** de paginación y zoom, anclada al pie a `--rf-space-md` del
  borde.

Ningún control de la firma visible vive fuera del propio recuadro: el resto
está en el panel.

### Geometría

- Hoja con borde de 1 px en `--rf-border-subtle`, `--rf-radius-sm` y
  `--rf-shadow-card`. En el artboard, 326 px al 100 %, a 40 px del borde
  superior.
- **Píldora**: `--rf-radius-pill`, fondo `--rf-surface`, borde
  `--rf-border-subtle`, `--rf-shadow-elevated`, 4 px de relleno, 2 px entre
  botones. Cada botón es un círculo de 32 px con icono de 16 px en `--rf-text`,
  no en el `--rf-text-muted` de `.rf-btn--ghost`; desactivado, a opacidad 0.45;
  al pasar por encima de uno activado, el círculo toma fondo
  `--rf-border-subtle`. El divisor entre
  los dos grupos es una línea de 1 × 24 px en `--rf-border-subtle`.
- **Número de página**: pastilla de 56 × 30 px con `--rf-radius-sm`, borde
  `--rf-border-strong` y el número a 13 px en peso 700. En la aplicación es un
  `<input type="number">` sin flechas (`appearance: textfield`); el artboard
  dibuja un `<span>` de 34 px. El «de 6» en `.rf-body rf-text-muted`; el
  porcentaje, 44 px mínimo y peso 700. `white-space: nowrap`.
- **Zona de soltar del estado vacío**: 520 × 170 px, borde de 2 px discontinuo
  en `--rf-border-strong`, `--rf-radius-lg`, fondo `--rf-surface`. Dentro, el
  icono de subir de 32 px en `--rf-text-muted`, «Arrastra un PDF o pulsa para
  abrirlo» en `.rf-title` a 16 px y «No sale de tu ordenador» en
  `.rf-body rf-text-muted`. Debajo, a 24 px y al mismo ancho, los recientes
  ([pestañas de documentos](pestanas-de-documentos.md)). El conjunto, centrado
  en el visor.

## Lo que se ve dentro del recuadro es la firma de verdad

El recuadro enseña **la firma visible que se va a estampar**, pintada por quien
la estampará. No se maqueta en HTML: un ciclo trifásico en seco con un `PK1`
inventado produce bytes visibles idénticos a los del firmado de verdad, y
`pdf.js` los pinta ([prefirma en seco con pdf.js](../research/prefirma-en-seco-pdfjs.md),
[ADR-0006](../adr/0006-firma-visible-se-configura-sobre-el-documento.md)). El
mini-render del artboard es el dibujo de ese resultado para cada modelo: Completa
con firmante, fecha y emisor; Solo rúbrica; Personalizada con la frase ya
sustituida; la rúbrica a la izquierda si está encendida.

**O es la firma de verdad, o el recuadro va vacío.** Nunca una aproximación:

- **Sin certificado elegido** —buscando, sin ninguno o sin elegir— no hay firma
  que componer ni recuadro: la firma visible no se enciende hasta elegir uno.
- **Mientras se arrastra o se redimensiona**, la vista anterior se congela y se
  atenúa: recalcular por fotograma cuesta 1,9 s y 507 MB de RSS en un escaneado
  de 37 MB.
- **Al soltar se recalcula sola**, salvo en documentos de más de 8 MB, donde
  pasa a pedirse. El umbral es el tamaño porque se sabe antes de pagar el primer
  ciclo.
- **Si no se puede dibujar, se dice y se firma igual.** La vista previa no es una
  puerta; sobre si se puede firmar manda el botón. Los tres fallos posibles
  —contraseña, PDF/A, el puente— siguen sin medir.
- **Con varias páginas se pide una sola prefirma**: el widget replicado es
  idéntico en todas.

**No hay fuente, tamaño ni color que elegir**: el texto se ajusta al recuadro, y
al hacerlo más pequeño se reduce. Hay un **tamaño mínimo**, donde se paran los
tiradores.

Lo que exige a la implementación: `standard_fonts` de `pdfjs-dist` copiadas a
`dist/standard_fonts/` por un complemento de `vite.config.ts` y pasadas en
`standardFontDataUrl`; y `annotationMode` en su valor por omisión, vigilado por
`viewer/pdfjsLoader.test.ts`, que se pone rojo si alguien lo escribe.

## El aviso de la vista previa

Flota sobre la píldora, centrado: `--rf-radius-pill`, fondo `--rf-surface`,
borde `--rf-border-strong`, `--rf-shadow-elevated`, 40 px de alto mínimo. Habla
de la vista previa de la firma visible y de nada más:

| Cuándo | Texto | Botón | El recuadro |
| --- | --- | --- | --- |
| Al día | — (no hay aviso) | — | la firma |
| Documento grande, sin recalcular | «Documento grande: la firma visible no se recalcula sola» | `Ver cómo queda` | la firma anterior al 35 % |
| No se ha podido dibujar | «No se ha podido dibujar la firma visible» | `Volver a intentarlo` | marco y tiradores, con un icono de información dentro |

Moviendo o redimensionando no lleva aviso: el atenuado ya lo dice mientras dura
el gesto.

**Mide lo mismo tenga botón o no**: altura fija y el hueco del botón reservado,
para que no salte bajo la hoja mientras se coloca. No dice nada de colocación:
eso lo dice el panel.

## La píldora

```
⏮ ‹ [6] de 6 › ⏭  │  − 100 % + ⤢
```

Iconos `<svg>` en línea sobre lienzo `0 0 24 24`, trazo 1.5 (ID-53), salvo los
cuatro chevrons de paginación, a trazo 2. El − y el + del zoom siguen a 1.5.

- **Páginas**: primera, anterior, número editable, total, siguiente, última.
  Ocupa lo mismo con 4 páginas que con 400.
- **Zoom**: alejar, porcentaje editable, acercar, ajustar a la ventana.

El zoom forma parte de colocar la firma, por eso va aquí y no en un menú.

- **Continuo, del 25 % al 400 %** (ID-116). `Ctrl`+rueda amplía anclado al
  puntero —el pellizco del trackpad llega por ahí—; el porcentaje tecleado se
  recorta al rango; `Ctrl+0` vuelve al 100 % con el foco en la hoja o en el
  recuadro. Los botones ± saltan entre siete escalones redondos.
- **Un documento nuevo abre en «ajustar a la página»**, también en apaisado.
- **«Ajustar» es un modo** (ID-117): sobrevive al cambio de página, de tamaño de
  ventana y de documento. **Un zoom fijado a mano también sobrevive** al
  documento siguiente: manda lo último que dijo el usuario. No se recuerda entre
  sesiones.
- **El lienzo no pasa de 4×** (ID-119): en HiDPI se acota `zoom ×
  devicePixelRatio`. Se recorta la resolución, no el zoom, y no se avisa.

## El recuadro

**Mientras se puede editar**: borde de 1,5 px en `--rf-primary`, fondo `--rf-bg`
y cuatro tiradores en las esquinas (9 px en el artboard, `--rf-bg` con borde
`--rf-primary`). **Firmando o firmado**: sin borde ni tiradores; ya no se mueve.
Sin pastilla encima.

- **Posición libre**, sin rejilla de nueve posiciones: la firma va donde lo dice
  el documento, normalmente bajo el nombre de la persona
  ([ADR-0006](../adr/0006-firma-visible-se-configura-sobre-el-documento.md)).
- **Se redimensiona por los tiradores**, con `Mayús` para mantener la
  proporción, y no baja del mínimo.
- **Los tiradores son cromo, no papel**: miden lo mismo en pantalla al 50 %, al
  100 % y al 300 %. El recuadro sí escala, porque es la hoja.
- **Se guarda en espacio de usuario PDF**, no en píxeles: acercarse no mueve la
  firma.
- **Pasar ese rectángulo a los `extraParams` de PAdES no es solo invertir el
  viewport de `pdf.js`**: iText aplica además una transformación según la
  `/Rotate` de la página. Está medido en
  [coordenadas-recuadro-pades.md](../research/coordenadas-recuadro-pades.md);
  el fallo no da excepción, coloca la firma en otro sitio.

## En qué páginas se dibuja

**En todas las del conjunto, idéntico, y en ninguna más**: mismo `/Rect`, mismo
contenido.

- La página donde se trazó **no se dibuja distinta**: el PDF no tiene esa
  diferencia.
- **Fuera del conjunto la hoja se ve limpia**, sin fantasma a trazos. Lo dice el
  panel, con «Ponerla aquí».

## Los tres gestos del recuadro

- **Trazar**: pulsar sobre la hoja y arrastrar hace nacer el recuadro, de esquina
  a esquina. Es el único camino que elige sitio en el mismo gesto; encender la
  firma visible y «Ponerla aquí» usan la posición estándar.
- **Arrastrar**: mover el que ya existe.
- **Redimensionar**: tirar de una esquina.

Trazar es «Ponerla aquí» con sitio elegido: con **Una página** sustituye la
página, con **Varias** la añade y con **Todas** el conjunto ya estaba completo.
Como el PDF lleva un solo campo con el widget replicado, el rectángulo trazado
se mueve en todas las páginas del conjunto.

Reglas del trazo, en orden:

1. Se **normaliza**: vale en cualquier dirección.
2. Lo que sale de la hoja se **recorta al borde**.
3. **Al soltar**, por debajo del mínimo (ID-103) crece hasta él, anclado a la
   esquina donde arrancó. Durante el trazo no, para que no despegue del cursor.
4. Lo que el mínimo saque del papel se **empuja hacia dentro** (ID-22).

**Un clic seco no coloca nada, enfoca la hoja** (ID-113): por debajo de 4 px de
pantalla no hay trazo, y así `AvPág`/`RePág` pasan de página. Durante el trazo se
pinta un rectángulo a trazos; al soltar, el recuadro se lleva el foco.

**Con la firma visible apagada la hoja no traza**, y el cursor lo dice:
`crosshair` solo cuando se puede.

## Arrastrar y desplazar

**No hay pan por arrastre**: el documento se desplaza con la barra y la rueda, y
el arrastre es siempre del recuadro. Con el zoom al 300 % el recuadro puede
ocupar casi todo el visor y da igual.

- **Flechas**: mueven el recuadro un punto de espacio de usuario (ID-115); diez
  con `Mayús`.
- **Dos elementos enfocables** (ID-113): la hoja atiende `AvPág`, `RePág`,
  `Inicio` y `Fin`; el recuadro, las flechas, y las teclas de página burbujean
  desde él. `Esc` devuelve el foco a la hoja. Al cambiar de página, un recuadro
  fuera de la parte visible se trae a ella una vez (ID-118).
- **Ni el zoom ni el redimensionado de la ventana escriben la colocación**
  (ID-114): solo el trazo, el arrastre, el redimensionado y las flechas.
- **Soltar fuera de la página no se acepta**: «El recuadro se ha quedado fuera de
  la página, así que sigue donde estaba», y vuelve. El backend lo comprueba
  otra vez antes de firmar, porque iText recortaría en silencio (ID-22).
- **La firma no sigue a la página que miras**: se queda donde se puso.
- **Las páginas donde el recuadro no cabe** se avisan una vez, antes de firmar,
  en el [diálogo de páginas sin firma visible](dialogo-paginas-sin-firma-visible.md).

## Estados

En el artboard `Main`, palancas «Estado», «Firma visible», «Vista previa de la
firma visible», «Contenido de la firma» y «Visor»:

- **Vacío**: la zona de soltar y los recientes; sin píldora ni panel.
- **Firma visible apagada**: la hoja limpia.
- **Encendida, en la página a la vista**: el recuadro editable.
- **Encendida, en otra página**: la hoja limpia.
- **Arrastrándola**: el recuadro fuera de su sitio, con `--rf-shadow-elevated`, y
  su sitio anterior a trazos al 60 %.
- **Tamaño pequeño**: el mismo contenido con la letra reducida.
- **Recalculando** y **no se ha podido dibujar**: ver el aviso.
- **Sin certificado**: la hoja limpia, sin recuadro.
- **Zoom** 50 %, 100 % y 300 %: la hoja y el recuadro escalan; los tiradores no.
  Al 300 % el artboard enseña la esquina de la firma.
- **Firmando**: la hoja al 45 %, bajo el velo.
- **Firmado** y **error al firmar**: el recuadro sin borde ni tiradores.

## Componentes y tokens

Maquetación propia con `var(--rf-*)`: `--rf-bg`, `--rf-surface`,
`--rf-border-subtle`, `--rf-border-strong`, `--rf-primary`,
`--rf-shadow-card`, `--rf-shadow-elevated`, `--rf-radius-sm|lg|pill`,
`--rf-space-md`.

## Decisiones

- **El mini-render es la firma real**, no un dibujo nuestro (25/09/2026). Main
  v4 D la dibuja con el modelo elegido, y se lee como el resultado del ciclo en
  seco que la aplicación ya pinta: la regla «o es la de verdad, o no hay nada»
  del ADR-0006 sigue en pie.
- **Se conservan «recalculando» y «no se ha podido dibujar»**, que Main v4 D no
  dibujaba: el caso existe igual. «Moviendo» pierde su aviso: era una frase para
  lo que el atenuado ya enseña. El aviso flota sobre la píldora porque en
  flujo, bajo la hoja, se iba con ella al ampliar.
- **Sin pastilla sobre el recuadro ni etiqueta de página**: el recuadro solo
  lleva marco y tiradores; la página la dice el panel.
- **Sin tira de miniaturas.** La propusieron V3 A (columna de 128 px) y V3 B
  (tira bajo la hoja) como navegación y selección de páginas. A las 200 páginas
  es un desplazador que hay que recorrer, y el número editable de la píldora
  llega en un gesto.
- **La paginación es una píldora del visor**, no una pastilla por página: con 27
  páginas ya no cabían.
- Mecánica que hace que la pantalla se sienta como se ve: las pintadas de
  `pdf.js` pasan por una cola que cancela la anterior, y el arrastre no toca el
  estado hasta soltar.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`, el 25/09/2026.
