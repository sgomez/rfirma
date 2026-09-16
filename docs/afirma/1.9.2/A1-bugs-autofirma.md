# Anexo A1 — Potenciales bugs y defectos de implementación en AutoFirma 1.9.2

Este anexo cataloga los comportamientos anómalos, fugas de recursos, fallos
de terminación e inconsistencias internas identificadas en el código original de
AutoFirma 1.9.2 (`clienteafirma` tag `v1.9.2`, commit `b4fe147c3`).
 Es producto de una auditoría con IA y no pueden tomarse como hechos, solo como cuestiones a explorar.

Estas cuestiones **no forman parte de la especificación funcional del protocolo
`afirma://`**, sino que constituyen presuntos defectos de la implementación de referencia.
Se documentan aquí de forma aislada para:
1. Mantener los capítulos 01 al 16 como una especificación limpia y rigurosa del protocolo.
2. Evitar que una implementación alternativa (como rFirma) replique defectos o fugas de recursos del software original por exceso de celo en la compatibilidad de bajo nivel.
3. Facilitar la comprensión de fallos y bloqueos observados en entornos de producción con AutoFirma oficial.

El campo **Estado en `master`** de cada ficha no describe la 1.9.2, que es lo que
documenta este anexo, sino si el defecto sigue vivo en la rama de desarrollo del
original (`origin/master`, commit `0d7f3cf01`, 219 commits por delante de
`v1.9.2`), y está para distinguir las heridas cerradas de las abiertas río abajo.
Esa rama reescribió por completo el paquete `protocol` de `afirma-simple`, así
que varias correcciones lo son por cambio de arquitectura y no por un parche
dirigido al defecto.

---

## Catálogo de bugs y defectos

### BUG-01: Proceso huérfano indefinido en WebSocket sin conexión inicial

* **Estado en `master`:** **Corregido.** `AfirmaWebSocketServer.java:101` arranca un `InnactivityWatcherThread` con `INITIAL_INACTIVITY_TIMEOUT` que `markAsWorking()` interrumpe en cuanto llega la primera petición.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.AfirmaWebSocketServerManager.java:52-94`, `AfirmaWebSocketServer.java:70-91` frente a `ServiceInvocationManager.java:127-134`.
* **Origen de auditoría:** Anteriormente AUD-03 ([01-vision-general.md](01-vision-general.md), [05-transporte-websocket.md](05-transporte-websocket.md)).
* **Descripción:** En el transporte por socket tradicional (`service`), `ServiceInvocationManager` inicializa un temporizador `Timer` de 90 segundos (`SOCKET_TIMEOUT = 90000`) que invoca `forceCloseApplication(0)` si transcurre dicho tiempo sin actividad o sin recibir peticiones. En contraste, el servidor WebSocket (`AfirmaWebSocketServerManager` y `AfirmaWebSocketServer`) carece por completo de temporizador de arranque o de inactividad inicial.
* **Comportamiento y consecuencia:** Si un navegador abre la URI `afirma://websocket?ports=...` y la persona usuaria cancela el diálogo del navegador que solicita permiso para abrir la aplicación, o cierra la pestaña web antes de que el JavaScript conecte al puerto, el proceso de AutoFirma se inicia y queda a la escucha en segundo plano indefinidamente. Debido a que el servidor WebSocket mantiene hilos no demonio en ejecución, la JVM nunca finaliza. Las cancelaciones sucesivas provocan una acumulación de procesos huérfanos de Java en el sistema, agotando memoria y dejando puertos locales ocupados.
* **Causa raíz:** Ausencia de un temporizador de inicialización (*handshake timeout*) en `AfirmaWebSocketServerManager` análogo al existente en `ServiceInvocationManager`.

---

### BUG-02: Cierre omitido del proceso ante excepciones no capturadas en invocaciones sin WebSocket

* **Estado en `master`:** **Sigue presente.** `SimpleAfirma.java:1074-1079` conserva el bloque idéntico, con `forceCloseApplication(-1)` condicionado al prefijo `afirma://websocket`.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.SimpleAfirma.java:1073-1078`.
* **Origen de auditoría:** Anteriormente AUD-04 ([01-vision-general.md](01-vision-general.md)).
* **Descripción:** En `SimpleAfirma.main`, el bloque global de captura de excepciones gestiona los errores no controlados que escapan de `ProtocolInvocationLauncher.launch(args[0])`:
  ```java
  } catch (final Exception e) {
      LOGGER.log(Level.SEVERE, "Error global en la aplicacion: " + e, e);
      // En caso de error
      if (args != null && args.length > 0 && args[0].startsWith(WEBSOCKET_REQUEST_PREFIX)) {
          forceCloseApplication(-1);
      }
  }
  ```
* **Comportamiento y consecuencia:** La llamada de terminación `forceCloseApplication(-1)` está condicionada erróneamente a que la URI comience por `afirma://websocket`. Si una excepción no controlada (como un `NullPointerException` o fallo de red no capturado) ocurre durante una invocación directa por servidor intermedio (`sign`, `batch`, etc.) o por socket TCP local (`service`), el hilo principal de `main` termina silenciosamente tras escribir en el log, pero **no** invoca `forceCloseApplication` ni `System.exit`.
* **Causa raíz:** Si durante las fases previas (como la configuración del *Look & Feel* o la instanciación de diálogos de `JOptionPane`) se puso en marcha el despachador de eventos Swing/AWT (`AWT-EventQueue`), dichos hilos no demonio mantienen viva la máquina virtual de Java de forma zombi, sin ventana visible ni respuesta hacia la sede electrónica.

---

### BUG-03: Script de terminación en macOS (`closeMacService`) invoca `kill` sobre un proceso lanzador extinto

* **Estado en `master`:** **Sigue presente.** `MacUtils.java:86-88` mantiene literalmente `kill -9 $(ps -ef | grep <sessionId> | awk '{print $2}')`, y el lanzador de macOS sigue auto-terminándose.
* **Código fuente:** `afirma-simple-installer` · `macos/Lanzador/Autofirma para macOS/AppDelegate.m:117-121`; `afirma-simple` · `es.gob.afirma.standalone.so.macos.MacUtils.java:74-95`; `ServiceInvocationManager.java:127-134`.
* **Origen de auditoría:** Anteriormente AUD-05 ([01-vision-general.md](01-vision-general.md), [04-transporte-socket.md](04-transporte-socket.md)).
* **Descripción:** En macOS, el lanzador nativo compilado en Objective-C ejecuta la JVM con `NSTask`, duerme 5 segundos y se auto-termina deliberadamente:
  ```objc
  [task launch];
  [NSThread sleepForTimeInterval:5.0];
  [NSApp terminate:nil];
  ```
  Por tanto, el proceso lanzador nativo ya no existe 5 segundos después de haber arrancado Java. Sin embargo, al expirar el temporizador de 90 segundos por inactividad en `ServiceInvocationManager`, el código Java ejecuta:
  ```java
  if (Platform.OS.MACOSX.equals(Platform.getOS())) {
      MacUtils.closeMacService(channelInfo.getIdSession());
  }
  ```
  `MacUtils.closeMacService` lanza un script de terminal:
  ```bash
  kill -9 $(ps -ef | grep <sessionId> | awk '{print $2}')
  ```
  bajo la asunción comentada en su Javadoc de que «el proceso intermedio no es la aplicación Java, sino el ejecutable que traslada los parámetros proporcionados por el navegador».
* **Comportamiento y consecuencia:** El script intenta terminar un proceso nativo extinto hace 85 segundos. Si la línea de ejecución de Java visible en `ps -ef` incluye la URL de invocación completa (que contiene el parámetro `idsession`), el comando `kill -9` puede terminar la propia JVM o fallar con advertencias de sintaxis. Además, la concatenación directa de `sessionIdText` sin filtrado en la cadena de bash expone a errores de ejecución en el subshell si el identificador incluye caracteres no alfanuméricos.
* **Causa raíz:** Desacople entre el ciclo de vida real del lanzador nativo en macOS y la lógica de limpieza posterior asumida por el backend en Java.

---

### BUG-04: Inoperancia funcional de `afirma://load` por servidor intermedio y `NullPointerException` en gestión de errores

* **Estado en `master`:** **Sigue presente.** `UrlParametersToLoad` sigue sin declarar `id` ni `stservlet`. El envío al servidor intermedio se corrigió en el camino de éxito (`ProtocolInvocationLauncher.java:882`), de modo que el `NullPointerException` se ha desplazado del `catch` a la invocación normal.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncher.java:767-810`, `ProtocolInvocationLauncherLoad.java:151`; `afirma-core` · `es.gob.afirma.core.misc.protocol.UrlParametersToLoad.java:15-202`; `afirma-ui-miniapplet-deploy` · `autoscript.js:4149-4170`.
* **Origen de auditoría:** Anteriormente AUD-18 ([01-vision-general.md](01-vision-general.md), [02-uri-y-parametros-comunes.md](02-uri-y-parametros-comunes.md), [03-transporte-servidor-intermedio.md](03-transporte-servidor-intermedio.md), [10-operaciones-save-load.md](10-operaciones-save-load.md)).
* **Descripción:** La operación `afirma://load` contiene en `ProtocolInvocationLauncher.java:767-810` la estructura para recibir parámetros por servidor intermedio mediante `fileid` y `rtservlet`, e invoca `requestWait(params.getStorageServletUrl(), params.getId())` (línea 791). Asimismo, en caso de excepción `SocketOperationException` (línea 801), intenta notificar el error invocando `sendDataToServer(msg, params.getStorageServletUrl().toString(), params.getId())` (línea 808). Sin embargo, `UrlParametersToLoad` no define ni analiza las constantes `id` ni `stservlet`.
* **Comportamiento y consecuencia:** Si se invoca `afirma://load` mediante servidor intermedio:
  1. En el flujo exitoso, `ProtocolInvocationLauncherLoad.processLoad` devuelve la cadena con los datos cargados (`dataToSend.toString()`) a `ProtocolInvocationLauncher.launch()`, pero `launch()` simplemente la retorna a `SimpleAfirma.main`, que concluye el proceso con `forceCloseApplication(0)` sin subir los datos al servidor (`sendDataToServer` nunca se ejecuta para el resultado exitoso de `load`). La sede web nunca recibe el fichero cargado.
  2. Si se produce un error capturado en `SocketOperationException`, la llamada `params.getStorageServletUrl().toString()` arroja un `NullPointerException` fulminante, impidiendo que el error se notifique al servlet intermedio.
  3. Debido a esta desconexión, el cliente JavaScript de referencia `autoscript.js:4149-4170` deshabilita de forma explícita las llamadas a carga por servidor intermedio arrojando `java.lang.UnsupportedOperationException`.
