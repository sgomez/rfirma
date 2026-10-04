# Visor de documento

La región izquierda, bajo las pestañas. Enseña el PDF y es donde se coloca la
firma visible. Responde a «cómo va a quedar». Sin documento, es la zona de
soltar.

**La verdad del dibujo es el código y sus historias:**
`viewer/DocumentViewer.stories.tsx` («Ventana principal/5 · Visor»). Esta ficha
cuenta el flujo y el porqué; los textos salen del catálogo (`viewer.*` en
`po/messages.pot`) y no se copian aquí. Las historias pintan una hoja de
mentira (`viewer/storyPdf.ts`): no hay `pdf.js` en Storybook, y lo que se ve
dentro del recuadro en la aplicación es el PDF que compone el backend.

## Casos de uso que la usan

- Firmar un PDF en local — desde que se abre el documento hasta que se guarda.
- Ver las firmas de un documento con `rfirma verify -i <doc> -gui`: la hoja sin
  recuadro; si el fichero no es PDF (CAdES, XAdES, un formato desconocido), en
  lugar de la hoja, centrados, el icono de documento, el nombre del fichero en
  negrita y `viewer.noPreview`, y sin la píldora.

## Flujo entre estados

| Estado | Historia | Qué se ve |
| --- | --- | --- |
| Vacío | `Empty` | La zona de soltar (`viewer.dropZone`, `viewer.dropZoneHint`); sin píldora ni panel |
| Vacío con recientes | `EmptyWithRecents` | La zona de soltar y, debajo, los [recientes](pestanas-de-documentos.md) |
| Con documento | `WithDocument` | La hoja y la píldora; sin firma visible, la hoja limpia |
| Firma visible en una página | `WithSignatureBox` | El recuadro editable en la página a la vista |
| Firma visible en todas | `SignatureBoxOnAllPages` | El mismo recuadro, replicado en cada página del conjunto |
| Sin vista previa | `WithoutPreview` | El nombre del fichero y la indicación, sin hoja ni píldora |
| El fichero no se deja abrir | `FailureNotAPdf` | El aviso de error en el visor, que sin documento no tiene panel donde ir |
| Fallo con documento delante | `FailureOverDocument` | El aviso y el primer documento intacto, para que el rechazo no sea mudo |

Los estados con certificado, sello compuesto, recalculado o error del sello
dependen del panel y del backend y no tienen historia aquí: los cubre la prueba
de `DocumentViewer` con su doble de `pdf.js`. Quien los quiera ver en Storybook
los añade con la tanda del panel de firma.

## Estructura

Superficie flexible sobre `--rf-bg`, sin desbordar:

- **La hoja**, centrada, de proporción A4, con `data-theme="light"` forzado: el
  papel no cambia con el tema.
- **La firma visible**, sobre la hoja, solo si está encendida en el
  [panel](panel-de-firma.md) y la página está en su conjunto.
- **El aviso de la vista previa**, flotando sobre la píldora, solo cuando hay
  algo que decir.
- **La píldora** de paginación y zoom, anclada al pie del borde.

Ningún control de la firma visible vive fuera del propio recuadro: el resto está
en el panel.

**La zona de soltar** es un recuadro de borde discontinuo con el icono de subir,
la invitación a soltar o abrir y una línea que dice que el documento no sale del
ordenador. Debajo, al mismo ancho, los recientes. El conjunto, centrado en el
visor.

**La hoja** lleva borde, esquinas pequeñas y la sombra de tarjeta. **La píldora**
es del radio de píldora, de superficie elevada, y cada botón es un círculo con el
icono en `--rf-text` —no en el atenuado de `.rf-btn--ghost`—; desactivado, a
opacidad baja. El número de página es un `<input type="number">` sin flechas, y
el porcentaje no se parte de línea. La geometría exacta la dan
`DocumentViewer.css` y las historias.

## Lo que se ve dentro del recuadro es la firma de verdad

El recuadro enseña **la firma visible que se va a estampar**, pintada por quien
la estampará. No se maqueta en HTML: un ciclo trifásico en seco con un `PK1`
inventado produce bytes visibles idénticos a los del firmado de verdad, y
`pdf.js` los pinta ([prefirma en seco con pdf.js](../research/prefirma-en-seco-pdfjs.md),
[ADR-0006](../adr/0006-firma-visible-se-configura-sobre-el-documento.md)).

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

Flota sobre la píldora, centrado, en superficie elevada y de altura mínima fija.
Habla de la vista previa de la firma visible y de nada más. Sus textos son
`viewer.stamp.*`:

| Cuándo | Clave del texto | Botón | El recuadro |
| --- | --- | --- | --- |
| Al día | — (no hay aviso) | — | la firma |
| Documento grande, sin recalcular | `viewer.stamp.onDemand` | `viewer.stamp.show` | la firma anterior, atenuada |
| Recalculando | `viewer.stamp.composing` | — | la firma anterior, atenuada |
| No se ha podido dibujar | `viewer.stamp.failed` | reintentar | marco y tiradores, con un icono de información dentro |

Moviendo o redimensionando no lleva aviso: el atenuado ya lo dice mientras dura
el gesto.

**Mide lo mismo tenga botón o no**: altura fija y el hueco del botón reservado,
para que no salte bajo la hoja mientras se coloca. No dice nada de colocación:
eso lo dice el panel.

## La píldora

