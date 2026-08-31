# La postfirma PAdES con rúbrica visible en GraalVM CE 25

Medición para el issue [#14](https://github.com/sgomez/rfirma/issues/14). Cierra el cuadrante que
faltaba: [#13](https://github.com/sgomez/rfirma/issues/13) midió la postfirma en CE 21 y
[#12](https://github.com/sgomez/rfirma/issues/12) la prefirma en CE 25; aquí se mide **postfirma +
rúbrica + CE 25**. **Registra hechos, no decide** el modelo de distribución: eso es el
[#6](https://github.com/sgomez/rfirma/issues/6).

Entorno: GraalVM CE 25.3.4.1 (JDK 25.0.4.1, Substrate VM), Maven 3.9.12, Linux x86_64, poppler
(`pdfsig`, `pdftoppm`) como validador y visor. Banco de pruebas: `rfirma-native-bridge/`, con los
dos `@CEntryPoint` que dejó el #13 y tres guiones nuevos.

## Veredicto

**Sí: la postfirma con rúbrica visible funciona en CE 25, igual que la prefirma y con el mismo
coste.** El flujo trifásico completo —prefirma y postfirma, las dos en la imagen nativa, bajo
`env -i` y sin Java instalado— produce un PDF que `pdfsig` valida, cuya rúbrica se ve en la página,
y que es **idéntico bit a bit** al que produce la JVM con la misma sesión trifásica.

## Resumen

| | Sin rúbrica | Rúbrica de **texto** | Rúbrica de **imagen** (PNG y JPEG) |
|---|---|---|---|
| ¿Completa el flujo completo en nativo? | **Sí** | **Sí** | **Sí** |
| `.so` necesarios | **1** | **1** | **6** |
| ¿Metadatos del agente de trazado? | No | No | **Sí** (`reachability-metadata.json`, 42 KB) |
| ¿Carga `libawt.so`? | No | No | **Sí**, y supera su `JNI_OnLoad` |
| `pdfsig` | *Signature is Valid* | *Signature is Valid* | *Signature is Valid* |
| ¿Rúbrica visible en la página? | — | **Sí**, comprobado con `pdftoppm` | **Sí**, comprobado con `pdftoppm` |
| ¿Idéntico bit a bit a la JVM? | **Sí** | **Sí** | **Sí** |

Todas las filas son del **flujo entero**: prefirma nativa → firma PK1 fuera de Java → postfirma
nativa, con los **mismos** `extraParams` y el **mismo** `TIME` en las dos fases (ver «Control de la
restricción»). El PDF de partida es el mismo de siempre (22 páginas, sin firmas previas) y el
certificado, el autofirmado RSA 2048 del banco de pruebas.

## 1. Rúbrica de texto: un solo `.so`, y AWT ni aparece

`testbench/run-visible-ce25.sh <imagen> visible-texto.properties texto 1`, con **únicamente**
`librfirma_crypto.so` y el `loader` en el directorio, bajo
`env -i PATH=/usr/bin:/bin HOME=/tmp` (sin `JAVA_HOME`, sin GraalVM ni JDK en el `PATH`, sin
`DISPLAY`, sin `LD_LIBRARY_PATH`):

```
PRESIGN OK (615 bytes) -> presign.xml
PRE: 218 bytes DER -> PK1: 256 bytes de firma RSA
POSTSIGN OK (178777 bytes de PDF) -> postsign.pdf
IDENTICOS   (nativo == jvm-postsign.pdf)
  - Total document signed
  - Signature Validation: Signature is Valid.
```

Los 178.777 bytes coinciden exactamente con el PDF que el #13 obtuvo en **JVM** para este mismo
caso, que allí era el único motor capaz de producirlo.

`LD_DEBUG=libs` sobre la **postfirma**, lista completa de bibliotecas inicializadas:

```
./librfirma_crypto.so
/lib64/ld-linux-x86-64.so.2
libc.so.6   libm.so.6   libz.so.1
```

Cero `libawt`, `libfontmanager`, `liblcms`, `libjavajpeg`. Es el mismo comportamiento que el #12
midió para la prefirma: en CE 25 la ruta `PdfVisibleAreasUtils.getFont` → `java.awt.Color` →
`Toolkit` **ya no carga AWT nativo** en ninguna de las dos fases. `objdump -p` sobre la imagen
sigue dando solo `libz`, `libm` y `libc` en `NEEDED`.

Sigue haciendo falta `-H:IncludeResources=com/lowagie/text/pdf/fonts/.*`: sin los `.afm` de iText
la rúbrica de texto falla con `Courier not found as resource` (medido en el #12; aquí todas las
imágenes se construyen ya con ese flag).

## 2. Rúbrica de imagen: seis ficheros y los metadatos de la prefirma

`run-visible-ce25.sh <imagen> visible-imagen.properties png 6`, con los seis ficheros en el
directorio y el mismo `env -i`:

```
POSTSIGN OK (179789 bytes de PDF)
IDENTICOS   (nativo == jvm-postsign.pdf)
  - Signature Validation: Signature is Valid.
```

`LD_DEBUG=libs` sobre la **postfirma** con rúbrica de imagen:

```
./librfirma_crypto.so
libawt.so   libawt_headless.so   libjava.so   libjavajpeg.so   libjvm.so
ld-linux, libc, libdl, libm, libz
```

Es decir, **la postfirma carga `libawt.so` y supera su `JNI_OnLoad`**, que es justo donde el
proceso moría en CE 21 (#13, sección 5). No hay ningún segundo frente: las bibliotecas que
inicializa la postfirma son exactamente las cinco que el #12 registró para la prefirma.

### Los metadatos de AWT son los mismos que los de la prefirma

Trazando con el agente (`-agentlib:native-image-agent`) primero **solo la prefirma** y luego
**además la postfirma** sobre la misma sesión (`testbench/trace-awt-metadata.sh`, que deja las dos
instantáneas), la comparación de los `reachability-metadata.json` es:

- Sección `resources`: **idéntica**.
- Sección `reflection`: 37 tipos con solo la prefirma, 39 al añadir la postfirma. Los dos que
  aparecen son `es.gob.afirma.signers.tsp.pkcs7.CMSTimestamper` y
  `es.gob.afirma.signers.tsp.pkcs7.TsaParams` — la rama de sello de tiempo, exactamente las mismas
  clases que el #13 vio aparecer en el árbol de alcanzabilidad al añadir la postfirma.
- **Ningún tipo de AWT nuevo.** Los 16 tipos de AWT/ImageIO (`java.awt.Color`,
  `java.awt.GraphicsEnvironment`, `java.awt.image.BufferedImage`, `ColorModel`, `Raster`,
  `SampleModel`, `SinglePixelPackedSampleModel`, `sun.awt.image.ByteComponentRaster`,
  `sun.java2d.Disposer`, `javax.imageio.ImageIO`, los `spi`, las tablas JPEG y
  `com.sun.imageio.plugins.jpeg.JPEGImageWriter`) ya estaban por la prefirma.

**Respuesta a la pregunta 2 del issue: sirven los mismos metadatos de AWT que la prefirma.**
Regenerar la traza incluyendo la postfirma solo añade las dos clases de TSA, que no son de AWT.

### Los metadatos sí dependen del **formato** de la imagen de rúbrica

Hallazgo lateral, pero relevante para el #6: la traza generada con una rúbrica **PNG** no cubre una
rúbrica **JPEG**. Con esa imagen, el caso JPEG degrada a

```
WARNING: No se pudo normalizar la imagen de rubrica. Se agregara tal cual:
         java.lang.reflect.InvocationTargetException
```

y el PDF sale **válido y con la rúbrica visible**, pero **ya no es idéntico al de la JVM**
(179.531 bytes frente a 179.788): el nativo incrusta el JPEG tal cual y la JVM lo reencoda. Al
trazar también el caso JPEG, el único tipo que se añade es
`com.sun.imageio.plugins.jpeg.JPEGImageReader`; con esa traza mezclada (42 KB), **PNG y JPEG
vuelven a salir idénticos bit a bit a la JVM**. Todas las cifras de este documento son de esa
imagen mezclada.

### Sin metadatos, el caso de imagen no arranca

Con los seis `.so` pero **sin** `-H:ConfigurationFileDirectories`, la **prefirma** muere con un
error no capturado (rc=134), antes incluso de llegar a la postfirma:

```
Exception in thread "main": java.lang.NoClassDefFoundError: java/awt/GraphicsEnvironment
  at com.oracle.svm.core.jni.functions.JNIFunctions$Support.findClassInReflectionDictionary
  ← JNILibraryInitializer.initialize(JNILibraryInitializer.java:123)
  ← java.lang.System.loadLibrary → java.awt.Toolkit.<clinit>
  ← javax.imageio.spi.IIORegistry.getDefaultInstance ← javax.imageio.ImageIO.<clinit>
  ← es.gob.afirma.ui.utils.ImageUtils.normalizeImageToPdf(ImageUtils.java:29)
  ← es.gob.afirma.signers.pades.PdfPreProcessor.getImage(PdfPreProcessor.java:306)
  ← es.gob.afirma.signers.pades.PdfSessionManager.getSessionData(PdfSessionManager.java:108)
```

Es la misma excepción recuperable que describió el #12 (no un `FatalError` como en CE 21), pero
aquí escapa del `@CEntryPoint` y aborta el proceso.

## 3. La rúbrica se ve de verdad en la página

Comprobado, no supuesto. `pdftoppm -r 100 -png` de la página 1 del PDF firmado, comparada píxel a
píxel con la misma página del PDF original:

| Caso | Píxeles distintos / `bbox` en la página | Región pedida en `extraParams` |
|---|---|---|
| Rúbrica de texto | 654 px, `bbox` (142, 858)–(389, 871) | (100, 100)–(300, 180) pt → (139, 850)–(417, 961) px |
| Rúbrica de imagen | `bbox` (142, 850)–(391, 962) | la misma |

Las dos caen dentro del rectángulo pedido. La imagen de diferencia del caso de texto se lee sin
ambigüedad: dice **«Firmado por Prueba rfirma»** en Courier. En el caso de imagen se ve el PNG de
prueba (un degradado de 40×40) estampado en el recuadro, con el `layer2Text` encima.

`pdftotext` **no** extrae el texto de la rúbrica: la apariencia vive en el `appearance stream` del
widget de firma, no en el contenido de la página. Comprobar la rúbrica con `pdftotext` da un falso
negativo; hay que rasterizar.

## 4. Equivalencia bit a bit con la JVM

Alimentando el **mismo** `TriphaseData` (mismo `PRE`, `PK1`, `PID` y `TIME`) a la imagen nativa y a
la JVM, con los **mismos** `extraParams`:

| Caso | Bytes del PDF | `sha256` nativo == JVM |
|---|---|---|
| Sin rúbrica | 177.870 | **sí** |
| Rúbrica de texto | 178.777 | **sí** |
| Rúbrica de imagen PNG | 179.789 | **sí** |
| Rúbrica de imagen JPEG | 179.788 | **sí** (con la traza que incluye el JPEG) |

La imagen nativa de CE 25 no ensambla un PDF distinto del que ensambla AutoFirma en JVM, tampoco
con rúbrica.

## 5. Control de la restricción dura

La restricción medida en el #13 —`extraParams` o instante de firma distintos entre prefirma y
postfirma invalidan la firma **en silencio**— se controló en todas las ejecuciones: el guion pasa
el mismo fichero de `extraParams` a las dos fases y a la JVM de control, y la postfirma consume el
`TriphaseData` de la prefirma sin tocar el `TIME`.

Verificado además que **sigue vigente en CE 25 y también con rúbrica**: alterando el `TIME` en
60 s en el caso de rúbrica de texto, la postfirma completa sin error y

```
Signature Validation: Digest Mismatch.
```

## 6. Conjunto mínimo de ficheros para el flujo completo (prefirma + postfirma)

Medido copiando a un directorio vacío solo los ficheros indicados y ejecutando las **dos** fases
bajo `env -i`. Retirando cada auxiliar de uno en uno, los cinco resultan necesarios; los tres
restantes que `native-image` emite, no.

| Caso | Ficheros a distribuir | Tamaño en disco |
|---|---|---|
| Sin rúbrica y **rúbrica de texto** | **1**: `librfirma_crypto.so` | **35.391.584 B (35,4 MB)** |
| **Rúbrica de imagen** (PNG o JPEG) | **6**: el anterior + `libawt.so`, `libawt_headless.so`, `libjavajpeg.so`, `libjava.so`, `libjvm.so` | **36.610.048 B (36,6 MB)** |

Desglose de los cinco auxiliares: `libawt.so` 924.120 B, `libjavajpeg.so` 239.912 B,
`libawt_headless.so` 35.968 B, `libjava.so` 9.232 B, `libjvm.so` 9.232 B — **1.218.464 B** en
total. Los tres emitidos que **no** hacen falta son `libawt_xawt.so`, `libfontmanager.so` y
`liblcms.so` (comprobado: con los nueve ficheros el resultado es idéntico al de seis).

Quitando cualquiera de los cinco, el caso de imagen degrada siempre igual —
`WARNING: No se pudo normalizar la imagen de rubrica` y luego
`ERROR: Se ha proporcionado una imagen de rubrica que no esta codificada en JPEG` — y la prefirma
falla con rc=3. No hay ninguno prescindible.

**El `reachability-metadata.json` (43.122 B) es de tiempo de construcción: no se distribuye.**

Tamaños de imagen según lo que se compile, para el #6:

| Imagen | Bytes | Sirve para |
|---|---|---|
| `--shared` + `.afm` de iText | 34.736.224 | sin rúbrica y rúbrica de texto |
| `+` metadatos AWT trazados con PNG | 35.326.048 | añade rúbrica de imagen PNG |
| `+` metadatos AWT trazados con PNG **y** JPEG | 35.391.584 | los cuatro casos |

Una **sola** imagen sirve para todo: la de 35.391.584 B ejecuta el caso sin rúbrica y el de texto
con un único fichero, y el de imagen añadiendo los cinco auxiliares. Los metadatos cuestan
**+655.360 B (+1,9 %)** sobre la imagen sin ellos y no penalizan los casos que no los usan.

## 7. Subir a CE 25 no rompe nada de lo que midió el #13

| Lo que midió el #13 en CE 21 | En CE 25 |
|---|---|
| Postfirma sin rúbrica completa con **un solo `.so`**, sin Java | **Igual**, 177.870 bytes |
| PDF válido (`pdfsig`: *Signature is Valid*, *Total document signed*) | **Igual** |
| Idéntico bit a bit al de la JVM | **Igual** |
| AWT no aparece en la traza de la postfirma sin rúbrica | **Igual** |
| `TIME` desparejado → `Digest Mismatch` en silencio | **Igual** |
| Postfirma con rúbrica: **aborto fatal** en `libawt.so` | **Ya no**: completa |

No hay ninguna regresión medida. El `pom.xml` **no se ha tocado**: sigue compilando con
`source/target 21` y `graal-sdk` 23.1.0 en `provided`; se compiló el `jar` con el JDK 25 y se pasó
al `native-image` de la línea 25 sin cambios, igual que hizo el #12.

## 8. Cambios en el banco de pruebas

Tres guiones nuevos en `rfirma-native-bridge/testbench/`, ninguno destructivo de los anteriores:

- `run-visible-ce25.sh <dir-imagen> <properties> <etiqueta> [1|6]`: flujo trifásico **completo** en
  nativo con rúbrica visible, con 1 o 6 `.so` en el directorio, más el control en JVM con la misma
  sesión y la comparación bit a bit y con `pdfsig`.
- `trace-awt-metadata.sh [properties] [dir-salida]`: genera los metadatos de alcanzabilidad de AWT
  ejercitando en JVM las **dos** fases, y deja aparte la instantánea de solo la prefirma para poder
  compararlas.
- `build-native-awt.sh [dir-imagen] [dir-config]`: construye la imagen con los `.afm` de iText y
  `-H:ConfigurationFileDirectories`.

Las fixtures siguen siendo las del #2/#12 (`target/fixtures/`, bajo `target/`, ignorado):
`test.pdf`, el par `cert.pem`/`key.pem`, `rubrica.png`, `rubrica.jpg` y los tres `.properties`.

## No determinado

- **Validación ante un validador oficial** (VALIDe, cadena confiada). Aquí solo hay validez
  estructural con `pdfsig` y un certificado autofirmado. Igual que en el #13.
- **Sello de tiempo (`TS_TYPE`/`TSA_URL`)**: la postfirma vuelve a hacer alcanzables
  `CMSTimestamper` y `TsaParams`, y ahora además aparecen en los metadatos del agente, pero **no se
  ha ejecutado ninguna postfirma contra una TSA**.
- **PDFs difíciles** (cifrados, con permisos, ya firmados) y **certificados EC**: sin medir.
- **Formatos de rúbrica distintos de PNG y JPEG**: sin medir. El hallazgo de la sección 2 dice que
  cada formato de entrada puede exigir su `ImageReader` en los metadatos; qué formatos se admiten
  es una decisión del inventario de capacidades, no de esta medición.
- **Otras plataformas**: solo Linux x86_64.
- **Por qué en CE 25 la rúbrica de texto no carga `libawt.so`**: sigue sin verificarse en el código
  de Substrate VM, igual que lo dejó el #12.

## Cómo reproducir

```bash
export GRAALVM_HOME=$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce
cd rfirma-native-bridge && JAVA_HOME=$GRAALVM_HOME mvn package -DskipTests

# metadatos de AWT (PNG; para JPEG, repetir con visible-jpeg.properties y config-merge-dir)
bash testbench/trace-awt-metadata.sh ../target/fixtures/visible-imagen.properties awt-config

bash testbench/build-native-awt.sh ce25-awt awt-config      # imagen para los cuatro casos
bash testbench/run-native.sh       ce25-awt                 # sin rúbrica, 1 .so, env -i
bash testbench/run-visible-ce25.sh ce25-awt ../target/fixtures/visible-texto.properties  texto  1
bash testbench/run-visible-ce25.sh ce25-awt ../target/fixtures/visible-imagen.properties png    6

# la rúbrica se ve: rasterizar y comparar con el PDF original
pdftoppm -f 1 -l 1 -r 100 -png target/lab-texto/texto-nativo.pdf tx
pdftoppm -f 1 -l 1 -r 100 -png ../target/fixtures/test.pdf base
```
