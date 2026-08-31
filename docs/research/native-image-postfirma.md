# La postfirma PAdES en la imagen nativa de GraalVM

Medición para el issue [#13](https://github.com/sgomez/rfirma/issues/13). Continúa el
[#2](https://github.com/sgomez/rfirma/issues/2), que midió solo la prefirma. **Registra hechos, no
decide** el modelo de distribución: eso es el [#6](https://github.com/sgomez/rfirma/issues/6).

Entorno: GraalVM CE 21+35.1 (Substrate VM, serial GC), Maven 3.9.12, Linux x86_64, poppler `pdfsig`
como validador. Banco de pruebas: `rfirma-native-bridge/` en esta rama, ampliado con un segundo
`@CEntryPoint` (`rfirma_pades_postsign`) y los guiones de `rfirma-native-bridge/testbench/`.

## Resumen

| | Postfirma **sin** rúbrica visible | Postfirma **con** rúbrica visible |
|---|---|---|
| ¿Completa en nativo? | **Sí** | **No** |
| `.so` necesarios | **1** (`librfirma_crypto.so`) | no existe conjunto que funcione |
| ¿Necesita Java/`JAVA_HOME`? | **No** (ejecutado bajo `env -i`) | — |
| ¿Toca AWT en ejecución? | **No**, ni una librería AWT cargada | **Sí**, aborto fatal en `libawt.so` |
| ¿PDF válido? | **Sí**, `pdfsig`: *Signature is Valid* | — |
| Crecimiento de la imagen | **+78.024 bytes (+0,15 %)** | — |

**Veredicto**: la postfirma sin rúbrica visible funciona en nativo y el daño de AWT queda acotado
exactamente donde lo dejó el #2 — en la rúbrica visible. Pero la rúbrica rompe **las dos** fases,
no solo la prefirma, y por la **misma causa**.

## 1. La postfirma sin rúbrica completa con un solo `.so`, sin Java

Recorrido completo del contrato trifásico con el banco de pruebas:

1. **Prefirma** en la imagen nativa → `TriphaseData` de 615 bytes con `PRE`, `NEED_PRE`, `TIME` y `PID`.
2. **Firma** fuera de Java: el campo `PRE` (218 bytes de DER de atributos firmados CAdES) se firma
   con la clave RSA 2048 de prueba mediante `openssl dgst -sha256 -sign`, equivalente a
   `Signature.getInstance("SHA256withRSA")` y a `CKM_SHA256_RSA_PKCS`
   (ver `docs/research/pkcs11-mecanismo-firma.md`). Resultado: 256 bytes, depositados en `PK1`.
3. **Postfirma** en la imagen nativa con el mismo PDF, el mismo certificado, los mismos
   `extraParams` (vacíos) y el `TriphaseData` completo → **PDF firmado de 177.870 bytes**.

Condiciones de la ejecución, idénticas a las del #2:

- Directorio de trabajo con **únicamente** `librfirma_crypto.so` y el `loader` en C.
- `env -i PATH=/usr/bin:/bin HOME=/tmp`: sin `JAVA_HOME`, sin GraalVM ni JDK en el `PATH`, sin
  `LD_LIBRARY_PATH`, sin `DISPLAY`.
- Carga con `dlopen(RTLD_NOW|RTLD_LOCAL)`, igual que hará `libloading` en Rust (ADR-0004).

`objdump -p librfirma_crypto.so | grep NEEDED` sigue devolviendo solo `libz.so.1` y `libc.so.6`.

## 2. El PDF es estructuralmente válido — comprobado

`pdfsig` (poppler) sobre el PDF producido **por la imagen nativa**:

```
Signature #1:
  - Signature Field Name: Signature1
  - Signer Certificate Common Name: Prueba rfirma
  - Signing Hash Algorithm: SHA-256
  - Signature Type: ETSI.CAdES.detached
  - Signed Ranges: [0 - 233], [54235 - 177870]
  - Total document signed
  - Signature Validation: Signature is Valid.
  - Certificate Validation: Certificate issuer isn't Trusted.
```

*Signature is Valid* + *Total document signed*: el `ByteRange` cubre el documento entero y el
resumen firmado cuadra. *Certificate issuer isn't Trusted* es lo esperado: el certificado del banco
de pruebas es autofirmado, no encadena a ninguna raíz del sistema. Esto **no** es una validación
ante un validador oficial (eso es otro ticket, con el certificado de la FNMT del #5); es la
comprobación estructural que pedía la pregunta 2.

**Equivalencia bit a bit con la JVM**: alimentando la **misma** sesión trifásica (mismo `PRE`,
`PK1`, `PID` y `TIME`) a la imagen nativa y a la JVM, los dos PDF son idénticos —
`sha256 414fcdf5619428f01ff7e3df823b1ac64225d91abcb9f62255328d3f0a531ed6` en ambos casos, `cmp` sin
diferencias. La imagen nativa no ensambla un PDF distinto del que ensambla AutoFirma.

**Control en JVM** (`testbench/run-jvm-control.sh`): el mismo recorrido de tres pasos con
`java -cp ...` da también `Signature is Valid`. El montaje de la prueba es correcto y no hay ningún
fallo atribuible a AutoFirma.

**Confirmación de la restricción dura** de `docs/research/firma-visible-trifasica.md`: repitiendo la
postfirma con el `TIME` alterado en 60 segundos respecto al de la prefirma, la postfirma **completa
sin lanzar ningún error** y produce un PDF del mismo tamaño, pero `pdfsig` dice
`Signature Validation: Digest Mismatch`. La firma queda inválida en silencio, tal y como estaba
previsto. (`testbench/run-mismatch.sh`)

## 3. AWT no aparece en la ruta de la postfirma sin rúbrica

`LD_DEBUG=libs` sobre la postfirma nativa sin rúbrica. Librerías inicializadas, lista completa:

```
./librfirma_crypto.so
/lib64/ld-linux-x86-64.so.2
/usr/lib/x86_64-linux-gnu/libc.so.6
/usr/lib/x86_64-linux-gnu/libz.so.1
```

Cero coincidencias de `libawt`, `libfontmanager`, `liblcms`, `libjavajpeg` o `libmlib_image` en toda
la traza. La postfirma no intenta cargar AWT.

**Alcanzabilidad estática**: comparando los informes `used_classes` de la imagen con solo prefirma y
de la imagen con prefirma + postfirma, añadir la postfirma suma **16 clases** y quita 11 (formas
lambda y proxies que se reordenan). Las clases de AutoFirma nuevas son exactamente cuatro:
`PdfTimestamper`, `CMSTimestamper`, `TsaParams` (la rama de sello de tiempo, que la postfirma
consulta vía `TS_TYPE`/`TSA_URL`) y `LoggerUtil`. **Ninguna clase de AWT nueva**: las 769 clases de
`java.awt`/`sun.awt`/`sun.font`/`sun.java2d`/`javax.swing`/`javax.imageio` alcanzables son las
mismas antes y después. La postfirma no amplía la superficie de AWT ni un ápice.

## 4. Tamaños

Mismos flags de compilación en ambos casos (`--shared -H:Name=librfirma_crypto --no-fallback
-H:+PrintAnalysisCallTree`), misma máquina, mismo GraalVM:

| Imagen | Bytes | |
|---|---|---|
| Solo prefirma (línea base del #2) | 51.424.680 | 51 MB |
| Prefirma **+ postfirma** | 51.502.704 | 51 MB |
| Diferencia | **+78.024** | **+0,15 %** |

La postfirma es prácticamente gratis en tamaño: reutiliza casi todo el árbol que ya arrastraba la
prefirma. Los nueve `.so` auxiliares se siguen emitiendo y se siguen sin necesitar.

Como referencia lateral, la imagen construida además con los `.afm` de iText
(`-H:IncludeResources=com/lowagie/text/pdf/fonts/.*`, necesaria para la sección 5) pesa 52.555.376
bytes: +1,0 MB sobre la anterior.

## 5. Con rúbrica visible, la postfirma aborta — misma causa que la prefirma

**Sí, la postfirma también toca AWT si se pide rúbrica visible, y es exactamente el mismo punto.**

Montaje para aislarlo: como la prefirma con rúbrica ya aborta en nativo (#2), el `PRE` con rúbrica
se genera **en JVM**, se firma con la clave de prueba, y solo la **postfirma** se ejecuta en la
imagen nativa con los mismos `extraParams` de rúbrica de texto (`testbench/run-cross-visible.sh`).

Dos escaladas, las mismas que produjo el #2 para la prefirma:

| Imagen | Resultado de la postfirma con rúbrica |
|---|---|
| base | `ExceptionConverter: Courier not found as resource` |
| `+ -H:IncludeResources` fuentes iText | `UnsatisfiedLinkError: No awt in java.library.path` |
| `+` los 10 `.so` presentes y `java.library.path` apuntado a ellos | **aborto fatal del proceso** (rc=99) |

El aborto es el mismo de siempre:

```
./librfirma_crypto.so: error: symbol lookup error: undefined symbol: JNI_OnLoad_awt (fatal)
Fatal error reported via JNI: Could not allocate library name
```

`LD_DEBUG=libs` lo sitúa dentro de `calling init: .../libawt.so`, es decir en su `JNI_OnLoad`. Pila
real de la imagen nativa, de dentro afuera:

```
com.oracle.svm.core.jni.JNILibraryInitializer.callOnLoadFunction(JNILibraryInitializer.java:71)
  ← java.lang.System.loadLibrary(System.java:2059)
  ← java.awt.Toolkit$2.run(Toolkit.java:1384)
  ← java.awt.Toolkit.loadLibraries(Toolkit.java:1381)
  ← java.awt.Toolkit.<clinit>(Toolkit.java:1393)
  ← java.awt.Color.<clinit>(Color.java:277)
  ← es.gob.afirma.signers.pades.PdfVisibleAreasUtils.getFont(PdfVisibleAreasUtils.java:129)
  ← es.gob.afirma.signers.pades.PdfSessionManager.getSessionData(PdfSessionManager.java:291)
  ← es.gob.afirma.signers.pades.PAdESTriPhaseSigner.insertSignatureOnPdf(PAdESTriPhaseSigner.java:344)
  ← es.gob.afirma.signers.pades.PAdESTriPhaseSigner.postSign(PAdESTriPhaseSigner.java:264)
  ← es.gob.afirma.triphase.signer.processors.PAdESTriPhasePreProcessor.preProcessPostSign(:196)
  ← es.gob.afirma.nativebridge.NativeBridge.padesPostSign
```

**Es la misma causa que la prefirma**, no una segunda distinta. Las tres líneas superiores de la
pila (`PdfVisibleAreasUtils.getFont:129` → `Color.<clinit>` → `Toolkit.<clinit>`) son literalmente
las mismas que registró el #2 para `preSign`. La razón es que la postfirma **vuelve a llamar a
`PdfSessionManager.getSessionData`** con los mismos `extraParams` y el mismo instante:

```java
// PAdESTriPhaseSigner.java:344, dentro de insertSignatureOnPdf
pts = PdfSessionManager.getSessionData(inPdf, signerCertificateChain,
        signature.getSignTime(), signature.getExtraParams(), secureMode);
```

Regenera el PDF entero y luego parchea el File ID, que es justo la mecánica que documentó el #7. Por
eso **la dependencia de AWT no está en la prefirma: está en `PdfSessionManager`, y las dos fases
pasan por él**. Como el origen es `afirma-lib-itext` (#7), tampoco aquí se esquiva renunciando a la
firma visible desde la interfaz: se esquiva no pidiendo rúbrica, que es lo mismo que ya se sabía.

**Control en JVM**: la misma postfirma con rúbrica, con `java -Djava.awt.headless=true` y sin
`DISPLAY`, produce un PDF de 178.777 bytes sin incidencias. El fallo es exclusivo de `native-image`.

## 6. Cambios en el banco de pruebas

- `NativeBridge.java`: nuevo `@CEntryPoint` `rfirma_pades_postsign(thread, pdfB64, algorithm,
  certChainB64, extraParams, triphaseXml)` que devuelve el PDF firmado en Base64 y se libera con
  `rfirma_free_string`. La lógica de pre y postfirma se extrae a métodos `static` compartidos con un
  `main` de control, para que la prueba en JVM ejercite exactamente el mismo código que el nativo.
- `testbench/loader.c`: acepta modo `presign` o `postsign`; en `postsign` decodifica el Base64 y
  escribe `postsign.pdf`.
- `testbench/inject-pk1.py`: simula la fase 2 (la firma) tomando `PRE` y depositando `PK1`.
- `testbench/run-jvm-control.sh`, `run-native.sh`, `run-equivalence.sh`, `run-mismatch.sh`,
  `run-cross-visible.sh`, `run-cross-visible-libs.sh`, `trace-libs.sh`, `build-native.sh`,
  `build-native-fonts.sh`: los recorridos de esta medición.

Las fixtures (`target/fixtures/`: `test.pdf`, `cert.pem`/`key.pem` autofirmados, los
`.properties` de rúbrica) son las del #2 y viven bajo `target/`, que está ignorado.

## Cómo reproducir

```bash
cd rfirma-native-bridge && JAVA_HOME=$GRAALVM_HOME mvn package -DskipTests
bash testbench/build-native.sh native-post        # imagen con pre + postfirma
bash testbench/run-native.sh native-post          # pre -> PK1 -> post, env -i, un solo .so
pdfsig target/lab/postsign.pdf
bash testbench/run-jvm-control.sh                 # el mismo recorrido en JVM
bash testbench/run-equivalence.sh                 # nativo vs JVM, bit a bit
bash testbench/trace-libs.sh postsign             # LD_DEBUG=libs
bash testbench/build-native-fonts.sh              # imagen con los .afm de iText
bash testbench/run-cross-visible.sh               # postfirma con rúbrica: UnsatisfiedLinkError
bash testbench/run-cross-visible-libs.sh          # postfirma con rúbrica y 10 .so: aborto fatal
```

## No determinado

- **Validación ante un validador oficial** (VALIDe, la firma encadenando a una raíz confiada). Aquí
  solo se ha comprobado la validez estructural con `pdfsig` y un certificado autofirmado de prueba.
- **PDFs difíciles**: cifrados, con permisos, o ya firmados (cofirma). La medición usa un PDF simple
  de 22 páginas sin firmas previas.
- **Sello de tiempo (`TS_TYPE`/`TSA_URL`)**: la postfirma hace alcanzables `PdfTimestamper` y
  `CMSTimestamper`, pero no se ha ejecutado ninguna postfirma con TSA. Si esa ruta funciona en
  nativo (abre sockets, TLS, parsea la respuesta RFC 3161) está sin medir.
- **Certificados EC**: solo se ha probado RSA 2048.
- Si otra versión de GraalVM o GraalVM Oracle levantan la limitación de AWT — sigue pendiente del
  [#12](https://github.com/sgomez/rfirma/issues/12), igual que en el #2.
