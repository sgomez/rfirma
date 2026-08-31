# Qué produce `native-image --shared` con solo PAdES

Medición para el issue [#2](https://github.com/sgomez/rfirma/issues/2). **Registra hechos, no decide**
el modelo de distribución: eso es el issue [#6](https://github.com/sgomez/rfirma/issues/6).

Entorno: GraalVM CE 21+35.1 (Substrate VM, serial GC), Maven 3.9.12, Linux x86_64.
Código del banco de pruebas: `rfirma-native-bridge/` y `target/loader.c` en esta rama.

## Resumen

| | Prefirma **sin** rúbrica visible | Prefirma **con** rúbrica visible |
|---|---|---|
| ¿Funciona en nativo? | **Sí** | **No, en ninguna configuración probada** |
| `.so` necesarios | **1** (`librfirma_crypto.so`) | no existe conjunto que funcione |
| Tamaño | **51 MB** | — |
| ¿Necesita Java/JAVA_HOME? | **No** | — |
| ¿Necesita `reflect-config.json`? | **No**, ninguna entrada | — |

## 1. Ficheros emitidos y ficheros necesarios

`native-image --shared` emite **10 ficheros**: `librfirma_crypto.so` más nueve auxiliares que el
propio log etiqueta como `(jdk_library)` — `libawt.so`, `libawt_headless.so`, `libawt_xawt.so`,
`libfontmanager.so`, `libjavajpeg.so`, `liblcms.so`, `libmlib_image.so` — y `(jdk_library_shim)`
— `libjava.so`, `libjvm.so`, de 15 KB cada uno.

Emitidos ≠ necesarios. Dos comprobaciones:

- **Estática**: `objdump -p librfirma_crypto.so | grep NEEDED` → solo `libc.so.6` y `libz.so.1`.
  Ninguno de los nueve auxiliares es dependencia de enlace.
- **Dinámica** (la que cuenta): prefirma PAdES real ejecutada con `dlopen` sobre un directorio que
  contenía **únicamente** `librfirma_crypto.so`, bajo `env -i` (sin `JAVA_HOME`, sin GraalVM ni JDK
  en el `PATH`). Resultado: `PRESIGN OK`, `TriphaseData` válido de 615 bytes con su campo `PRE`.

**Control**: un `@CEntryPoint` trivial compilado con `--shared` emite **solo** su `.so`. Los nueve
auxiliares no son un peaje fijo de `--shared`: los provoca nuestro árbol de dependencias.

## 2. De dónde viene AWT

Ya respondido en el issue [#7](https://github.com/sgomez/rfirma/issues/7): de `afirma-lib-itext`,
transversal a cualquier operación sobre el PDF. Esta medición lo **confirma en ejecución** y añade
el punto exacto de entrada, con pila real:

```
es.gob.afirma.signers.pades.PAdESTriPhaseSigner.preSign(PAdESTriPhaseSigner.java:176)
  → PdfSessionManager.getSessionData(PdfSessionManager.java:291)
    → PdfVisibleAreasUtils.getFont(PdfVisibleAreasUtils.java:129)   // Class.forName("java.awt.Color")
      → java.awt.Toolkit.<clinit>(Toolkit.java:1393)
        → java.awt.Toolkit.loadLibraries(Toolkit.java:1381)
          → System.loadLibrary("awt")
```

Datos de alcanzabilidad: 658 clases de `java.awt`/`sun.awt`/`sun.font`/`sun.java2d`/`javax.swing`,
incluidas `Toolkit`, `GraphicsEnvironment` y `sun.awt.X11.*`.

Dos matices medidos:

- **Solo se recorre si se pide rúbrica visible.** Sin `signaturePage`/`signaturePositionOnPage*`,
  `PdfSessionManager` no entra en esa rama y la prefirma no toca AWT en absoluto.
- **No se puede esquivar por `extraParams`.** La línea 129 construye el color de la fuente por
  reflexión y `colorName` cae a `BLACK` por defecto: siempre se instancia `java.awt.Color`.
- Excluir `afirma-ui-utils` del classpath **no** reduce la alcanzabilidad de AWT (siguen 658 clases
  y los mismos 10 ficheros). Solo rompe la normalización de la rúbrica a JPEG.

## 3. ¿Basta con AWT headless?

**No.** Refutada la hipótesis de que `libawt.so` + `libfontmanager.so` basten y `libawt_xawt.so`
sea omitible: **ningún** subconjunto funciona, porque el aborto ocurre en `libawt.so`, antes de que
headless pueda elegir backend.

Cinco imágenes construidas y probadas, todas con prefirma real, sin `DISPLAY`, bajo `env -i`:

| # | Configuración | Rúbrica texto | Rúbrica JPEG |
|---|---|---|---|
| 1 | base | `ExceptionConverter: Courier not found as resource` | — |
| 2 | `+IncludeResources` fuentes, `−afirma-ui-utils` | `UnsatisfiedLinkError: No awt in java.library.path` | rechaza el PNG |
| 3 | `+ -Djava.awt.headless=true` | `UnsatisfiedLinkError` | — |
| 4 | `+ java.library.path` apuntado a los `.so` | **aborto fatal** | aborto fatal |
| 5 | `+ --initialize-at-run-time=java.awt,sun.awt,sun.font,sun.java2d,javax.imageio` | **aborto fatal** | aborto fatal |

El aborto es:

```
Fatal error reported via JNI: Could not allocate library name
```

`LD_DEBUG=libs` sitúa el punto exacto: ocurre en `calling init: libawt.so`, es decir dentro de su
`JNI_OnLoad`, y **mata el proceso** en vez de lanzar una excepción recuperable. Se reprodujo con
5, 6 y 10 `.so` presentes: añadir ficheros no cambia el resultado.

Por qué headless no puede ayudar: la pila muestra que `Toolkit.loadLibraries` (`Toolkit.java:1381`)
hace `System.loadLibrary("awt")` **incondicionalmente**; `java.awt.headless` solo decide, después,
entre `libawt_headless.so` y `libawt_xawt.so`. El aborto es anterior a esa bifurcación.

**Control en JVM**: el mismo PDF, el mismo certificado y los mismos `extraParams` de rúbrica de
texto, ejecutados con `java -Djava.awt.headless=true` y `env -u DISPLAY`, dan `PRESIGN OK`. El
fallo es exclusivo de `native-image`, no de AutoFirma ni de la ausencia de X11.

**No determinado**: si una versión más reciente de GraalVM, GraalVM Oracle, o un modo distinto de
`--shared` levantan esta limitación. No se ha probado otra versión.

## 4. Tamaños

- `librfirma_crypto.so` (solo PAdES, sin `PreProcessorFactory`): **51 MB**.
- Los nueve auxiliares juntos: ~5 MB. Conjunto completo: ~56 MB.
- Referencia: la PoC previa en `clienteafirma`, con CAdES + PAdES + XAdES + la factoría, produjo
  **75 MB**. Recortar a solo PAdES ahorra ~24 MB (−32 %).

## 5. Configuración necesaria

- **`reflect-config.json`: no hace falta ninguna entrada** para la prefirma sin rúbrica. La imagen
  registra 2.505 tipos por reflexión de forma automática y la prefirma real funciona sin aportar
  fichero de configuración.
- **Sí hace falta configuración de recursos** para la rúbrica de texto:
  `-H:IncludeResources=com/lowagie/text/pdf/fonts/.*`. Sin ella falla con
  `Courier not found as resource`. Ojo: el mensaje de error de iText dice `com.aowagie...`, pero
  los `.afm` viven en `com/lowagie/text/pdf/fonts/` dentro de `afirma-lib-itext-1.7.jar`.
  Corregirlo no desbloquea la rúbrica: solo mueve el fallo al aborto de AWT.

## 6. Dos hallazgos laterales

- **No usar `PreProcessorFactory`.** Referencia a los preprocesadores CAdES, XAdES, FacturaE, ASiC
  y PKCS#1, y usarla haría alcanzable todo el árbol de formatos. El bridge instancia
  `PAdESTriPhasePreProcessor` directamente.
- **CAdES entra igualmente**, por `afirma-crypto-cades` vía `cades-multi` y `afirma-crypto-pdf`.
  Es el caso que el mapa ya autoriza: PAdES lo usa internamente.

## Cómo reproducir

```bash
cd rfirma-native-bridge && mvn package -DskipTests
cd target && mkdir -p native && cd native
native-image --shared -H:Name=librfirma_crypto --no-fallback \
  -Djava.awt.headless=true "-H:IncludeResources=com/lowagie/text/pdf/fonts/.*" \
  -cp "../rfirma-native-bridge-0.1.0.jar:$(cat ../cp.txt)"
# prueba aislada
gcc -o loader ../../../target/loader.c -ldl
env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so test.pdf.b64 cert.b64
```
