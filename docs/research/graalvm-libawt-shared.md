# ¿Otra versión de GraalVM levanta la limitación de `libawt.so` en `--shared`?

Medición para el issue [#12](https://github.com/sgomez/rfirma/issues/12), que cierra el
**no determinado** que dejó el [#2](https://github.com/sgomez/rfirma/issues/2) en
`docs/research/native-image-shared-pades.md`. **Registra hechos, no decide** el modelo de
distribución: eso es el [#6](https://github.com/sgomez/rfirma/issues/6).

## Veredicto

**Sí.** Con **GraalVM CE 25.3.4.1 (JDK 25)** la prefirma PAdES **con rúbrica visible** funciona
dentro de una imagen `--shared`, ejecutada bajo `env -i`, sin `DISPLAY` y sin Java instalado.
La rúbrica **de texto** no carga `libawt.so` en absoluto y sigue bastando **un único `.so`**;
la rúbrica **de imagen** sí carga `libawt.so`, **supera su `JNI_OnLoad`** y funciona con
**cinco `.so`** más metadatos de alcanzabilidad de AWT.

**La limitación es de la línea Java 21, no de `--shared`.** Tanto GraalVM CE 21.0.2 (la última
CE de la línea 21) como **Oracle GraalVM 21.0.12** (julio de 2026, la última LTS de esa línea)
abortan exactamente igual que CE 21+35.1. Cambiar de distribución dentro de Java 21 no sirve;
lo que sirve es **subir a Java 25**.

## Versiones probadas

Todas con el mismo banco de pruebas, el mismo PDF, el mismo certificado y los mismos
`extraParams` (ver «Cómo reproducir»).

| Distribución y versión | Sin rúbrica | Rúbrica de texto | Rúbrica de imagen (PNG) | Tamaño del `.so` |
|---|---|---|---|---|
| GraalVM CE 21+35.1 (línea base del #2) | OK | **aborto fatal** | aborto fatal | 52,5 MB |
| GraalVM CE 21.0.2+13.1 | OK | **aborto fatal** | aborto fatal | 52,5 MB |
| Oracle GraalVM 21.0.12+7.1 (LTS) | OK | **aborto fatal** | aborto fatal | 56,1 MB |
| **GraalVM CE 25.3.4.1 (JDK 25)** | OK | **OK** | **OK** (con metadatos) | 34,6 MB / 35,3 MB con metadatos |

El aborto es siempre el mismo, con y sin `RFIRMA_LIB_DIR` apuntando a los `.so` auxiliares:

```
Fatal error reported via JNI: Could not allocate library name
```

`LD_DEBUG=libs` lo sitúa, como en el #2, en `calling init: libawt.so`, o sea dentro de su
`JNI_OnLoad`. Mata el proceso.

**Oracle GraalVM 25 no se ha probado**: es innecesario, porque la CE de la misma línea ya
funciona y su licencia (GPLv2 + Classpath Exception) no plantea ninguna duda. Ver «Licencia».

## Qué cambia exactamente en CE 25

`LD_DEBUG=libs` sobre la imagen de CE 25, en el mismo caso mínimo:

| Caso | Bibliotecas que el proceso llega a inicializar |
|---|---|
| Rúbrica de **texto** | `librfirma_crypto.so` y nada más (`libc`, `libm`, `libz`) |
| Rúbrica de **imagen** | además `libjvm.so`, `libjava.so`, **`libawt.so`**, `libawt_headless.so`, `libjavajpeg.so` |

Es decir, dos cambios independientes respecto de Java 21:

1. **La rúbrica de texto ya no toca AWT nativo.** En Java 21 la pila
   `PdfVisibleAreasUtils.getFont` → `Class.forName("java.awt.Color")` → `Toolkit.<clinit>` →
   `System.loadLibrary("awt")` se recorría siempre; en la imagen de CE 25 no se carga `libawt.so`.
   **No determinado**: el mecanismo exacto (la hipótesis razonable es que `java.awt.Color` y
   `Toolkit` quedan inicializados en tiempo de construcción, pero no se ha verificado en el código
   de Substrate VM).
2. **Cuando `libawt.so` sí se carga, su `JNI_OnLoad` ya no aborta.** Se ejecuta y, si faltan los
   metadatos de AWT, falla con una **excepción recuperable**, no con un `FatalError`:

   ```
   java.lang.NoClassDefFoundError: java/awt/GraphicsEnvironment
     at com.oracle.svm.core.jni.functions.JNIFunctions$Support.findClassInReflectionDictionary
     at com.oracle.svm.core.jni.functions.JNIFunctions.FindClass
     at com.oracle.svm.core.jni.JNIOnLoadFunctionPointer.invoke
     at com.oracle.svm.core.jni.JNILibraryInitializer.initialize(JNILibraryInitializer.java:123)
   ```

   Esa excepción se arregla con los metadatos de alcanzabilidad que el propio `native-image`
   reclama en su log de construcción (`AWT: Use the tracing agent to collect metadata for AWT.`).

### La rúbrica de texto se ejecuta de verdad

Comprobación de que CE 25 no está saltándose silenciosamente el dibujo de la rúbrica: la misma
imagen construida **sin** `-H:IncludeResources=com/lowagie/text/pdf/fonts/.*` falla en el caso
de rúbrica de texto con el error que el #2 ya había visto:

```
Courier not found as resource. (The *.afm files must exist as resources in the package
com.aowagie.text.pdf.fonts)
→ PRESIGN ERROR
```

O sea: la ruta de dibujo se recorre entera y sigue necesitando los `.afm` de iText incrustados.

## Conjunto mínimo de `.so` en CE 25

Medido copiando a un directorio vacío solo los ficheros indicados y ejecutando la prefirma real
bajo `env -i` (sin `DISPLAY`, sin `JAVA_HOME`, sin GraalVM ni JDK en el `PATH`):

| Caso | `.so` necesarios | Total en disco |
|---|---|---|
| Sin rúbrica y **rúbrica de texto** | **1**: `librfirma_crypto.so` | **34,6 MB** |
| **Rúbrica de imagen** | **6**: la imagen más **cinco auxiliares** — `libawt.so`, `libawt_headless.so`, `libjavajpeg.so`, `libjava.so`, `libjvm.so` | **36,5 MB** |

Quitar cualquiera de los cinco auxiliares rompe el caso de imagen; `libawt_xawt.so`,
`libfontmanager.so` y `liblcms.so` **no** hacen falta (y `libmlib_image.so` ya ni se emite en la
línea 25). `native-image` sigue emitiendo los ocho auxiliares: emitidos ≠ necesarios, igual que
en el #2.

Sin los metadatos de AWT, el caso de imagen falla aunque estén los cinco `.so`:

```
WARNING: No se pudo normalizar la imagen de rubrica. Se agregara tal cual:
         java.lang.reflect.InvocationTargetException
ERROR:java.io.IOException: Se ha proporcionado una imagen de rubrica que no esta codificada en JPEG
```

Los metadatos se obtienen con el agente de trazado sobre una ejecución normal en JVM
(`-agentlib:native-image-agent=config-output-dir=…`): produce un único
`reachability-metadata.json` de 41 KB con secciones `reflection` y `resources`, que incluye
`java.awt.Color`, `java.awt.GraphicsEnvironment`, `java.awt.image.BufferedImage`,
`java.awt.image.ColorModel`, `java.awt.image.Raster`, `SampleModel`,
`SinglePixelPackedSampleModel` y `sun.awt.image.ByteComponentRaster`. Se pasa con
`-H:ConfigurationFileDirectories=<dir>`. Coste en tamaño: **+0,7 MB** (34,6 → 35,3 MB).

Sigue sin hacer falta un `reflect-config.json` escrito a mano, y la prefirma **sin** rúbrica
tampoco necesita metadatos de ningún tipo.

## Estado en el repositorio de GraalVM

Búsqueda previa a descargar nada. **No hay un issue abierto que declare AWT no soportado en
imágenes `--shared`**; lo que hay es un historial de fallos de `libawt` en `native-image`, casi
siempre en ejecutables, no en `--shared`:

- [oracle/graal#6244, «AWT dynamic shared libraries loading seems broken»](https://github.com/oracle/graal/issues/6244)
  (cerrado, marzo de 2023): mismo aborto en `JNI_OnLoad` de `libawt.so` con GraalVM CE 23.0-dev.
  Se cierra con la respuesta de que **hacen falta los metadatos de AWT** del agente de trazado
  (`-H:ConfigurationFileDirectories`). Es la pista que aquí resulta ser correcta para el caso de
  imagen, pero **no** basta en la línea 21: allí el proceso muere antes de poder lanzar la
  excepción que los metadatos evitarían.
- [oracle/graal#9485, «Fatal error reported via JNI: Could not allocate library name»](https://github.com/oracle/graal/issues/9485)
  (cerrado en agosto de 2024 **por falta de reproductor**, no por arreglo): el mismo mensaje con
  Oracle GraalVM 22.0.2 en Linux. Reabierto de hecho en comentarios de 2025 sin respuesta.
- [oracle/graal#8475](https://github.com/oracle/graal/issues/8475) y
  [oracle/graal#13272](https://github.com/oracle/graal/issues/13272): el mismo `JNI_OnLoad` de
  `libawt`, pero en macOS, donde el soporte de AWT directamente no está implementado.

Conclusión de la búsqueda: **no había respuesta publicada**; la medición era necesaria.

## Licencia de Oracle GraalVM (GFTC)

No se ha llegado a necesitar, pero se deja registrado porque el issue lo pedía. Oracle GraalVM
se distribuye bajo las [GraalVM Free Terms and Conditions](https://www.oracle.com/downloads/licenses/graal-free-license.html)
(GFTC). Según la [FAQ oficial](https://www.graalvm.org/faq/) y el
[anuncio de Oracle](https://blogs.oracle.com/graal/graalvm-free-license):

- Permite el uso gratuito, **incluido el uso comercial y en producción**, y explícitamente el uso
  y la distribución de la **salida de Native Image**.
- La **redistribución del propio GraalVM** solo se permite **si no se cobra por ella**.
- **No es una licencia OSI**: GraalVM **CE** sí lo es (GPLv2 + Classpath Exception, las mismas
  condiciones que Java).

Como CE 25 resuelve el problema, **no hay ninguna razón para asumir la GFTC** en este proyecto.
Sí se probó Oracle GraalVM **21.0.12** porque era la única forma de saber si la línea Java 21
llegaba a arreglarse en alguna distribución: no se arregla.

## Consecuencias para el mapa (sin decidir nada)

- La frase del #2 «la firma visible no tiene camino en Java» **queda refutada**: lo tiene, en
  Java 25.
- El coste es **subir el `pom.xml` y la cadena de construcción de Java 21 a Java 25**. La imagen
  además **encoge**: 34,6 MB frente a 52,5 MB (el #2 midió 51 MB en una imagen equivalente).
- Si el caso de uso incluye **imagen de rúbrica** (y el inventario de capacidades del mapa la
  incluye), hay que distribuir **cinco `.so` en vez de uno** y añadir un
  `reachability-metadata.json` generado con el agente. Qué implica eso para el `.deb` es del #6.

## No determinado

- **En qué versión concreta entre 21.0.2 y 25.3.4.1 cambió el comportamiento.** No se ha
  bisecado: SDKMAN no ofrece CE para JDK 22, 23 ni 24.
- **Oracle GraalVM 25**: no probado (innecesario).
- **Mandrel**: no probado. En el #6244 fallaba de otra forma (`no awt in java.library.path`).
- **Solo se ha medido la prefirma.** La postfirma, que según el #7 regenera el PDF entero y por
  tanto también dibuja, **no se ha probado en nativo** en ninguna versión.
- **Otras plataformas**: solo Linux x86_64. En macOS los issues citados dicen que AWT no está
  implementado en `native-image`; Windows sin medir.
- Si `reachability-metadata.json` puede sustituir a `-H:IncludeResources` para los `.afm` de
  iText: no comprobado; aquí se pasaron ambas cosas.

## Cómo reproducir

Banco de pruebas: `rfirma-native-bridge/` (`pom.xml`, `NativeBridge.java`, `testbench/loader.c`),
el mismo del #2. Fixtures: un PDF generado con iText, un certificado autofirmado RSA 2048 de
`openssl req -x509` (la prefirma no valida la cadena) y un fichero de `extraParams`:

```properties
signaturePage=1
signaturePositionOnPageLowerLeftX=100
signaturePositionOnPageLowerLeftY=100
signaturePositionOnPageUpperRightX=300
signaturePositionOnPageUpperRightY=180
layer2Text=Firmado por RFIRMA TEST
# para el caso de imagen, en vez de layer2Text:
# signatureRubricImage=<PNG en Base64>
```

```bash
sdk install java 25.3.4+1.r25-graalce
cd rfirma-native-bridge && mvn package -DskipTests
cd target && mkdir -p native && cd native
native-image --shared -H:Name=librfirma_crypto --no-fallback \
  -Djava.awt.headless=true "-H:IncludeResources=com/lowagie/text/pdf/fonts/.*" \
  [-H:ConfigurationFileDirectories=<dir con reachability-metadata.json>] \
  -cp "../rfirma-native-bridge-0.1.0.jar:$(cat ../cp.txt)"
gcc -o loader ../../testbench/loader.c -ldl
env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so test.pdf.b64 cert.b64 visible.properties
```

El `pom.xml` no se tocó: se compiló con `source/target 21` y `graal-sdk` 23.1.0 en `provided`, y
`native-image` de la línea 25 lo aceptó sin cambios.
