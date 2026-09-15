# Manual del protocolo `afirma://` — AutoFirma 1.9.2

Manual de referencia del protocolo de invocación de AutoFirma, redactado
**leyendo el código original** de [ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma)
en el tag `v1.9.2` (commit `b4fe147c3`). No compara con rFirma ni describe lo
que rFirma implementa: es la fuente contra la que se auditará rFirma después.

## Convenciones

* **Toda afirmación cita el código**, con la forma `módulo/ruta/Fichero.java:línea`
  relativa a la raíz del repositorio original en el tag `v1.9.2`. Lo que no se
  ha podido confirmar en el código se marca como tal, no se supone.
* Cada capítulo cierra con una sección **«Lo que el código no aclara»**: las
  ambigüedades, incoherencias entre ficheros y comportamientos que dependen de
  un valor por defecto no documentado. Son las preguntas que la auditoría
  posterior tendrá que responder con pruebas.
* Los nombres de parámetro, constantes y mensajes se escriben tal cual aparecen
  en el código (`ver`, `stservlet`, `SAF_09`), nunca traducidos.
* Prosa en castellano; identificadores y literales del protocolo en su forma
  original.

## Dónde vive el protocolo en el código original

| Zona | Módulo y paquete | Qué contiene |
|---|---|---|
| Parser de la URI | `afirma-core` · `es.gob.afirma.core.misc.protocol` | `ProtocolInvocationUriParser`, `UrlParameters*` (un tipo por operación), `ProtocolVersion`, excepciones de parámetros |
| Lanzadores y transportes | `afirma-simple` · `es.gob.afirma.standalone.protocol` | `ProtocolInvocationLauncher` (despacho por tipo de URI), un `ProtocolInvocationLauncher<Op>` por operación, `ServiceInvocationManager` + `CommandProcessorThread` (socket), `AfirmaWebSocketServer[V4]` (websocket), `IntermediateServerUtil` (servidor intermedio), `ProtocolInvocationLauncherErrorManager` (códigos `SAF_nn`) |
| Arranque de la aplicación | `afirma-simple` · `es.gob.afirma.standalone.SimpleAfirma` | Cómo llega la URI a `launch()` desde la línea de órdenes |
| Cliente de referencia | `afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js` | El JavaScript que construye cada URI, elige transporte y parsea las respuestas |
| Servidor intermedio | `afirma-signature-storage`, `afirma-signature-retriever` | Los servlets `StorageService` y `RetrieveService` con los que habla la aplicación cuando no hay socket |
| Lotes | `afirma-crypto-batch-client`, `afirma-server-triphase-signer` (`signers.batch`) | Formato XML y JSON del lote y contrato con `batchpresignerurl` / `batchpostsignerurl` |
| Parámetros extra | `afirma-core` `ExtraParamsProcessor`, `*ExtraParams.java` de cada módulo `afirma-crypto-*`, `afirma-keystores-filters` | El diccionario `properties` y los filtros de certificado |

## Índice