* **Causa raíz:** Omisión del análisis de `id` y `stservlet` en `UrlParametersToLoad` e inexistencia de llamada a `sendDataToServer` para el flujo exitoso de carga en `ProtocolInvocationLauncher`.

---

### BUG-05: Rechazo de algoritmos ECDSA en la operación `signandsave`

* **Estado en `master`:** **Sigue presente.** `UrlParametersToSignAndSave.java:68-77` sigue sin las variantes ECDSA que `UrlParametersToSign.java:74-77` sí declara. Ambos aceptan ya el nombre de solo huella (`SHA256`), que es la vía que queda abierta a una clave elíptica.
* **Código fuente:** `afirma-core` · `es.gob.afirma.core.misc.protocol.UrlParametersToSignAndSave.java:67-77, 284-287` frente a `UrlParametersToSign.java:60-74`.
* **Origen de auditoría:** Anteriormente AUD-32 ([02-uri-y-parametros-comunes.md](02-uri-y-parametros-comunes.md), [07-operacion-signandsave.md](07-operacion-signandsave.md)).
* **Descripción:** En `UrlParametersToSign.java:70-73`, la lista de algoritmos soportados `SUPPORTED_SIGNATURE_ALGORITHMS` incluye las variantes de firma sobre curvas elípticas: `SHA1withECDSA`, `SHA256withECDSA`, `SHA384withECDSA` y `SHA512withECDSA`. Sin embargo, en `UrlParametersToSignAndSave.java:67-77`, el conjunto homónimo solo incluye algoritmos basados en RSA (`SHA*withRSA`).
* **Comportamiento y consecuencia:** Si un invocador solicita una operación `afirma://signandsave` con un algoritmo de curva elíptica (por ejemplo `algorithm=SHA256withECDSA`), la validación en `UrlParametersToSignAndSave.java:284-287` falla arrojando `ParameterException("El algoritmo de firma no es valido: " + this.signAlgorithm)`. La operación se aborta de inmediato con el código de error `SAF_03` (`ERROR_PARAMS`). La misma firma con idéntico certificado funciona sin inconvenientes en `afirma://sign`.
* **Causa raíz:** Actualización incompleta del conjunto `SUPPORTED_SIGNATURE_ALGORITHMS` en `UrlParametersToSignAndSave` al incorporar soporte para certificados de clave elíptica en la aplicación.

---

### BUG-06: Persistencia de certificado (`sticky`) en campo estático de JVM

* **Estado en `master`:** **Sigue presente.** `ProtocolInvocationLauncher.java:100` mantiene `private static PrivateKeyEntry stickyKeyEntry`, con los mismos accesores.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncher.java:90, 109-121`, `ProtocolInvocationLauncherSign.java:518-521, 643`, `ProtocolInvocationLauncherSelectCert.java:139-142, 196`.
* **Origen de auditoría:** Anteriormente AUD-77 ([01-vision-general.md](01-vision-general.md), [02-uri-y-parametros-comunes.md](02-uri-y-parametros-comunes.md), [09-operacion-selectcert.md](09-operacion-selectcert.md), [13-almacenes.md](13-almacenes.md)).
* **Descripción:** La reutilización de certificado entre operaciones consecutivas (`sticky=true`) guarda el `PrivateKeyEntry` en una variable estática única de la JVM. Lo escriben los cuatro lanzadores de operación (`Sign`, `SignAndSave`, `SelectCert`, `Batch`) y lo lee la firma para saltarse la apertura del almacén cuando ya hay entrada fijada.
* **Comportamiento:** El campo no guarda `idsession`, pestaña ni origen: el estado es del proceso, no de la sesión. Solo se purga por lógica de operación —petición con `sticky=false`, almacén bloqueado, error o cancelación en lote—, nunca al cerrarse el canal ni por inactividad.
* **Causa raíz:** Estado de sesión modelado en un campo estático de clase en lugar de en un almacén acotado a la sesión que lo fijó.
---

### BUG-07: Pérdida de la versión negociada y metadatos `extraData` en servidor intermedio con URLs largas

* **Estado en `master`:** **Corregido.** La versión se fija ahora **después** de descargar y parsear el XML remoto, con el mismo patrón en las seis operaciones que admiten `fileid` (`ProtocolInvocationLauncher.java:753-756` y homólogos).
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncher.java:653-679`, `NativeSignDataProcessor.java:77, 97`; `afirma-ui-miniapplet-deploy` · `autoscript.js:3803, 4413-4424`.
* **Origen de auditoría:** Anteriormente AUD-09 ([14-versiones.md](14-versiones.md)).
* **Descripción:** Cuando una petición de firma excede la longitud admisible para una URL, `autoscript.js` genera un documento XML con todos los parámetros (incluyendo la versión del protocolo solicitada, típicamente `<e k="ver" v="3"/>`) y lo sube al servlet `StorageService`. A continuación, invoca la aplicación pasando en la URI únicamente `fileid`, `rtservlet` y `key`, omitiendo el parámetro `ver` en la URL de arranque.
* **Comportamiento y consecuencia:** En `ProtocolInvocationLauncher.java:650-655`, AutoFirma inicializa `params` con los parámetros presentes en la URI. Al no figurar `ver`, `params.getMinimumProtocolVersion()` devuelve `"0"`, fijando `requestedProtocolVersion = 0`. Posteriormente (líneas 660-678), la aplicación descarga el XML desde el servlet remoto y reasigna `params = ProtocolInvocationUriParser.getParametersToSign(xmlData, true)`, pero **omite volver a sincronizar `requestedProtocolVersion`**. Cuando finaliza el proceso de firma (línea 691), `processSign` recibe `requestedProtocolVersion == 0`, por lo que no activa el formateador de respuesta para versiones modernas (v3/v4). Como resultado, la aplicación nunca devuelve los metadatos `extraData` (como el nombre del fichero firmado o información adicional de firma) a la sede electrónica cuando los parámetros se transfirieron por servidor intermedio debido al tamaño del documento.
* **Causa raíz:** `ProtocolInvocationLauncher.launch` calcula la versión definitiva `requestedProtocolVersion` antes de recuperar los parámetros remotos y no la actualiza tras parsear el XML descargado.

---

### BUG-08: Invocación incondicional de `sendDataToServer` en `SocketOperationException` provoca `NullPointerException` en conexiones por socket

* **Estado en `master`:** **Corregido.** El `catch (SocketOperationException)` solo compone el mensaje; el envío es un paso común posterior y único por operación, siempre bajo `if (!bySocket)` (`ProtocolInvocationLauncher.java:410-413` y homólogos).
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncher.java:353, 427, 501, 808` frente a `ProtocolInvocationLauncher.java:600-605, 709-714`.
* **Origen de auditoría:** Anteriormente AUD-16 ([03-transporte-servidor-intermedio.md](03-transporte-servidor-intermedio.md), [09-operacion-selectcert.md](09-operacion-selectcert.md), [10-operaciones-save-load.md](10-operaciones-save-load.md), [15-errores.md](15-errores.md)).
* **Descripción:** En las operaciones de lotes (`batch`, línea 353), selección de certificado (`selectcert`, línea 427), guardado en disco (`save`, línea 501) y carga de ficheros (`load`, línea 808), el bloque de captura de `SocketOperationException` dentro de `ProtocolInvocationLauncher.launch()` invoca `sendDataToServer(msg, params.getStorageServletUrl().toString(), params.getId())` de forma incondicional. En contraposición, en las operaciones de firma (`sign`, líneas 709-714) y guardado de firma (`signandsave`, líneas 600-605), la llamada a `sendDataToServer` está debidamente condicionada mediante la comprobación `if (!bySocket)`.
* **Comportamiento y consecuencia:** Si cualquiera de las operaciones `batch`, `selectcert`, `save` o `load` se ejecuta sobre un canal local de socket (`afirma://service?`) o WebSocket (`afirma://websocket?`) y detona una excepción `SocketOperationException` (debido a la cancelación de la persona usuaria, errores en el almacén de claves o fallos de lectura/escritura), la ausencia del parámetro `stservlet` en la URI de socket provoca que `params.getStorageServletUrl()` devuelva `null`. La evaluación de `.toString()` sobre la referencia nula desencadena un `NullPointerException` fulminante que escapa del capturador de errores de la operación, abortando el procesamiento y privando al cliente web de recibir el código normalizado de cancelación o error (`CANCEL`, `SAF_*`) por el canal de comunicación establecido.
* **Causa raíz:** Omisión de la guarda condicional `if (!bySocket)` antes de transferir el mensaje a `sendDataToServer` en los bloques `catch (SocketOperationException)` de `batch`, `selectcert`, `save` y `load`.

---

### BUG-09: Estado estático sin sincronización y condiciones de carrera en `CommandProcessorThread`

