# PROTOTIPO #9 — el recuadro de firma, de `pdf.js` a los `extraParams` de PAdES

Desechable. Existe para contestar una pregunta y morir:
[#9](https://github.com/sgomez/rfirma/issues/9).

> ¿Funciona colocar el recuadro de firma sobre un PDF renderizado con `pdf.js`,
> y da coordenadas que PAdES entienda?

Sí, **pero no con la conversión que uno escribiría de primeras**, ni con la que
parece obviamente correcta. Son dos pasos, y el segundo no se ve venir.

## La respuesta

```js
// 1) canvas -> espacio de usuario PDF.  Lo hace pdf.js.
const [ax, ay] = viewport.convertToPdfPoint(x0, y0);
const [bx, by] = viewport.convertToPdfPoint(x1, y1);
const U = { llx: min(ax,bx), lly: min(ay,by), urx: max(ax,bx), ury: max(ay,by) };

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

## Por qué el paso 2

`convertToPdfPoint` ya deshace la escala, el volteo del eje Y, la rotación y el
origen de la MediaBox. Da el rectángulo en espacio de usuario PDF, que es
**exactamente el `/Rect` que acaba teniendo el widget de firma**. Lo natural
sería pasar eso a los `extraParams`. Y está mal.

AutoFirma entrega el rectángulo tal cual a `PdfSignatureAppearance.setVisible
Signature(Rectangle, page, null)` de `afirma-lib-itext`, que lo guarda sin
tocar (comprobado en el bytecode: la rama que sí rota es la de
`setVisibleSignature(String)`, la de campos preexistentes, que no usamos).
Pero **más adelante, al cerrar, iText transforma ese rectángulo** según la
`/Rotate` de la página antes de escribir el `/Rect`. Medido con
`./sonda-rotacion.sh` sobre una MediaBox `[20 30 615 872]`, pasando siempre
`[100 200 300 260]`:

| `/Rotate` | `/Rect` resultante  | transformación                |
| --------- | ------------------- | ----------------------------- |
| 0         | `[100 200 300 260]` | identidad                     |
| 90        | `[415 100 355 300]` | `T(x,y) = (mx1 − y, x)`       |
| 180       | `[515 672 315 612]` | `T(x,y) = (mx1 − x, my1 − y)` |
| 270       | `[200 772 260 572]` | `T(x,y) = (y, my1 − x)`       |

Usa `mx1`/`my1` —los **límites superiores** de la MediaBox—, no la anchura ni
la altura. Con la MediaBox en el origen las dos cosas coinciden y el error se
esconde; con la MediaBox desplazada, no. De ahí que haya casos `offset-*`.

Como el widget acaba en `T(entrada)` y queremos que acabe en `U`, hay que
entregar `T⁻¹(U)`. Eso es el paso 2.

## Las trampas que dejó por escrito

1. **La conversión ingenua** (`x/escala`, `alto − y/escala`) acierta solo si la
   página no está rotada **y** la MediaBox empieza en `(0,0)`. El visor la
   enseña al lado de la buena para que se vea cuándo divergen.
2. **Un rectángulo que se sale de la página no da error**: iText lo recorta al
   `/Rect` que quepa. Durante la investigación un caso salió con 13 pt de ancho
   en vez de 200 y la firma se generó igual, válida. Si el recuadro tiene que
   caber, hay que comprobarlo antes de firmar.
3. **Redondeo**: AutoFirma lee las cuatro coordenadas como `int`
   (`PdfUtil.getPositionOnPage`). Los decimales del arrastre se pierden; se
   redondea aquí y punto.
4. **El texto de la rúbrica sale derecho** en páginas rotadas: iText rota la
   apariencia por su cuenta. No hay que compensar nada por ese lado.
5. **`pdftotext` no vale para comprobar esto** (ya lo decía #14). Aquí se
   comprueba leyendo el `/Rect` del widget del PDF firmado, que es el dato
   duro, y en dos casos además rasterizando con `pdftoppm` para verlo.

## Qué se midió

Trece casos, todos con el **mismo arrastre de pantalla** `(60,80)–(260,160)` a
zoom 1, firmados de verdad con el ciclo trifásico entero y comprobados contra
el `/Rect` del PDF resultante:

- A4 con `/Rotate` 0, 90, 180 y 270
- A5 y Letter (tamaños que no son A4)
- MediaBox `[20 30 615 872]` con `/Rotate` 0, 90, 180 y 270
- `mixto.pdf`, tres páginas de distinto tamaño y rotación, firmando en cada una

**13 de 13 coinciden al punto** (`diferencia [0,0,0,0]`).

## Cómo se corre

```bash
./motor/preparar.sh        # una vez: jar, classpath y certificado de usar y tirar
python3 casos/gen-pdfs.py  # genera los PDFs de prueba
./servir.sh                # visor en http://localhost:8099/
./comprobar-todo.sh        # firma y comprueba los trece casos
./sonda-rotacion.sh        # vuelve a medir T (si cambia la version de itext)
```

El visor arrastra, enseña las dos conversiones y escribe el `.properties`.
`firmar.sh` hace el ciclo trifásico completo sobre la JVM de GraalVM CE 25 —
que #14 dejó demostrado que da un PDF idéntico bit a bit al de la imagen
nativa, así que para esta pregunta la imagen nativa no aporta nada.
`comprobar.py` compara el `/Rect` del widget con lo que se dibujó.

## Piezas

| Fichero              | Qué es                                                        |
| -------------------- | ------------------------------------------------------------- |
| `index.html`, `app.js` | El visor. La conversión está en `aEspacioUsuario` / `aExtraParams`. |
| `casos/gen-pdfs.py`  | Genera los PDFs con rejilla y ancla `ORIGEN`.                  |
| `firmar.sh`          | Ciclo trifásico: prefirma → PK1 → postfirma.                   |
| `comprobar.py`       | Lee el `/Rect` del widget y lo compara con lo dibujado.        |
| `sonda-rotacion.sh`  | Mide `T`. Es de donde sale la tabla de arriba.                 |
| `motor/`             | Jar, classpath y certificado. No se versiona.                  |
