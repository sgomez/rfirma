# ¿Sobrevive la firma visible a la trifásica?

Investigación contra el código fuente original de `clienteafirma` (ruta local `/home/sergio/Developer/SideProjects/clienteafirma`, módulo `afirma-crypto-pdf`). Responde a las cuatro preguntas del issue [#7](https://github.com/sgomez/rfirma/issues/7), que dependía del issue [#3](https://github.com/sgomez/rfirma/issues/3) (ya resuelto en `docs/research/pades-triphase-contract.md`).

Todas las citas son `fichero:línea` relativas a `afirma-crypto-pdf/src/main/java/es/gob/afirma/signers/pades/` salvo que se indique lo contrario.

## Resumen ejecutivo — veredicto

**La firma visible SOBREVIVE a la trifásica.** No es un añadido de la postfirma: es parte constitutiva de la prefirma. `PdfSessionManager.getSessionData(...)` (invocado desde `PAdESTriPhaseSigner.preSign`, `PAdESTriPhaseSigner.java:176`) construye el diccionario de firma completo —incluida la apariencia visible (posición, página, imagen de rúbrica, texto de capas)— **antes** de calcular el hash del rango de bytes del PDF (`PAdESTriPhaseSigner.java:185-190`). Por tanto, el recuadro visible ya forma parte de los bytes cuyo hash se firma: no hay conflicto entre el ADR-0001 (la clave nunca entra en el isolate Java) y la firma visible.

No hay dependencia directa de AWT/`javax.imageio` en el código propio de `afirma-crypto-pdf` (confirmado por grep exhaustivo, ver pregunta 3): la vía de la firma visible en este módulo usa exclusivamente la librería `com.aowagie.text` (fork tipo iText/OpenPDF), no `java.awt.*`. Si el `.so` de GraalVM arrastra `libawt`, la causa más probable no está en este módulo — es "no determinado" con el código disponible, porque el propio `com.aowagie` es una dependencia externa (jar no presente en el entorno local) que podría usarlo internamente para decodificar imágenes.

Hay un punto de riesgo real pero controlado en la postfirma (pregunta 4): `insertSignatureOnPdf` (`PAdESTriPhaseSigner.java:322-361`) **no reutiliza los bytes generados en la prefirma**, sino que vuelve a invocar `PdfSessionManager.getSessionData(...)` desde cero sobre el PDF original. El propio código evidencia que esta regeneración diverge al menos en el File ID del PDF (lo corrige con un `String.replace` explícito, línea 361), lo que demuestra que las dos ejecuciones de `getSessionData` **no son bit-a-bit idénticas**. Si esa divergencia cayera dentro del rango de bytes ya hasheado, la firma se invalidaría en silencio. El código no lo permite estructuralmente (el `ByteRange` excluye el hueco reservado a `/Contents`, y el File ID vive en el trailer, fuera del rango firmado), pero no he podido confirmarlo con una cita de código que delimite exactamente qué queda dentro y fuera del `ByteRange` frente al trailer — este punto queda como **no determinado** por presupuesto de investigación.

## 1. ¿`PdfSessionManager` aplica de verdad los `extraParams` de firma visible en la prefirma?

Sí, de forma extensa y explícita. `PdfSessionManager.getSessionData(...)` (`PdfSessionManager.java:85`) lee directamente de `extraParams` (un `java.util.Properties`) todos los parámetros de apariencia visible antes de construir el `PdfStamper`:

- Imagen de rúbrica: `PdfPreProcessor.getImage(extraParams.getProperty(PdfExtraParams.SIGNATURE_RUBRIC_IMAGE), secureMode)` (`PdfSessionManager.java:108`).
- Rotación de la firma: `PdfExtraParams.SIGNATURE_ROTATION` (`PdfSessionManager.java:105`).
- Campo de firma preexistente: `PdfExtraParams.SIGNATURE_FIELD` (`PdfSessionManager.java:114`).
- Textos de capas (razón, ciudad, contacto, capa 2 y capa 4) y su tipografía/tamaño/estilo/color: `PdfExtraParams.SIGN_REASON`, `SIGNATURE_PRODUCTION_CITY`, `SIGNER_CONTACT`, `LAYER2_TEXT`, `LAYER2_FONTFAMILY`, `LAYER2_FONTSIZE`, `LAYER2_FONTSTYLE`, `LAYER2_FONTCOLOR`, `LAYER4_TEXT` (`PdfSessionManager.java:111-289`).
- Posición y página: `PdfVisibleAreasUtils.getSignaturePositionOnPage(extraParams)` y `PdfUtil.getPages(extraParams, totalPages)` (`PdfSessionManager.java:385-391`).

Después estos valores se aplican de verdad sobre el objeto `PdfSignatureAppearance sap` (obtenido en `PdfSessionManager.java:410`): `sap.setImage(rubric)` / `sap.setImageScale(-1)` (imagen), `sap.setVisibleSignature(signaturePositionOnPage, pagina, null)` o `sap.setVisibleSignature(signatureField)` (posición/campo), `sap.setLayer2Text(...)`, `sap.setLayer2Font(...)`, `sap.setLayer4Text(...)`, `sap.setReason(...)`, `sap.setLocation(...)`, `sap.setContact(...)` — todo entre `PdfSessionManager.java:410` y `:543`. No es un "aceptar y descartar": los parámetros se traducen en llamadas reales a la API de apariencia de iText/aowagie que modifican el `PdfStamper` en memoria.

## 2. ¿En qué fase se aplica la apariencia visible?

Confirmado: **en la prefirma**, no en la postfirma. La cadena de llamadas es:

```
PAdESTriPhaseSigner.preSign (línea 176)
  → PdfSessionManager.getSessionData(inPDF, cert, signTime, extraParams, secureMode)
      (aquí se construye toda la apariencia visible, líneas 383-543 de PdfSessionManager.java)
  → ptps.getSAP().getRangeStream()           (línea 178: ya con la apariencia aplicada)
  → MessageDigest...digest(pdfRangeBytes)    (líneas 185-190: se hashea el PDF CON el recuadro)
```

El orden es inequívoco dentro del propio método `preSign`: `getSessionData` se ejecuta en la línea 176, y el hash del rango se calcula en las líneas 185-190, once líneas después y usando el `rangeStream` que ya salió de la sesión con la apariencia aplicada. La hipótesis del issue ("tiene que ser en la prefirma porque el recuadro forma parte del PDF cuyo hash se firma") queda confirmada literalmente por el código, no solo por lógica.

## 3. ¿Depende la firma visible de AWT?

**En el código propio de `afirma-crypto-pdf`, no.** Comprobación exhaustiva:

```
$ grep -rln "^import java\.awt\|^import javax\.imageio" afirma-crypto-pdf/src/main/java/
(sin resultados)
```

Ningún fichero del módulo `afirma-crypto-pdf` —incluidos `PdfSessionManager.java`, `PdfVisibleAreasUtils.java`, `PdfPreProcessor.java` y `PdfUtil.java`, que son los que intervienen en la construcción de la apariencia visible— importa `java.awt.*` ni `javax.imageio.*`. Los imports relevantes de `PdfSessionManager.java` (líneas 22-35) y `PdfVisibleAreasUtils.java` (líneas 23-36) son todos `com.aowagie.text.*` / `com.aowagie.text.pdf.*` (`Font`, `Image`, `Rectangle`, `PdfSignatureAppearance`, `ColumnText`, `BaseFont`, `PdfTemplate`, etc.), es decir, las clases propias de la librería PDF (fork de iText/OpenPDF), no las de `java.awt`.

**Conexión con el ticket #2 — no determinado.** `com.aowagie.text.Image` y `com.aowagie.text.Font` son clases de una dependencia externa (el jar `aowagie`, no presente en el `~/.m2` de este entorno ni en el repositorio, así que no se ha podido inspeccionar su bytecode). Es una práctica habitual en librerías de este linaje (iText clásico) apoyarse internamente en `java.awt.Image`/`java.awt.Color`/`javax.imageio.ImageIO` para decodificar formatos de imagen (JPEG, PNG) antes de empotrarlos en el PDF. Si el `.so` de GraalVM arrastra `libawt`, el candidato más plausible según esta investigación es esa dependencia transitiva de `com.aowagie`, activada precisamente por el camino de `PdfPreProcessor.getImage(...)` / `sap.setImage(rubric)` cuando se usa una imagen de rúbrica — **pero esto no está confirmado con código, solo es una hipótesis razonada**. Para confirmarlo haría falta inspeccionar el jar `com.aowagie:*` (o su código fuente) directamente, lo cual queda fuera del presupuesto de esta investigación.

## 4. ¿Modifica la postfirma bytes que la prefirma ya había incluido en el hash?

Este es el punto más delicado y donde la investigación encuentra una señal de alarma parcialmente mitigada por el propio diseño del código, pero no verificada al 100%.

`PAdESTriPhaseSigner.postSign` → `insertSignatureOnPdf` (`PAdESTriPhaseSigner.java:322-361`) **no reutiliza el `PdfStamper`/`PdfSignatureAppearance` generado en la prefirma** (ese objeto vive en memoria durante `preSign` y no se serializa fuera de `TriphaseData`; lo único que cruza la frontera de la firma trifásica es el `SignedAttributes` DER, según el contrato de `pades-triphase-contract.md`). En su lugar, la postfirma **reconstruye la sesión completa desde cero**, volviendo a invocar `PdfSessionManager.getSessionData(inPdf, signerCertificateChain, signature.getSignTime(), signature.getExtraParams(), secureMode)` (`PAdESTriPhaseSigner.java:344`), es decir, vuelve a aplicar exactamente la misma lógica de construcción de apariencia visible que en la pregunta 1, con el mismo `inPdf` original y el mismo `extraParams` (heredado íntegro de la prefirma vía `PdfSignResult`).

Evidencia de que esta segunda ejecución **no es bit-a-bit idéntica** a la primera: el propio código recoge un `badFileID = pts.getFileID()` de esta segunda sesión (línea 353) y, tras cerrar el `PdfStamper` con la firma real (`sap.close(dic2)`, línea 355), hace un `String.replace(badFileID, signature.getFileID())` sobre los bytes resultantes (línea 361) para sustituirlo por el File ID correcto, el que se generó en la prefirma (`ptps.getFileID()`, guardado en `PdfSignResult.getFileID()`). Esto demuestra explícitamente que regenerar la sesión desde cero produce al menos un valor distinto (el File ID, presumiblemente aleatorio o basado en un componente no determinista como la hora del sistema de generación), y que los autores de `clienteafirma` tuvieron que parchearlo a mano.

**Lo que no he podido determinar con certeza:** si el `ByteRange` firmado (el rango cuyo hash se calculó en la prefirma) excluye estrictamente el trailer del PDF donde vive el File ID, de forma que este parche sea inocuo por construcción, o si existe algún otro campo además del File ID que también diverja entre las dos ejecuciones de `getSessionData` y que sí caiga dentro del rango hasheado (por ejemplo, si la reserva de tamaño de firma, el offset del `ByteRange` o algún metadato de fecha variase entre ejecuciones). No se ha localizado ni leído el código de `PdfSignatureAppearance.preClose`/`getRangeStream` de la librería `com.aowagie` (externa, no disponible localmente) que sería necesario para cerrar esta pregunta con una cita literal. **No determinado.**

En síntesis: el diseño evidencia que los autores originales conocían y mitigaron al menos una fuente de no-determinismo entre prefirma y postfirma (el File ID), lo que es indicio razonable de que el resto del contenido del `ByteRange` sí es determinista y estable entre ambas ejecuciones (de lo contrario, AutoFirma llevaría produciendo firmas PAdES trifásicas rotas desde su publicación, cosa que no consta). Pero esta investigación no puede confirmarlo con una cita de código que delimite el `ByteRange` frente al trailer, así que el veredicto de la pregunta 4 queda como **riesgo mitigado por diseño pero no verificado al 100% con código**.

## Referencias directas al issue #3

Este documento asume como ya establecido (`docs/research/pades-triphase-contract.md`):
- `TriphaseData["PRE"] = Base64(DER(SignedAttributes CAdES))`.
- La llamada a `PdfSessionManager.getSessionData(...)` en `PAdESTriPhaseSigner.java:176` es la que fija los bytes del PDF (incluida la apariencia) antes del hash del rango.
