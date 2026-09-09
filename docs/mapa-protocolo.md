# Mapa del protocolo AutoFirma (v1.9.2) y auditoría de sus parámetros

Este documento recoge los literales declarados en el árbol del cliente AutoFirma a etiqueta fijada (**v1.9.2**), su cruce con el trámite de sede de rFirma y la auditoría de compatibilidad de cada puerta de parámetros (#603).

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
| `keystore` | `domain/protocol/codes.rs`, `domain/protocol/key_store.rs` |
| `ksb64` | `domain/protocol/codes.rs`, `domain/protocol/key_store.rs`, `domain/protocol/operation/tests.rs` |
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

## 5. Auditoría de cada puerta de parámetros (#603)

Una fila por parámetro, con lo que **exige rFirma**, la **regla que el original
ejecuta** —el código que la aplica, nunca su javadoc ni su XSD— y el veredicto.
El vocabulario de veredictos es cerrado:

| Veredicto | Qué significa |
|---|---|
| **Igual** | La sede recibe lo mismo de los dos, aunque el camino de dentro sea otro. |
| **Desviación declarada** | Una de las cinco de `CONTEXT.md` (SHA1, XMLDSig, XAdES explícita, lote local en XML, tarjetas), la del algoritmo decidida en el #602, o una decisión con ADR propio. |
| **Hueco** | Divergencia sin decidir. Lleva el número del sub-issue del #588 que la recoge. |

Y el sentido de cada divergencia, porque **aceptar de más también es
incompatibilidad**: **más estricta** (rFirma rechaza lo que el original
atiende) o **más laxa** (rFirma atiende lo que el original rechaza, o le da
otro significado). La sede prueba su trámite contra el original; donde rFirma
es más laxa, la sede no se entera de nada hasta que le llega una firma que no
esperaba.

Las citas de línea son de la etiqueta **v1.9.2** del original y de `HEAD` de
este repositorio en el momento de la auditoría.

### 5.1 Obligatoriedad y orden de validación, operación por operación

Lo que no se extrae mecánicamente: quién exige qué, en qué orden lo comprueba y
qué vale cuando falta.

**`sign` / `cosign` / `countersign`.** El original encadena
`UrlParameters.setCommonParameters` (`UrlParameters.java:253`),
`UrlParametersToSign.setSignParameters` (`UrlParametersToSign.java:207`) y
`setAnotherParams`, en este orden: `key` (8 caracteres o `SAF_03`) → `aw` →
`mcv` → `dat` **o** `fileid` + `rtservlet` → `id`/`fileid` (≤ 20 y
alfanumérico) → `ver` (por defecto `0`) → `appname` → `op` → si vino `fileid`,
**corta ahí** y el resto de parámetros ni se miran → `stservlet` (obligatorio si
hay servicios e `id`) → `format` (**obligatorio**) → `algorithm`
(**obligatorio**, contra una lista de doce literales) → `properties`
(tolerante: si no se puede leer, se sigue con propiedades vacías) → `sticky`
(por defecto `false`) → `resetsticky` (`false`) → `keystore`/`ksb64`. Los datos
ausentes **no son un error**: el lanzador abre el diálogo de carga
(`ProtocolInvocationLauncherSign.java:301-360`).

rFirma (`read_operation` y `sign_request`, `site/domain/protocol/operation.rs`)
comprueba `mcv` → `dat` no empieza por `file:/` → verbo → `format`
(obligatorio) → los dos rechazos de multifirma y contrafirma → `dat`
(**obligatorio**) → `algorithm` → `properties` (estricto). El orden relativo de
`format` y `algorithm` es el mismo que el del original, y por eso una petición
con los dos mal culpa a `format` en los dos sitios.

**`signandsave`.** Igual que `sign` salvo en tres cosas del original: el verbo
lo da `cop` y no `op`, no se lee `appname`, y `filename` pasa por la guarda de
caracteres inválidos (`UrlParametersToSignAndSave.java:202`). Su lista de
algoritmos son ocho literales, sin las variantes `withECDSA`. rFirma exige
`cop` (`sign`, `cosign` o `countersign`; cualquier otro valor o su ausencia son
`SAF_04`) y admite el `dat` ausente, que es el único sitio donde ya reproduce el
diálogo de carga del original.

**`save`.** El original exige `dat` **o** `fileid`
(`UrlParametersToSave.setSaveParameters:135`), valida `filename` y `exts`, pone
un título por defecto si falta `title` y le añade las extensiones entre
paréntesis a `desc`. rFirma exige `dat` (el camino de `fileid` lo resuelve
antes el adaptador del servidor intermedio, que sustituye el parámetro) y no
valida ni `filename` ni `exts`.

**`load`.** No hay nada obligatorio en ninguno de los dos
(`UrlParametersToLoad.setLoadParameters:153`). `multiload` vale `false` si
falta, `filePath` vacío es como ausente, y las cuatro claves restantes son
opcionales. **Igual, sin matices.**

**`batch`.** El original: `id`/`fileid` con sus guardas → `ver` → `appname` →
si vino `fileid`, corta → `localBatchProcess` → si **no** es local, exige
`batchpostsignerurl` y `batchpresignerurl` y las valida → `stservlet` →
`properties` → `sticky`, `resetsticky`, `needcert` (`false` por defecto) →
`jsonbatch` → almacén (`UrlParametersForBatch.setBatchParameters:193`). El
algoritmo del lote **no se valida en ninguna parte**: `BatchSigner` lo lee del
`<signbatch>` o del objeto raíz y lo devuelve tal cual —ese es el hallazgo que
abrió el #602—. rFirma: `localBatchProcess` sin `jsonbatch` es `SAF_03`
(desviación declarada), luego las dos URL de servlet, luego `dat`, y del lote
saca `algorithm` (obligatorio) y `stoponerror` (`false` por defecto).

**`selectcert`.** El original
(`UrlParametersToSelectCert.setSelectCertParameters:121`) no exige nada más que
las guardas comunes. rFirma lee el filtro de `properties` y los dos indicadores
de certificado pegado. **Igual.**

**El arranque (`websocket`, `service`) y el camino del servidor intermedio.**
El original toma `v` (por defecto `1`), `ports` —`Math.abs` de cada valor, y
`IllegalArgumentException` si alguno no es un entero—, `idsession` —si tiene
algún carácter que no sea letra o dígito, **lo descarta y sigue sin
credencial**— y `jvc`, que solo enciende un aviso visual
(`ProtocolInvocationLauncher.java:907-1000`). Sin `ports`, el `websocket` cae
al puerto por defecto. En el camino del servidor intermedio no hay arranque:
la operación viaja en la propia URL y la versión sale de `ver`.

rFirma exige `ports` en la versión 4 y en `service`, rechaza el `idsession` mal
formado en vez de descartarlo —**más estricta a propósito**, es la invariante
del ADR-0016—, y no lee `jvc`. En el camino del servidor intermedio la versión
de la operación sale de `ver`, como en el original.

### 5.2 Un parámetro por fila

| Parámetro | Origen en el original | Qué exige rFirma | Regla que el original ejecuta | Veredicto |
|---|---|---|---|---|
| `algorithm` | `UrlParametersToSign`, `UrlParametersToSignAndSave`, `autoscript`: `sign`, `cosign`, `countersign`, `signandsave` | Obligatorio; se reconoce por prefijo, OID y URI de XMLDSig; SHA1 y RIPEMD160 salen con `SAF_03` nombrando `algorithm` | `UrlParametersToSign.java:207` compara contra doce literales exactos (ocho en `signandsave`, sin `withECDSA`); quien firma, `AOSignConstants.composeSignatureAlgorithmName`, reconoce por prefijo e **ignora el sufijo** que escribió la sede | **Desviación declarada** (#602 y SHA1). Más laxa que la lista declarativa —atiende `SHA-256`, los OID y las URI—, igual que el código que firma; más estricta con SHA1 |
| `appname` | `UrlParameters`, `autoscript`: `batch`, `sign`, `cosign`, `countersign`, `signandsave` | No se lee | Se guarda en `UrlParametersToSign` y `UrlParametersForBatch`, y **ningún código lo lee**: `getAppName()` no tiene usos en el árbol | **Igual** |
| `aw` | `UrlParameters` | Espera activa solo con `true` sin distinguir mayúsculas y sin recortar (`asks_for_active_wait`, `launch.rs`) | `Boolean.parseBoolean` en `UrlParameters.java:253`: solo `true` sin distinguir mayúsculas; la espera solo ocurre fuera del socket (`ProtocolInvocationLauncher.java:337`) | **Igual** |
| `batchpostsignerurl` | `UrlParametersForBatch`, `autoscript:batch` | Obligatoria salvo lote local; `http` o `https`, host no local y sin `?` ni `=`, al leer la operación | `UrlParametersForBatch.java:193` la exige salvo `localBatchProcess`, y `validateURL` (`UrlParameters.java:351`) admite **`http` y `https`**, prohíbe host local y prohíbe `?` y `=` | **Igual** |
| `batchpresignerurl` | `UrlParametersForBatch`, `autoscript:batch` | Igual que la anterior | Igual que la anterior | **Igual** |
| `cop` | `UrlParametersToSignAndSave`, `autoscript:signandsave` | Obligatorio: `sign`, `cosign` o `countersign`; cualquier otra cosa, `SAF_04` | `UrlParametersToSignAndSave.java:202` lo guarda sin validar; `SingleSignOperation.Operation.getOperation` devuelve `null` para cualquier otro valor y la operación no llega a firmar | **Igual** en el desenlace; rFirma lo rechaza antes y con un código que lo nombra |
| `dat` | `UrlParameters`, `autoscript`: `batch`, `sign`, `cosign`, `countersign`, `save`, `signandsave` | Base64 URL-safe; obligatorio en `sign`, `cosign`, `countersign`, `save` y `batch`; opcional en `signandsave`; `file:/` es `SAF_03`; vacío es `SAF_44` | `UrlParameters.java:253` solo prohíbe `file:/`; el resto lo resuelve `DataDownloader.downloadData` (`DataDownloader.java:153`), que **descarga el contenido si el valor empieza por `http://` o `https://`** y si no lo trata como Base64. Ausente **no es error** | **Hueco doble** (#612): la URL y el `dat` ausente en `sign`. Más estricta en las dos |
| `desc` | `UrlParametersToLoad`, `UrlParametersToSave`, `autoscript`: `load`, `save` | Descripción del tipo de fichero, tal cual | `UrlParametersToSave.verifyFileTypeDescription:242` le añade `(*.ext)` con las extensiones de `exts` si no acaba en `)` | **Igual** (la diferencia es el rótulo de un diálogo, no lo que sale al cable) |
| `exts` | `UrlParametersToLoad`, `UrlParametersToSave`, `autoscript`: `load`, `save` | Lista separada por comas, con la misma guarda de caracteres al leer `save` | `UrlParametersToSave.verifyExtensions:221` rechaza con `SAF_03` cualquiera de `\ / : * ? " < > \| ;` y el espacio | **Igual** |
| `filePath` | `UrlParametersToLoad`, `autoscript:load` | Carpeta inicial del diálogo de carga; vacío es como ausente | `UrlParametersToLoad.java:153`: idéntico, vacío se trata como ausente | **Igual** |
| `fileid` | `UrlParameters` | Referencia del documento o del XML de parámetros en el camino del servidor intermedio, con las mismas dos guardas de identificador | `UrlParameters.java:253` exige que lo acompañe `rtservlet`, y en las cinco clases hace además de `id` de sesión, con las guardas de ≤ 20 y alfanumérico | **Igual** |
| `filename` | `UrlParametersToSave`, `UrlParametersToSignAndSave`, `autoscript`: `save`, `signandsave` | Nombre propuesto al guardar, con la misma guarda de caracteres en `save` y en `signandsave` | `verifyFilename` (`UrlParametersToSave.java:207`) y `UrlParametersToSignAndSave.java:202` rechazan con `SAF_03` cualquiera de `\ / : * ? " < > \|` | **Igual** |
| `format` | `UrlParametersToSign`, `UrlParametersToSignAndSave`, `autoscript`: `sign`, `cosign`, `countersign`, `signandsave` | Obligatorio; catálogo cerrado con los alias de `AOSignConstants`; `auto` se resuelve por la cabecera del documento; XMLDSig sale con `SAF_06` | Obligatorio (`SAF_03` si falta); el **valor** no se valida al leer la URL, lo resuelve más tarde `AOSignerFactory`, y un formato que no existe acaba en `SAF_06` | **Desviación declarada** (XMLDSig, XAdES explícita y el resto de lo que el original no firma en tres fases). Mismo código, más temprano |
| `gzip` | `UrlParameters` | `true` sin distinguir mayúsculas; descomprime lo que traía `dat` | `Boolean.parseBoolean`; `DataDownloader` descomprime **antes** de mirar si el valor era una URL | **Igual** |
| `id` | `UrlParametersForBatch`, `UrlParametersToSave`, `UrlParametersToSelectCert`, `UrlParametersToSign`, `UrlParametersToSignAndSave` | Obligatorio en el camino del servidor intermedio con `stservlet`; ≤ 20 caracteres y alfanumérico | ≤ 20 caracteres y alfanumérico en las cinco clases, porque se usa como nombre de fichero | **Igual** |
| `idsession` | `autoscript`: todas las operaciones y los dos arranques | Credencial del canal; alfanumérica ASCII; obligatoria en el `websocket` de la versión 4, opcional en la 3 y en `service` | `ProtocolInvocationLauncher.getChannelInfo`: si tiene algún carácter que no sea letra o dígito **la descarta y sigue sin credencial** | **Desviación declarada** (ADR-0016). Más estricta a propósito |
| `jsonbatch` | `UrlParametersForBatch`, `autoscript:batch` | `true` sin distinguir mayúsculas; `false` por defecto | `Boolean.parseBoolean`, `false` por defecto | **Igual** |
| `jvc` | `autoscript`: `service`, `websocket` | No se lee | Solo enciende un aviso visual si es menor que el mínimo (`ProtocolInvocationLauncher.java:194-215`); no cambia ninguna respuesta | **Igual** |
| `key` | `UrlParameters` | Ocho caracteres exactos; si no, `SAF_03` nombrando `key` | `verifyCipherKey` (`UrlParameters.java:327`) exige ocho y falla con `SAF_03`; ausente o vacío significa «sin cifrado» | **Igual** |
| `keystore` | `UrlParameters`, `autoscript`: `batch`, `sign`, `cosign`, `countersign`, `selectcert`, `signandsave` | Gana al `ksb64` y se lee con su misma regla: el nombre visible o el de la constante, la familia NSS se obedece y el resto del catálogo de `AOKeyStore` sale con `SAF_07` | `getKeyStoreName` (`UrlParameters.java:382`) elige el almacén; los cuatro lanzadores lo construyen con él | **Desviación declarada** (ADR-0022). Más estricta: rFirma no abre más almacén que el suyo |
| `ksb64` | Igual que `keystore` | Se lee con la misma regla, con el valor en Base64, que se ignora entero si no lo es | Igual que `keystore`, con el valor en Base64, y `getDefaultKeyStoreLib` (`UrlParameters.java:415`) saca de él la ruta de la biblioteca PKCS#11 | **Desviación declarada** (ADR-0022). La biblioteca que nombre la sede no se carga |
| `localBatchProcess` | `UrlParametersForBatch`, `autoscript:batch` | `true` sin `jsonbatch` sale con `SAF_03` nombrando `dat` | `UrlParametersForBatch.java:193`: con `true` no exige las URL de servlet, y el lote en XML heredado también se procesa | **Desviación declarada** (lote local solo en JSON) |
| `mcv` | `UrlParameters`, `autoscript`: todas las operaciones | Se comprueba en toda operación con el comparador del original; una cadena sin forma de versión sale con `SAF_03` | Se comprueba en los seis lanzadores (`ProtocolInvocationLauncherSign.java:143` y equivalentes) con `SAF_41`; una cadena sin forma de versión revienta con `NumberFormatException` | **Igual** en el caso que importa; rFirma nombra el parámetro en vez de reventar |
| `multiload` | `UrlParametersToLoad`, `autoscript:load` | `true` sin distinguir mayúsculas; `false` por defecto | `Boolean.parseBoolean`, `false` por defecto | **Igual** |
| `needcert` | `UrlParametersForBatch`, `autoscript:batch` | `true` sin distinguir mayúsculas; `false` por defecto | `Boolean.parseBoolean`, `false` por defecto | **Igual** |
| `op` | `autoscript`: todas las operaciones | El parámetro `op` si vino y no está vacío, y si no el dominio de la URL, **distinguiendo mayúsculas** | `ProtocolInvocationUriParser.parserUri` mete el dominio en `op`, y `launch` compara con `startsWith("afirma://sign?")`, **sensible a mayúsculas** | **Igual** |
| `ports` | `autoscript`: `service`, `websocket` | Obligatorio en la versión 4 y en `service`; enteros de 1 a 65535 | `Math.abs` de cada valor, `IllegalArgumentException` si alguno no es numérico; sin `ports`, el `websocket` cae al puerto por defecto y `service` falla con `SAF_03` | **Igual** |
| `properties` | `UrlParameters`, `autoscript`: `batch`, `sign`, `cosign`, `countersign`, `selectcert`, `signandsave` | Base64 URL-safe; si no se puede leer, se descarta con traza y el trámite sigue sin parámetros adicionales | Si `AOUtil.base642Properties` falla, **registra y sigue con propiedades vacías** (`UrlParametersToSign.java:207` y las tres clases hermanas) | **Igual** |
| `resetsticky` | Cuatro clases y seis operaciones de `autoscript` | `true` sin distinguir mayúsculas y **sin recortar espacios**; `false` por defecto | `Boolean.parseBoolean`, **sin recortar**: `" true"` es `false` | **Igual** |
| `rtservlet` | `UrlParameters` | Se valida al leer la invocación; un host local sale con `SAF_13` | `validateURL` al leer la URL (`UrlParameters.java:351`): host local es `SAF_13`, y `?` o `=` en la URL es `SAF_03` | **Igual** |
| `sticky` | Cuatro clases y seis operaciones de `autoscript` | Como `resetsticky` | Como `resetsticky` | **Igual** |
| `stservlet` | `UrlParameters` | Destino de la respuesta en el camino del servidor intermedio; se valida al leer la invocación | `validateURL` al leer la URL, y es obligatorio cuando hay `id` y servicios | **Igual** |
| `title` | `UrlParametersToLoad`, `UrlParametersToSave`, `autoscript`: `load`, `save` | Título del diálogo; ausente es ausente | `verifyTitle` (`UrlParametersToSave.java:235`) pone un título por defecto si falta | **Igual** (el rótulo de un diálogo local) |
| `v` | `autoscript`: `service`, `websocket` | `service`: 1, 2 o 3; `websocket`: 3 o 4; cualquier otra, `SAF_21`. Ausente vale 1 | Cualquier entero se acepta al leer, y el lanzador contesta `SAF_21` si supera la versión 4. Ausente vale 1 | **Igual** en el desenlace |
| `ver` | `UrlParametersForBatch`, `UrlParametersToLoad`, `UrlParametersToSave`, `UrlParametersToSelectCert`, `UrlParametersToSign`, `UrlParametersToSignAndSave` | Se lee en toda operación: ausente vale `0` y un valor que no es entero vale `1`; por encima de la versión 4 sale `SAF_21` antes de firmar, y en el camino del servidor intermedio es además la versión de la operación | Versión mínima de protocolo de la operación (`0` por defecto); en el camino sin arranque manda ella, y si supera la versión 4 sale `SAF_21` (`ProtocolInvocationLauncher.java:301` y las cinco líneas hermanas) | **Desviación declarada (ADR-0021)**: igual en el camino del servidor intermedio y **más estricta a propósito** por el canal ya abierto, donde el original deja mandar a `v` y no mira `ver` |

### 5.3 Lo que viaja dentro de `properties`

`properties` es un `.properties` en Base64, y sus claves son la segunda puerta:
casi todas cruzan al firmador, pero unas pocas las interpreta el propio
lanzador. Estas son las que deciden algo.

| Clave | Qué exige rFirma | Regla que el original ejecuta | Veredicto |
|---|---|---|---|
| `filter`, `filters`, `filters.N` | Se recogen con la precedencia del original y cruzan enteras al motor (`site/domain/protocol/filters.rs`) | `CertFilterManager.getFilterValues` (`CertFilterManager.java:165`): `filter` gana a `filters`, y `filters` a la serie numerada; los criterios se separan por `;` | **Igual** |
| `headless` | Con `true`, si un solo certificado pasa el filtro no se pregunta; nunca cruza al puente | `CertFilterManager.isMandatoryCertificate:145`: con `true`, si un solo certificado pasa el filtro **no se enseña el diálogo** | **Igual** |
| `mandatoryCertSelection` | `false` dice lo mismo que `headless=true`; nunca cruza al puente | `false` tiene el mismo efecto que `headless=true` | **Igual** |
| `profile` | Se descarta antes de firmar, venga por `properties` o por los `extraparams` de un lote | `ProtocolInvocationLauncherSign.java:153` y `…SignAndSave.java:150` lo **borran** antes de firmar | **Igual** |
| `mode=explicit` | Con XAdES, `SAF_06` | El original avisa de que está obsoleto y hashea el dato con SHA1 | **Desviación declarada** (XAdES explícita) |
| `target` | `tree` o `leafs`; cualquier otro valor, `SAF_03` nombrando `properties`; ausente es `leafs` | `CounterSignTarget.getTarget`, con `leafs` por defecto en la contrafirma | **Igual** |
| `filenameExts`, `filenameDescription`, `filenameCurrentDir` | Gobiernan el diálogo de carga de `signandsave` | `ProtocolInvocationLauncherSignAndSave.java:324-337`: lo mismo, y también en `sign` cuando faltan los datos | **Igual** en `signandsave`; en `sign` va con el #612 |
| `filenameActualName` | Se propone como nombre en el diálogo de carga de `signandsave`; nunca cruza al puente | `ProtocolInvocationLauncherSignAndSave.java:337`: nombre propuesto en el diálogo de carga | **Igual** |
| `filenameSaveExts`, `filenameSaveDescription`, `filenameSaveCurrentDir` | Gobiernan el diálogo de guardado de `signandsave` | `ProtocolInvocationLauncherSignAndSave.java:537-546`: lo mismo | **Igual** |
| `signaturePositionOnPage*`, `signaturePage`, `signaturePages`, `visibleSignature`, `signatureRubricImage` | Solo se leen con PAdES; con cualquier otro formato se olvidan antes del consentimiento; `signaturePage(s)=append` sale con `SAF_03` | Las lee el firmador PDF; `append` añade una página en blanco al documento | **Desviación declarada** (ADR-0019 y ADR-0006: no se modifica el documento antes de firmarlo) |
| `allowSigningUnregisteredSignatures` y las claves de política | Se leen tras expandir con el motor del puente | El mismo `ExtraParamsProcessor.expandProperties` del original, que es quien las expande | **Igual** |
| Cualquier otra clave | Cruza al puente tal cual | Cruza al firmador tal cual | **Igual** |

### 5.4 Lo que viaja dentro de la definición del lote

El lote llega en `dat`, en el XML heredado o en JSON (`jsonbatch=true`), y su
contenido es la tercera puerta. El original lo lee en dos sitios distintos: el
**lote remoto** solo mira la cabecera y manda el resto a los dos servlets
(`BatchSigner.getAlgorithmForXML` y `getAlgorithmForJSON`, que **devuelven el
atributo tal cual, sin contrastarlo con nada**), y el **lote local** lo desmonta
entero en `JSONBatchManager.parseBatchConfig`.

| Campo | Dónde | Qué exige rFirma | Regla que el original ejecuta | Veredicto |
|---|---|---|---|---|
| `algorithm` | Cabecera del lote (atributo de `<signbatch>` o campo raíz) | Obligatorio; reconocido por prefijo, OID y URI; SHA1 sale con `SAF_03` | Obligatorio solo en el sentido de que su ausencia revienta; el valor **no se valida** | **Desviación declarada** (#602) |
| `stoponerror` | Cabecera | `true` en el JSON como booleano, en el XML como texto; `false` por defecto | `json.getBoolean` / atributo; `false` por defecto | **Igual** |
| `suboperation` | Cabecera y cada firma | `sign` o `cosign`; `countersign` sale con `SAF_04`; por defecto `sign` | `Operation.getOperation` admite las tres, y `null` para el resto | **Desviación declarada** (la contrafirma en lote va con el #567 y el #593; el original tampoco la lleva a ninguna parte en el lote local) |
| `format` | Cabecera y cada firma | Obligatorio en la cabecera; catálogo cerrado; `auto` se resuelve por la cabecera del documento | Obligatorio en la cabecera (`JSONBatchManager.java:60`); cada firma puede tener el suyo | **Igual** |
| `extraparams` | Cabecera y cada firma | Base64, con las `\n` escritas como texto convertidas en saltos; la firma que trae los suyos **no hereda** los del lote | `expanExtraParams`: idéntico, incluida la herencia por sustitución y no por mezcla | **Igual** |
| `singlesigns` | Raíz | Obligatorio | `json.getJSONArray` revienta si falta | **Igual** |
| `id` de cada firma | Cada firma | Obligatorio | Obligatorio (`JSONBatchManager.java:85`) | **Igual** |
| `datareference` | Cada firma | Obligatorio; Base64 del alfabeto normal, tolerante con `-` y `_` | Obligatorio; `Base64.decode`, que ignora lo que no reconoce | **Igual** |

### 5.5 Dónde rFirma es más laxa que el original

La lista que pide el criterio de aceptación, junta y sin diluir. En todas
ellas rFirma **acepta lo que el original rechaza**, o le da otro significado, y
por tanto una sede que funcione aquí puede no funcionar contra AutoFirma:

1. **`profile` cruzado al puente**, cuando el original lo borra antes de firmar → #615.
2. **El `algorithm` reconocido por prefijo** en vez de contra la lista de doce literales: aquí es deliberado y está decidido en el #602, porque es lo que hace el código que firma, y solo la puerta declarativa del original es más cerrada.

Y donde rFirma es **más estricta** —que también rompe trámites, pero de forma
visible—: `dat` que es una URL y `sign` sin `dat` (ambos #612), `properties`
ilegible (#615), el `ver` de la operación comprobado también por el canal ya
abierto (ADR-0021), y las tres desviaciones declaradas que rechazan formatos y
algoritmos.

### 5.6 Ninguna fila queda sin veredicto

Treinta y cinco parámetros de URL, trece claves de `properties` con
comportamiento propio y ocho campos de la definición del lote. Veredictos:
**Igual** en la mayoría, **desviación declarada** en diez casos —las cinco de
`CONTEXT.md`, la del algoritmo (#602), la del `idsession` (ADR-0016), las del
recuadro (ADR-0019, ADR-0006) y la del `ver` de la operación (ADR-0021)— y
**hueco** en lo que abre esta auditoría, repartido en las sub-issues #612, #615
y #617. Dos sub-issues ya están cerradas: el #614 devuelve a **Igual** sus
filas —`aw`, `exts`, `fileid`, `filename`, `id`, `key`, `op`, `rtservlet`,
`stservlet`, `sticky`, `resetsticky` y las dos URL de servlet del lote—, y el
#618 deja la de `ver` en **desviación declarada**.
