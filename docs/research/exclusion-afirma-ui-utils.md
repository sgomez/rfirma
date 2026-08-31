# Excluir `afirma-ui-utils` devuelve la imagen nativa a un solo fichero

Medición para el issue [#36](https://github.com/sgomez/rfirma/issues/36). Verifica la decisión
que tomó el [ADR-0012](../adr/0012-normalizacion-de-la-rubrica-en-rust.md) —normalizar la rúbrica
en Rust y excluir `afirma-ui-utils` del `pom.xml` del puente— contra la medición de partida del
[#14](https://github.com/sgomez/rfirma/issues/14), que dejó el caso de rúbrica de imagen en
**seis `.so` y 36,6 MB**. **Registra hechos**; la decisión ya está tomada y aquí solo se comprueba
si se sostiene.

Entorno: GraalVM CE 25.3.4.1 (`native-image 25.0.4.1 2026-08-18`, Substrate VM, serial GC), Maven
3.9.12, Linux x86_64, `rustc 1.98.0`, crate `image` 0.25.10, poppler (`pdfsig`, `pdftoppm`) como
validador y visor. Banco de pruebas: `rfirma-native-bridge/testbench/`, con tres guiones nuevos.

## Veredicto

**El ADR-0012 se confirma, y por un margen mayor del que anticipaba.** Con `afirma-ui-utils`
excluido y **sin** `-H:ConfigurationFileDirectories`, el ciclo trifásico completo con rúbrica de
imagen firma con **un solo `.so`**, `pdfsig` lo valida, la rúbrica se ve rasterizada, y AWT no
aparece en la traza del enlazador. La imagen baja de **35.391.584 B a 27.723.872 B**: no es solo
que se ahorren los cinco auxiliares, es que **la propia imagen adelgaza 7,7 MB**, que el issue no
esperaba.

Lo que se pierde es lo que el ADR-0012 ya había anunciado: el PDF con rúbrica de imagen deja de
ser idéntico bit a bit al que ensambla AutoFirma en una JVM completa. **Sigue siéndolo** frente a
una JVM con el mismo recorte de dependencias, lo que conserva el criterio de verificación en una
forma útil (sección 5).

Y aparece una trampa que el ADR no contemplaba: **el modo de fallo del JPEG con perfil ICC depende
de qué `.so` haya en el directorio**. Con un fichero es un error recuperable; con los auxiliares
del ADR-0004 al lado es un **aborto del proceso** (sección 4).

## Resumen

| | Sin rúbrica | Rúbrica de **texto** | Rúbrica de **imagen** (JPEG) | JPEG **con perfil ICC** | Rúbrica **PNG** |
|---|---|---|---|---|---|
| ¿Completa el flujo completo en nativo? | **Sí** | **Sí** | **Sí** | **No** | **No** |
| `.so` necesarios | **1** | **1** | **1** | — | — |
| ¿Metadatos del agente de trazado? | No | No | **No** | — | — |
| ¿Carga `libawt.so`? | No | No | **No** | intenta y falla | No |
| `pdfsig` | *Signature is Valid* | *Signature is Valid* | *Signature is Valid* | — | — |
| ¿Rúbrica visible en la página? | — | Sí (`pdftoppm`) | **Sí** (`pdftoppm`) | — | — |
| ¿Idéntico a la JVM **sin** `ui-utils`? | Sí | Sí | **Sí** | — | — |
| ¿Idéntico a la JVM **con** `ui-utils`? | Sí | Sí | **No** | — | — |

Todas las filas son del **flujo entero** —prefirma nativa → firma PK1 fuera de Java → postfirma
nativa— bajo `env -i PATH=/usr/bin:/bin HOME=/tmp`, sin `JAVA_HOME`, sin JDK en el `PATH`, sin
`DISPLAY` y sin `LD_LIBRARY_PATH`, con los mismos `extraParams` y el mismo `TIME` en las dos
fases. PDF de partida: 22 páginas A4 sin firmas previas; certificado autofirmado RSA 2048.

Que el caso **PNG falle es lo correcto y era el objetivo**: al no estar `ImageUtils`, nadie
reencoda, y `new Jpeg(bytes)` recibe un PNG. Es exactamente el contrato que fija el ADR-0012 —el
puente recibe un JPEG ya normalizado por Rust— convertido en un fallo temprano y ruidoso en vez de
en una conversión silenciosa.

## 1. La exclusión es legal y la degradación es la que dice el issue

`afirma-crypto-pdf` declara `afirma-ui-utils` con `<scope>runtime</scope>`, y `PdfPreProcessor` lo
invoca por reflexión, no por referencia estática
(`clienteafirma/afirma-crypto-pdf/src/main/java/es/gob/afirma/signers/pades/PdfPreProcessor.java:304-311`):

```java
final Class<?> ImageUtilsClass = Class.forName("es.gob.afirma.ui.utils.ImageUtils");   // :304
final Method normalizeImageToPdfMethod = ImageUtilsClass.getMethod("normalizeImageToPdf", byte[].class);
final Object normalizedImageObject = normalizeImageToPdfMethod.invoke(null, image);
normalizedImage = (byte[]) normalizedImageObject;
}
catch (final Throwable e) {                                                            // :309
    LOGGER.log(Level.WARNING, "No se pudo normalizar la imagen de rubrica. Se agregara tal cual: " + e);
    normalizedImage = image;
}
```

La pista del issue es correcta: el `catch` es de `Throwable`, así que recoge también el
`ClassNotFoundException`. Verificado en ejecución — con la exclusión puesta, la salida de la
prefirma nativa dice, literalmente:

```
WARNING: No se pudo normalizar la imagen de rubrica. Se agregara tal cual:
  java.lang.ClassNotFoundException: es.gob.afirma.ui.utils.ImageUtils. This exception was
  synthesized during native image building from a call to java.lang.Class.forName(String)
  with constant arguments.
```

La frase final es el hallazgo útil: **`native-image` resuelve el `Class.forName` en tiempo de
construcción** y sustituye la llamada por el lanzamiento de la excepción. No queda reflexión en
tiempo de ejecución, y por eso el árbol de alcanzabilidad no arrastra `ImageUtils` ni, tras él,
`javax.imageio.ImageIO`.

En el `pom.xml` la exclusión son ocho líneas dentro de la única dependencia funcional
(`rfirma-native-bridge/pom.xml`), y basta para que `afirma-ui-utils-1.9.1.jar` desaparezca de
`target/cp.txt`.

## 2. Cifras: de seis ficheros a uno, y 7,7 MB menos de imagen

El conjunto mínimo se determinó **por eliminación empírica**, como en el #14: copiando a un
directorio vacío solo `librfirma_crypto.so` y el `loader`, y ejecutando las **dos** fases bajo
`env -i`. No hay ningún auxiliar que quitar porque no hace falta ninguno.

| Configuración | Ficheros a distribuir | Bytes a distribuir |
|---|---|---|
| #14, rúbrica de texto (con `ui-utils`, con metadatos) | 1 | 35.391.584 |
| #14, rúbrica de **imagen** (con `ui-utils`, con metadatos) | **6** | **36.610.048** |
| **Este issue: los cuatro casos, sin `ui-utils`, sin metadatos** | **1** | **27.723.872** |

**Ahorro frente al caso de imagen del #14: 8.886.176 B (−24,3 %) y cinco ficheros.**

Tamaños de imagen construidos aquí, todos con el mismo `native-image`:

| Imagen | Bytes | Sirve para |
|---|---|---|
| `--shared` + `.afm` de iText, **con** `ui-utils` | 34.736.224 | sin rúbrica y texto (reproduce el #14 al byte) |
| `--shared` + `.afm`, **sin** `ui-utils` | **27.723.872** | los tres casos que funcionan |
| `--shared` **sin** `.afm`, sin `ui-utils` | 27.199.584 | nada: falla el `Courier` (sección 6) |

La primera fila coincide **exactamente** con los 34.736.224 B que el #14 registró para
«`--shared` + `.afm` de iText», lo que confirma que este entorno reproduce aquella medición y que
la comparación es legítima.

**Excluir `afirma-ui-utils` quita 7.012.352 B a la imagen** (34.736.224 → 27.723.872, −20,2 %) y
hace innecesarios los +655.360 B de metadatos de AWT que el #14 midió. La cifra sorprende para un
módulo de 90 líneas: lo que se va no es `ImageUtils`, es el subárbol de `javax.imageio` y
`java.awt.image` que arrastraba.

Se nota también en lo que `native-image` **deja de emitir**: con `ui-utils` emite ocho auxiliares;
sin él, seis. Desaparecen `libjavajpeg.so` (239.912 B) y `libfontmanager.so` (2.017.552 B). Los
seis que sigue emitiendo —`libawt.so`, `libawt_headless.so`, `libawt_xawt.so`, `libjava.so`,
`libjvm.so`, `liblcms.so`— **no se cargan nunca** en ninguno de los tres casos que funcionan; el
#14 ya había avisado de que lo que emite el compilador no es lo que hace falta distribuir.

`LD_DEBUG=libs` sobre la **postfirma** con rúbrica de imagen y un solo fichero en el directorio,
lista completa de bibliotecas inicializadas:

```
./librfirma_crypto.so
/lib64/ld-linux-x86-64.so.2
libc.so.6   libm.so.6   libz.so.1
```

Idéntica a la que el #14 registró para la rúbrica de **texto**. Cero `libawt`, cero `liblcms`,
cero `libjavajpeg`.

## 3. La rúbrica se ve de verdad en la página

Comprobado rasterizando, no con `pdftotext` —que da falso negativo, como avisó el #14, porque la
apariencia vive en el *appearance stream* del widget de firma y no en el contenido de la página—.
`pdftoppm -f 1 -l 1 -r 100 -png` de la página 1 del PDF firmado, comparada píxel a píxel con la
misma página del PDF original (`testbench/diff-pagina.py`):

| Caso | Píxeles distintos | `bbox` en la página | Región pedida en `extraParams` |
|---|---|---|---|
| Rúbrica de texto | 889 | (142, 927)–(391, 942) | (100, 100)–(300, 180) pt → (139, 919)–(417, 1031) px |
| Rúbrica de imagen (JPEG de Rust) | 13.051 | (142, 919)–(391, 1031) | la misma |

Las dos caen dentro del rectángulo pedido. El recorte de la región distinta se inspeccionó
visualmente: se ve el degradado de 40×40 estampado, con el texto «Firmado por Prueba rfirma» en
Courier encima. El guion deja el PNG del recorte en `target/lab-<etiqueta>/<etiqueta>-rubrica.png`
para que la comprobación no sea de fe.

## 4. El JPEG con perfil ICC: dos modos de fallo, y uno es una trampa

El ADR-0012 exige emitir el JPEG **sin** perfil ICC porque `com.aowagie.text.Jpeg` parsea el
segmento APP2 y construye un `java.awt.color.ICC_Profile`. La regla es correcta, y además es la
**única** que hace falta: desensamblando el `.class` del `jar` de iText,

```
javap -p -c -classpath ~/.m2/.../afirma-lib-itext-1.7.jar com.aowagie.text.Jpeg \
  | grep -o "java/awt/[A-Za-z/_]*" | sort -u
```

devuelve **una sola** referencia a AWT en toda la clase: `java/awt/color/ICC_Profile`. (Para
contraste, `com.aowagie.text.Image` referencia además `Color`, `Image`, `BufferedImage` y
`PixelGrabber`, pero por sus `getInstance(java.awt.Image)`, que no están en este camino: aquí se
llega por `new Jpeg(byte[])`.)

El bytecode también dice que la llamada está mal protegida para nuestro caso: el único `catch` que
la envuelve es de `IllegalArgumentException`, así que un `Error` la atraviesa.

```
930: aload_3
931: invokestatic  java/awt/color/ICC_Profile.getInstance:([B)Ljava/awt/color/ICC_Profile;
934: astore 4
...
Exception table:  930 942 945 Class java/lang/IllegalArgumentException
```

Alimentando un JPEG idéntico salvo por un perfil sRGB incrustado en APP2
(`target/fixtures/rubrica-icc.jpg`, generado con Pillow), la prefirma nativa falla, y **falla de
dos maneras distintas según lo que haya en el directorio**:

| Ficheros en el directorio | `rc` | Salida |
|---|---|---|
| Solo `librfirma_crypto.so` (lo que distribuye rFirma) | **3** | `ERROR:java.lang.UnsatisfiedLinkError: Can't load library: awt \| java.library.path = [/usr/lib64, /lib64, /lib, /usr/lib]` |
| Los seis auxiliares que emite `native-image` al lado | **134** | `Exception in thread "main": java.lang.NoClassDefFoundError: java/awt/GraphicsEnvironment` |

El primero es un error recuperable: lo recoge el `catch (Throwable)` del `@CEntryPoint` y sale por
el valor de retorno como una cadena `ERROR:...`. El segundo **aborta el proceso**, con esta pila
(`env -i ... ./loader ./librfirma_crypto.so presign ...`, traza recortada):

```
java.lang.NoClassDefFoundError: java/awt/GraphicsEnvironment
  at ...JNIFunctions$Support.findClassInReflectionDictionary(JNIFunctions.java:2053)
  at ...JNILibraryInitializer.initialize(JNILibraryInitializer.java:123)
  at java.base/java.lang.System.loadLibrary(System.java:1686)
  at java.desktop/sun.java2d.cmm.lcms.LCMS.getModule(LCMS.java:155)
  at java.desktop/sun.java2d.cmm.CMSManager.getModule(CMSManager.java:37)
  at java.desktop/java.awt.color.ICC_Profile.getInstance(ICC_Profile.java:809)
  at com.aowagie.text.Jpeg.processParameters(Jpeg.java:323)
  at com.aowagie.text.Jpeg.<init>(Jpeg.java:131)
  at es.gob.afirma.signers.pades.PdfPreProcessor.getImage(PdfPreProcessor.java:316)
  at es.gob.afirma.signers.pades.PdfSessionManager.getSessionData(PdfSessionManager.java:108)
  at es.gob.afirma.signers.pades.PAdESTriPhaseSigner.preSign(PAdESTriPhaseSigner.java:176)
```

Es el mismo `rc=134` que el #14 midió «con los seis `.so` pero sin metadatos», y ahora se sabe por
qué: **el aborto no lo causa la falta de metadatos, lo causa que `libawt.so` esté disponible para
cargarse**. Sin ella, la carga falla antes y el fallo es un `Error` normal que el puente atrapa.

**Trampa, y la más importante de este informe:** el `ADR-0004` y el manifiesto de flatpak del
[#22](https://github.com/sgomez/rfirma/issues/22) instalan los **seis** ficheros en
`/app/lib/rfirma/`. Si esa lista no se recorta a uno, un JPEG con perfil ICC que se cuele hasta el
puente **mata el proceso**, y como la librería se carga dentro del ejecutable de Tauri, se lleva la
aplicación entera por delante. Con un solo fichero instalado el mismo JPEG produce un error que se
puede enseñar al usuario. Es decir: **el recorte del empaquetado no es solo una optimización de
tamaño, es lo que convierte un aborto en un error**.

Esto no es un fallo silencioso, pero sí es una trampa de configuración: el modo de fallo cambia
según ficheros que ni el código Java ni el de Rust mencionan.

Nota de contraste: en una JVM con `afirma-ui-utils` presente el JPEG con ICC **no da ningún
problema**, porque `ImageUtils.normalizeImageToPdf` lo reencoda y de paso le quita el perfil. Por
eso AutoFirma no ha tropezado nunca con esto y no hay nada al respecto en su código.

## 5. El JPEG del crate de Rust pasa por `new Jpeg(...)`

**Crate elegido: `image` 0.25.10**, con `default-features = false` y solo `["png", "jpeg"]`.
Razones, en este orden:

1. Es el decodificador/codificador de referencia del ecosistema y cubre **de una pieza** las
   cuatro operaciones que pide el ADR-0012: decodificar PNG y JPEG, aplanar el alfa, reescalar y
   volver a codificar en JPEG. `jpeg-encoder` solo codifica —habría que emparejarlo con un
   decodificador de PNG y otro de JPEG—, y `mozjpeg` es un enlace a la biblioteca de C, que
   contradice la razón de ser de este cambio: quitar dependencias nativas del empaquetado.
2. Sin `default-features` no arrastra los demás formatos, así que la superficie de decodificación
   queda exactamente en los dos que decide el ADR-0012, sin depender de disciplina en el código.

El prototipo vive en `rfirma-native-bridge/testbench/rubrica-rs/` (no es código de producción:
existe para esta medición) e implementa las constantes del ADR —calidad 90, lado mayor 1000 px,
tope de entrada 10 MB, fondo blanco— y el filtrado por contenido, no por extensión
(`ImageReader::with_guessed_format`).

El JPEG que emite es **JFIF baseline pelado**: los segmentos son `APP0(JFIF)`, `SOF0`, dos `DQT`,
cuatro `DHT`, `SOS`. **No hay `APP2`**, que es la condición que exige la sección 4.

Alimentado al ciclo completo con un solo `.so`:

```
POSTSIGN OK (60049 bytes de PDF) -> postsign.pdf
nativo == jvm-sin-uiutils: IDENTICOS
nativo == jvm-con-uiutils: DIFIEREN
  - Total document signed
  - Signature Validation: Signature is Valid.
pixeles distintos: 13051   bbox: (142, 919, 391, 1031)
```

Y el aplanado del alfa coincide con el del original. Comprobado con un PNG que tiene un cuadrante
totalmente transparente (`target/fixtures/alfa.png`), pasándolo por
`ImageUtils.normalizeImageToPdf` en una JVM y por el prototipo de Rust:

| | Zona transparente | Zona opaca (rojo puro) |
|---|---|---|
| `ImageUtils` (Java) | (255, 255, 255) | (254, 0, 3) |
| `image` (Rust) | (255, 255, 255) | (254, 0, 0) |

**El fondo blanco que afirma el ADR-0012 es real**, medido y no deducido del código comentado. Las
diferencias en la zona opaca son ruido de cuantización JPEG entre dos codificadores distintos.

## 6. Equivalencia bit a bit: se pierde contra AutoFirma, se conserva contra el recorte

El guion nuevo ejecuta **dos** controles en JVM sobre la misma sesión trifásica: uno con
`afirma-ui-utils` en el *classpath* (AutoFirma tal cual) y otro sin él (el mismo recorte que la
imagen nativa). Así se distingue «el nativo ensambla otro PDF» de «excluir el módulo ensambla otro
PDF».

| Caso | Bytes (nativo) | == JVM **sin** `ui-utils` | == JVM **con** `ui-utils` |
|---|---|---|---|
| Sin rúbrica | 57.933 | **sí** | **sí** |
| Rúbrica de texto | 58.806 | **sí** | **sí** |
| Rúbrica de imagen, JPEG de Pillow | 59.931 | **sí** | no (la JVM da 59.822) |
| Rúbrica de imagen, JPEG de Rust | 60.049 | **sí** | no (la JVM da 59.823) |

**Se cumple lo que anticipaba el #32 y recoge el ADR-0012**: el PDF con rúbrica de imagen deja de
ser idéntico al de AutoFirma en JVM, porque el JPEG incrustado ya no lo produce `ImageIO`. Pero la
pérdida es menor de lo que parecía: **la imagen nativa sigue siendo bit a bit indistinguible de
una JVM con las mismas dependencias**, así que el criterio de equivalencia que usaron el #13, el
#14 y el #23 sigue sirviendo como prueba de regresión; lo único que cambia es contra qué se
compara. Quien escriba las pruebas de firma debe fijar el control en «JVM con el classpath del
puente», no en «AutoFirma».

Los dos casos que no llevan imagen (**sin rúbrica y rúbrica de texto**) siguen siendo idénticos a
las dos JVM: para ellos la exclusión **no cambia absolutamente nada** salvo que la imagen pesa 7 MB
menos.

## 7. Lo que sigue haciendo falta declarar en tiempo de construcción

**Formatos de imagen: ya no.** El hallazgo lateral del #14 —que la traza de una rúbrica PNG no
cubre una JPEG, y que por tanto la lista de formatos admitidos quedaba congelada en el comando de
`native-image`— **desaparece**, porque no hay traza ni metadatos. Los formatos pasan a ser una
característica del crate de Rust en tiempo de ejecución, que es justo lo que buscaba el ADR-0012.

**Los `.afm` de iText: sí, y ahora también para la rúbrica de imagen.** Construyendo sin
`-H:IncludeResources=com/lowagie/text/pdf/fonts/.*`, los dos casos con rúbrica fallan igual:

```
ERROR:com.aowagie.text.ExceptionConverter: Courier not found as resource.
  (The *.afm files must exist as resources in the package com.aowagie.text.pdf.fonts)
```

Es la misma exigencia que midió el #12 para la rúbrica de texto y no la toca esta exclusión: el
`layer2Text` se compone con Courier también cuando hay imagen. Ese flag cuesta 524.288 B
(27.199.584 → 27.723.872).

## 8. Cambios en el banco de pruebas y en el `pom.xml`

**El cambio en `rfirma-native-bridge/pom.xml` queda commiteado como propuesta**, no revertido: es
la implementación literal de lo que decidió el ADR-0012, con un comentario que apunta a este
documento. Son ocho líneas de `<exclusions>` dentro de la dependencia de
`afirma-server-triphase-signer-core`. Si el ADR-0012 no llegara a mergearse, hay que quitarlas.

Tres guiones nuevos en `rfirma-native-bridge/testbench/`, ninguno destructivo de los anteriores:

- `make-fixtures.sh`: genera `target/fixtures/` desde cero —el PDF de 22 páginas, el par
  `cert.pem`/`key.pem`, las rúbricas y los `.properties`—. Los tickets #2, #12, #13 y #14 daban
  esas fixtures por existentes pero **ninguno las dejó escritas en un guion**; sin esto, el banco
  no se podía reproducir en una máquina limpia. Añade `rubrica-icc.jpg` (con APP2),
  `rubrica-rust.jpg` (del prototipo de Rust) y `alfa.png` (con un cuadrante transparente).
- `run-noui-ce25.sh <dir-imagen> <properties> <etiqueta> [lista-so]`: ciclo trifásico completo en
  nativo con la lista de `.so` auxiliares que se le pase (vacía = un solo fichero), más los **dos**
  controles en JVM de la sección 6, la comparación bit a bit, `pdfsig` y el rasterizado.
- `diff-pagina.py`: compara dos rasterizados de la misma página, da el `bbox` y el número de
  píxeles distintos, y deja un recorte ampliado de la región que cambia para mirarlo.

Y un prototipo de Rust en `testbench/rubrica-rs/` (sección 5).

## No determinado

- **No se ha reconstruido la imagen de 35.391.584 B del #14** (la que lleva los metadatos de AWT
  trazados con PNG y JPEG). Esa cifra se cita del #14. Sí se reconstruyó la de 34.736.224 B, que
  coincide al byte, así que el entorno reproduce aquella medición.
- **Validación ante un validador oficial** (VALIDe, cadena confiada): sigue sin medirse, igual que
  en el #13 y el #14. Aquí solo hay validez estructural con `pdfsig` y un certificado autofirmado.
- **Sello de tiempo (`TS_TYPE`/`TSA_URL`)**: ninguna postfirma se ha ejecutado contra una TSA.
- **PDFs difíciles** (cifrados, con permisos, ya firmados) y **certificados EC**: sin medir.
- **El reescalado de imágenes grandes** (>1000 px) del prototipo de Rust se ejecuta pero **no se ha
  medido** contra ninguna referencia de calidad; solo se ha comprobado que el camino de 40×40 no
  lo activa.
- **Otras plataformas**: solo Linux x86_64. Que `native-image` deje de emitir `libjavajpeg.so` y
  `libfontmanager.so` es un dato de esta plataforma.
- **Si algún otro camino del puente vuelve a hacer alcanzable `ImageIO`** (por ejemplo si algún día
  se instancia `PreProcessorFactory` en vez de `PAdESTriPhasePreProcessor` a pelo, como avisa
  `NativeBridge.java:23-29`): no se ha comprobado. La cifra de 27.723.872 B vale para el puente tal
  como está hoy.
- **Cuánto de los 7 MB es `javax.imageio` y cuánto `java.awt.image`**: no se ha desglosado. Haría
  falta `-H:+PrintAnalysisCallTree` sobre las dos imágenes y comparar.

## Cómo reproducir

```bash
export GRAALVM_HOME=$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce

# fixtures (incluye el JPEG del crate de Rust si hay cargo)
bash rfirma-native-bridge/testbench/make-fixtures.sh

# el jar, ya con la exclusion de afirma-ui-utils del pom
cd rfirma-native-bridge && JAVA_HOME=$GRAALVM_HOME mvn -B package -DskipTests && cd ..
grep -c afirma-ui-utils rfirma-native-bridge/target/cp.txt   # 0

# la imagen: .afm de iText, SIN -H:ConfigurationFileDirectories
bash rfirma-native-bridge/testbench/build-native-fonts.sh ce25-noui
stat -c %s rfirma-native-bridge/target/ce25-noui/librfirma_crypto.so   # 27723872

# los tres casos que funcionan, con UN SOLO .so, bajo env -i
F=$PWD/target/fixtures
bash rfirma-native-bridge/testbench/run-noui-ce25.sh ce25-noui $F/sin-rubrica.properties    sin
bash rfirma-native-bridge/testbench/run-noui-ce25.sh ce25-noui $F/visible-texto.properties  texto
bash rfirma-native-bridge/testbench/run-noui-ce25.sh ce25-noui $F/visible-rust.properties   rust

# los dos que fallan, y por que
bash rfirma-native-bridge/testbench/run-noui-ce25.sh ce25-noui $F/visible-imagen.properties png   # PNG: no es JPEG
bash rfirma-native-bridge/testbench/run-noui-ce25.sh ce25-noui $F/visible-icc.properties    icc   # rc=3
bash rfirma-native-bridge/testbench/run-noui-ce25.sh ce25-noui $F/visible-icc.properties    icc6 \
    "libawt.so libawt_headless.so libjava.so libjvm.so liblcms.so libawt_xawt.so"                 # rc=134

# la rubrica se ve: el guion deja el recorte ampliado
xdg-open rfirma-native-bridge/target/lab-rust/rust-rubrica.png

# AWT no aparece en la traza del enlazador
cd rfirma-native-bridge/target/lab-rust && env -i PATH=/usr/bin:/bin HOME=/tmp LD_DEBUG=libs \
    ./loader ./librfirma_crypto.so postsign $F/test.pdf.b64 $F/cert.b64 signed.xml \
    $F/visible-rust.properties 2>&1 | grep -o "calling init: .*" | sort -u

# el .afm sigue haciendo falta (construir sin el flag y repetir 'texto')
# la unica referencia a AWT que queda en com.aowagie.text.Jpeg
javap -p -c -classpath ~/.m2/repository/es/gob/afirma/lib/afirma-lib-itext/1.7/afirma-lib-itext-1.7.jar \
    com.aowagie.text.Jpeg | grep -o "java/awt/[A-Za-z/_]*" | sort -u
```