* **Estado en `master`:** **Sigue presente.** `CommandProcessorThread.java:72-74` conserva los tres campos estáticos mutables sin sincronización.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.CommandProcessorThread.java:67-69`, `ServiceInvocationManager.java:136-143`.
* **Origen de auditoría:** Anteriormente AUD-22 ([01-vision-general.md](01-vision-general.md), [04-transporte-socket.md](04-transporte-socket.md), [13-almacenes.md](13-almacenes.md)).
* **Descripción:** En el bucle de escucha de conexiones de `ServiceInvocationManager.startService`, cada conexión entrante aceptada en el socket TCP (`ssocket.accept()`) instancia y arranca inmediatamente un nuevo hilo de ejecución `CommandProcessorThread`. Sin embargo, las estructuras de datos donde se acumulan los fragmentos recibidos, las respuestas generadas y el recuento de bloques están declaradas como variables estáticas mutables compartidas a nivel de clase:
  ```java
  private final static List<String> request = new ArrayList<>();
  private final static List<String> toSend = new ArrayList<>();
  private static int parts = 0;
  ```
  Estas colecciones (`ArrayList`) carecen por completo de sincronización (`synchronized`, colecciones concurrentes o primitivas de bloqueo).
* **Comportamiento y consecuencia:** Si dos peticiones HTTP coinciden temporalmente en el socket local (por ejemplo, si el cliente web emite peticiones asíncronas solapadas, si ocurren reintentos antes de concluir una lectura previa, o si dos pestañas del navegador interactúan simultáneamente con la misma sesión de AutoFirma), ambos hilos leen y modifican de forma concurrente las listas estáticas `request` y `toSend`. Esto desencadena excepciones de modificación concurrente (`ConcurrentModificationException`), errores de índice fuera de rango (`IndexOutOfBoundsException`), fragmentos traspuestos o corrupción cruzada de datos, llegando a devolver fragmentos de la firma de una transacción en la respuesta de otra.
* **Causa raíz:** Uso de campos estáticos mutables y no sincronizados en `CommandProcessorThread` para modelar el estado temporal de peticiones y respuestas en una arquitectura de servidor multihilo.

---

### BUG-10: Silenciamiento de excepciones en `ServiceInvocationManager.startService` y retorno erróneo de `OK` tras fallo de inicialización del socket

* **Estado en `master`:** **Corregido.** `ServiceInvocationManager.startService` declara ya `throws SllKeyStoreException, IOException` y `ProtocolInvocationLauncher.java:323-332` las captura por separado: el `OK_RESPONSE` solo se alcanza si no hubo excepción.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ServiceInvocationManager.java:148-168`, `ProtocolInvocationLauncher.java:279-291`.
* **Origen de auditoría:** Anteriormente AUD-26 ([01-vision-general.md](01-vision-general.md), [04-transporte-socket.md](04-transporte-socket.md)).
* **Descripción:** En `ProtocolInvocationLauncher.launch()`, el inicio del modo socket ejecuta:
  ```java
  try {
      ServiceInvocationManager.startService(channelInfo, requestedProtocolVersion);
  } catch (final UnsupportedProtocolException e) {
      ...
      return ProtocolInvocationLauncherErrorManager.getErrorMessage(errorCode);
  }

  return RESULT_OK;
  ```
  En `ServiceInvocationManager.startService()`, si la inicialización del servidor tiene éxito, se entra en un bucle infinito `while (true) { new CommandProcessorThread(...).start(); }` del que solo se sale cuando el temporizador de inactividad ejecuta `Runtime.getRuntime().halt(0)`.
  Por tanto, en una ejecución correcta la línea 290 (`return RESULT_OK;`) es código inalcanzable.
  Sin embargo, si se produce un fallo al intentar ligar el socket (`tryPorts` arroja `IOException` porque todos los puertos candidatos están ocupados) o durante la carga del certificado SSL (`SecureSocketUtils` arroja `GeneralSecurityException`, `KeyStoreException`, etc.), los bloques `catch` de las líneas 148-168 de `ServiceInvocationManager` registran el error en el log pero **no relanzan la excepción ni terminan la JVM**. El método `startService` finaliza normalmente (`void`).
* **Comportamiento y consecuencia:** Tras capturar y silenciar el fallo de red o de certificados, el flujo retorna a `ProtocolInvocationLauncher.java:290`, que ejecuta `return RESULT_OK;` informando falsamente a `SimpleAfirma.main` de que el servicio se inició correctamente. La aplicación Java finaliza con código de salida `0` sin mostrar ningún diálogo de error a la persona usuaria. El cliente JavaScript en el navegador web no recibe respuesta alguna y reintenta la conexión infructuosamente durante 30 segundos (15 intentos $\times$ 2000 ms) hasta arrojar `ApplicationNotFoundException`, desconociendo por completo que la aplicación sí se ejecutó pero fracasó en la asignación de puertos o en la carga de su almacén TLS.
* **Causa raíz:** Absorción silenciosa de excepciones críticas de inicialización en `ServiceInvocationManager.startService` sin propagación al invocador, combinada con un `return RESULT_OK;` incondicional en `ProtocolInvocationLauncher.launch`.

---

### BUG-11: Rechazo del bucle local IPv6 (`::1`) en el WebSocket versión 4

* **Estado en `master`:** **Sigue presente.** Solo cambia el código de error emitido.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.AfirmaWebSocketServerV4.java:38, 57-68`.
* **Origen de auditoría:** Anteriormente AUD-28 ([05-transporte-websocket.md](05-transporte-websocket.md)).
* **Descripción:** La comprobación de procedencia local de la versión 4 es una comparación textual, `LOCALHOST_ADDRESS.equals(address.getHostAddress())` con `LOCALHOST_ADDRESS = "127.0.0.1"`, en lugar de `address.isLoopbackAddress()`.
* **Comportamiento:** Un cliente que conecte por el bucle local IPv6 obtiene `"::1"` o `"0:0:0:0:0:0:0:1"` de `getHostAddress()`, la comparación falla y la petición se clasifica como externa: el servidor responde `SAF_47` (*«Peticion al socket desde IP externa o sin identificar»*) y aborta. El cliente oficial no lo nota porque fija `SERVER_HOST = "127.0.0.1"` (`autoscript.js:1749`), pero cualquier integración que resuelva `localhost` sobre IPv6 queda inoperativa.
* **Causa raíz:** Verificar la procedencia comparando una cadena fija en vez de consultar la propiedad de bucle local de la dirección.
---

### BUG-12: Opción de configuración «Modo VDI» inoperativa por propiedad de sistema huérfana sin consumidor (`websockets.optimizedForVdi`)

* **Estado en `master`:** **Sigue presente.** La propiedad `websockets.optimizedForVdi` se sigue asignando en `AfirmaWebSocketServerManager.java:69` y ninguna clase la consulta.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.AfirmaWebSocketServerManager.java:39, 59-61`, `es.gob.afirma.standalone.ui.preferences.PreferencesPanelGeneral.java:701, 740`; `afirma-ui-simple-configurator-common` · `es.gob.afirma.standalone.configurator.common.PreferencesManager.java:182`.
* **Origen de auditoría:** Anteriormente AUD-29 ([05-transporte-websocket.md](05-transporte-websocket.md)).
* **Descripción:** En la pestaña de configuración General de AutoFirma existe la preferencia de usuario `vdiOptimization` (*«Funcionamiento optimizado para VDI. No recomendado en otros entornos»*). Al arrancar el servidor WebSocket en `AfirmaWebSocketServerManager.startService`, el código recupera este valor booleano y lo asigna a una propiedad de sistema de la JVM:
  ```java
  final boolean optimizedForVdi = PreferencesManager
          .getBoolean(PreferencesManager.PREFERENCE_GENERAL_VDI_OPTIMIZATION);
  System.setProperty(SYSTEM_PROPERTY_OPTIMIZED_FOR_VDI, Boolean.toString(optimizedForVdi));
  ```
  Donde `SYSTEM_PROPERTY_OPTIMIZED_FOR_VDI = "websockets.optimizedForVdi"`.
* **Comportamiento y consecuencia:** El commit original (`20b20d1b94c`) indicaba la intención de introducir retardos en la comunicación WebSocket para evitar bloqueos del canal en entornos de escritorio virtual (VDI). Sin embargo, en el código fuente de AutoFirma 1.9.2 ninguna clase, biblioteca ni método consulta jamás la propiedad `websockets.optimizedForVdi` ni aplica ningún tipo de retardo, pausa o ajuste temporal en la comunicación por WebSocket. La opción del panel de preferencias es un placebo funcional sin efecto alguno en el comportamiento de la aplicación ni en la comunicación con el navegador.
* **Causa raíz:** Implementación incompleta de la funcionalidad VDI: se introdujo la casilla de verificación en la interfaz gráfica y la asignación de la propiedad del sistema, pero nunca se implementó la lógica consumidora en el servidor de WebSocket.

---

### BUG-13: Fuga de estado y asignación cruzada en `showRubricIsCanceled` entre operaciones de firma

