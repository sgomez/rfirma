# 07. Operación `signandsave`

Este capítulo describe la operación `signandsave` del protocolo `afirma://` de
AutoFirma 1.9.2. Esta operación combina la generación de una firma electrónica
(o multifirma: cofirma o contrafirma) con la exportación y guardado directo del
resultado en el sistema de ficheros local del usuario mediante un diálogo nativo,
devolviendo simultáneamente la firma y el certificado del firmante a la aplicación
web invocadora. Se documenta la gramática de invocación, el procesamiento de
parámetros, la interacción gráfica para la selección de fichero de entrada (cuando
aplique), la selección de certificado y firma visible, la presentación del diálogo
de guardado con control de colisiones, el formato de respuesta en los distintos
transportes y el catálogo completo de errores.

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. Visión general y ciclo de vida de `signandsave`

A diferencia de la operación estándar `sign` (capítulo 06), concebida para
devolver la firma íntegramente a través del canal de comunicación (servidor
intermedio, socket o WebSocket) para su custodia en el servidor web, `signandsave`
introduce un paso intermedio en la estación de trabajo: una vez generado el
objeto criptográfico, AutoFirma despliega una ventana de diálogo para que el
usuario elija la carpeta y el nombre con los que desea almacenar el fichero firmado
en su disco local.

A pesar de su nombre, `signandsave` **no sustituye la devolución del resultado**:
la aplicación continúa devolviendo la firma digital codificada en Base64 (junto al
certificado del firmante y metadatos) a través del transporte hacia la página web,
exactamente igual que una operación `sign`.

```
                  ┌────────────────────────────────────────────────────────┐
                  │ Invocación: afirma://signandsave?cop=...&format=...    │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │ 1. ¿Datos presentes?                                   │
                  │    - Sí: usa dat / fileid                              │
                  │    - No: AOUIFactory.getLoadFiles() [Diálogo selector] │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │ 2. Selección de certificado (AOKeyStoreDialog)         │
                  │    y comprobación previa de firma visible (PDF)        │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │ 3. Ejecución criptográfica:                            │
                  │    signer.sign() / cosign() / countersign()            │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │ 4. Guardado local en disco:                            │
                  │    AOUIFactory.getSaveDataToFile()                     │
                  │    [Diálogo guardar como... + confirmación sobrescrit.]│
                  └──────────────────────────┬─────────────────────────────┘
                                             │
                                             ▼
                  ┌────────────────────────────────────────────────────────┐
                  │ 5. Devolución de la respuesta al llamante web:         │
                  │    certEncoded | signature [ | extraData ]             │
                  └────────────────────────────────────────────────────────┘
```

### 1.1 Prefijos de URI reconocidos

El despachador principal `ProtocolInvocationLauncher.launch` intercepta esta
operación en la línea 532 de
`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncher.java`:

```java
else if (urlString.startsWith("afirma://signandsave?") || urlString.startsWith("afirma://signandsave/?")) {
```

Al igual que el resto de operaciones, se aceptan tanto la sintaxis estándar sin barra
(`afirma://signandsave?`) como la variante con barra final (`afirma://signandsave/?`).

### 1.2 Diferencias estructurales frente a `sign`

