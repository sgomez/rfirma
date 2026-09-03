# Prefirma en seco pintada con `pdf.js`: ver el sello dentro del recuadro

Sondeo del [#115](https://github.com/sgomez/rfirma/issues/115), hijo del mapa
[#113](https://github.com/sgomez/rfirma/issues/113). Decide la **ficha 7** del
informe *rFirma después de v0.1* (la vista previa dentro del recuadro, v0.3) sin
desmentir el **ID-44**, que hoy deja el recuadro vacío porque una maqueta HTML
imita al compositor y miente.

La pregunta, tal cual la trae el ticket: ¿se puede enseñar dentro del recuadro
exactamente lo que va a quedar, pidiéndoselo a quien lo dibuja de verdad?

**Respuesta corta: sí, y el resultado no se parece al final: es el final.** Un
ciclo trifásico completo con un `PK1` inventado produce un PDF cuyos bytes
visibles son **idénticos** a los del PDF firmado de verdad —los 482 bytes que
cambian caen todos dentro del hueco de `/Contents`, que no se ve—, y `pdf.js`
con sus opciones de fábrica lo pinta con texto y rúbrica dentro del `/Rect`
pedido, en las ocho combinaciones del banco de rotaciones. **No hace falta
ningún certificado de relleno ni ningún PIN**: basta el certificado que el
usuario ya ha elegido, que es público, y 256 bytes de `/dev/urandom` donde iría
la firma.

El pero es el coste, y no lo marca el número de páginas sino **el tamaño del
fichero**: 22 ms de ciclo en un PDF de texto de una página, 40 ms en uno de 200,
pero **1,7 s y 507 MB de RSS** en un escaneado de 200 páginas y 37 MB. La vista
previa no puede ser un efecto del arrastre.

---

## El veredicto

**Camino A (prefirma en seco pintada con `pdf.js`): viable.** Se descarta el
camino B (pedirle un PNG al puente), que sigue contraindicado por el
[ADR-0012](../adr/0012-normalizacion-de-la-rubrica-en-rust.md) —rasterizar dentro de
Java es exactamente la puerta a AWT que ese ADR cerró—, y no hace falta la caída
elegante de pintar solo la rúbrica dentro del recuadro.

Lo que el camino A obliga a decidir está en [«Lo que esto obliga a
decidir»](#lo-que-esto-obliga-a-decidir), al final.

---

## Cómo se midió

### El banco

Un programa desechable en C, enlazado **directamente contra la
`librfirma_crypto.so` que hay construida hoy** (27,7 MB,
`rfirma-native-bridge/target/lib/rfirma/`, un solo fichero como manda el
ADR-0004), copiada a un directorio donde no hay ningún otro `.so` —ni `libawt`,
ni `liblcms`— para no caer en la trampa que mide
`exclusion-afirma-ui-utils.md`. Llama a `graal_create_isolate` una vez y luego a
`autofirma_pades_presign` y `autofirma_pades_postsign` tantas veces como se le
pida, cronometrando cada llamada con `CLOCK_MONOTONIC`.

Se mide contra la imagen nativa, no contra la JVM, porque la pregunta es de
coste y ahí sí se separan.

Material, todo generado en el borrador y fuera del repositorio:

- **Certificado**: el `active-rsa.p12` del kit FNMT de pruebas
  (`testdata/fnmt/active-rsa.p12`, contraseña `1234`), `CN=EIDAS CERTIFICADO
  PRUEBAS - 99999999R`, válido hasta el 30/10/2028. El certificado personal del
  titular no se usa en ningún punto, como manda `CLAUDE.md`.
- **Rúbrica**: JPEG opaco de 200×100, calidad 90, sin perfil ICC —lo que deja
  `rubric::normalize`—, 5 885 bytes.
- **PDFs de texto**: 1, 27 y 200 páginas generados a mano (601 B, 8,0 KB y
  58,6 KB), con `MediaBox` y `/Rotate` configurables.
- **PDFs pesados**: 27 y 200 páginas con un JPEG a 100 ppp por página (5,05 MB y
  37,4 MB), que es la forma que tiene de verdad un documento que llega a firmar.
- **`extraParams`**: los que emite `SignatureConfig::extra_params`
  (`rfirma-app/src-tauri/src/signing/config.rs`) — `signatureSubFilter`,
  `signaturePage`, las cuatro esquinas, `layer2Text` y `signatureRubricImage`.

Para pintar, `pdfjs-dist` **6.3.289**, la misma versión que declara
`rfirma-app/package.json`, sobre `@napi-rs/canvas` en Node, y con la misma
llamada que hace la aplicación: `page.render({ canvas, viewport })`, sin tocar
`annotationMode` (`rfirma-app/src/viewer/pdfjsLoader.ts:42`).

### Cómo se compara el sello con el recuadro

No a ojo. Se pinta la página del PDF firmado y la misma página del PDF original,
se restan píxel a píxel, y el *bounding box* de la tinta nueva se devuelve a
espacio de usuario PDF con `viewport.convertToPdfPoint`. Lo que se compara es
ese rectángulo contra el `/Rect` que `pdf.js` lee del widget en
`page.getAnnotations()`.

---

## 1. Cuánto cuesta la prefirma en seco

Milisegundos por llamada. La primera columna es la **primera llamada en un
isolate recién creado**; la banda es el resto de llamadas del mismo isolate.
Crear el isolate cuesta aparte **1,3–1,6 ms**, y solo se paga una vez por
proceso.

### PDFs de texto

| Documento | Rúbrica | Prefirma 1.ª | Prefirma en caliente | Postfirma 1.ª | Postfirma en caliente |
| --- | --- | --- | --- | --- | --- |
| 1 pág., 601 B | no | 10,0 | 1,1 | 12,2 | 2,9–4,4 |
| 1 pág., 601 B | sí | 10,5 | 1,4–2,0 | 12,6 | 3,1–3,9 |
| 27 pág., 8,0 KB | no | 11,8 | 2,3–2,6 | 13,3 | 3,7–4,7 |
| 27 pág., 8,0 KB | sí | 12,2 | 2,6–2,9 | 13,5 | 4,1–4,7 |
| 200 pág., 58,6 KB | no | 20,6 | 6,2–12,3 | 19,5 | 6,0–12,3 |
| 200 pág., 58,6 KB | sí | 20,3 | 6,5–15,2 | 19,9 | 6,4–12,2 |

Doce llamadas por caso. El recuadro se pide en la página 1, 14 y 100
respectivamente.

**La rúbrica no se nota**: entre 0,3 y 0,5 ms de más en la prefirma, dentro del
ruido en la postfirma. Es coherente con el ADR-0012: al estar excluido
`afirma-ui-utils`, el JPEG pasa tal cual a `new Jpeg(...)` y no se reencoda
nada. Cada llamada, con rúbrica, emite el `WARNING` de
`ClassNotFoundException: es.gob.afirma.ui.utils.ImageUtils` que ese ADR predice.

### PDFs pesados (un JPEG por página)

| Documento | Prefirma 1.ª | Prefirma en caliente | Postfirma 1.ª | Postfirma en caliente |
| --- | --- | --- | --- | --- |
| 27 pág., 5,05 MB | 84,0 | 70,3–76,1 | 174,6 | 162,7–167,2 |
| 200 pág., 37,4 MB | 588,9 | 567,6–570,1 | 1 263,8 | 1 104,0–1 126,2 |

**El coste va con los bytes, no con las páginas.** Doscientas páginas de texto
cuestan 12 ms de ciclo; doscientas páginas escaneadas, **1,7 s**. La postfirma
cuesta el doble que la prefirma porque regenera el documento entero
(`PAdESTriPhaseSigner.java:342-364`, ver `firma-visible-trifasica.md`).

El proceso llegó a **507 MB de RSS máximo** con el PDF de 37,4 MB. Es lo que hay
que tener en cuenta al decidir dónde vive el ciclo en seco.

Lo que **no** cuesta: la conversión a Base64 que hace Rust en la frontera FFI,
**0,02 s para los 37,4 MB**.

### Lo que cuesta pintarlo

`pdf.js` 6.3.289, escala 1,5, tres repeticiones por caso:

| Documento firmado | `getDocument` | `page.render` |
| --- | --- | --- |
| 1 pág., 61 KB | 54,3–56,3 ms | 42,7–46,2 ms |
| 200 pág. escaneadas, 37,4 MB (pág. 100) | 81,1–83,5 ms | 141,6–149,2 ms |

Medido en Node con `@napi-rs/canvas`, no en WebKitGTK: el camino de código de
`pdf.js` es el mismo, el rasterizador no. Los números valen como orden de
magnitud, no como promesa.

**Ciclo en seco completo, de extremo a extremo** (prefirma + postfirma + cargar
y pintar la página): **≈ 0,15 s** para un PDF de una página y **≈ 1,9 s** para
un escaneado de 200 páginas y 37 MB.

---

## 2. ¿Basta la prefirma?

**No, y no por una limitación del formato sino por la frontera FFI: la prefirma
no devuelve ningún PDF.**

El sello se fija en la prefirma —eso lo dejó cerrado el #7—, pero el documento
sellado se queda dentro de `PdfSessionManager` y nunca sale. Lo que
`autofirma_pades_presign` devuelve son tres cadenas y ninguna es un PDF
(`rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/NativeBridge.java:83-105`):

```java
final StringBuilder json = new StringBuilder("{\"ok\":true");
field(json, "session", result.session());   // el TriphaseData en XML
field(json, "pre", result.preSignB64());    // los atributos firmados CAdES en DER
field(json, "stamp", result.stamp());       // el sello de sesión del ADR-0016
```

El único punto del puente que emite bytes de PDF es
`autofirma_pades_postsign` (`NativeBridge.java:121-145`, campo `pdf`). Así que
**el ciclo en seco es prefirma + postfirma**, las dos, y las dos se tiran.

Se podría abrir una cuarta entrada FFI que devolviera el PDF sellado de la
prefirma y ahorrarse la postfirma —que es la fase cara, el doble—. **No se
recomienda**: sería una entrada nueva en la frontera solo para la vista previa,
sobre un camino de código que el puente hoy no expone, y el ahorro es de 1,1 s
en el peor caso medido y de 3 ms en el normal.

---

## 3. Qué pasa cuando la clave no firma nada

**Se completa, y el PDF es indistinguible salvo dentro del `/Contents`.**

Se hizo una prefirma y **dos postfirmas desde la misma sesión y el mismo sello**:
una con el `PK1` real (`openssl dgst -sha256 -sign` sobre los atributos firmados
del kit FNMT) y otra con **256 bytes de `/dev/urandom`**.

| | |
| --- | --- |
| Tamaño de los dos PDF | 62 483 bytes, **el mismo** |
| Bytes distintos | **482** |
| Dónde caen | del desplazamiento **11 644 al 12 155** |
| Rangos firmados (`pdfsig`) | `[0 – 6 296]`, `[60 298 – 62 483]` |

Los 482 bytes que cambian están **enteros dentro del hueco de `/Contents`**, el
único trozo del fichero que la firma deja fuera de su propio rango. **Todo lo
que se ve es byte a byte el mismo fichero.**

`pdfsig` hace su trabajo y los distingue:

- con el `PK1` real: `Signature Validation: Signature is Valid.`
- con el `PK1` inventado: `Signature Validation: Signature is Invalid.`

y `pdf.js` pinta los dos exactamente igual.

### La vista previa no cambia con la hora

Dos ciclos en seco **separados por 3 segundos** sobre el mismo documento
producen PDF que difieren en **593 bytes** —el `/Contents`, el `TIME` del
diccionario de firma y el File ID— y, al pintarlos y restarlos píxel a píxel,
**0 píxeles de diferencia**. La apariencia no depende del instante porque
rFirma compone el `layer2Text` en Rust y lo envía ya resuelto; si el
texto llevara los comodines de AutoFirma, esto no valdría.

Consecuencia práctica: **la vista previa se puede calcular una vez y cachear
hasta que cambie el recuadro, el texto, la rúbrica o el certificado.**

### Cofirma

Un ciclo en seco sobre un PDF **que ya llevaba una firma válida** (el del caso
real de arriba) también completa: 12,1 ms de prefirma y 14,3 ms de postfirma,
`pdf.js` lee **los dos widgets** y pinta los dos sellos, y `pdfsig` deja la
firma anterior en `Valid` y solo declara `Invalid` la nueva. La vista previa
funciona igual en cofirma.

---

## 4. ¿Cae el sello donde se pidió, con las cuatro rotaciones?

**Sí. Ocho de ocho, al punto.**

El banco es el del #9: A4 con `MediaBox` en el origen y una `MediaBox`
desplazada `[20 30 615 872]`, cada una con `/Rotate` 0, 90, 180 y 270. Para cada
caso se fija el rectángulo deseado en espacio de usuario **U**, se le aplica la
inversa `T⁻¹` que documenta `coordenadas-recuadro-pades.md`, se firma en seco
con esos `extraParams` y se lee el `/Rect` del widget con `pdf.js`.

| Página | `/Rotate` | U pedida | `extraParams` (`T⁻¹`) | `/Rect` que lee `pdf.js` | Tinta nueva |
| --- | --- | --- | --- | --- | --- |
| A4 | 0 | `100 200 300 260` | `100 200 300 260` | `100 200 300 260` | `102 200 260 260` |
| A4 | 90 | `100 200 300 260` | `200 295 260 495` | `100 200 300 260` | `106 200 282 260` |
| A4 | 180 | `100 200 300 260` | `295 582 495 642` | `100 200 300 260` | `140,3 200 298,3 260` |
| A4 | 270 | `100 200 300 260` | `582 100 642 300` | `100 200 300 260` | `118,3 200 294,3 260` |
| Desplazada | 0 | `120 230 320 290` | `120 230 320 290` | `120 230 320 290` | `122 230 280 290` |
| Desplazada | 90 | `120 230 320 290` | `230 295 290 495` | `120 230 320 290` | `126 230 302 290` |
| Desplazada | 180 | `120 230 320 290` | `295 582 495 642` | `120 230 320 290` | `160,3 230 318,3 290` |
| Desplazada | 270 | `120 230 320 290` | `582 120 642 320` | `120 230 320 290` | `138,3 230 314,3 290` |

Dos lecturas:

1. **El `/Rect` es exactamente U en los ocho casos, diferencia `[0,0,0,0]`.** Es
   la tabla del #9 vuelta a medir por otro camino —el widget leído por `pdf.js`
   en vez de por un lector de PDF— y con un ciclo en seco en vez de uno real.
   Sale igual.
2. **La tinta que `pdf.js` pinta está contenida en U en los ocho casos**, y toca
   siempre los dos bordes horizontales de U (`200`/`260`, `230`/`290`). Lo que
   sobra por los lados es el interlineado del texto y el reescalado con
   proporción de la rúbrica dentro del recuadro, no un desplazamiento.

Es decir: **lo que se ve en la vista previa cae dentro del recuadro que el
usuario arrastró, también en páginas rotadas y con la `MediaBox` desplazada.**

### Lo que `pdf.js` decide y podría romperlo

El sello se pinta porque `page.render` deja `annotationMode` en su valor por
omisión. Medido, sobre el mismo par de PDF:

| `annotationMode` | Tinta nueva |
| --- | --- |
| por omisión | 6 255 px |
| `DISABLE` (0) | **0 px** |
| `ENABLE` (1) | 6 255 px |
| `ENABLE_FORMS` (2) | 6 255 px |
| `ENABLE_STORAGE` (3) | 6 255 px |

Solo `DISABLE` deja el recuadro en blanco. `ENABLE_FORMS` —el modo que haría
falta el día que el visor rellene formularios— **no** rompe la vista previa: la
apariencia del widget de firma se sigue pintando en el lienzo. Aun así conviene
que quede dicho: **la vista previa depende de una opción de `pdf.js` que hoy
nadie escribe, y un `annotationMode: 0` la apagaría sin dar ningún error.**

---

## Lo que se ve, y no es bonito

Dos cosas que la vista previa va a enseñar y que hoy nadie ve, precisamente
porque el ID-44 deja el recuadro vacío:

1. **El texto y la rúbrica se solapan.** AutoFirma dibuja la imagen como fondo
   del recuadro y el texto encima, sin reservarle sitio a ninguno de los dos. En
   el PDF de prueba, el texto de tres líneas cruza la rúbrica de lado a lado. No
   es un fallo del sondeo: es lo que produce el compositor, y hoy sale así en
   cada PDF que rFirma firma con texto y rúbrica a la vez.
2. **En páginas rotadas el texto se maqueta para la forma del recuadro.** iText
   rota la apariencia para que salga derecha —lo que ya decía el #9—, pero la
   maqueta dentro de la caja tal y como el usuario la arrastró: en un recuadro
   alto y estrecho, «Firmado por: EIDAS CERTIFICADO PRUEBAS - 99999999R» sale
   partido en ocho líneas.

Las dos son razones **a favor** del camino A, no en contra: son justo lo que una
maqueta HTML no adivinaría y lo que el usuario descubre hoy al abrir el PDF ya
firmado.

Aparte, `pdfjsLoader` no le pasa a `pdf.js` ni `standardFontDataUrl` ni
`cMapUrl`. La apariencia usa Courier, una de las catorce fuentes estándar, así
que `pdf.js` avisa (`Ensure that the standardFontDataUrl API parameter is
provided`) y sustituye. **Pinta igual** —está medido—, pero las métricas de los
glifos no son las del compositor, así que el corte de línea que se ve en la
vista previa puede no ser el que produzca otro lector. Es la única falta de
fidelidad encontrada, y se cierra empaquetando `pdfjs-dist/standard_fonts`.

---

## Lo que no se midió

Se dice en voz alta para que nadie lo dé por medido:

- **No se midió dentro del flatpak.** Todo corre en el equipo de desarrollo,
  contra la `.so` del `target/`. El sandbox no debería cambiar nada aquí —no hay
  portales de por medio, los bytes ya están en memoria—, pero no está
  comprobado.
- **No se midió a través de la envoltura de Rust** (`ffi.rs`), sino con un
  programa en C contra las mismas entradas FFI. Lo que Rust añade encima es
  Base64 y análisis de JSON; el Base64 está medido aparte (0,02 s para 37 MB) y
  el JSON no. Para un PDF de 37 MB la cadena JSON de vuelta pasa de 50 MB por el
  C-heap, y **ese pico de memoria no está medido en la aplicación real**.
- **No se midió en WebKitGTK**, solo en Node con `@napi-rs/canvas`.
- **No se midió con documentos protegidos por contraseña ni con PDF/A**, ni con
  un `signatureField` preexistente.

---

## Lo que esto obliga a decidir

Cuatro decisiones que el sondeo destapa y que el mapa debería recoger antes de
que la ficha 7 se convierta en spec:

1. **No hace falta ningún certificado de relleno.** Es el hallazgo que cambia la
   forma de la ficha. El ciclo en seco funciona con **el certificado que el
   usuario ya ha elegido** —que es público y se lee del token sin PIN— y con 256
   bytes de basura donde iría el `PK1`. Usar un certificado de relleno sería
   peor: el nombre del recuadro sería otro y la longitud del `/Contents`
   cambiaría. Lo que queda pendiente es qué enseñar **antes** de que haya un
   certificado elegido.
2. **La vista previa no puede colgar del arrastre.** 1,7 s de ciclo y 507 MB de
   RSS en un escaneado de 37 MB lo prohíben. O es una acción explícita («ver
   cómo queda»), o va con un retardo largo y un corte por tamaño de documento.
   Cachear es barato y seguro: está medido que el resultado no cambia con la
   hora.
3. **Se paga un ciclo trifásico entero que se tira.** La prefirma sola no
   devuelve un PDF, y abrir una cuarta entrada FFI para que lo devuelva no
   compensa. La firma de verdad vuelve a prefirmar desde cero.
4. **Hay que empaquetar `standard_fonts` de `pdfjs-dist`** y pasarle
   `standardFontDataUrl` a `getDocument`, o aceptar por escrito que el corte de
   línea de la vista previa es aproximado.

Y un aviso para quien escriba las pruebas: **la vista previa depende de que
`annotationMode` siga en su valor por omisión.** Ninguna guardia lo vigila hoy.