De izquierda a derecha, en dos grupos separados por un divisor: **páginas**
—primera, anterior, número editable, total, siguiente, última, con
`viewer.firstPage`, `viewer.previousPage`, `viewer.pageNumber`, `viewer.pageOf`,
`viewer.nextPage` y `viewer.lastPage`— y **zoom** —alejar, porcentaje editable,
acercar y ajustar a la ventana (`viewer.zoomOut`, `viewer.zoomLevel`,
`viewer.zoomIn`, `viewer.fitWidth`, `viewer.fitPage`)—. La barra de páginas
ocupa lo mismo con 4 páginas que con 400.

Iconos `<svg>` en línea sobre lienzo `0 0 24 24`, trazo 1.5 (ID-53), salvo los
cuatro chevrons de paginación, a trazo 2.

El zoom forma parte de colocar la firma, por eso va aquí y no en un menú.

- **Continuo, del 25 % al 400 %** (ID-116). `Ctrl`+rueda amplía anclado al
  puntero —el pellizco del trackpad llega por ahí—; el porcentaje tecleado se
  recorta al rango; `Ctrl+0` vuelve al 100 % con el foco en la hoja o en el
  recuadro. Los botones de acercar y alejar saltan entre siete escalones
  redondos.
- **Un documento nuevo abre en «ajustar a la página»**, también en apaisado.
- **«Ajustar» es un modo** (ID-117): sobrevive al cambio de página, de tamaño de
  ventana y de documento. **Un zoom fijado a mano también sobrevive** al
  documento siguiente: manda lo último que dijo el usuario. No se recuerda entre
  sesiones.
- **El lienzo no pasa de 4×** (ID-119): en HiDPI se acota `zoom ×
  devicePixelRatio`. Se recorta la resolución, no el zoom, y no se avisa.

## El recuadro

**Mientras se puede editar**: borde en `--rf-primary`, fondo `--rf-bg` y cuatro
tiradores en las esquinas (`viewer.dragHandle`). **Firmando o firmado**: sin
borde ni tiradores; ya no se mueve. Sin pastilla encima. Su nombre accesible es
`viewer.signatureBox`.

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
  panel, con su acción de ponerla en la página a la vista.

## Los tres gestos del recuadro

- **Trazar**: pulsar sobre la hoja y arrastrar hace nacer el recuadro, de esquina
  a esquina. Es el único camino que elige sitio en el mismo gesto; encender la
  firma visible y ponerla en la página a la vista usan la posición estándar.
- **Arrastrar**: mover el que ya existe.
- **Redimensionar**: tirar de una esquina.

Trazar es «ponerla aquí» con sitio elegido: con **Una página** sustituye la
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
- **Dos elementos enfocables** (ID-113): la hoja (`viewer.sheet`) atiende
  `AvPág`, `RePág`, `Inicio` y `Fin`; el recuadro, las flechas, y las teclas de
  página burbujean desde él. `Esc` devuelve el foco a la hoja. Al cambiar de
  página, un recuadro fuera de la parte visible se trae a ella una vez (ID-118).
- **Ni el zoom ni el redimensionado de la ventana escriben la colocación**
  (ID-114): solo el trazo, el arrastre, el redimensionado y las flechas.
- **Soltar fuera de la página no se acepta** (`viewer.outOfPage`): el recuadro
  vuelve a donde estaba. El backend lo comprueba otra vez antes de firmar,
  porque iText recortaría en silencio (ID-22).
- **La firma no sigue a la página que miras**: se queda donde se puso.
- **Las páginas donde el recuadro no cabe** se avisan una vez, antes de firmar,
  en el [diálogo de páginas sin firma visible](dialogo-paginas-sin-firma-visible.md).

## Componentes y tokens

`Button`, `Row`, los iconos del sistema de diseño y `ErrorNotice` para los
fallos; maquetación propia con `var(--rf-*)`: `--rf-bg`, `--rf-surface`,
`--rf-border-subtle`, `--rf-border-strong`, `--rf-primary`, `--rf-shadow-card`,
`--rf-shadow-elevated`, `--rf-radius-sm|lg|pill`, `--rf-space-md`.

## Decisiones

- **El mini-render es la firma real**, no un dibujo nuestro (25/09/2026). La
  regla «o es la de verdad, o no hay nada» del ADR-0006 sigue en pie, y es la
  razón de que Storybook no pinte ningún recuadro con contenido.
- **Se conservan «recalculando» y «no se ha podido dibujar»**, que el dibujo de
  la ventana no tenía: el caso existe igual. «Moviendo» pierde su aviso: era una
  frase para lo que el atenuado ya enseña. El aviso flota sobre la píldora porque
  en flujo, bajo la hoja, se iba con ella al ampliar.
- **Sin pastilla sobre el recuadro ni etiqueta de página**: el recuadro solo
  lleva marco y tiradores; la página la dice el panel.
- **Sin tira de miniaturas.** La propusieron dos de las composiciones que se
  descartaron (ver [ventana principal](ventana-principal.md#decisiones)) como
  navegación y selección de páginas. A las 200 páginas es un desplazador que hay
  que recorrer, y el número editable de la píldora llega en un gesto.
- **La paginación es una píldora del visor**, no una pastilla por página: con 27
  páginas ya no cabían.
- **El fallo de apertura vive en el visor y no en el panel**, porque sin
  documento abierto no hay panel, y se pinta también con un documento delante:
  el segundo PDF que no se deja abrir deja el primero en pantalla, y sin el aviso
  el rechazo sería mudo.
- Mecánica que hace que la pantalla se sienta como se ve: las pintadas de
  `pdf.js` pasan por una cola que cancela la anterior, y el arrastre no toca el
  estado hasta soltar.