* **Estado en `master`:** **Sigue presente.** Ambas clases conservan su `static boolean showRubricIsCanceled` sin reposición, y `ProtocolInvocationLauncherSignAndSave.java:1050` sigue asignando el de la clase `Sign`. La `VisibleSignatureMandatoryException` nueva cubre otro caso, no este.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncherSign.java:108, 960-980, 1014`, `ProtocolInvocationLauncherSignAndSave.java:107, 985-1006, 1040`.
* **Origen de auditoría:** Anteriormente AUD-30 ([06-operaciones-firma.md](06-operaciones-firma.md), [07-operacion-signandsave.md](07-operacion-signandsave.md)).
* **Descripción:** Para gestionar la cancelación del diálogo modal interactivo de posicionamiento de rúbrica en firmas PDF, `ProtocolInvocationLauncherSign` y `ProtocolInvocationLauncherSignAndSave` declaran campos estáticos mutables a nivel de clase: `static boolean showRubricIsCanceled = false;`.
  1. En `ProtocolInvocationLauncherSignAndSave.java:1040`, dentro del listener `SignPdfListener.propertiesCreated`:
     ```java
     if (newParams.isEmpty()) {
         ProtocolInvocationLauncherSign.showRubricIsCanceled = true;
     } else {
         updateOptions(this.extraParams, newParams);
     }
     ```
     El código asigna `true` a `ProtocolInvocationLauncherSign.showRubricIsCanceled` (la variable estática de la clase `Sign`) en lugar de a su propio campo `ProtocolInvocationLauncherSignAndSave.showRubricIsCanceled`.
  2. En ambas clases, ninguna función o método restablece jamás `showRubricIsCanceled` a `false`, ni al arrancar una nueva operación, ni al finalizarla, ni al confirmar una rúbrica exitosa.
* **Comportamiento y consecuencia:**
  1. **En `signandsave`:** La comprobación de cancelación `checkShowRubricDialogIsCalceled` en `ProtocolInvocationLauncherSignAndSave.java:986` evalúa `if (showRubricIsCanceled)`. Al referirse a su propia variable de clase (que nunca fue modificada y permanece siempre en `false`), la condición jamás se cumple. En consecuencia, si la sede web solicita firma visible obligatoria (`visibleSignature=want`) y la persona usuaria pulsa «Cancelar» o cierra el diálogo de posicionamiento de rúbrica, la aplicación no lanza `AOCancelledOperationException` ni eleva `SAF_43`; en su lugar, prosigue silenciosamente y genera la firma en disco sin estampa visual en el PDF, desobedeciendo la restricción de la sede.
  2. **En `sign`:** Si el diálogo de rúbrica se cancela en una operación previa de `sign` o de `signandsave`, `ProtocolInvocationLauncherSign.showRubricIsCanceled` queda fijado a `true` indefinidamente en la memoria de la JVM. En transportes de larga duración (Socket TCP o WebSocket), cualquier firma posterior con `sign` hereda este residuo booleano. Aunque en firmas exitosas posteriores la presencia de las coordenadas inyectadas evita el disparo inmediato del error, si se presenta cualquier flujo donde el diálogo no defina área y `visibleSignature=want`, se produce una cancelación forzada inmediata por el estado arrastrado de una llamada previa.
* **Causa raíz:** Uso de campos estáticos mutables compartidos para modelar el estado transitorio de un diálogo de usuario en lugar de pasar el resultado por el listener o el contexto de la invocación, combinado con un error de cualificación de clase al asignar la variable en `ProtocolInvocationLauncherSignAndSave`.

---

### BUG-14: Omisión de `setAnotherParams` en `signandsave` descarta parámetros de configuración para plugins

* **Estado en `master`:** **Sigue presente.** `ProtocolInvocationUriParserUtil.java:160-166` sigue sin `ret.setAnotherParams(params)`, que sí conserva la fábrica de `sign` en la línea 149.
* **Código fuente:** `afirma-core` · `es.gob.afirma.core.misc.protocol.ProtocolInvocationUriParserUtil.java:156-162` frente a `145`; `afirma-core` · `es.gob.afirma.core.misc.protocol.UrlParametersToSignAndSave.java:101, 365, 372-378`; `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncherSignAndSave.java:159`.
* **Origen de auditoría:** Anteriormente AUD-36 ([07-operacion-signandsave.md](07-operacion-signandsave.md)).
* **Descripción:** La clase `UrlParametersToSignAndSave` implementa la colección de parámetros no reconocidos `anotherParams` junto con sus métodos asociados `getAnotherParams()` y `setAnotherParams(params)`, cuyo cometido es almacenar cualquier parámetro de la URI o del XML intermedio no incluido en la lista estricta `KNOWN_PARAMETERS` para transferirlo a posibles plugins cargados en la aplicación. Asimismo, en el despachador de ejecución `ProtocolInvocationLauncherSignAndSave.java:159`, el objeto `SignOperation` inicializa sus parámetros secundarios invocando expresamente `operation.setAnotherParams(options.getAnotherParams())`. Sin embargo, en el método de fábrica `ProtocolInvocationUriParserUtil.getParametersToSignAndSave` (líneas 156-162), el parser ejecuta:
  ```java
  final UrlParametersToSignAndSave ret = new UrlParametersToSignAndSave(servicesRequired);
  ret.setCommonParameters(params);
  ret.setSignAndSaveParameters(params);
  return ret;
  ```
  omitiendo por completo la llamada `ret.setAnotherParams(params)`, a diferencia de lo que realiza en `getParametersToSign` (línea 145).
* **Comportamiento y consecuencia:** En cualquier invocación a `afirma://signandsave`, la colección `options.getAnotherParams()` permanece siempre como un mapa vacío (`Map.isEmpty() == true`), independientemente de cuántos parámetros de configuración adicionales hayan sido enviados por la sede electrónica o declarados en el XML intermedio. En consecuencia, si se utiliza un plugin que dependa de directivas personalizadas recibidas por protocolo para modular su comportamiento en `preProcess` o `postProcess`, dichas directivas se pierden silenciosamente y el plugin no puede recibirlas durante una operación `signandsave`.
* **Causa raíz:** Omisión de la invocación a `ret.setAnotherParams(params)` en el método de ensamblado `ProtocolInvocationUriParserUtil.getParametersToSignAndSave`.

---

### BUG-15: Ausencia de validación de `cop` en `signandsave` provoca `NullPointerException` y reporte engañoso con `SAF_09`

* **Estado en `master`:** **Sigue presente.** `UrlParametersToSignAndSave.java:238-239` sigue asignando `cop` sin comprobar presencia ni pertenencia al conjunto de operaciones.
* **Código fuente:** `afirma-core` · `es.gob.afirma.core.misc.protocol.UrlParametersToSignAndSave.java:237-238`; `afirma-simple-plugins` · `es.gob.afirma.standalone.plugins.SignOperation.java:67-78`; `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncherSignAndSave.java:155, 297, 728, 882-887`.
* **Origen de auditoría:** Anteriormente AUD-37 ([07-operacion-signandsave.md](07-operacion-signandsave.md)).
* **Descripción:** En la operación `afirma://signandsave`, la suboperación criptográfica a ejecutar (`sign`, `cosign` o `countersign`) se especifica mediante el parámetro `cop`. En `UrlParametersToSignAndSave.java:237-238`, el parser se limita a asignar el valor recibido sin validar si está presente ni si pertenece al conjunto de operaciones soportadas:
  ```java
  final String op = params.get(CRYPTO_OPERATION_PARAM);
  setOperation(op);
  ```
  Posteriormente, en `ProtocolInvocationLauncherSignAndSave.processSign` (línea 155), se establece `operation.setCryptoOperation(Operation.getOperation(options.getOperation()))`. Si `cop` se omitió en la invocación o contiene un valor no reconocido, `Operation.getOperation` retorna `null`.
* **Comportamiento y consecuencia:**
  1. Si los datos a firmar vienen provistos (`dat` o `fileid`) o el usuario los selecciona en el diálogo interactivo, el flujo continúa sin detectar el error de parámetros. AutoFirma inicializa el gestor de certificados, muestra la interfaz gráfica de selección `AOKeyStoreDialog` y solicita la intervención de la persona usuaria para elegir certificado e introducir el PIN de su tarjeta o token criptográfico.
  2. Una vez seleccionado el certificado, el flujo invoca `executeSign` (línea 728), donde la evaluación `switch (cryptoOperation)` sobre una referencia `null` detona inmediatamente un `java.lang.NullPointerException`.
  3. Dicha excepción es capturada por el bloque genérico `catch (final Exception e)` de la línea 882, que registra en el log `"Error al realizar la operacion de firma"` y lanza `SocketOperationException(ERROR_SIGNATURE_FAILED)`, retornando a la sede electrónica el código de error `SAF_09` (*«Error durante la operación de firma»*).
  4. La sede electrónica recibe una notificación que atribuye el fallo a un error interno del proceso criptográfico del certificado en lugar de a un error sintáctico de parámetros (`SAF_03`), habiendo sometido previamente a la persona usuaria a una interacción innecesaria con el diálogo de certificados y PIN.
  *(Nota: Si `data == null` y el firmador implementa `OptionalDataInterface`, el `NullPointerException` se detona anticipadamente en la línea 297 en `signOperation.getCryptoOperation().toString()`, escapando de `sign` y `processSign` y siendo absorbido por el capturador genérico de `ProtocolInvocationLauncher.java:748`, el cual devuelve `SAF_03`).*
* **Causa raíz:** Falta de comprobación de presencia y obligatoriedad del parámetro `cop` en `UrlParametersToSignAndSave.setSignAndSaveParameters`, que debería arrojar `ParameterException` en caso de valor nulo o no reconocido.

---

### BUG-16: Incompatibilidad de `localBatchProcess` con lotes XML provoca fallo tardío con `SAF_03` tras seleccionar certificado y PIN

