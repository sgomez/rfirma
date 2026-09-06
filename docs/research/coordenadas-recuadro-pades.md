# Del recuadro dibujado con `pdf.js` a los `extraParams` de posición de PAdES

Medición de extremo a extremo para el issue
[#9](https://github.com/sgomez/rfirma/issues/9), que depende de #3
(`docs/research/pades-triphase-contract.md`) y de #4. No es aritmética sobre el
papel: cada caso se firmó de verdad con el ciclo trifásico entero y se comprobó
leyendo el `/Rect` del widget de firma del PDF resultante.

El banco de pruebas vive en la rama `research/9-coordenadas-pdfjs`, en
`prototipos/9-coordenadas-pdfjs/`. Es desechable; esta nota es la referencia.

## La conversión

Son **dos** pasos. El segundo no se ve venir, y sin él la firma cae en el sitio
equivocado en cualquier página rotada.

```js
// 1) canvas -> espacio de usuario PDF.  Lo hace pdf.js.
const [ax, ay] = viewport.convertToPdfPoint(x0, y0);
const [bx, by] = viewport.convertToPdfPoint(x1, y1);
const U = { llx: min(ax, bx), lly: min(ay, by),
            urx: max(ax, bx), ury: max(ay, by) };

// 2) espacio de usuario -> extraParams.  T^-1, segun la /Rotate de la pagina.
//    mx1, my1 son las esquinas SUPERIORES de la MediaBox (page.view[2], [3]).
const inv = {
  0:   (x, y) => [x, y],
  90:  (x, y) => [y, mx1 - x],
  180: (x, y) => [mx1 - x, my1 - y],
  270: (x, y) => [my1 - y, x],
}[((page.rotate % 360) + 360) % 360];
// aplicar a las dos esquinas de U, normalizar de nuevo (min/max) y redondear.
```

`signaturePage` es el número de página de `pdf.js` tal cual: 1-based, sin
corrección.

## Por qué hace falta el paso 2

`convertToPdfPoint` invierte la matriz del viewport, o sea deshace de golpe la
escala, el volteo del eje Y, la rotación `/Rotate` y el origen de la MediaBox.
Devuelve el rectángulo en espacio de usuario PDF, que es **exactamente el
`/Rect` que acaba teniendo el widget de firma**. Lo natural sería pasar eso a
los `extraParams`. Y está mal.

AutoFirma entrega el rectángulo tal cual a
`PdfSignatureAppearance.setVisibleSignature(Rectangle, page, null)`
(`afirma-crypto-pdf/.../PdfSessionManager.java:532`), y esa sobrecarga de
`afirma-lib-itext` lo guarda sin tocar — comprobado en el bytecode: la rama que
sí rota al fijar es `setVisibleSignature(String)`, la de campos preexistentes,
que queda fuera de alcance. Pero **al cerrar el documento iText transforma ese
rectángulo** según la `/Rotate` antes de escribir el `/Rect`.

Medido pasando siempre `[100 200 300 260]` sobre un PDF de MediaBox
`[20 30 615 872]`, una vez por rotación:

| `/Rotate` | `/Rect` resultante  | transformación                |
| --------- | ------------------- | ----------------------------- |
| 0         | `[100 200 300 260]` | identidad                     |
| 90        | `[415 100 355 300]` | `T(x,y) = (mx1 − y, x)`       |
| 180       | `[515 672 315 612]` | `T(x,y) = (mx1 − x, my1 − y)` |
| 270       | `[200 772 260 572]` | `T(x,y) = (y, my1 − x)`       |

Usa `mx1`/`my1` —los **límites superiores** de la MediaBox—, no la anchura ni
la altura. Con la MediaBox en el origen las dos cosas coinciden y el error se
esconde; con la MediaBox desplazada, no.

Como el widget acaba en `T(entrada)` y queremos que acabe en `U`, hay que
entregar `T⁻¹(U)`. Eso es el paso 2.

El script `sonda-rotacion.sh` del banco vuelve a medir esta tabla. **Si sube la
versión de `afirma-lib-itext`, córrelo antes que nada**: la tabla es un hecho
sobre esa librería, no sobre el formato PAdES.

## Trampas

1. **Un rectángulo que se sale de la página no da error.** iText lo recorta al
   `/Rect` que quepa. Durante la medición un caso salió con 13 pt de ancho en
   vez de 200 y la firma se generó igual, válida ante `pdfsig`. Si el recuadro
   tiene que caber, hay que comprobarlo antes de firmar, no después.
2. **Las coordenadas se leen como `int`** (`PdfUtil.getPositionOnPage`). Los
   decimales del arrastre se pierden; hay que redondear en origen.
3. **El recuadro no se guarda en píxeles de pantalla.** Guardado así, al
   cambiar el zoom se queda clavado en la pantalla y se mueve sobre el
   documento sin que nadie lo toque, cambiando los `extraParams` de paso. Se
   guarda en espacio de usuario PDF y los píxeles se derivan en cada pintada
   con `convertToViewportPoint`.
4. **La conversión ingenua** (`x/escala`, `alto − y/escala`) acierta solo si la
   página no está rotada, la MediaBox empieza en `(0,0)` y el zoom es 1. Las
   tres condiciones se cumplen en el PDF de prueba de cualquiera, que es por lo
   que este fallo llega lejos.
5. **El texto de la rúbrica sale derecho** en páginas rotadas: iText rota la
   apariencia por su cuenta. No hay que compensar nada por ese lado.
6. **`pdftotext` no vale para comprobar esto** (ya lo decía #14). Se comprueba
   leyendo el `/Rect` del widget del PDF firmado, y rasterizando con `pdftoppm`
   si se quiere ver.

## `page.view` es la CropBox, e iText transforma con la MediaBox

Medido en el #354, sobre `pdfjs-dist` 6.3.289 y `afirma-lib-itext` 1.7:

- El `page.view` que la ventana manda como `mediaBox` **no siempre es la
  MediaBox**: en el trabajador de `pdf.js` (`pdf.worker.mjs`, `get view()`) es
  la intersección de la CropBox con la MediaBox, y sólo cae en la MediaBox
  cuando no hay CropBox o las dos coinciden. Es además **lo único que cruza al
  hilo principal**: el manejador `GetPage` devuelve `rotate`, `ref`, `userUnit`
  y `view`, y nada más. Desde la API pública de `pdf.js` **no hay forma de
  conocer la MediaBox**.
- iText, en cambio, mide con la MediaBox: `PdfReader.getPageSize(int)` lee
  `/MediaBox` (`javap -c` sobre `PdfReader.class`, `PdfName.MEDIABOX`), y
  `getPageSizeWithRotation` se construye sobre ella. La CropBox sólo aparece en
  `getBoxSize(int, String)`, que el camino de la firma no llama.

Consecuencia, y su alcance:

- **Paso 1 (lienzo → espacio de usuario) está bien** precisamente porque usa
  `page.view`: es la misma caja con la que `pdf.js` construyó el viewport, así
  que la inversa devuelve coordenadas absolutas correctas.
- **Paso 2 (`T⁻¹`) usa los límites superiores de esa misma caja**, y ahí
  debería usar los de la MediaBox. Con `/Rotate` 0 el `T⁻¹` es la identidad y
  **no hay error ninguno**; el desplazamiento sólo aparece en páginas rotadas de
  documentos donde las dos cajas difieren, y vale exactamente la diferencia
  entre ellas.
- La guardia del ID-22 (`check_fits`) queda comprobando contra el área visible
  en vez de contra el papel, que es **más estricta**, no menos.

**Corregirlo exige la MediaBox, y hoy no está al alcance de la ventana.** No la
da `pdf.js` y no la puede sacar Rust: `signing/admissibility.rs` es
explícitamente «esto no es un lector de PDF». La vía sería una entrada nueva del
puente que devuelva la geometría de cada página, lo que arrastra un cambio en
Java y una reconstrucción de la imagen nativa. Queda escrito aquí y anotado en
`signing/placement.rs`; **nada de esto toca al camino de la sede**, donde no hay
conversión ninguna (ID-282).

## Qué se midió

Dieciséis casos, todos con el mismo arrastre de pantalla `(60,80)–(260,160)`,
firmados con el ciclo trifásico entero y comprobados contra el `/Rect` del PDF
resultante:

- A4 con `/Rotate` 0, 90, 180 y 270
- A5 y Letter (tamaños que no son A4)
- MediaBox `[20 30 615 872]` con `/Rotate` 0, 90, 180 y 270
- un documento de tres páginas de distinto tamaño y rotación, firmando en cada
  una
- tres a zoom distinto de 1 (A4 a 1.75, `offset-rot270` a 1.75, `a4-rot90` a
  0.6), que es donde se vería si la escala se cuela en la conversión

**16 de 16 coinciden al punto** (`diferencia [0,0,0,0]`). Dos de los casos
rotados se verificaron además rasterizando: el texto cae dentro del recuadro
dibujado y sale derecho.

Se firmó sobre la JVM de GraalVM CE 25, no sobre la imagen nativa: #14 dejó
demostrado que producen un PDF idéntico bit a bit, así que para esta pregunta
la imagen nativa no aporta nada.
