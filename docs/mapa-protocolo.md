# Mapa del protocolo AutoFirma (v1.9.2) y esqueleto de auditoría

Este documento recoge los literales declarados en el árbol del cliente AutoFirma a etiqueta fijada (**v1.9.2**), su cruce con el trámite de sede de rFirma y el esqueleto formal para la auditoría de compatibilidad de parámetros (#603).

> [!IMPORTANT]
> **Un mapa sin diferencias significa «no han cambiado los nombres», nunca «somos compatibles».**  
> La compatibilidad real depende del flujo de control, la obligatoriedad, los valores por defecto y el orden de validación en el código ejecutable de ambos lados.

---

## 1. Parámetros declarados en las siete clases `UrlParameters*`

Literales de parámetros de entrada declarados como constantes en `afirma-core` (`es.gob.afirma.core.misc.protocol`).

### `UrlParameters`

| Constante | Parámetro URL |
|---|---|
| `APP_NAME_PARAM` | `appname` |
| `ACTIVE_WAITING_PARAM` | `aw` |
| `DATA_PARAM` | `dat` |
| `FILE_ID_PARAM` | `fileid` |
| `GZIPPED_DATA_PARAM` | `gzip` |
| `KEY_PARAM` | `key` |
| `KEYSTORE_OLD_PARAM` | `keystore` |
| `KEYSTORE_PARAM` | `ksb64` |
| `MINIMUM_CLIENT_VERSION_PARAM` | `mcv` |
| `PROPERTIES_PARAM` | `properties` |
| `RETRIEVE_SERVLET_PARAM` | `rtservlet` |
| `STORAGE_SERVLET_PARAM` | `stservlet` |

### `UrlParametersForBatch`

| Constante | Parámetro URL |
|---|---|
| `PARAM_BATCH_POSTSIGNER` | `batchpostsignerurl` |
| `PARAM_BATCH_PRESIGNER` | `batchpresignerurl` |
| `ID_PARAM` | `id` |
| `PARAM_JSON_BATCH` | `jsonbatch` |
| `PARAM_LOCAL_BATCH_PROCESS` | `localBatchProcess` |
| `PARAM_NEED_CERT` | `needcert` |
| `RESET_STICKY_PARAM` | `resetsticky` |
| `PARAM_STICKY` | `sticky` |
| `PARAM_VER` | `ver` |

### `UrlParametersToLoad`

| Constante | Parámetro URL |
|---|---|
| `DESCRIPTION_PARAM` | `desc` |
| `EXTENSIONS_PARAM` | `exts` |
| `FILEPATH_PARAM` | `filePath` |
| `MULTILOAD_PARAM` | `multiload` |
| `TITLE_PARAM` | `title` |
| `VER_PARAM` | `ver` |

### `UrlParametersToSave`

| Constante | Parámetro URL |
|---|---|
| `FILETYPE_DESCRIPTION_PARAM` | `desc` |
| `FILENAME_EXTS_PARAM` | `exts` |
| `FILENAME_PARAM` | `filename` |
| `ID_PARAM` | `id` |
| `TITLE_PARAM` | `title` |
| `VER_PARAM` | `ver` |

### `UrlParametersToSelectCert`

| Constante | Parámetro URL |
|---|---|
| `ID_PARAM` | `id` |
| `RESET_STICKY_PARAM` | `resetsticky` |
| `STICKY_PARAM` | `sticky` |
| `VER_PARAM` | `ver` |

### `UrlParametersToSign`

| Constante | Parámetro URL |
|---|---|
| `ALGORITHM_PARAM` | `algorithm` |
| `FORMAT_PARAM` | `format` |
| `ID_PARAM` | `id` |
| `RESET_STICKY_PARAM` | `resetsticky` |
| `STICKY_PARAM` | `sticky` |
| `VER_PARAM` | `ver` |

**`KNOWN_PARAMETERS` declarados:**

| Constante referenciada | Parámetro URL resuelto |
|---|---|
| `FORMAT_PARAM` | `format` |
| `ALGORITHM_PARAM` | `algorithm` |
| `ID_PARAM` | `id` |
| `VER_PARAM` | `ver` |
| `STICKY_PARAM` | `sticky` |
| `RESET_STICKY_PARAM` | `resetsticky` |
| `PROPERTIES_PARAM` | `properties` |
| `DATA_PARAM` | `dat` |
| `GZIPPED_DATA_PARAM` | `gzip` |
| `RETRIEVE_SERVLET_PARAM` | `rtservlet` |
| `STORAGE_SERVLET_PARAM` | `stservlet` |
| `KEY_PARAM` | `key` |
| `FILE_ID_PARAM` | `fileid` |
| `KEYSTORE_OLD_PARAM` | `keystore` |
| `KEYSTORE_PARAM` | `ksb64` |
| `ACTIVE_WAITING_PARAM` | `aw` |
| `MINIMUM_CLIENT_VERSION_PARAM` | `mcv` |
| `APP_NAME_PARAM` | `appname` |

**Algoritmos de firma declarados en `SUPPORTED_SIGNATURE_ALGORITHMS`:**

- `SHA1`
- `SHA256`
- `SHA384`
- `SHA512`
- `SHA1withRSA`
- `SHA256withRSA`
- `SHA384withRSA`
- `SHA512withRSA`
- `SHA1withECDSA`
- `SHA256withECDSA`
- `SHA384withECDSA`
- `SHA512withECDSA`

### `UrlParametersToSignAndSave`

| Constante | Parámetro URL |
|---|---|
| `ALGORITHM_PARAM` | `algorithm` |
| `CRYPTO_OPERATION_PARAM` | `cop` |
| `FILENAME_PARAM` | `filename` |
| `FORMAT_PARAM` | `format` |
| `ID_PARAM` | `id` |
| `RESET_STICKY_PARAM` | `resetsticky` |
| `STICKY_PARAM` | `sticky` |
| `VER_PARAM` | `ver` |

**`KNOWN_PARAMETERS` declarados:**

| Constante referenciada | Parámetro URL resuelto |
|---|---|
| `CRYPTO_OPERATION_PARAM` | `cop` |
| `FORMAT_PARAM` | `format` |
| `ALGORITHM_PARAM` | `algorithm` |
| `FILENAME_PARAM` | `filename` |
| `ID_PARAM` | `id` |
| `VER_PARAM` | `ver` |
| `STICKY_PARAM` | `sticky` |
| `RESET_STICKY_PARAM` | `resetsticky` |
| `PROPERTIES_PARAM` | `properties` |
| `DATA_PARAM` | `dat` |
| `GZIPPED_DATA_PARAM` | `gzip` |
| `RETRIEVE_SERVLET_PARAM` | `rtservlet` |
| `STORAGE_SERVLET_PARAM` | `stservlet` |
| `KEY_PARAM` | `key` |
| `FILE_ID_PARAM` | `fileid` |
| `KEYSTORE_OLD_PARAM` | `keystore` |
| `KEYSTORE_PARAM` | `ksb64` |
| `ACTIVE_WAITING_PARAM` | `aw` |
| `MINIMUM_CLIENT_VERSION_PARAM` | `mcv` |

**Algoritmos de firma declarados en `SUPPORTED_SIGNATURE_ALGORITHMS`:**

- `SHA1`
- `SHA256`
- `SHA384`
- `SHA512`
- `SHA1withRSA`
- `SHA256withRSA`
- `SHA384withRSA`
- `SHA512withRSA`

---

## 2. Códigos de error del protocolo (`SAF_xx`)

Catálogo de códigos `SAF_00`…`SAF_52` declarados en `ProtocolInvocationLauncherErrorManager.java` con su mensaje oficial en castellano de `protocolmessages.properties`.

| Código | Constante Java | Clave de recurso | Mensaje en castellano |
|---|---|---|---|
| `SAF_00` | `ERROR_CANNOT_READ_DATA` | `ProtocolLauncher.0` | No se han podido leer los datos a firmar |
| `SAF_01` | `ERROR_NULL_URI` | `ProtocolLauncher.1` | La URL recibida es nula |
| `SAF_02` | `ERROR_UNSUPPORTED_PROTOCOL` | `ProtocolLauncher.2` | Protocolo no soportado |
| `SAF_03` | `ERROR_PARAMS` | `ProtocolLauncher.3` | Error en los parámetros de entrada |
| `SAF_04` | `ERROR_UNSUPPORTED_OPERATION` | `ProtocolLauncher.4` | Operación no soportada. Compruebe que dispone de la última versión de Autofirma. |
| `SAF_05` | `ERROR_CANNOT_SAVE_DATA` | `ProtocolLauncher.5` | No se ha podido guardar los datos |
| `SAF_06` | `ERROR_UNSUPPORTED_FORMAT` | `ProtocolLauncher.6` | Formato de firma no soportado |
| `SAF_07` | `ERROR_CANNOT_FIND_KEYSTORE` | `ProtocolLauncher.7` | No se ha podido determinar el almacén de claves a utilizar |
| `SAF_08` | `ERROR_CANNOT_ACCESS_KEYSTORE` | `ProtocolLauncher.8` | Error accediendo al almacén de claves y certificados |
| `SAF_09` | `ERROR_SIGNATURE_FAILED` | `ProtocolLauncher.9` | Error realizando la firma electrónica |
| `SAF_10` | `ERROR_NO_CERTIFICATES_SYSTEM` | `ProtocolLauncher.10` | No hay certificados de firma instalados en el sistema |
| `SAF_11` | `ERROR_SENDING_RESULT` | `ProtocolLauncher.11` | Error en el envio del resultado de la operación. |
| `SAF_12` | `ERROR_ENCRIPTING_DATA` | `ProtocolLauncher.12` | Error en el cifrado de los datos a enviar |
| `SAF_13` | `ERROR_LOCAL_ACCESS_BLOCKED` | `ProtocolLauncher.13` | Se ha pedido acceso a una dirección local, pero por seguridad se ha bloqueado el acceso |
| `SAF_14` | `ERROR_OBSOLETE_APP` | `ProtocolLauncher.14` | <html>La aplicaci&oacute;n est&aacute; obsoleta y no puede procesarse la petici&oacute;n.<br>Por favor, instale una versi&oacute;n actualizada y reintente el proceso de nuevo.</html> |
| `SAF_15` | `ERROR_DECRYPTING_DATA` | `ProtocolLauncher.15` | Error en el descifrado de los datos |
| `SAF_16` | `ERROR_RECOVERING_DATA` | `ProtocolLauncher.16` | Error al recuperar los datos del servidor intermedio |
| `SAF_17` | `ERROR_UNKNOWN_SIGNER` | `ProtocolLauncher.17` | Los datos proporcionados no son una firma electrónica reconocida |
| `SAF_18` | `ERROR_DECODING_CERTIFICATE` | `ProtocolLauncher.18` | Error al descodificar el certificado de firma |
| `SAF_19` | `ERROR_NO_CERTIFICATES_KEYSTORE` | `ProtocolLauncher.19` | No hay ningun certificado válido en su almacén. Compruebe las fechas de caducidad e instale un certificado válido. |
| `SAF_20` | `ERROR_LOCAL_BATCH_SIGN` | `ProtocolLauncher.20` | Error en el procesado del lote de firma. |
| `SAF_21` | `ERROR_UNSUPPORTED_PROCEDURE` | `ProtocolLauncher.21` | La versión de Autofirma instalada no es compatible con este trámite. Actualice a la última versión disponible. |
| `SAF_22` | `ERROR_UNSOPPORTED_WEB_PROCEDURE` | `ProtocolLauncher.22` | El trámite web no es compatible con la versión de Autofirma instalada. Consulte las instrucciones del trámite para saber que versión debe instalar. |
| `SAF_23` | `ERROR_INVALID_POLICY` | `ProtocolLauncher.33` | Se ha establecido una política de firma no válida o parámetros no compatibles con ella. |
| `SAF_24` | `ERROR_RECOVERING_LOG` | `ProtocolLauncher.34` | Error al obtener el registro de log acumulado hasta la ejecución actual. |
| `SAF_25` | `ERROR_CANNOT_LOAD_DATA` | `ProtocolLauncher.35` | Error en la lectura de los datos a cargar. |
| `SAF_26` | `ERROR_CONTACT_BATCH_SERVICE` | `ProtocolLauncher.36` | Error en la comunicación con el servicio de firma de lotes. |
| `SAF_27` | `ERROR_BATCH_SIGNATURE` | `ProtocolLauncher.37` | El servicio informó de un error durante la firma del lote. |
| `SAF_28` | `ERROR_INVALID_PDF` | `ProtocolLauncher.38` | El fichero no es un PDF o es un PDF no soportado. |
| `SAF_29` | `ERROR_INVALID_XML` | `ProtocolLauncher.39` | Las firmas XAdES Enveloped solo pueden realizarse sobre datos XML. |
| `SAF_30` | `ERROR_INVALID_DATA` | `ProtocolLauncher.40` | El formato de los datos a firmar no es adecuado para el tipo de firma seleccionado. |
| `SAF_31` | `ERROR_NO_SIGN_DATA` | `ProtocolLauncher.41` | Los datos introducidos no se corresponden con un objeto de firma. |
| `SAF_32` | `ERROR_FACE_ALREADY_SIGNED` | `ProtocolLauncher.42` | La factura ya tiene una firma electrónica y no admite firmas adicionales. |
| `SAF_33` | `ERROR_PDF_WRONG_PASSWORD` | `ProtocolLauncher.43` | La contraseña proporcionada no es válida para el PDF actual o no se proporcionó ninguna contraseña. |
| `SAF_34` | `ERROR_PDF_UNREG_SIGN` | `ProtocolLauncher.44` | El PDF contiene firmas no registradas. |
| `SAF_35` | `ERROR_PDF_CERTIFIED` | `ProtocolLauncher.45` | El PDF está certificado. |
| `SAF_36` | `ERROR_CANNOT_FIND_SSL_KEYSTORE` | `ProtocolLauncher.46` | No se ha podido encontrar el almacén de claves SSL para la comunicación segura. Restaure la instalación de Autofirma para generar uno nuevo. |
| `SAF_37` | `ERROR_CANNOT_ACCESS_SSL_KEYSTORE` | `ProtocolLauncher.47` | No se ha podido acceder al almacén de claves SSL para la comunicación segura. Restaure la instalación de Autofirma para generar uno nuevo. |
| `SAF_38` | `ERROR_INVALID_FACTURAE` | `ProtocolLauncher.48` | El archivo que intenta firmar no es una factura electrónica reconocida. |
| `SAF_39` | `ERROR_INVALID_SIGNATURE` | `ProtocolLauncher.49` | La firma de entrada no es válida. |
| `SAF_40` | `ERROR_RECOVER_SERVER_DOCUMENT` | `ProtocolLauncher.50` | Error al recuperar el documento |
| `SAF_41` | `ERROR_MINIMUM_VERSION_NON_SATISTIED` | `ProtocolLauncher.53` | El uso de este trámite web requiere una versión más reciente de Autofirma. Actualice a la última versión disponible. |
| `SAF_42` | `ERROR_POSTPROCESSING_DATA` | `ProtocolLauncher.54` | Error al postprocesar una firma, probablemente debido a un plugin que afecte al sistema de firma. |
| `SAF_43` | `ERROR_VISIBLE_SIGNATURE` | `ProtocolLauncher.55` | Error durante la firma visible del PDF. |
| `SAF_44` | `ERROR_SIGN_WITHOUT_DATA` | `ProtocolLauncher.56` | La firma no contiene los datos y no se compatible con la configuración seleccionada |
| `SAF_45` | `ERROR_CANNOT_OPEN_SOCKET` | `ProtocolLauncher.57` | No se pudo abrir un socket para la comunicación con la aplicación |
| `SAF_46` | `ERROR_INVALID_SESSION_ID` | `ProtocolLauncher.58` | Id de sesión inválido |
| `SAF_47` | `ERROR_EXTERNAL_REQUEST_TO_SOCKET` | `ProtocolLauncher.59` | Peticion al socket desde IP externa o sin identificar |
| `SAF_48` | `ERROR_PDF_SHADOW_ATTACK` | `ProtocolLauncher.63` | Posible PDF Shadow Attack |
| `SAF_49` | `ERROR_SIGNING_LTS_SIGNATURE` | `ProtocolLauncher.64` | Multifirma de firma de archivo |
| `SAF_50` | `ERROR_CONFIRMATION_NEEDED` | `ProtocolLauncher.65` | La operación puede generar firmas no validas, por lo que no se puede continuar sin confirmacion de usuario. |
| `SAF_51` | `ERROR_INCOMPATIBLE_KEY_TYPE` | `ProtocolLauncher.66` | El tipo de clave del certificado no está soportado. |
| `SAF_52` | `ERROR_LOCKED_KEYSTORE` | `ProtocolLauncher.67` | El almacén de claves esta bloqueado. Siga las instrucciones del proveedor del almacén o tarjeta para desbloquearlo. |

---

## 3. Claves puestas en URL por operación en `autoscript.js`

Parámetros que el cliente web publicado (`autoscript.js`) incluye en la URL según el tipo de petición hacia el cliente nativo.

| Operación | Claves en la URL |
|---|---|
| `batch` | `op`, `idsession`, `batchpresignerurl`, `batchpostsignerurl`, `properties`, `ksb64`, `keystore`, `sticky`, `resetsticky`, `appname`, `needcert`, `dat`, `jsonbatch`, `localBatchProcess`, `mcv` |
| `cosign` | `op`, `idsession`, `algorithm`, `format`, `properties`, `ksb64`, `keystore`, `sticky`, `resetsticky`, `appname`, `dat`, `mcv` |
| `countersign` | `op`, `idsession`, `algorithm`, `format`, `properties`, `ksb64`, `keystore`, `sticky`, `resetsticky`, `appname`, `dat`, `mcv` |
| `load` | `op`, `idsession`, `title`, `exts`, `desc`, `filePath`, `multiload`, `mcv` |
| `save` | `op`, `idsession`, `title`, `filename`, `exts`, `desc`, `dat`, `mcv` |
| `selectcert` | `op`, `idsession`, `properties`, `ksb64`, `keystore`, `sticky`, `resetsticky`, `mcv` |
| `service (arranque)` | `ports`, `v`, `jvc`, `idsession` |
| `sign` | `op`, `idsession`, `algorithm`, `format`, `properties`, `ksb64`, `keystore`, `sticky`, `resetsticky`, `appname`, `dat`, `mcv` |
| `signandsave` | `op`, `idsession`, `cop`, `algorithm`, `format`, `properties`, `filename`, `ksb64`, `keystore`, `sticky`, `resetsticky`, `appname`, `dat`, `mcv` |
| `websocket (arranque)` | `ports`, `v`, `jvc`, `idsession` |

---

## 4. Cruce contra el vocabulario del trámite de sede de rFirma

Contraste del vocabulario de parámetros del original contra el código del backend en `rfirma-app/src-tauri/src/site/`.

Total de nombres únicos identificados en el original: **35**.

### Nombres del original que rFirma NO menciona

> [!WARNING]  
> Los siguientes parámetros existen en el original pero no aparecen referenciados en ningún fichero de `src/site/`:

- `appname`
- `keystore`
- `ksb64`
- `ver`

### Nombres del original mencionados en rFirma

| Parámetro | Módulos de `site` que lo mencionan |
|---|---|
| `algorithm` | `adapters/relay/tests.rs`, `application/local_batch/tests.rs`, `domain/batch/local.rs`, `domain/protocol/codes.rs`, `domain/protocol/operation.rs`, `domain/protocol/relay_parameters/tests.rs`, `domain/protocol/url/tests.rs` |
| `aw` | `adapters/relay/tests.rs`, `domain/protocol/launch.rs` |
| `batchpostsignerurl` | `domain/protocol/codes.rs`, `domain/protocol/operation.rs` |
| `batchpresignerurl` | `domain/protocol/codes.rs`, `domain/protocol/operation.rs` |
| `cop` | `domain/protocol/operation.rs` |
| `dat` | `adapters/relay.rs`, `adapters/relay/tests.rs`, `adapters/servlets.rs`, `adapters/servlets/tests.rs`, `domain/protocol/codes.rs`, `domain/protocol/launch.rs`, `domain/protocol/operation.rs`, `domain/protocol/url/tests.rs` |
| `desc` | `domain/protocol/operation.rs` |
| `exts` | `domain/protocol/operation.rs` |
| `filePath` | `domain/protocol/operation.rs` |
| `fileid` | `domain/protocol/launch.rs` |
| `filename` | `adapters/views/tests.rs`, `domain/protocol/operation.rs` |
| `format` | `adapters/relay/tests.rs`, `application/local_batch/tests.rs`, `domain/batch/local.rs`, `domain/batch/presign.rs`, `domain/batch/triphase.rs`, `domain/protocol/codes.rs`, `domain/protocol/filters/tests.rs`, `domain/protocol/operation.rs`, `domain/protocol/relay_parameters/tests.rs` |
| `gzip` | `adapters/relay/tests.rs`, `domain/protocol/operation.rs` |
| `id` | `adapters/relay.rs`, `adapters/relay/tests.rs`, `adapters/servlets.rs`, `adapters/servlets/tests.rs`, `adapters/views/tests.rs`, `domain/batch/json/tests.rs`, `domain/batch/local.rs`, `domain/batch/presign.rs`, `domain/batch/result.rs`, `domain/batch/triphase.rs`, `domain/protocol/launch.rs`, `domain/protocol/relay_parameters/tests.rs` |
| `idsession` | `domain/protocol/codes.rs`, `domain/protocol/framing.rs`, `domain/protocol/launch.rs`, `domain/protocol/message.rs`, `domain/protocol/url/tests.rs` |
| `jsonbatch` | `domain/protocol/operation.rs` |
| `jvc` | `domain/protocol/url/tests.rs` |
| `key` | `domain/protocol/launch.rs`, `domain/protocol/operation/tests.rs` |
| `localBatchProcess` | `domain/protocol/operation.rs` |
| `mcv` | `domain/protocol/codes.rs`, `domain/protocol/operation.rs`, `domain/protocol/url/tests.rs` |
| `multiload` | `domain/protocol/operation.rs` |
| `needcert` | `domain/protocol/operation.rs` |
| `op` | `adapters/relay/tests.rs`, `adapters/servlets.rs`, `adapters/servlets/tests.rs`, `domain/protocol/operation.rs`, `domain/protocol/relay_parameters.rs`, `domain/protocol/relay_parameters/tests.rs`, `domain/protocol/url/tests.rs` |
| `ports` | `domain/protocol/codes.rs`, `domain/protocol/launch.rs`, `domain/protocol/url/tests.rs` |
| `properties` | `domain/protocol/codes.rs`, `domain/protocol/operation.rs`, `domain/protocol/relay_parameters/tests.rs` |
| `resetsticky` | `domain/protocol/parameters.rs` |
| `rtservlet` | `domain/protocol/launch.rs` |
| `sticky` | `domain/protocol/parameters.rs` |
| `stservlet` | `adapters/relay.rs`, `adapters/relay/tests.rs`, `domain/protocol/launch.rs`, `domain/protocol/relay_parameters/tests.rs` |
| `title` | `domain/protocol/operation.rs` |
| `v` | `adapters/servlets.rs`, `adapters/servlets/tests.rs`, `domain/protocol/codes.rs`, `domain/protocol/launch.rs`, `domain/protocol/relay_parameters.rs`, `domain/protocol/url/tests.rs` |

---

## 5. Esqueleto de la tabla de auditoría (#603)

> [!WARNING]
> **Aviso:** Una celda de veredicto vacía **no significa conforme** ni compatible. Este esqueleto generado mecánicamente fija los nombres literales y sus puntos de origen para que la auditoría del #603 aporte la semántica (obligatoriedad, orden de comprobación, valores por defecto y desviaciones declaradas vs huecos).

| Parámetro | Origen original | Mencionado en rFirma | Veredicto (#603) | Comportamiento rFirma | Regla en original | Notas |
|---|---|:---:|---|---|---|---|
| `algorithm` | UrlParametersToSign, UrlParametersToSignAndSave, autoscript:cosign, autoscript:countersign, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `appname` | UrlParameters, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:sign, autoscript:signandsave | **No** | | | | |
| `aw` | UrlParameters | Sí | | | | |
| `batchpostsignerurl` | UrlParametersForBatch, autoscript:batch | Sí | | | | |
| `batchpresignerurl` | UrlParametersForBatch, autoscript:batch | Sí | | | | |
| `cop` | UrlParametersToSignAndSave, autoscript:signandsave | Sí | | | | |
| `dat` | UrlParameters, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:save, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `desc` | UrlParametersToLoad, UrlParametersToSave, autoscript:load, autoscript:save | Sí | | | | |
| `exts` | UrlParametersToLoad, UrlParametersToSave, autoscript:load, autoscript:save | Sí | | | | |
| `filePath` | UrlParametersToLoad, autoscript:load | Sí | | | | |
| `fileid` | UrlParameters | Sí | | | | |
| `filename` | UrlParametersToSave, UrlParametersToSignAndSave, autoscript:save, autoscript:signandsave | Sí | | | | |
| `format` | UrlParametersToSign, UrlParametersToSignAndSave, autoscript:cosign, autoscript:countersign, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `gzip` | UrlParameters | Sí | | | | |
| `id` | UrlParametersForBatch, UrlParametersToSave, UrlParametersToSelectCert, UrlParametersToSign, UrlParametersToSignAndSave | Sí | | | | |
| `idsession` | autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:load, autoscript:save, autoscript:selectcert, autoscript:service, autoscript:sign, autoscript:signandsave, autoscript:websocket | Sí | | | | |
| `jsonbatch` | UrlParametersForBatch, autoscript:batch | Sí | | | | |
| `jvc` | autoscript:service, autoscript:websocket | Sí | | | | |
| `key` | UrlParameters | Sí | | | | |
| `keystore` | UrlParameters, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:selectcert, autoscript:sign, autoscript:signandsave | **No** | | | | |
| `ksb64` | UrlParameters, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:selectcert, autoscript:sign, autoscript:signandsave | **No** | | | | |
| `localBatchProcess` | UrlParametersForBatch, autoscript:batch | Sí | | | | |
| `mcv` | UrlParameters, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:load, autoscript:save, autoscript:selectcert, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `multiload` | UrlParametersToLoad, autoscript:load | Sí | | | | |
| `needcert` | UrlParametersForBatch, autoscript:batch | Sí | | | | |
| `op` | autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:load, autoscript:save, autoscript:selectcert, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `ports` | autoscript:service, autoscript:websocket | Sí | | | | |
| `properties` | UrlParameters, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:selectcert, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `resetsticky` | UrlParametersForBatch, UrlParametersToSelectCert, UrlParametersToSign, UrlParametersToSignAndSave, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:selectcert, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `rtservlet` | UrlParameters | Sí | | | | |
| `sticky` | UrlParametersForBatch, UrlParametersToSelectCert, UrlParametersToSign, UrlParametersToSignAndSave, autoscript:batch, autoscript:cosign, autoscript:countersign, autoscript:selectcert, autoscript:sign, autoscript:signandsave | Sí | | | | |
| `stservlet` | UrlParameters | Sí | | | | |
| `title` | UrlParametersToLoad, UrlParametersToSave, autoscript:load, autoscript:save | Sí | | | | |
| `v` | autoscript:service, autoscript:websocket | Sí | | | | |
| `ver` | UrlParametersForBatch, UrlParametersToLoad, UrlParametersToSave, UrlParametersToSelectCert, UrlParametersToSign, UrlParametersToSignAndSave | **No** | | | | |