* **Estado en `master`:** **Sigue presente.** `UrlParametersForBatch.java:251` sigue saltándose la validación de las URLs cuando `localBatchProcess=true`, sin exigir en ningún punto `jsonbatch=true`.
* **Código fuente:** `afirma-core` · `es.gob.afirma.core.misc.protocol.UrlParametersForBatch.java:236-260`; `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncherBatch.java:341-346, 400-422`; `afirma-crypto-batch-client` · `es.gob.afirma.signers.batch.client.BatchSigner.java:223-228`.
* **Origen de auditoría:** Anteriormente AUD-41 ([08-operacion-batch.md](08-operacion-batch.md)).
* **Descripción:** La modalidad de proceso de lote local monofásico (`localBatchProcess=true`) solo está implementada para lotes en formato JSON (`JSONBatchManager` y `LocalBatchSigner`). En `UrlParametersForBatch.java:236-260`, cuando se detecta `localBatchProcess=true`, el analizador omite validar la presencia de las URLs remotas `batchpresignerurl` y `batchpostsignerurl`. Sin embargo, `UrlParametersForBatch` no valida que `jsonbatch` sea `true` (o que el lote sea efectivamente JSON). En `ProtocolInvocationLauncherBatch.java:400-422`, el método `signBatch` desvía al procesador local exclusivamente si `options.isJsonBatch()` es verdadero; si es falso (el valor por defecto cuando no se declara `jsonbatch=true`), desvía la ejecución a `BatchSigner.signXML(..., options.getBatchPresignerUrl(), options.getBatchPostSignerUrl(), ...)`.
* **Comportamiento y consecuencia:**
  1. Si un llamante invoca un lote XML con `localBatchProcess=true` y omite `batchpresignerurl` o `batchpostsignerurl`, el parser `UrlParametersForBatch` no detecta el error de sintaxis y la invocación continúa sin alertas.
  2. AutoFirma inicializa el gestor de claves, despliega la interfaz gráfica modal de selección `AOKeyStoreDialog` y solicita a la persona usuaria la elección de certificado y la introducción de PIN si se trata de un dispositivo criptográfico.
  3. Tras la interacción del usuario, `signBatch` invoca `BatchSigner.signXML` pasando referencias nulas en `batchPresignerUrl` y `batchPostSignerUrl`.
  4. En `BatchSigner.java:223-228`, la comprobación inicial detecta la URL nula y lanza `IllegalArgumentException("La URL de preproceso de lotes no puede se nula ni vacia")`.
  5. Dicha excepción es capturada en `ProtocolInvocationLauncherBatch.java:341-346`, que la convierte en `SocketOperationException(ERROR_PARAMS)` arrojando tardíamente el código de error `SAF_03`.
  6. Si la conexión se realizaba mediante socket TCP local o WebSocket, esta excepción desata además el defecto [BUG-08](#bug-08-invocación-incondicional-de-senddatatoserver-en-socketoperationexception-provoca-nullpointerexception-en-conexiones-por-socket) (invocación incondicional de `sendDataToServer` con URL nula), provocando un `NullPointerException` fulminante.
  7. El usuario ha sido forzado a seleccionar certificado e introducir credenciales privadas para una operación que estaba condenada a fallar desde el análisis de la URI.
* **Causa raíz:** Falta de validación en `UrlParametersForBatch` de que `localBatchProcess=true` requiere indispensablemente `jsonbatch=true`, omitiendo validar las URLs de pre/postfirma en lotes XML locales no soportados.

---

### BUG-17: Corrupción de nombres y fallo de filtrado en `save` por omisión de división de extensiones múltiples (`exts`)

* **Estado en `master`:** **Sigue presente.** `ProtocolInvocationLauncherSave.java:73-74` sigue envolviendo la cadena cruda en `new String[] { options.getExtensions() }` sin dividir por comas.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncherSave.java:86` frente a `ProtocolInvocationLauncherLoad.java:98`; `afirma-ui-core-jse` · `es.gob.afirma.ui.core.jse.JSEUIManager.java:738-744, 771-778`.
* **Origen de auditoría:** Anteriormente AUD-51 ([10-operaciones-save-load.md](10-operaciones-save-load.md)).
* **Descripción:** En la operación `afirma://save`, `ProtocolInvocationLauncherSave.java:86` construye el filtro de extensiones mediante:
  ```java
  Collections.singletonList(
      new GenericFileFilter(
          options.getExtensions() != null ? new String[] { options.getExtensions() } : null,
          options.getFileTypeDescription()
      )
  )
  ```
  enviando la cadena cruda de `options.getExtensions()` (p. ej. `"pdf,txt"`) dentro de un array de un solo elemento (`new String[] { "pdf,txt" }`), en lugar de dividir la lista separada por comas mediante `.split(",")` tal y como hace correctamente `ProtocolInvocationLauncherLoad.java:98`.
* **Comportamiento y consecuencia:**
  1. En `JSEUIManager.java:738-744`, se instancia `new FileNameExtensionFilter(gff.getDescription(), gff.getExtensions())`. Al recibir `new String[] { "pdf,txt" }`, el filtro de Swing asume que la extensión buscada es literalmente `.pdf,txt`.
  2. Como consecuencia, en el diálogo nativo `Guardar como...`, los ficheros con extensiones individuales legítimas (como `documento.pdf` o `archivo.txt`) no son aceptados por el filtro (`ff.accept(file) == false`), ocultándose del selector salvo que el usuario cambie manualmente a «Todos los archivos».
  3. Si el usuario introduce un nombre de fichero con extensión válida (p. ej. `informe.pdf`) o sin ella (`informe`), la comprobación de `JSEUIManager.java:773-777` evalúa `!ff.accept(file)`. Al resultar falsa la aceptación del filtro, concatena `exts[0]`, renombrando el fichero físicamente a `informe.pdf.pdf,txt` o `informe.pdf,txt`.
* **Causa raíz:** Omisión de la llamada a `.split(",")` al construir el array de extensiones en `ProtocolInvocationLauncherSave.java:86`.

---

### BUG-18: Incoherencia de respuesta en `save` por WebSocket (`"OK"` frente a `"SAVE_OK"`) provoca procesamiento erróneo como firma en `autoscript.js`

* **Estado en `master`:** **Corregido.** El cliente acepta ya ambas respuestas: `autoscript.js:2803`, `if (data == "OK" || data == "SAVE_OK")`. El backend sigue emitiendo `"OK"` por WebSocket, pero ya no se interpreta como firma.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncherSave.java:135`, `CommandProcessorThread.java:291-295`, `AfirmaWebSocketServer.java:113`, `AfirmaWebSocketServerV4.java:91`; `afirma-ui-miniapplet-deploy` · `autoscript.js:3423-3428, 3480-3500`.
* **Origen de auditoría:** Anteriormente AUD-52 ([10-operaciones-save-load.md](10-operaciones-save-load.md)).
* **Descripción:** `ProtocolInvocationLauncherSave.processSave` devuelve `"OK"` (`RESULT_OK`) al completar con éxito el guardado. En el transporte por socket local HTTP, `CommandProcessorThread.java:293-295` intercepta este valor y lo transforma expresamente en `"SAVE_OK"` (`if (operationResult.equals(OK)) { sendData(createHttpResponse(true, SAVE_OK), ...); }`). Sin embargo, en el servidor WebSocket (`AfirmaWebSocketServer` y `AfirmaWebSocketServerV4`), el resultado devuelto por `ProtocolInvocationLauncher.launch` se retransmite directamente en broadcast sin transformación alguna.
* **Comportamiento y consecuencia:** En el cliente JavaScript de referencia `autoscript.js:3423`, el receptor de mensajes WebSocket comprueba de forma estricta:
  ```javascript
  if (data == "SAVE_OK") {
      if (successCallback) { successCallback(data); }
      return;
  }
  ```
  Al recibir `"OK"` en lugar de `"SAVE_OK"`, la comprobación resulta falsa. El flujo no detecta el guardado exitoso y desciende hasta la línea 3482, donde asume que la respuesta corresponde a una firma en Base64. A continuación, ejecuta `Base64.decode(data, true)` sobre `"OK"` e invoca `successCallback(signature, certificate)` pasando los bytes residuales de la decodificación de `"OK"` como firma y `null` como certificado. La aplicación web invocadora recibe un resultado corrupto en lugar de la confirmación de guardado.
* **Causa raíz:** Asimetría en la transformación del resultado de `save` entre `CommandProcessorThread` y los servidores WebSocket, combinada con la falta de tolerancia a `"OK"` en el manejador de WebSocket de `autoscript.js`.

---

### BUG-19: Discrepancia de nombres de parámetros (`extension`/`description` vs `exts`/`desc`) en `AppAfirmaJSWebService.saveDataToFile` ignora los filtros en servidor intermedio

* **Estado en `master`:** **Sigue presente.** `autoscript.js:4575` sigue enviando la clave `extension`, y `UrlParametersToSave.java:26, 32` sigue leyendo solo `desc` y `exts`.
* **Código fuente:** `afirma-ui-miniapplet-deploy` · `autoscript.js:4122-4123` frente a `2057-2058, 3580-3581`; `afirma-core` · `es.gob.afirma.core.misc.protocol.UrlParametersToSave.java:23-30, 223-224, 244-245`.
* **Origen de auditoría:** Anteriormente AUD-87 ([10-operaciones-save-load.md](10-operaciones-save-load.md), [16-cliente-javascript.md](16-cliente-javascript.md)).
* **Descripción:** Al construir la petición de guardado para la invocación por servidor intermedio (WebService), la función `AppAfirmaJSWebService.saveDataToFile` en `autoscript.js:4122-4123` asigna los parámetros con las claves `"extension"` y `"description"`:
  ```javascript
  if (extension != null && extension != undefined) { params[params.length] = {key:"extension", value:extension}; }
  if (description != null && description != undefined) { params[params.length] = {key:"description", value:description}; }
  ```
  En cambio, las implementaciones para WebSocket (`AppAfirmaWebSocketClient`, línea 2057) y Socket TCP local (`AppAfirmaJSSocket`, línea 3580) utilizan correctamente `"exts"` y `"desc"`.
* **Comportamiento y consecuencia:** En el backend de AutoFirma, la clase `UrlParametersToSave.java` busca exclusivamente las constantes `FILENAME_EXTS_PARAM = "exts"` y `FILETYPE_DESCRIPTION_PARAM = "desc"`. No reconoce ni traduce las claves `"extension"` ni `"description"`. Por lo tanto, cuando una aplicación web utiliza el transporte por servidor intermedio e invoca `saveDataToFile` indicando extensiones o descripción, ambos parámetros son descartados de forma silenciosa. El diálogo nativo de guardado se abre sin filtros de extensión configurados (mostrando únicamente «Todos los archivos»), ignorando las restricciones solicitadas por la sede web.
* **Causa raíz:** Incoherencia en los nombres de los parámetros generados en `AppAfirmaJSWebService` dentro de `autoscript.js`, que no se corresponden con la especificación de `UrlParametersToSave`.

---

### BUG-20: Incompatibilidad entre `policy.properties` y `ExtraParamsProcessor` impide el uso del identificador oficial `FirmaAGE19`

* **Estado en `master`:** **Sigue presente.** `ExtraParamsProcessor.java:218-221` sigue admitiendo únicamente `FirmaAGE` y `FirmaAGE18`.
* **Código fuente:** `afirma-core` · `src/main/resources/policy.properties:11-17`; `afirma-core` · `es.gob.afirma.core.signers.AdESPolicyPropertiesManager.java:35-38`; `afirma-core` · `es.gob.afirma.core.signers.ExtraParamsProcessor.java:140-143, 217-220`; `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncherSign.java:492-496`, `ProtocolInvocationLauncherSignAndSave.java:484-488`.
* **Origen de auditoría:** Anteriormente AUD-56 ([11-extraparams-y-filtros.md](11-extraparams-y-filtros.md)).
* **Descripción:** El archivo de configuración de políticas de firma `policy.properties` define explícitamente los parámetros de la versión 1.9 de la política de firma de la AGE e indica en sus comentarios que se utiliza mediante `"expPolicy=FirmaAGE19"` (líneas 11-17):
  ```properties
  # Politica de firma de la AGE v1.9. Se utiliza con "expPolicy=FirmaAGE19"
  FirmaAGE19.policyIdentifier=urn:oid:2.16.724.1.3.1.1.2.1.9
  FirmaAGE19.policyQualifier=https://sede.administracion.gob.es/politica_de_firma_anexo_1.pdf
  FirmaAGE19.policyIdentifierHashAlgorithm=http://www.w3.org/2000/09/xmldsig#sha1
  FirmaAGE19.policyIdentifierHash.XAdES=G7roucf600+f03r/o0bAOQ6WAs0=
  FirmaAGE19.policyIdentifierHash.CAdES=G7roucf600+f03r/o0bAOQ6WAs0=
  FirmaAGE19.policyIdentifierHash.PAdES=G7roucf600+f03r/o0bAOQ6WAs0=
  ```
  Sin embargo, en la clase `AdESPolicyPropertiesManager.java:35-38` solo se declaran como constantes reconocidas `POLICY_ID_AGE = "FirmaAGE"` y `POLICY_ID_AGE_1_8 = "FirmaAGE18"`. Asimismo, en `ExtraParamsProcessor.java:217-220`, el método de validación `isSupportedPolicy(final String policyName)` evalúa de forma estricta:
  ```java
  return AdESPolicyPropertiesManager.POLICY_ID_AGE.equals(policyName) ||
         AdESPolicyPropertiesManager.POLICY_ID_AGE_1_8.equals(policyName);
  ```
  omitiendo por completo el identificador `"FirmaAGE19"`.
* **Comportamiento y consecuencia:**
  1. Cuando una aplicación web o integrador envía el parámetro `expPolicy=FirmaAGE19` (siguiendo las directrices documentadas en los propios recursos de AutoFirma), la llamada a `ExtraParamsProcessor.expandProperties` invoca `expandPolicyKeys`.
  2. En `ExtraParamsProcessor.java:140-143`, la comprobación `if (!isSupportedPolicy(policyName))` resulta verdadera, por lo que elimina `expPolicy` de las propiedades y lanza inmediatamente `IncompatiblePolicyException("No se soporta la expansion de atributos para la politica: FirmaAGE19")`.
  3. En `ProtocolInvocationLauncherSign.java:492-496` y `ProtocolInvocationLauncherSignAndSave.java:484-488`, la excepción es capturada y relanzada como `SocketOperationException(ProtocolInvocationLauncherErrorManager.ERROR_INVALID_POLICY, e)`, abortando la operación y retornando al cliente el código de error `SAF_23` (*«Política de firma no soportada o incompatible»*).
  4. La sede electrónica no puede utilizar el identificador versionado `FirmaAGE19`, viéndose obligada a recurrir al alias genérico `FirmaAGE` (que actualmente apunta a la v1.9) o configurar manualmente todos los OIDs y hashes de política individuales.
* **Causa raíz:** Desincronización entre el fichero `policy.properties` y las constantes/validaciones de `AdESPolicyPropertiesManager` y `ExtraParamsProcessor.isSupportedPolicy`, habiéndose omitido la constante `POLICY_ID_AGE_1_9 = "FirmaAGE19"` y su contemplación en las ramas condicionales de expansión de formato.

---

### BUG-21: Errata tipográfica en constante `CAdESExtraParams.POLICY_IDENTIFIER_HASH_ALGORITHM` provoca `IllegalArgumentException` en firmas CAdES con política

* **Estado en `master`:** **Corregido.** `CAdESExtraParams.java:121` declara ya `"policyIdentifierHashAlgorithm"`.
* **Código fuente:** `afirma-crypto-cades` · `src/main/java/es/gob/afirma/signers/cades/CAdESExtraParams.java:121`; `afirma-core` · `src/main/java/es/gob/afirma/core/signers/AdESPolicy.java:84, 163`; `afirma-simple` · `src/main/java/es/gob/afirma/standalone/ui/ExtraParamsHelper.java:222`.
* **Origen de auditoría:** Anteriormente AUD-65 ([12-extraparams-por-formato.md](12-extraparams-por-formato.md)).
* **Descripción:** En la clase `CAdESExtraParams`, la constante pública oficial que define el nombre de la propiedad del algoritmo de resumen de la política de firma contiene una errata tipográfica en su valor literal:
  ```java
  public static final String POLICY_IDENTIFIER_HASH_ALGORITHM = "poliyIdentifierHashAlgorithm";
  ```
  donde falta la letra `'c'` en `"poliy"`. Por el contrario, el parser común `AdESPolicy.buildAdESPolicy` (`AdESPolicy.java:163`), invocado durante la carga de parámetros en `CAdESParameters.load` (`CAdESParameters.java:173`), consulta la clave con la ortografía correcta:
  ```java
  extraParams.getProperty("policyIdentifierHashAlgorithm")
  ```
* **Comportamiento y consecuencia:**
  1. Si un integrador o módulo Java (incluida la propia interfaz gráfica de AutoFirma en `ExtraParamsHelper.java:222`) configura los parámetros adicionales de una firma CAdES utilizando la constante oficial `CAdESExtraParams.POLICY_IDENTIFIER_HASH_ALGORITHM`, la propiedad se inserta en el mapa con la clave errónea `"poliyIdentifierHashAlgorithm"`.
  2. Al procesar la firma en `CAdESParameters.load`, la llamada a `AdESPolicy.buildAdESPolicy` busca `"policyIdentifierHashAlgorithm"`, obteniendo un valor `null`.
  3. Al haberse definido la huella digital (`policyIdentifierHash`) pero resultar nulo el algoritmo, la validación de `AdESPolicy.setValues` (`AdESPolicy.java:84-86`) falla arrojando una excepción fatal:
     ```java
     throw new IllegalArgumentException("Si se indica la huella digital del identificador de politica es obligatorio indicar tambien el algoritmo");
     ```
  4. En cambio, si una sede web invoca la aplicación mediante el protocolo `afirma://` y envía la clave bien escrita (`policyIdentifierHashAlgorithm`), `AdESPolicy` la resuelve con éxito, evidenciando que el fallo afecta exclusivamente a los consumidores programáticos de la constante pública de la biblioteca y al configurador interno `ExtraParamsHelper`.
  *(Nota: En XAdES, la constante `XAdESExtraParams.SIGNATURE_PRODUCCTION_STREET_ADDRESS` en `XAdESExtraParams.java:292` presenta una errata en el nombre del identificador Java con doble `'c'` en `PRODUCCTION`, pero su valor literal en cadena es correcto `"signatureProductionStreetAddress"`, por lo que no altera la ejecución).*
* **Causa raíz:** Error de mecanografía en el literal de la constante `CAdESExtraParams.POLICY_IDENTIFIER_HASH_ALGORITHM`, que omite la `'c'` en `"policy"`, desalineándose de `AdESPolicy.buildAdESPolicy`.

---

### BUG-22: Bucle con error por exceso (*off-by-one*) en el procesamiento de declaraciones de compromiso (`commitmentTypeIndications`)

* **Estado en `master`:** **Corregido a medias.** Los dos bucles siguen con la cota inclusiva (`CommitmentTypeIndicationsHelper.java:98`, `XAdESUtil.java:327`), pero ahora se rechaza `nCtis < 1`, de modo que el caso de declarar cero elementos ya no incorpora ninguno. El elemento de más sigue apareciendo para cualquier $N \geq 1$.
* **Código fuente:** `afirma-crypto-cades` · `src/main/java/es/gob/afirma/signers/cades/CommitmentTypeIndicationsHelper.java:98`; `afirma-crypto-xades` · `src/main/java/es/gob/afirma/signers/xades/XAdESUtil.java:326`.
* **Origen de auditoría:** Anteriormente AUD-69 ([12-extraparams-por-formato.md](12-extraparams-por-formato.md)).
* **Descripción:** En la extracción de declaraciones de compromiso tipificadas (*Commitment Type Indications*) para firmas CAdES, PAdES y XAdES, la cantidad de elementos declarada en el parámetro `commitmentTypeIndications` se parsea como un entero `nCtis`. Sin embargo, el bucle que itera sobre los parámetros indexados está implementado como:
  ```java
  for (int i = 0; i <= nCtis; i++) {
  ```
  en lugar de utilizar una cota estricta `i < nCtis` (asumiendo indexación base cero `0..N-1`).
* **Comportamiento y consecuencia:**
  1. Si un llamante declara `commitmentTypeIndications=1`, el bucle evalúa tanto `i = 0` como `i = 1`. Si las propiedades contienen definidos los identificadores `commitmentTypeIndication0Identifier` y `commitmentTypeIndication1Identifier`, el motor incorpora **dos compromisos** en la firma en lugar de uno.
  2. Si se configura `commitmentTypeIndications=0` con la intención de desactivar o limpiar los compromisos, el bucle ejecuta la iteración `i = 0`. Si existe la propiedad `commitmentTypeIndication0Identifier`, se incorpora a la firma a pesar de haber declarado cero elementos.
  3. En general, para cualquier valor $N$, el motor evalúa $N + 1$ candidatos (desde `0` hasta $N$). En las pruebas internas del propio repositorio (`TestPadesBaseline.java:81-85`), los desarrolladores configuraron `commitmentTypeIndications=2` junto con los índices `0` y `1`, enmascarando el defecto porque el índice `2` era nulo y se omitía mediante `continue`.
* **Causa raíz:** Condición de parada de bucle con comparación inclusiva `<= nCtis` sobre un índice base cero que itera $N + 1$ veces en vez de $N$.

---

### BUG-23: Silenciamiento de excepciones en la inicialización de TSA provoca degradación silenciosa a firma sin sello de tiempo en XAdES y CAdES

* **Estado en `master`:** **Sigue presente.** `XAdESTspUtil.java:82-84` y `AOCAdESSigner.java:576-579` conservan el `catch (Exception)` que devuelve la firma sin sello sin registrar nada.
* **Código fuente:** `afirma-crypto-xades` · `src/main/java/es/gob/afirma/signers/xades/XAdESTspUtil.java:73-81`, `AOXAdESSigner.java:395-400`; `afirma-crypto-cades` · `src/main/java/es/gob/afirma/signers/cades/AOCAdESSigner.java:547-555`; `afirma-crypto-core-pkcs7-tsp` · `src/main/java/es/gob/afirma/signers/tsp/pkcs7/TsaParams.java:111-135`.
* **Origen de auditoría:** Anteriormente AUD-71 ([12-extraparams-por-formato.md](12-extraparams-por-formato.md)).
* **Descripción:** En la generación de firmas avanzadas con sello de tiempo (CAdES-T y XAdES-T), los firmadores `AOCAdESSigner` y `AOXAdESSigner` (vía `XAdESTspUtil`) invocan el proceso de sellado de forma incondicional en cada firma. Para determinar si la persona usuaria o la sede solicitaron sello de tiempo, intentan instanciar `new TsaParams(extraParams)`.
  Si no se configuró sellado de tiempo, el constructor de `TsaParams` detecta que `tsaURL == null` y arroja `IllegalArgumentException("La URL del servidor de sello de tiempo no puede ser nula")`. Para no interrumpir el flujo cuando no se solicita TSA, `XAdESTspUtil` y `AOCAdESSigner` capturan cualquier excepción genérica `Exception`:
  ```java
  final TsaParams tsaParams;
  try {
      tsaParams = new TsaParams(extraParams);
  }
  catch (final Exception e) {
      return xml;
  }
  ```
* **Comportamiento y consecuencia:**
  1. Si un invocador intenta emitir una firma con sello de tiempo (CAdES-T / XAdES-T) pero comete un error sintáctico o de configuración en los parámetros de la TSA (por ejemplo, una sintaxis de URI no válida con espacios en `tsaURL`, un nombre de algoritmo erróneo en `tsaHashAlgorithm`, o un tipo de almacén mTLS no soportado), `new TsaParams` arroja `IllegalArgumentException`.
  2. El bloque `catch (final Exception e)` atrapa la excepción indiscriminadamente, sin diferenciar si se debía a la ausencia intencionada de `tsaURL` o a un fallo de configuración.
  3. El motor silencia el error por completo y devuelve el documento firmado ordinario (XAdES-BES o CAdES-BES) sin estampar el sello de tiempo y sin registrar ninguna advertencia ni notificar ningún código de error (`SAF_*`) a la sede web.
  4. La sede electrónica recibe una firma sin cualificación de tiempo, asumiendo falsamente que el sellado fue aplicado con éxito o descubriendo la carencia únicamente durante la validación documental posterior.
  *(Nota: En FacturaE, `AOFacturaESigner` aplica un filtrado estricto `ALLOWED_PARAMS` donde `tsaURL` no está incluida, descartando los parámetros de TSA antes de alcanzar el motor de firma e impidiendo cualquier sellado de tiempo).*
* **Causa raíz:** Uso de una captura genérica de excepciones sobre `new TsaParams` para deducir si se requería TSA en lugar de comprobar previamente si el parámetro `tsaURL` estaba presente en `extraParams`, provocando la absorción silenciosa de errores de configuración legítimos.

---

### BUG-24: Mutación de estado global en el singleton `AOKeyStore.PKCS11` invalida la resolución de almacén en ejecuciones concurrentes o persistentes

* **Estado en `master`:** **Sigue presente.** `AOKeyStore.java:235` mantiene el mutador público y `SimpleKeyStoreManager.java:311` sigue ejecutando `result.setName(name)` sobre la constante del enum.
* **Código fuente:** `afirma-core-keystores` · `es.gob.afirma.keystores.AOKeyStore.java:226-228`, `AOKeyStoreDialog.java:596`; `afirma-simple` · `es.gob.afirma.standalone.SimpleKeyStoreManager.java:286-288`, `es.gob.afirma.standalone.ui.preferences.PreferencesPanelKeystores.java:225, 233, 573, 585, 600, 692`.
* **Origen de auditoría:** Anteriormente AUD-74 ([13-almacenes.md](13-almacenes.md)).
* **Descripción:** En la enumeración `AOKeyStore`, cada elemento es una instancia única (singleton) en la JVM. A pesar de ello, la constante `AOKeyStore.PKCS11` declara un atributo no inmutable `name` accesible mediante el método público mutador `public void setName(final String name)`.
  Al procesar una petición de firma o selección cuyo nombre coincide con una tarjeta inteligente registrada en las preferencias de usuario o sistema (`existSmartCard == true`), `SimpleKeyStoreManager.getKeyStore` ejecuta:
  ```java
  final AOKeyStore result = AOKeyStore.PKCS11;
  result.setName(name);
  return result;
  ```
  De forma similar, cuando el usuario interactúa con el diálogo modal de cambio de almacén (`AOKeyStoreDialog.openPkcs11KeyStore`), se ejecuta `AOKeyStore.PKCS11.setName(ksName)`.
* **Comportamiento y consecuencia:**
  1. La llamada sobrescribe de forma global y permanente el atributo `name` de la constante enum compartida en toda la máquina virtual de Java.
  2. En procesos de larga duración (como el servidor WebSocket o el demonio Socket local), una vez que se ha seleccionado una tarjeta PKCS#11 registrada o personalizada (asignando por ejemplo el nombre `"DNIe"` o `"FNMT"`), el valor devuelto por `AOKeyStore.PKCS11.getName()` pasa a ser ese nombre particular en lugar del valor de fábrica `"PKCS#11"`.
  3. Si una llamada posterior en la misma sesión solicita explícitamente el almacén `"PKCS#11"`:
     - El primer paso del resolutor (`tempKs.getName().equalsIgnoreCase("PKCS#11")`) falla porque la constante `PKCS11` ya no devuelve `"PKCS#11"`.
     - El segundo paso (comprobación de tarjetas registradas) no coincide salvo que exista una tarjeta nombrada exactamente `"PKCS#11"`.
     - El tercer paso invoca `AOKeyStore.valueOf("PKCS#11")`, que arroja un `IllegalArgumentException` fatal porque el identificador literal de la constante en el enum es `PKCS11` (sin el carácter almohadilla `#`).
     - `SimpleKeyStoreManager.getKeyStore` captura la excepción, registra en el log `WARNING: Almacen de claves no reconocido (PKCS#11)` y devuelve `null`.
     - Como resultado, la aplicación no puede volver a seleccionar el almacén genérico de PKCS#11 por su nombre oficial y degrada forzosamente al almacén predeterminado del sistema operativo (Nivel 4), corrompiendo la selección de certificados hasta que se reinicie por completo el proceso de AutoFirma.
* **Causa raíz:** Violación del principio de inmutabilidad en constantes de enumeración Java compartidas en la JVM al proporcionar un mutador público `setName()` sobre `AOKeyStore.PKCS11`, acoplando el estado mutable de una tarjeta específica al singleton global del tipo de almacén.


---

### BUG-25: Colapso de la distinción entre protocolo obsoleto y protocolo no soportado en el arranque de canales locales

* **Estado en `master`:** **Sigue presente.** La distinción se ha perdido por construcción: `UnsupportedProtocolException.java:31` fija un código único en el constructor y `isNewVersionNeeded()` sigue sin consumidor.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncher.java:283-285` (socket) y `:240-244` (WebSocket); `UnsupportedProtocolException.java:33-47`; `ServiceInvocationManager.java:42-45, 212-220`; `AfirmaWebSocketServerManager.java:27-36, 100-107`; `ProtocolInvocationLauncherErrorManager.java:45, 53, 102, 110`.
* **Origen de auditoría:** Anteriormente AUD-84 ([01-vision-general.md](01-vision-general.md), [14-versiones.md](14-versiones.md), [15-errores.md](15-errores.md)).
* **Descripción:** Ambos gestores de canal local calculan y propagan explícitamente la causa del rechazo de una versión de protocolo. `ServiceInvocationManager.checkSupportProtocol` y `AfirmaWebSocketServerManager.checkSupportProtocol` construyen la excepción con un segundo argumento booleano que discrimina las dos situaciones posibles:
  ```java
  throw new UnsupportedProtocolException(protocolVersion, protocolVersion > CURRENT_PROTOCOL_VERSION);
  ```
  El valor queda accesible mediante `UnsupportedProtocolException.isNewVersionNeeded()`, documentado como *«`true` si se requiere actualizar a una nueva versión, `false` cuando el protocolo es antiguo y no compatible con esta versión»* (`UnsupportedProtocolException.java:40-47`).

  En el punto de captura del canal por socket, el operador ternario que debería traducir esa discriminación a un código de error asigna **el mismo valor en sus dos ramas**:
  ```java
  final String errorCode = e.isNewVersionNeeded()
          ? ProtocolInvocationLauncherErrorManager.ERROR_UNSUPPORTED_PROCEDURE
          : ProtocolInvocationLauncherErrorManager.ERROR_UNSUPPORTED_PROCEDURE;
  ```
  En el punto de captura del canal WebSocket la discriminación ni siquiera se consulta: se asigna directamente `ERROR_UNSUPPORTED_PROCEDURE` (`ProtocolInvocationLauncher.java:242`).
* **Comportamiento y consecuencia:**
  1. Las dos causas opuestas de rechazo se notifican con el mismo código `SAF_21` y, por tanto, con el mismo literal: *«La versión de Autofirma instalada no es compatible con este trámite.\nActualice a la última versión disponible.»* (`protocolmessages.properties:ProtocolLauncher.21`).
  2. El mensaje es **engañoso justamente en el caso contrario al que describe**. Una sede con un `autoscript.js` antiguo que abre `afirma://websocket?v=1` —o que omite el parámetro `v`, cuyo valor por defecto es `1`— solicita un protocolo *obsoleto* frente al conjunto admitido por el canal (`{3, 4}` en WebSocket, `{1, 2, 3}` en socket). En esa situación `isNewVersionNeeded()` vale `false`, es decir, quien está desactualizado es el trámite web y no la aplicación; sin embargo AutoFirma exige a la persona usuaria que actualice AutoFirma, acción que no puede resolver el fallo.
  3. La distinción tampoco queda registrada en el log: la traza de `ProtocolInvocationLauncher.java:282` y `:241` imprime la versión solicitada, pero no el sentido del desajuste, de modo que el diagnóstico exige conocer de memoria los conjuntos admitidos de cada canal.
  4. El código `SAF_22` (`ERROR_UNSOPPORTED_WEB_PROCEDURE`), cuyo literal es exactamente el mensaje que correspondería a este caso —*«El trámite web no es compatible con la versión de Autofirma instalada.\nConsulte las instrucciones del trámite para saber que versión debe instalar.»* (`protocolmessages.properties:ProtocolLauncher.22`)—, está declarado y registrado en el mapa de errores pero **ninguna instrucción ejecutable lo emite** en toda la base de código de 1.9.2. La rama muerta del ternario es el único punto del programa donde su emisión estaba prevista.
* **Causa raíz:** Error de copia en la rama negativa del operador ternario, que repite `ERROR_UNSUPPORTED_PROCEDURE` en lugar de emitir el código complementario `ERROR_UNSOPPORTED_WEB_PROCEDURE`, unido a la omisión completa de la comprobación en el punto de captura homólogo del canal WebSocket.

---

### BUG-26: Los errores anteriores al inicio de la operación no se suben al servidor intermedio y la sede los percibe como «AutoFirma no instalada»

* **Estado en `master`:** **Corregido a medias.** Los fallos de recuperación y descifrado de la configuración remota se suben ya mediante `IntermediateServerErrorSendedException` y `processIntermediateServiceError` (`ProtocolInvocationLauncher.java:922`). Los seis anteriores al inicio de la operación —`SAF_01`, `SAF_02`, `SAF_03`, `SAF_04`, `SAF_13` y `SAF_14`— siguen mostrando diálogo y devolviendo la cadena sin subir nada.
* **Código fuente:** `afirma-simple` · `es.gob.afirma.standalone.protocol.ProtocolInvocationLauncher.java:165-177` (URI nula y esquema no reconocido), `:356-367, 429-440, 504-527, 616-639, 726-749, 811-832` (bloques `catch` de parámetros de cada operación), `:663-677` (recuperación de la configuración remota, replicado en los seis bloques), `:837-842` (operación no reconocida), frente a los únicos puntos de subida en `:353, 426, 501, 603, 612, 712, 721, 808`; `afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js:3722, 4729-4768`.
* **Origen de auditoría:** Anteriormente AUD-83 ([15-errores.md](15-errores.md)).
* **Descripción:** En el transporte por servidor intermedio la respuesta llega a la sede web únicamente si AutoFirma la deposita en `stservlet` mediante `sendDataToServer`. Ese envío se ejecuta desde dos puntos de cada bloque de operación: la entrega del resultado correcto y el `catch (SocketOperationException e)` que recoge los errores de la operación **ya iniciada**.

  Todos los errores detectados antes de alcanzar esos puntos siguen un patrón distinto: muestran un diálogo modal con `showError` o `showErrorDetail` y devuelven la cadena `SAF_nn: <texto>` como valor de retorno de `launch()`. En la invocación por URI del sistema operativo ese valor de retorno no tiene consumidor: `SimpleAfirma.main` lo descarta y llama a `forceCloseApplication(0)` (`SimpleAfirma.java:978-980`).

  Los códigos que quedan atrapados en esta situación son ocho, y cubren la totalidad de los fallos de invocación:

  | Código | Situación | Punto de emisión |
  |---|---|---|
  | `SAF_01` | URI nula | `ProtocolInvocationLauncher.java:166-171` |
  | `SAF_02` | La URI no empieza por `afirma://` | `:172-177` |
  | `SAF_03` | Error de sintaxis o parámetros (`ParameterException`), y cualquier excepción no prevista del parseo | `:356, 362, 429, 435, 516, 522, 628, 634, 738, 744, 823, 829` |
  | `SAF_04` | Operación no reconocida tras agotar la cadena de `else if` | `:837-842` |
  | `SAF_13` | Acceso local bloqueado (`ParameterLocalAccessRequestedException`) | `:510, 622, 732, 817` |
  | `SAF_14` | Versión mínima de aplicación insatisfecha (`ParameterNeedsUpdatedVersionException`) | `:504, 616, 726, 811` |
  | `SAF_15` | Fallo al descifrar la configuración descargada del servidor intermedio | `:671-677` y homólogos |
  | `SAF_16` | Fallo al recuperar la configuración con `fileid`+`rtservlet` | `:663-670` y homólogos |

  Los dos últimos son especialmente llamativos, porque solo pueden producirse **cuando ya se está operando contra el servidor intermedio**: el identificador `id` y la URL `stservlet` están disponibles y validados en ese punto, y aun así el error no se sube.
* **Comportamiento y consecuencia:**
  1. La persona usuaria ve el diálogo modal de AutoFirma con el código real del fallo; la sede web no recibe nada.
  2. `autoscript.js` continúa sondeando `rtservlet` cada `WAITING_CYCLE_MILLIS` (3.000 ms en escritorio, 4.000 ms en Android e iOS) hasta superar `NUM_MAX_ITERATIONS` (10 y 15 respectivamente), es decir, unos 33 segundos en escritorio y 64 en móvil (`autoscript.js:3718-3722, 4740`).
  3. Al agotarse los intentos, la variable `afirmaConnected` sigue valiendo `false` —solo se activa cuando el servidor intermedio devuelve algo— y el cliente toma la rama que atribuye el fallo a la ausencia de la aplicación: muestra el diálogo `ERROR_CONNECTING_AFIRMA` y, si la sede no lo intercepta, invoca `errorCallback` con `java.lang.IOException` y el mensaje de «no se ha podido conectar con AutoFirma» (`autoscript.js:4741-4766`).
  4. El diagnóstico que llega a la sede es por tanto **el contrario del real**: AutoFirma se instaló, arrancó, leyó la URI y rechazó la petición por un error concreto y accionable (un parámetro mal formado, una versión insuficiente, un `fileid` caducado), pero la traza que queda en el registro de la sede acusa a la instalación de la aplicación.
  5. El efecto no se manifiesta en los transportes locales: en socket y WebSocket el valor de retorno de `launch()` **sí** es la respuesta que se entrega al cliente, de modo que los mismos ocho códigos llegan íntegros a la sede.
* **Causa raíz:** La subida al servidor intermedio se implementa como una instrucción puntual dentro de cada bloque de operación en lugar de como el punto único de salida de `launch()`. Los bloques `catch` anteriores a la operación se escribieron siguiendo la forma del canal local —mostrar el error y devolverlo— sin replicar en cada uno de ellos la llamada a `sendDataToServer` que exige el canal remoto. La asimetría es exacta y de signo opuesto a la de [BUG-08](#bug-08-invocación-incondicional-de-senddatatoserver-en-socketoperationexception-provoca-nullpointerexception-en-conexiones-por-socket), donde el mismo envío se ejecuta sin comprobar que el transporte sea el remoto.