| Característica | Operación `sign` | Operación `signandsave` | Cita en código |
|---|---|---|---|
| Clase despachadora | `ProtocolInvocationLauncherSign` | `ProtocolInvocationLauncherSignAndSave` | `ProtocolInvocationLauncher.java:582, 690` |
| Modelo de parámetros | `UrlParametersToSign` | `UrlParametersToSignAndSave` | `ProtocolInvocationLauncher.java:536, 651` |
| Suboperación criptográfica | Indicada en `op` (`sign`, `cosign`, `countersign`) | Indicada en `cop` (`sign`, `cosign`, `countersign`), mientras `op` es `signandsave` | `UrlParametersToSignAndSave.java:237`, `UrlParametersToSign.java:246` |
| Nombre propuesto de archivo | Ignorado en URI | Parámetro `filename` en URI (con validación de caracteres ilegales) | `UrlParametersToSignAndSave.java:330-341` |
| Exportación a disco | No existe | Obligatoria: diálogo interactivo `AOUIFactory.getSaveDataToFile` | `ProtocolInvocationLauncherSignAndSave.java:543` |
| Parámetros adicionales de guardado | No reconocidos | `filenameSaveExts`, `filenameSaveDescription`, `filenameSaveCurrentDir` en `extraParams` | `ProtocolInvocationLauncherSignAndSave.java:537-546` |
| Soporte de `anotherParams` | Sí (`getParametersToSign` llama a `ret.setAnotherParams`) | No (`getParametersToSignAndSave` omite la llamada; siempre vacío — ver [BUG-14](A1-bugs-autofirma.md#bug-14-omisión-de-setanotherparams-en-signandsave-descarta-parámetros-de-configuración-para-plugins)) | `ProtocolInvocationUriParserUtil.java:145, 156-162` |

---

## 2. Parámetros de la invocación `signandsave`

La deserialización y validación de los parámetros de la URI o del XML intermedio
está implementada en la clase `UrlParametersToSignAndSave`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSignAndSave.java`),
que extiende la clase base abstracta `UrlParameters`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java`).

El método de fábrica encargado de instanciarla y poblarla es
`ProtocolInvocationUriParserUtil.getParametersToSignAndSave`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParserUtil.java:156-162`):

```java
public static UrlParametersToSignAndSave getParametersToSignAndSave(final Map<String, String> params,
        final boolean servicesRequired) throws ParameterException {
    final UrlParametersToSignAndSave ret = new UrlParametersToSignAndSave(servicesRequired);
    ret.setCommonParameters(params);
    ret.setSignAndSaveParameters(params);
    return ret;
}
```

### 2.1 Catálogo de parámetros aceptados

`UrlParametersToSignAndSave` declara su lista blanca de parámetros conocidos en
`KNOWN_PARAMETERS` (`UrlParametersToSignAndSave.java:58-64`):

| Parámetro | Tipo / Formato | Obligatorio | Descripción | Cita |
|---|---|---|---|---|
| `cop` | `String` (`"sign"`, `"cosign"`, `"countersign"`) | No formalmente en parser, sí en ejecución | Operación criptográfica real a ejecutar. Mapea internamente a `SignOperation.Operation`. Si se omite o contiene un valor no reconocido, su valor es `null` y desencadena `NullPointerException` en `executeSign`, derivando en error `SAF_09` (o `SAF_03` si el firmador implementa `OptionalDataInterface`) tras solicitar certificado y PIN — ver [BUG-15](A1-bugs-autofirma.md#bug-15-ausencia-de-validación-de-cop-en-signandsave-provoca-nullpointerexception-y-reporte-engañoso-con-saf_09). | `UrlParametersToSignAndSave.java:29, 237`, `ProtocolInvocationLauncherSignAndSave.java:155, 728` |
| `format` | `String` | **Sí** | Formato de firma solicitado (`PAdES`, `CAdES`, `XAdES`, `FacturaE`, `AUTO`, etc.). Si falta, lanza `ParameterException("No se ha recibido el formato de firma")` (`SAF_03`). | `UrlParametersToSignAndSave.java:32, 272-277` |
| `algorithm` | `String` | **Sí** | Algoritmo de hash o firma. Debe pertenecer obligatoriamente a la lista fija `SUPPORTED_SIGNATURE_ALGORITHMS`. Si no coincide, lanza `ParameterException` (`SAF_03`). Omite soporte directo de algoritmos ECDSA — ver [BUG-05](A1-bugs-autofirma.md#bug-05-rechazo-de-algoritmos-ecdsa-en-la-operación-signandsave). | `UrlParametersToSignAndSave.java:35, 67-77, 280-288` |
| `filename` | `String` | No | Nombre de fichero propuesto para el diálogo de guardado. Se valida contra la lista de caracteres ilegales `\/:*?"<>|`. Si contiene alguno, lanza `ParameterException` (`SAF_03`). | `UrlParametersToSignAndSave.java:38, 330-341` |
| `dat` | Base64 URL-safe / URL | Condicional | Datos o documento a firmar, en Base64 o URL remota (salvo prefijo `file:/`). Si no se indica ni viene `fileid`, AutoFirma pedirá al usuario que seleccione un fichero local mediante un diálogo modal. | `UrlParameters.java:299-319`, `ProtocolInvocationLauncherSignAndSave.java:293-350` |
| `gzip` | `Boolean` (`"true"`/`"false"`) | No | Indica si los datos de `dat` están comprimidos con gzip. | `UrlParameters.java:311` |
| `fileid` | Alfanumérico (máx 40) | Condicional | Identificador en el servidor intermedio para descargar la configuración completa y los datos vía `rtservlet`. | `UrlParameters.java:264`, `UrlParametersToSignAndSave.java:209-226` |
| `id` | Alfanumérico (máx 40) | Condicional | Identificador de sesión para devolver el resultado al `stservlet`. Obligatorio si `servicesRequired=true` (invocación directa por servidor intermedio sin socket). | `UrlParametersToSignAndSave.java:206-226, 266-268` |
| `rtservlet` | URL HTTP/HTTPS | Condicional | URL del servlet de recuperación (`RetrieveService`). Obligatorio si se proporciona `fileid`. No puede apuntar a direcciones locales (`localhost`, `127.0.0.1`). | `UrlParameters.java:279-296` |
| `stservlet` | URL HTTP/HTTPS | Condicional | URL del servlet de almacenamiento (`StorageService`). Obligatorio si `servicesRequired=true` y no hay `fileid`. No puede ser local. | `UrlParametersToSignAndSave.java:247-269` |
| `key` | `String` (32 caracteres) | No | Clave simétrica de 32 bytes para descifrar la petición de `rtservlet` o cifrar el resultado hacia `stservlet` o socket. | `UrlParameters.java:327-345` |
| `ver` | `String` (entero) | No | Versión del protocolo declarada por la sede. Si no se pasa, se asume `"0"`. | `UrlParametersToSignAndSave.java:45, 229-234` |
| `properties` | Base64 | No | Propiedades adicionales de firma y filtros codificadas en formato Java Properties Base64. | `UrlParametersToSignAndSave.java:290-308` |
| `keystore` | `String` | No | Nombre del almacén de certificados solicitado (`"WINDOWS"`, `"APPLE"`, `"PKCS11"`, etc.). | `UrlParameters.java:369-373` |
| `ksb64` | Base64 | No | Nombre del almacén codificado en Base64. Tiene prioridad sobre `keystore`. | `UrlParameters.java:364-368` |
| `sticky` | `Boolean` (`"true"`/`"false"`) | No (def: `false`) | Si es `true`, mantiene en memoria de sesión la referencia de clave (`PrivateKeyEntry`) para firmas subsiguientes. | `UrlParametersToSignAndSave.java:48, 311-316` |
| `resetsticky` | `Boolean` (`"true"`/`"false"`) | No (def: `false`) | Si es `true`, ignora cualquier clave prefijada anteriormente en la sesión y obliga a seleccionar de nuevo. | `UrlParametersToSignAndSave.java:51, 319-324` |
| `aw` | `Boolean` (`"true"`/`"false"`) | No (def: `false`) | Si es `true` y el transporte es servidor intermedio, inicia llamadas periódicas de espera activa (*keepalive*) a `stservlet`. | `UrlParameters.java:271-274`, `ProtocolInvocationLauncher.java:573-575` |
| `mcv` | `String` (versión X.Y.Z) | No | Versión mínima requerida de la aplicación de escritorio AutoFirma. Si la versión local instalada es inferior, aborta con `SAF_41`. | `UrlParameters.java:413-417`, `ProtocolInvocationLauncherSignAndSave.java:140-146` |

### 2.2 Algoritmos de firma admitidos

La comprobación en `UrlParametersToSignAndSave.java:284-286` es taxativa:
el parámetro `algorithm` debe coincidir exactamente (distinguiendo mayúsculas y
minúsculas) con una de las siguientes cadenas (`68-77`):

* `SHA1`
* `SHA256`
* `SHA384`
* `SHA512`
* `SHA1withRSA`
* `SHA256withRSA`
* `SHA384withRSA`
* `SHA512withRSA`

Cualquier otro valor (por ejemplo, algoritmos con curvas elípticas directas como
`SHA256withECDSA`, o algoritmos no reconocidos) lanzará de inmediato:
`ParameterException("Algoritmo de firma no soportado: " + algo)`, interrumpiendo
el procesamiento con `SAF_03`. A diferencia de `UrlParametersToSign.java:70-73`
(que sí incluye `SHA1withECDSA`, `SHA256withECDSA`, `SHA384withECDSA` y
`SHA512withECDSA`), `UrlParametersToSignAndSave` omite estos identificadores,
rechazando peticiones legítimas de firma elíptica con `SAF_03` — ver
[BUG-05](A1-bugs-autofirma.md#bug-05-rechazo-de-algoritmos-ecdsa-en-la-operación-signandsave).
El algoritmo final compuesto con el tipo real de la
clave privada seleccionada (`RSA`, `EC`) se resuelve posteriormente durante la fase
de firma mediante `AOSignConstants.composeSignatureAlgorithmName`
(`ProtocolInvocationLauncherSignAndSave.java:664`).

### 2.3 Parámetros de `extraParams` específicos para guardado y carga

A través del parámetro `properties` (cadena Java Properties codificada en Base64),
`signandsave` procesa directivas adicionales declaradas en `AfirmaExtraParams`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/AfirmaExtraParams.java`):

#### Para el diálogo de guardado del fichero resultante:

* **`filenameSaveExts`** (`AfirmaExtraParams.SAVE_FILE_EXTS`, línea 30):
  Lista de extensiones permitidas separadas por comas (por ejemplo, `"pdf"`,
  `"csig,sig"`, `"xsig,xml"`). Configura los filtros seleccionables en el
  desplegable del diálogo `JFileChooser`.
* **`filenameSaveDescription`** (`AfirmaExtraParams.SAVE_FILE_DESCRIPTION`, línea 33):
  Texto descriptivo del filtro de guardado. Si no se indica, por defecto toma el literal
  localizado `ProtocolLauncher.30` (`"Firma"`) de `protocolmessages.properties:28`.
  Al texto se le añade automáticamente la máscara de extensiones
  (por ejemplo, `"Firma (*.pdf)"` o `"Firma (*.*)"` si no hay extensiones).
* **`filenameSaveCurrentDir`** (`AfirmaExtraParams.SAVE_FILE_CURRENT_DIR`, línea 36):
  Ruta absoluta del directorio inicial donde debe abrirse el diálogo de guardado.

#### Para el diálogo de carga interactiva (cuando `dat == null` y no hay `fileid`):

* **`filenameExts`** (`AfirmaExtraParams.LOAD_FILE_EXTS`, línea 12):
  Lista separada por comas de extensiones admitidas para la selección del documento de entrada.
* **`filenameDescription`** (`AfirmaExtraParams.LOAD_FILE_DESCRIPTION`, línea 15):
  Descripción de los ficheros de datos permitidos. Defecto: `ProtocolLauncher.32`
  (`"Ficheros de datos"`) concatenado con la lista de extensiones.
* **`filenameCurrentDir`** (`AfirmaExtraParams.LOAD_FILE_CURRENT_DIR`, línea 18):
  Ruta absoluta del directorio de apertura del selector de ficheros de entrada.
* **`filenameActualName`** (`AfirmaExtraParams.LOAD_FILE_FILENAME`, línea 21):
  Nombre de fichero preseleccionado en el cuadro de selección de entrada.

#### Comportamiento general:

* **`headless`** (`AfirmaExtraParams.HEADLESS`, línea 39):
  Si es `"true"` y la validación previa de firma requiere intervención del usuario
  (confirmaciones de riesgo o advertencias), la operación se aborta de inmediato con
  `SAF_50` (`ProtocolInvocationLauncherSignAndSave.java:424-427, 821-824`).
* **`checkSignatures`** (`AfirmaExtraParams.CHECK_SIGNATURES`, línea 9):
  Si es `"true"`, analiza la validez de las firmas existentes en el documento
  antes de añadir la nueva firma o multifirma.

---

## 3. Flujo de datos de entrada: resolución interactiva cuando `data == null`

En `ProtocolInvocationLauncherSignAndSave.sign`
(`ProtocolInvocationLauncherSignAndSave.java:293-370`), si los datos no vienen
provistos en la URI (`dat` nulo) ni se han descargado mediante `fileid`:

1. **Evaluación de necesidad de datos:**
   Se comprueba si el firmador implementa `OptionalDataInterface`
   (`ProtocolInvocationLauncherSignAndSave.java:295-299`):
   ```java
   if (data == null) {
       if (signer instanceof OptionalDataInterface) {
           needRequestData = ((OptionalDataInterface) signer).needData(signOperation.getCryptoOperation().toString(), extraParams);
       } else {
           needRequestData = true;
       }
   }
   ```
2. **Apertura del selector de ficheros:**
   Si `needRequestData` es verdadero, se invoca `AOUIFactory.getLoadFiles`
   (`335-345`):
   * Título del diálogo: si la operación es `Operation.SIGN`, se muestra
     `ProtocolLauncher.25` (`"Seleccione el fichero de datos a firmar"`). Si es
     `COSIGN` o `COUNTERSIGN`, se muestra `ProtocolLauncher.26`
     (`"Seleccione el fichero de firma"`).
   * En macOS, se ejecuta previamente `MacUtils.focusApplication()` (`333`) para
     asegurar que la ventana gráfica gane el foco sobre el navegador.
   * Si el usuario pulsa «Cancelar» en el selector, se captura
     `AOCancelledOperationException` (`346-349`) y se relanza como
     `SocketOperationException(RESULT_CANCEL)`.
3. **Registro del nombre del fichero cargado:**
   Al seleccionar un fichero en disco:
   * Se almacena su nombre simple en la variable local `inputFilename = selectedDataFile.getName();`
     (`352`).
   * Se leen íntegramente los bytes del fichero mediante `FileInputStream` y
     `AOUtil.getDataFromInputStream` (`355-360`). Si falla la lectura, se lanza `SAF_00`
     (`ERROR_CANNOT_READ_DATA`, `366`).
   * Al término de la firma, si `inputFilename != null`, se empaqueta en las propiedades
     de metadatos del resultado (`582-586`):
     ```java
     Properties extraData = null;
     if (inputFilename != null) {
         extraData = new Properties();
         extraData.setProperty("filename", inputFilename);
     }
     result.setDataFilename(extraData);
     ```
     Este metadato viajará posteriormente en el tercer campo de la respuesta del protocolo.

---

## 4. Proceso de firma y particularidades de ejecución

La ejecución de la firma en `signandsave` reproduce las mismas etapas de
seguridad y preparación que la operación `sign`:

1. **Supresión forzada de `profile`:**
   En `ProtocolInvocationLauncherSignAndSave.processSign` (`línea 150`):
   `options.getExtraParams().remove("profile");`.
   Cualquier perfil baseline solicitado (como perfiles PAdES-B o CAdES-B) es
   eliminado silenciosamente del mapa de propiedades.
2. **Selección de procesador y plugins:**
   `selectProcessor` (`210-239`) comprueba si existe algún plugin cargado con
   permiso `Permission.INLINE_PROCESS` cuyo `checkTrigger(operation)` devuelva
   `true`. En caso afirmativo, se utiliza el procesador del plugin; en caso
   contrario, se utiliza la instancia por defecto `NativeSignDataProcessor(protocolVersion)`.
   Si el procesador del plugin devuelve múltiples operaciones en `preProcess`
   (`isMassiveSign = operations.size() > 1`), el bucle iterativo (`167-188`) invoca
   `sign()` individualmente para cada una de ellas, desplegando un diálogo de
   guardado en disco (`AOUIFactory.getSaveDataToFile`) por cada documento firmado.
   No obstante, en la fase final de devolución, `NativeSignDataProcessor.postProcess`
   retiene exclusivamente el primer resultado (`results.get(0)`), descartando del
   canal de respuesta web las firmas secundarias y sus metadatos.
3. **Identificación de formato `AUTO`:**
   Si `format` es `"AUTO"`, se invoca `ProtocolInvocationLauncherUtil.identifyFormatFromData(data, cryptoOperation)`
   (`374`). Si no es posible identificar un firmador compatible, se aborta con
   `SAF_17` (`ERROR_UNKNOWN_SIGNER`).
4. **Firma XAdES explícita obsoleta:**
   Si la operación es `SIGN`, el formato es XAdES y `extraParams` contiene `mode=explicit`
   (`386-398`), se calcula el hash SHA-1 de los datos, se sustituyen los datos por dicho hash
   y se fija `mimeType=hash/sha1`.
5. **Validación previa de firmas (`checkSignatures=true`):**
   Si se activa la comprobación, `SignValiderFactory` analiza las firmas existentes (`403-475`).
   Si la validación lanza `RuntimeConfigNeededException`:
   * En modo `headless`, aborta con `SAF_50` (`ERROR_CONFIRMATION_NEEDED`).
   * En modo interactivo con `RequestType.CONFIRM`, muestra un diálogo de confirmación
     (`AOUIFactory.showConfirmDialog`). Si el usuario acepta, se fija la propiedad
     asociada y continúa; si rechaza, lanza `CANCEL`.
   * Si la validez es `KO` (y no se trata de una firma inicial sobre datos no firmados),
     aborta con `SAF_39` (`ERROR_INVALID_SIGNATURE`).
6. **Expansión de políticas de firma:**
   Se ejecuta `ExtraParamsProcessor.expandProperties(extraParams, data, format)` (`480-488`).
   Si la política indicada es incompatible con la configuración, lanza `SAF_23` (`ERROR_INVALID_POLICY`).
7. **Posicionamiento de firma visible (PDF / PAdES):**
   Si `isRubricPositionRequired(format, extraParams)` es verdadero (`495`), se despliega
   la interfaz interactiva de rúbrica `SignPdfDialog.getVisibleSignatureDialog` (`970-978`).
   * *Discrepancia entre Javadoc y código en la evaluación de coordenadas:* Aunque el
     Javadoc de `isRubricPositionRequired` (`920-934`) estipula cuatro condiciones
     acumulativas —siendo la cuarta que los parámetros adicionales NO definan
     previamente las coordenadas de firma (`signaturePositionOnPageLowerLeftX`, etc.)—,
     la implementación real (`936-965`) omite por completo la comprobación de coordenadas.
     * Si la sede envía coordenadas fijas **sin** incluir el parámetro `visibleSignature`,
       `isRubricPositionRequired` evalúa `false` y el diálogo gráfico no se abre,
       aplicándose las coordenadas fijas de forma desatendida en el PDF.
     * Si la sede envía coordenadas fijas e incluye simultáneamente `visibleSignature=want`
       u `optional`, la comprobación incompleta ignora las coordenadas preestablecidas y
       fuerza la apertura del diálogo modal de rúbrica, sobrescribiendo las coordenadas fijas
       con las que dibuje la persona usuaria.
   * *Gestión de cancelación y fuga de estado:* Si la persona usuaria cancela el diálogo de
     rúbrica con `visibleSignature=want`, el listener `SignPdfListener` (`1040`) asigna
     erróneamente `true` a la variable estática de otra clase
     (`ProtocolInvocationLauncherSign.showRubricIsCanceled`), dejando en `false` su propio
     campo; en consecuencia, `signandsave` no eleva `SAF_43` y genera silenciosamente un PDF
     firmado sin rúbrica visual — ver [BUG-13](A1-bugs-autofirma.md#bug-13-fuga-de-estado-y-asignación-cruzada-en-showrubriciscanceled-entre-operaciones-de-firma).
8. **Selección de almacén y certificado:**
   `selectCertAndSign` (`591-707`) gestiona el almacén de claves (`AOKeyStoreManager`),
   aplica el filtro de certificados (`CertFilterManager`), evalúa la presencia de
   clave persistente (`sticky`) y despliega `AOKeyStoreDialog` (`625-636`). Si el usuario
   cancela la selección de certificado, se lanza `CANCEL` (`646`). Si se produce un error de
   PIN (`PinException`), se reintenta automáticamente fijando un filtro directo por el
   certificado ya elegido (`687-704`).
9. **Invocación criptográfica final (`executeSign`):**
   Según el enumerado `cryptoOperation` (`728-763`):
   * `SIGN`: `signer.sign(data, algorithm, pke.getPrivateKey(), pke.getCertificateChain(), extraParams)`
   * `COSIGN`: `signer.cosign(data, algorithm, pke.getPrivateKey(), pke.getCertificateChain(), extraParams)`
   * `COUNTERSIGN`: `signer.countersign(data, algorithm, target, null, pke.getPrivateKey(), pke.getCertificateChain(), extraParams)`,
     donde `target` es `CounterSignTarget.TREE` si el parámetro extra `target` es `"tree"`, o
     `CounterSignTarget.LEAFS` en cualquier otro caso.
   Si `cop` fue omitido o contiene un valor no reconocido, `cryptoOperation` es `null`,
   lo que provoca un `NullPointerException` en la sentencia `switch` (línea 728) que es
   capturado genéricamente y retornado hacia la web como `SAF_09` en lugar de `SAF_03` — ver
   [BUG-15](A1-bugs-autofirma.md#bug-15-ausencia-de-validación-de-cop-en-signandsave-provoca-nullpointerexception-y-reporte-engañoso-con-saf_09).

---

## 5. Fase de guardado en disco: el diálogo interactivo

Una vez calculada con éxito la firma binaria (`byte[] signature`), el flujo entra
en el bloque de exportación a disco (`ProtocolInvocationLauncherSignAndSave.java:536-566`):

```java
// Damos la opcion de guardar la firma generada
final String fileExts = options.getExtraParams().getProperty(AfirmaExtraParams.SAVE_FILE_EXTS);

final String fileDesc = options.getExtraParams().getProperty(AfirmaExtraParams.SAVE_FILE_DESCRIPTION, ProtocolMessages.getString("ProtocolLauncher.30")) +
    (fileExts == null ? " (*.*)" : String.format(" (*.%1s)", fileExts.replace(",", ",*.")));

try {
    AOUIFactory.getSaveDataToFile(
        signature,
        ProtocolMessages.getString("ProtocolLauncher.31"), // Titulo: "Guardar firma"
        options.getExtraParams().getProperty(AfirmaExtraParams.SAVE_FILE_CURRENT_DIR),
        getFilename(options, inputFilename, signer),
        Collections.singletonList(
            new GenericFileFilter(
                fileExts != null ? fileExts.split(",") : null,
                fileDesc
            )
        ),
        null
    );
}
catch (final AOCancelledOperationException e) {
    LOGGER.severe("Operacion cancelada por el usuario: " + e);
    throw new SocketOperationException(RESULT_CANCEL);
}
catch (final Exception e) {
    LOGGER.severe("Error en el guardado de datos: " + e);
    final String errorCode = ProtocolInvocationLauncherErrorManager.ERROR_CANNOT_SAVE_DATA;
    throw new SocketOperationException(errorCode);
}
```

### 5.1 Resolución del nombre de fichero propuesto (`getFilename`)

La determinación del nombre sugerido por defecto en el diálogo sigue una regla de
precedencia estricta en `ProtocolInvocationLauncherSignAndSave.getFilename` (`1103-1126`):

```java
private static String getFilename(final UrlParametersToSignAndSave options,
                                  final String filename,
                                  final AOSigner signer) {

    if (options.getFileName() != null) {
        return options.getFileName();
    }

    final String name;
    if (filename != null) {
        final int dotPos = filename.lastIndexOf('.');
        if (dotPos > 0) {
            name = filename.substring(0, dotPos);
        }
        else {
            name = filename;
        }
    }
    else {
        name = ProtocolMessages.getString("ProtocolLauncher.30"); // "Firma"
    }

    return signer.getSignedName(name, null);
}
```

1. **Precedencia 1: Parámetro `filename` de la URI (`options.getFileName()`).**
   Si la URL de invocación incluyó el parámetro `filename`, su valor se devuelve
   **directamente sin ninguna modificación**. No se invoca a `signer.getSignedName()`,
   por lo que si la sede envió `filename=documento` sin extensión, esa cadena exacta
   aparecerá en el selector.
   * *Guardado sin extensión si `filename` carece de ella:* Si la sede proporciona un
     nombre sin extensión y en `properties` no se configuraron extensiones de guardado
     (`filenameSaveExts`), el diálogo sólo registra el filtro global «Todos los archivos (*.*)».
     Dado que el ajuste automático de extensión de `JSEUIManager.java:768-780` exige que el
     filtro activo sea un `FileNameExtensionFilter` distinto del filtro global
     (`fileChooser.getAcceptAllFileFilter() != ff`), el fichero se escribe físicamente en
     disco sin ninguna extensión (`documento`), requiriendo intervención manual posterior
     del usuario para abrirlo o asociarlo a una aplicación.
2. **Precedencia 2: Nombre base del fichero cargado interactivamente (`filename`).**
   Si la URI no incluía `filename` y el documento se obtuvo interactivamente mediante el
   selector de entrada (`data == null`), se toma el nombre del fichero seleccionado,
   se le retira su extensión original (todo lo que sigue al último `.`) y se pasa el
   nombre base a `signer.getSignedName(name, null)`.
3. **Precedencia 3: Nombre por defecto genérico (`"Firma"`).**
   Si no vino `filename` en la URI y los datos vinieron directamente en `dat` o `fileid`
   (por lo que no hubo diálogo de carga interactiva), se toma la constante localizada
   `ProtocolLauncher.30` (`"Firma"`) y se pasa a `signer.getSignedName("Firma", null)`.

#### Comportamiento de `signer.getSignedName(name, null)` por formato:

* **PAdES / PDF (`AOPDFSigner.java:348-357`):**
  Si `name` termina en `.pdf` (insensible a mayúsculas), lo conserva; si no, le añade `.pdf`.
  Resultado típico: `documento.pdf` o `Firma.pdf`.
* **CAdES (`AOCAdESSigner.java:449-451`):**
  Añade `.csig` al nombre.
  Resultado típico: `documento.csig` o `Firma.csig`.
* **XAdES y XMLDSig (`AOXAdESSigner.java:1215-1217`, `AOXMLDSigSigner.java:2337-2339`):**
  Añade `.xsig` al nombre.
  Resultado típico: `documento.xsig` o `Firma.xsig`.
* **ASiC-S (`AOXAdESASiCSSigner.java:245-247`):**
  Añade `.asics` al nombre.
  Resultado típico: `documento.asics` o `Firma.asics`.

### 5.2 Lógica del diálogo de guardado (`JSEUIManager.saveDataToFile`)

La implementación gráfica subyacente en
`afirma-ui-core-jse/src/main/java/es/gob/afirma/ui/core/jse/JSEUIManager.java:707-815`
ejecuta las siguientes acciones dentro de un bucle `while (tryAgain)`:

1. **Creación del diálogo:**
   Instancia un `CustomFileChooserForSave` (derivado de Swing `JFileChooser`). Fija el
   título a `ProtocolLauncher.31` (`"Guardar firma"`).
2. **Configuración de filtros de extensión:**
   Si se proporcionó `filenameSaveExts`, crea un `FileNameExtensionFilter` con la descripción
   y las extensiones configuradas, marcando `setAcceptAllFileFilterUsed(true)` para permitir
   también «Todos los archivos (*.*)».
3. **Ajuste automático de extensión:**
   Si el usuario escribe un nombre sin extensión y ha dejado seleccionado un filtro de
   extensión específico, el sistema extrae la primera extensión del filtro y se la concatena
   automáticamente (`JSEUIManager.java:775`):
   ```java
   file = new File(file.getParent(), file.getName() + '.' + exts[0].toLowerCase());
   ```
4. **Detección de colisión de ficheros y confirmación de sobrescritura:**
   Si el fichero destino ya existe en el disco (`file.exists()`, líneas 782-803), se
   muestra una ventana de confirmación (`JOptionPane.showConfirmDialog`) con el mensaje
   `JSEUIManager.77` (*«El fichero ya existe. ¿Desea sobrescribirlo?»*):
   * **Opción «Sí» (`JOptionPane.YES_OPTION`):** Se autoriza la sobrescritura y se
     prosigue con la escritura en disco.
   * **Opción «No» (`JOptionPane.NO_OPTION`):** Se activa `tryAgain = true`, lo que
     vuelve a presentar de inmediato el diálogo `JFileChooser` para que el usuario elija
     otra ruta o nombre distinto.
   * **Opción «Cancelar» (`JOptionPane.CANCEL_OPTION`) o cierre de ventana:** Lanza
     `AOCancelledOperationException`.
5. **Cancelación del guardado:**
   Si el usuario pulsa «Cancelar» en el selector de ficheros o en la ventana de colisión,
   se captura la excepción y se propaga como `SocketOperationException(RESULT_CANCEL)`
   (`ProtocolInvocationLauncherSignAndSave.java:559`), devolviendo la cadena textual
   `"CANCEL"` al entorno web invocador.
6. **Escritura binaria física:**
   Se abre un `FileOutputStream(file)` y se escriben íntegramente los bytes de la firma
   generada (`JSEUIManager.java:808-812`). Si ocurre un fallo de E/S (permisos insuficientes,
   disco lleno o ruta de red caída), se captura la excepción genérica y se responde con
   `SAF_05` (`ERROR_CANNOT_SAVE_DATA`, `ProtocolInvocationLauncherSignAndSave.java:564`).

---

## 6. Formato de la respuesta del protocolo

Completado el guardado en disco, la firma binaria y el certificado del firmante se
empaquetan en un objeto `SignResult` y se transfieren a `NativeSignDataProcessor.postProcess`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/NativeSignDataProcessor.java:54-104`).

### 6.1 Estructura textual de la respuesta

La respuesta generada por `NativeSignDataProcessor` consta de campos delimitados
por el carácter pleca (`|`):

```
campo_1 | campo_2 [ | campo_3 ]
```

| Posición | Contenido | Codificación sin clave (`key` ausente) | Codificación con clave (`key` presente) |
|---|---|---|---|
| Campo 1 | Certificado de firma (X.509 codificado en DER) | Base64 URL-safe estándar (`Base64.encode(certEncoded, true)`) | Cifrado AES con la clave `key` en Base64 (`cipher.cipher(certEncoded)`) |
| Campo 2 | Firma electrónica generada | Base64 URL-safe estándar (`Base64.encode(sign, true)`) | Cifrado AES con la clave `key` en Base64 (`cipher.cipher(sign)`) |
| Campo 3 *(opcional)* | Metadatos en JSON de los datos firmados | Base64 URL-safe del JSON UTF-8 (`Base64.encode(json, true)`) | Cifrado AES del JSON UTF-8 con la clave `key` |

#### Condiciones para la inclusión del Campo 3:

El tercer campo únicamente se añade si se cumplen conjuntamente dos requisitos
(`NativeSignDataProcessor.java:77, 97`):
1. La versión del protocolo es 3 o superior (`getProtocolVersion() >= 3`).
2. El documento se cargó de manera interactiva a través del selector de ficheros
   (`data == null` inicial), en cuyo caso contiene el JSON:
   ```json
   {"filename": "nombre_del_fichero_seleccionado.ext"}
   ```
Si los datos vinieron en la propia invocación (`dat` o `fileid`), `extraData` es nulo y
la respuesta consta estrictamente de dos campos (`cert|sign`). Asimismo, en caso de
multifirma expandida por un plugin (`isMassiveSign == true`), `NativeSignDataProcessor.java:58-60`
toma estrictamente `final SignResult result = results.get(0);`, devolviendo únicamente la
primera firma a la aplicación web y descartando todas las firmas secundarias y sus metadatos
del canal de respuesta.

### 6.2 Entrega del resultado según el transporte

1. **Servidor intermedio (`!bySocket`):**
   * En éxito: la cadena `dataToSend.toString()` se envía mediante un HTTP POST al
     `stservlet` asociado al identificador `id` (`ProtocolInvocationLauncher.java:612`).
   * En error: el código de error (`SAF_nn` o `"CANCEL"`) se codifica en URL con UTF-8
     (`URLEncoder.encode(msg, "UTF-8")`) y se sube igualmente a `stservlet` (`602-604`).
2. **Socket local y WebSocket (`bySocket == true`):**
   * La cadena `dataToSend.toString()` o el código de error no cifrado (`SAF_nn` o `"CANCEL"`)
     se devuelven directamente como valor de retorno de `processSign` hacia el hilo
     procesador del socket (`CommandProcessorThread`) o el servidor WebSocket.

### 6.3 Recepción y tratamiento en el cliente JavaScript (`autoscript.js`)

En el script cliente oficial (`afirma-ui-miniapplet-deploy/.../autoscript.js`), la
función `signAndSaveToFile` fija internamente la variable de operación activa como una
firma estándar:

```javascript
currentOperation = OPERATION_SIGN;
```

(`autoscript.js:1864, 2767, 3920`). En consecuencia, el cliente web procesa el resultado
a través del método común `processSignResponse(data)` (`autoscript.js:2512-2545` en WebSocket,
`3482-3496` en socket local, `4584-4608` en servidor intermedio):

* El cliente web recibe de vuelta la firma completa y el certificado del usuario,
  pudiendo utilizarlos para enviar la firma al backend de la sede o verificar la identidad
  del firmante, a pesar de que el usuario ya dispone de una copia guardada en su disco.
* Las funciones de callback de éxito reciben los parámetros deserializados:
  `successCallback(signature, certificate, extraInfo)`.

#### Tratamiento de metadatos `extraInfo` y delimitador pleca en WebSocket:
En el flujo por WebSocket (`autoscript.js:2537`), la extracción del tercer campo se ejecuta como
`extraInfo = Base64.decode(data.substring(sepPos2), true);`. Al no sumar `1` al índice `sepPos2`,
la subcadena extraída incluye el delimitador pleca inicial (`|`). No obstante, la función
`Base64.decode` (`autoscript.js:5091`) sanea previamente la entrada aplicando la expresión regular
`input.replace(/[^A-Za-z0-9\-\_\=]/g, "")`, purgando el carácter `|` antes de iniciar la decodificación
y garantizando que el objeto JSON con el nombre del fichero (`{"filename":"..."}`) se procese
correctamente sin provocar errores de sintaxis ni excepciones.

---

## 7. Catálogo de errores de `signandsave`

Cualquier excepción producida durante el ciclo de vida de `signandsave` se traduce
en un código de error normalizado con prefijo `SAF_` gestionado por
`ProtocolInvocationLauncherErrorManager` (`afirma-simple/.../ProtocolInvocationLauncherErrorManager.java`)
o en el literal `CANCEL` si la interrupción proviene de la voluntad expresa del usuario.

| Código | Mensaje localizado en AutoFirma | Causa y contexto en `signandsave` | Cita en código |
|---|---|---|---|
| `CANCEL` | *(Literal `"CANCEL"`)* | El usuario canceló activamente alguna de las ventanas modales: diálogo de carga de fichero (`AOUIFactory.getLoadFiles`), confirmación de advertencia de firma previa, selección de certificado (`AOKeyStoreDialog`), rúbrica PDF o diálogo de guardado en disco (`AOUIFactory.getSaveDataToFile`). | `ProtocolInvocationLauncherSignAndSave.java:348, 437, 559, 646, 873` |
| `SAF_00` | *Error en la lectura de los datos a firmar.* | Fallo de lectura del fichero seleccionado por el usuario al cargar datos interactivamente (`FileInputStream` / `AOUtil.getDataFromInputStream`). | `ProtocolInvocationLauncherSignAndSave.java:366` |
| `SAF_01` | *No se ha pasado la URI a procesar.* | El objeto de parámetros recibido es nulo (`options == null`). | `ProtocolInvocationLauncherSignAndSave.java:125` |
| `SAF_03` | *Parámetros incorrectos.* | Parámetros de invocación inválidos: falta `format` o `algorithm`, algoritmo no soportado en la lista blanca (incluyendo el rechazo de algoritmos ECDSA — ver [BUG-05](A1-bugs-autofirma.md#bug-05-rechazo-de-algoritmos-ecdsa-en-la-operación-signandsave)), nombre de fichero con caracteres ilegales (`\/:*?"<>|`), clave de cifrado con longitud distinta de 32, identificador de sesión no alfanumérico o mayor de 40 caracteres, o ausencia de `cop` cuando el firmador implementa `OptionalDataInterface`. | `UrlParametersToSignAndSave.java:215, 221, 273, 281, 285, 335`, `ProtocolInvocationLauncher.java:630, 636, 748` |
| `SAF_04` | *Operación no soportada.* | Operación no admitida: el valor de `cop` no corresponde a `SIGN`, `COSIGN` ni `COUNTERSIGN` (`SignOperation.Operation`), o el firmador no soporta la variante solicitada. | `ProtocolInvocationLauncherSignAndSave.java:761, 863` |
| `SAF_05` | *Error al guardar los datos.* | Fallo de E/S al escribir la firma resultante en el fichero local elegido por el usuario (permisos denegados, ruta bloqueada o fallo de disco). | `ProtocolInvocationLauncherSignAndSave.java:563` |
| `SAF_06` | *Formato de firma no soportado.* | El formato indicado en `format` no dispone de ningún proveedor registrado en `AOSignerFactory`. | `ProtocolInvocationLauncherSignAndSave.java:261` |
| `SAF_08` | *Error al acceder al almacén de claves.* | Imposible instanciar el gestor del almacén (`AOKeyStoreManagerFactory`) o fallo fatal al inicializar el diálogo de certificados. | `ProtocolInvocationLauncherSignAndSave.java:614, 655` |
| `SAF_09` | *Error durante la operación de firma.* | Excepción interna del motor criptográfico (`AOException` o genérica) al calcular la firma con la clave privada. También se genera si `cop` es omitido o no reconocido y los datos vienen provistos, provocando `NullPointerException` en el `switch` de `executeSign` tras haber solicitado certificado y PIN — ver [BUG-15](A1-bugs-autofirma.md#bug-15-ausencia-de-validación-de-cop-en-signandsave-provoca-nullpointerexception-y-reporte-engañoso-con-saf_09). | `ProtocolInvocationLauncherSignAndSave.java:877, 882` |
| `SAF_12` | *Error en el cifrado de datos.* | Error simétrico (`EncryptingException`) al cifrar la respuesta con la clave AES provista (`key`). | `ProtocolInvocationLauncherSignAndSave.java:198` |
| `SAF_13` | *Se ha pedido un acceso a una dirección local.* | La URL de `stservlet` o `rtservlet` apunta a `localhost` o `127.0.0.1`, vulnerando la política de seguridad contra SSRF local. | `ProtocolInvocationLauncher.java:624`, `UrlParameters.java:386` |
| `SAF_14` | *Se necesita una versión más moderna de Autofirma...* | La petición exige características no disponibles en la versión instalada (`ParameterNeedsUpdatedVersionException`). | `ProtocolInvocationLauncher.java:618` |
| `SAF_15` | *Error al descifrar los datos.* | Fallo al descifrar el XML o los datos obtenidos desde `rtservlet` con la clave `key`. | `ProtocolInvocationLauncher.java:562` |
| `SAF_16` | *No se pueden recuperar los datos del servidor.* | Fallo de comunicación HTTP al descargar los datos de la operación desde `rtservlet` mediante `fileid`. | `ProtocolInvocationLauncher.java:556` |
| `SAF_17` | *Firmador no reconocido.* | Con `format=AUTO`, no se ha podido inferir el tipo de documento ni asignar un firmador adecuado a partir de los datos. | `ProtocolInvocationLauncherSignAndSave.java:378` |
| `SAF_18` | *Error en la decodificación del certificado.* | Error al extraer la codificación DER binaria del certificado de firma (`CertificateEncodingException`). | `ProtocolInvocationLauncherSignAndSave.java:574` |
| `SAF_19` | *No hay ningún certificado válido en su almacén.* | El almacén de claves se cargó correctamente pero ningún certificado cumple los filtros o todos están caducados (`AOCertificatesNotFoundException`). | `ProtocolInvocationLauncherSignAndSave.java:650` |
| `SAF_21` | *Versión de procedimiento no soportada.* | La versión de protocolo solicitada supera la versión máxima soportada por la aplicación (`ProtocolVersion.VERSION_4`). | `ProtocolInvocationLauncherSignAndSave.java:135` |
| `SAF_23` | *Se ha establecido una política de firma no válida...* | Conflicto entre las propiedades de política configuradas (`expPolicy`) y las opciones del formato (`IncompatiblePolicyException`). | `ProtocolInvocationLauncherSignAndSave.java:486` |
| `SAF_28` | *Error al procesar el fichero PDF.* | El fichero provisto para firmar en PAdES no es un PDF válido o está corrupto (`InvalidPdfException`). | `ProtocolInvocationLauncherSignAndSave.java:784` |
| `SAF_29` | *Error al procesar el fichero XML.* | El fichero provisto para firmar en XAdES/XMLDSig está mal formado sintácticamente (`InvalidXMLException`). | `ProtocolInvocationLauncherSignAndSave.java:789` |
| `SAF_30` | *Los datos no tienen el formato adecuado.* | Violación del formato estructural esperado por el firmador (`AOFormatFileException`). | `ProtocolInvocationLauncherSignAndSave.java:794` |
| `SAF_31` | *El fichero no contiene datos a firmar.* | Documento de firma vacío o sin contenido firmable (`AOInvalidFormatException`). | `ProtocolInvocationLauncherSignAndSave.java:814` |
| `SAF_32` | *La factura ya está firmada.* | Se intenta firmar una FacturaE que ya contiene una firma previa no admitida (`EFacturaAlreadySignedException`). | `ProtocolInvocationLauncherSignAndSave.java:804` |
| `SAF_38` | *Error al procesar la factura electrónica.* | El documento XML no cumple con el esquema FacturaE (`InvalidEFacturaDataException`). | `ProtocolInvocationLauncherSignAndSave.java:799` |
| `SAF_39` | *La firma de entrada no es válida.* | Con `checkSignatures=true`, las firmas previas del documento no superaron la validación criptográfica (`SignValidity.KO`). | `ProtocolInvocationLauncherSignAndSave.java:471, 868` |
| `SAF_40` | *Error al recuperar el documento del servidor.* | Fallo de comunicación en fase trifásica (`AOTriphaseException`). | `ProtocolInvocationLauncherSignAndSave.java:779` |
| `SAF_41` | *El trámite web no es compatible con la versión de Autofirma...* | La versión mínima de cliente exigida en el parámetro `mcv` es estrictamente superior a la versión de AutoFirma instalada. | `ProtocolInvocationLauncherSignAndSave.java:144` |
| `SAF_42` | *Error en el postprocesado de datos a enviar.* | Error durante la fase de ensamblado de la respuesta en el procesador de datos (`processor.postProcess`). | `ProtocolInvocationLauncherSignAndSave.java:203` |
| `SAF_43` | *Es obligatorio mostrar la firma en el documento PDF.* | En firma PAdES con `visibleSignature=want`, el usuario canceló el diálogo interactivo de posicionamiento de rúbrica. | `ProtocolInvocationLauncherSignAndSave.java:179, 501` |
| `SAF_44` | *No se han indicado datos para firmar.* | La operación requiere contenido que no fue provisto (`ContainsNoDataException`). | `ProtocolInvocationLauncherSignAndSave.java:809` |
| `SAF_50` | *Se requiere confirmación del usuario para continuar.* | En modo `headless=true`, se detectó una situación ambigua en la firma previa que habría requerido confirmación interactiva. | `ProtocolInvocationLauncherSignAndSave.java:426, 823` |
| `SAF_51` | *Tipo de clave no compatible con el algoritmo.* | Incompatibilidad entre el tipo de clave privada del certificado (`RSA`, `EC`) y el algoritmo solicitado. | `ProtocolInvocationLauncherSignAndSave.java:667` |
| `SAF_52` | *El almacén de claves está bloqueado.* | El token criptográfico o tarjeta inteligente se encuentra bloqueado por exceso de intentos erróneos de PIN (`LockedKeyStoreException`). | `ProtocolInvocationLauncherSignAndSave.java:684` |