| Nº | Capítulo | Fichero | Contenido |
|---|---|---|---|
| 01 | Visión general | [01-vision-general.md](01-vision-general.md) | Actores, ciclo de vida de una invocación, cómo llega la URI a la app, los tres transportes y cuándo se usa cada uno |
| 02 | Gramática de la URI y parámetros comunes | [02-uri-y-parametros-comunes.md](02-uri-y-parametros-comunes.md) | Forma `afirma://<op>?` y `afirma://<op>/?`, codificación, parámetros compartidos por todas las operaciones (`id`, `ver`, `v`, `dat`, `fileid`, `rtservlet`, `stservlet`, `key`, `properties`, `keystore`/`ksb64`, `aw`, `mcv`, `appname`, `sticky`, `resetsticky`, `jvc`) |
| 03 | Transporte por servidor intermedio | [03-transporte-servidor-intermedio.md](03-transporte-servidor-intermedio.md) | Recogida de datos por `fileid`+`rtservlet`, devolución por `stservlet`+`id`, cifrado con `key`, espera activa, contrato HTTP de `StorageService` y `RetrieveService` |
| 04 | Transporte por socket local | [04-transporte-socket.md](04-transporte-socket.md) | `afirma://service?`, puertos, `idsession`, TLS, gramática de comandos (`echo=`, `cmd=`, `fragment=`, `send=`, `firm=`, `@EOF`), fragmentación y respuestas |
| 05 | Transporte por WebSocket | [05-transporte-websocket.md](05-transporte-websocket.md) | `afirma://websocket?`, versiones 3 y 4 del servidor, mensajes, sesión, puerto por defecto |
| 06 | Operaciones `sign`, `cosign`, `countersign` | [06-operaciones-firma.md](06-operaciones-firma.md) | Parámetros de firma (`cop`, `format`, `algorithm`, `dat`, `properties`), selección de certificado, formato de la respuesta, casos especiales (firma visible obligatoria, confirmaciones) |
| 07 | Operación `signandsave` | [07-operacion-signandsave.md](07-operacion-signandsave.md) | Firma con guardado en disco, `filename`, diálogo de guardado, respuesta |
| 08 | Operación `batch` | [08-operacion-batch.md](08-operacion-batch.md) | Lotes XML y JSON, `batchpresignerurl`/`batchpostsignerurl`, `needcert`, `jsonbatch`, `localBatchProcess`, formato del lote y del resultado |
| 09 | Operación `selectcert` | [09-operacion-selectcert.md](09-operacion-selectcert.md) | Selección de certificado sin firma, parámetros, respuesta |
| 10 | Operaciones `save` y `load` | [10-operaciones-save-load.md](10-operaciones-save-load.md) | Guardar datos en disco y cargar ficheros (`title`, `filename`, `exts`, `desc`, `filePath`, `multiload`), formato de la respuesta de carga |
| 11 | El diccionario `properties` y los filtros de certificado | [11-extraparams-y-filtros.md](11-extraparams-y-filtros.md) | Codificación y expansión de `properties` (`ExtraParamsProcessor`), parámetros generales (`headless`, política, `filters`, `filter`, etc.), sintaxis de cada filtro de certificado |
| 12 | Parámetros extra por formato de firma | [12-extraparams-por-formato.md](12-extraparams-por-formato.md) | Catálogo de claves que aceptan PAdES, CAdES, XAdES, FacturaE y ASiC con su tipo y valor por defecto. OOXML, ODF, CMS y XMLDSig se consideran deprecados por lo que los podemos ignorar. |
| 13 | Almacenes de claves | [13-almacenes.md](13-almacenes.md) | Nombres aceptados en `keystore`/`ksb64`, cómo se resuelve el almacén por sistema operativo, `defaultKeyStore`, `defaultKeyStoreLib`, certificado pegajoso (`sticky`) |
| 14 | Versiones y compatibilidad | [14-versiones.md](14-versiones.md) | `ProtocolVersion` 0–4, `v`, `ver`, `mcv`, `jvc`, qué versión exige cada transporte y operación, comprobación de versión mínima de la aplicación |
| 15 | Catálogo de errores | [15-errores.md](15-errores.md) | Tabla completa `SAF_00`–`SAF_52` con su mensaje, en qué operación se produce y cómo viaja por cada transporte; `CANCEL` y demás respuestas no numeradas |
| 16 | El cliente JavaScript de referencia | [16-cliente-javascript.md](16-cliente-javascript.md) | Cómo `autoscript.js` construye cada petición, elige transporte, genera `idsession` y puertos, reintenta, y cómo parsea cada respuesta |

## Estado

Los capítulos se redactan **de uno en uno**, cada uno por un agente distinto a
partir del código original, y se revisan antes de encargar el siguiente para
afinar las instrucciones del próximo con lo aprendido. Al terminar todos, el
índice se revisa para eliminar duplicidades entre capítulos y para consolidar
las secciones «Lo que el código no aclara» en una lista única de preguntas de
auditoría.
