# 02. Gramática de la URI y parámetros comunes

Este capítulo describe la estructura léxica y sintáctica de la URI del protocolo
`afirma://` de AutoFirma 1.9.2, los dos mecanismos de análisis sintáctico que
conviven en el código, el formato XML alternativo para la transferencia de
parámetros mediante servidor intermedio, y el catálogo completo de parámetros
compartidos por múltiples operaciones.

Todas las citas corresponden al código fuente original de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. Estructura y gramática de la URI

La invocación por protocolo se realiza mediante una URI cuyo esquema es `afirma`.
La URI transporta en la parte de autoridad/ruta la operación a realizar y, tras
el carácter delimitador interrogante (`?`), la cadena de consulta (*query string*)
con los parámetros codificados clave-valor.

### 1.1 Esquema y prefijo de operación

El esquema debe ser `afirma://`. Existen dos puntos de verificación en el flujo
de arranque con discrepancias de sensibilidad a mayúsculas:

1. **Filtro inicial en `SimpleAfirma.main`:**
   Comprueba si `args[0]` comienza por el esquema ignorando mayúsculas y
   minúsculas:
   `args[0].toLowerCase().startsWith("afirma://")`
   (`afirma-simple/src/main/java/es/gob/afirma/standalone/SimpleAfirma.java:957`).
2. **Validación en `ProtocolInvocationLauncher.launch`:**
   Exige que la URI comience **estrictamente en minúsculas**:
   `urlString.startsWith("afirma://")`
   (`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncher.java:172`).
   Si la cadena es nula se genera el error `SAF_01` (`ProtocolLauncher.1`,
   `166-171`); si no comienza por `afirma://` exacto, se produce el error `SAF_02`
   (`ProtocolLauncher.2`, `172-178`).

### 1.2 Formas canónicas: `afirma://<op>?` vs `afirma://<op>/?`

El despachador `ProtocolInvocationLauncher.launch` admite cada operación bajo
dos formas de prefijo idénticas funcionalmente: sin barra final
(`afirma://<op>?`) y con barra final (`afirma://<op>/?`)
(`ProtocolInvocationLauncher.java:225, 264, 293, 370, 443, 532, 643-645, 753`).

| Operación | Prefijos aceptados en `launch()` | Líneas en `ProtocolInvocationLauncher.java` |
|---|---|---|
| Canal WebSocket | `afirma://websocket?`, `afirma://websocket/?` | `225` |
| Canal Socket local | `afirma://service?`, `afirma://service/?` | `264` |
| Firma de lotes | `afirma://batch?`, `afirma://batch/?` | `293` |
| Selección de certificado | `afirma://selectcert?`, `afirma://selectcert/?` | `370` |
| Guardar fichero en disco | `afirma://save?`, `afirma://save/?` | `443` |
| Firmar y guardar en disco | `afirma://signandsave?`, `afirma://signandsave/?` | `532` |
| Firma / Multifirma | `afirma://sign?`, `afirma://sign/?`, `afirma://cosign?`, `afirma://cosign/?`, `afirma://countersign?`, `afirma://countersign/?` | `643-645` |
| Carga de ficheros | `afirma://load?`, `afirma://load/?` | `753` |

El cliente JavaScript oficial (`autoscript.js`) genera siempre la forma **sin
barra** (`autoscript.js:2081, 2878, 4381`).

### 1.3 Gramática de la cadena de consulta (*query string*)

La cadena de parámetros comienza inmediatamente tras el **primer carácter `?`**
de la URI. Se divide en pares clave-valor utilizando el carácter ampersand (`&`)
como separador:

```
afirma://<op>?<clave1>=<valor1>&<clave2>=<valor2>&...&<claveN>=<valorN>
```

Las reglas de descomposición léxica aplicadas en el código
(`ProtocolInvocationLauncher.java:946-966` y
`ProtocolInvocationUriParser.java:273-295`) son:

1. **Subcadena de parámetros:**
   Se calcula como `url.substring(url.indexOf('?') + 1)`. Si la URI no
   contiene `?`, `indexOf('?')` devuelve `-1`, tomando la subcadena desde el
   índice `0` (la URI entera).
2. **División de pares:**
   Se ejecuta `split("&")`. Si el valor de un parámetro contiene un carácter
   `&` no codificado en URL (`%26`), corromperá el análisis léxico.
3. **Identificación de clave y valor:**
   Para cada fragmento resultante, se busca la primera aparición del carácter `=`
   con `indexOf('=')`.
   * Si `equalsPos > 0`: el nombre de la clave es `param.substring(0, equalsPos)`.
   * El valor es la porción posterior: `param.substring(equalsPos + 1)`. Si el
     valor contiene caracteres `=`, se conservan íntegros (esencial para el
     relleno de Base64 `==` no codificado).
   * Si `equalsPos <= 0` (no hay signo `=` o el fragmento comienza por `=` sin
     nombre de clave): el fragmento se **descarta silenciosamente** sin
     generar error.
4. **Decodificación de valores:**
   Cada valor se decodifica con `URLDecoder.decode(valor, "UTF-8")`.

---

## 2. Los dos motores de análisis sintáctico en Java

En el código conviven dos implementaciones independientes para descomponer una
URI en un mapa de parámetros:

### 2.1 `ProtocolInvocationLauncher.extractParams`

Ubicado en `afirma-simple` (`ProtocolInvocationLauncher.java:946-966`), es el
método privado que procesa la URI al entrar a `launch()`.
* Extrae únicamente los parámetros presentes a la derecha del primer `?`.
* **No** extrae el nombre de la operación del cuerpo de la URI ni lo introduce
  en el mapa de parámetros.
* Si `URLDecoder.decode` arroja una `UnsupportedEncodingException`, emite una
  advertencia en el log y no inserta el parámetro en el mapa (`959-961`).

### 2.2 `ProtocolInvocationUriParser.parserUri`

Ubicado en `afirma-core` (`ProtocolInvocationUriParser.java:273-307`), es un
método auxiliar estático utilizado en pruebas y herramientas de análisis:
* Además de trocear los parámetros tras el primer `?`, analiza la ruta
  (*path*) previa:
  `String path = uri.substring(uri.indexOf("://") + "://".length(), uri.indexOf('?') != -1 ? uri.indexOf('?') : uri.length())`
  (`299-304`).
* Retira la barra final si existe y extrae el primer segmento antes de
  cualquier barra secundaria.
* Inserta automáticamente en el mapa el parámetro `op` con dicho nombre de
  operación: `params.put(ProtocolConstants.OPERATION_PARAM, path)` (`304`).
* Si `URLDecoder.decode` falla con `UnsupportedEncodingException`, almacena el
  valor crudo sin decodificar en lugar de descartarlo (`288-292`).
* Si un parámetro termina en `=`, asigna explícitamente la cadena vacía `""`
  (`283`).

---

## 3. Representación alternativa de parámetros en XML

Cuando la operación se invoca mediante el mecanismo de servidor intermedio y los
datos no caben en una URL, la aplicación recibe en la URI únicamente los
parámetros `fileid`, `rtservlet` y `key`. AutoFirma descarga entonces un flujo
de bytes desde el servlet remoto que contiene la definición de la operación
empaquetada en XML (o en JSON para lotes si `jsonbatch=true`).

La clase `ProtocolInvocationUriParserUtil.parseXml`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParserUtil.java:91-131`)
parsea este documento XML:

### 3.1 Estructura del XML

El documento se parsea mediante `SecureXmlBuilder.getSecureDocumentBuilder()`
(`96-97`). La estructura esperada es:

```xml
<sign>
  <e k="format" v="CAdES"/>
  <e k="algorithm" v="SHA256withRSA"/>
  <e k="id" v="000954750801"/>
  <e k="stservlet" v="https%3A%2F%2Fservidor%2FStorageService"/>
  <e k="properties" v="bW9kZT1leHBsaWNpdA..."/>
</sign>
```

1. **Elemento raíz:** Representa el código de operación. Si el nombre del
   elemento raíz es `op` (o `OP`), se asume por defecto la operación `SIGN`:
   `params.put(ProtocolConstants.OPERATION_PARAM, "op".equalsIgnoreCase(docElement.getNodeName()) ? "SIGN" : docElement.getNodeName())`
   (`ProtocolInvocationUriParserUtil.java:100-103`).
2. **Elementos hijo `<e>`:** Todos los nodos secundarios inmediatos deben
   llamarse estrictamente `e`; si aparece cualquier otro nombre de nodo, lanza
   `ParameterException("El XML no tiene la forma esperada")` (`113-115`).
3. **Atributos `k` y `v`:** Cada elemento `e` debe poseer ambos atributos; la
   ausencia de cualquiera lanza `ParameterException("El XML no tiene la forma esperada")` (`119-121`).
4. **Decodificación de valores:** El valor del atributo `v` se decodifica con
   `URLDecoder.decode(valueNode.getNodeValue(), "UTF-8")` (`123`). Por tanto,
   **los valores dentro del XML también deben estar codificados en formato URL**
   (URL-encoded) al ser generados por el servidor o el cliente JS (`autoscript.js:4397`).

---

## 4. Jerarquía de clases de parámetros

La abstracción de parámetros de entrada reside en el paquete
`es.gob.afirma.core.misc.protocol`:

```
UrlParameters (abstracta)
├── UrlParametersToSign
├── UrlParametersToSignAndSave
├── UrlParametersToSave
├── UrlParametersToLoad
├── UrlParametersToSelectCert
└── UrlParametersForBatch
```

### 4.1 Clase base `UrlParameters`

La clase `UrlParameters`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java:26-457`)
define las constantes compartidas y el método `setCommonParameters(Map<String, String> params)`
(`253-321`), que extrae:
* Clave de cifrado (`key`): mediante `verifyCipherKey(params)` (`255`).
* Indicador de espera activa (`aw`): `Boolean.parseBoolean(params.get("aw"))` (`257-258`).
* Versión mínima de cliente (`mcv`): almacena la cadena tal cual (`260-262`).
* Datos a procesar (`dat`): si está presente, comprueba que no sea una URL local
  (`file:/`) y delega en `DataDownloader.downloadData` con el flag de compresión
  `gzip` (`267, 298-320`).
* Identificador de fichero remoto (`fileid`) y servlet de descarga (`rtservlet`):
  si no hay parámetro `dat`, verifica la presencia de `fileid` y valida la URL
  de `rtservlet` con `validateURL` (`269-296`).

### 4.2 Métodos factoría en `ProtocolInvocationUriParserUtil`

Para cada tipo de operación, `ProtocolInvocationUriParserUtil.java` ofrece un
método `getParametersTo<Op>(Map<String, String> params, boolean servicesRequired)`:
* Instancia el objeto correspondiente pasándole el flag `servicesRequired`.
* Llama en primer lugar a `ret.setCommonParameters(params)`.
* Llama a continuación al método específico `ret.set<Op>Parameters(params)`.
* En el caso de `UrlParametersToSign`, llama adicionalmente a
  `ret.setAnotherParams(params)` (`ProtocolInvocationUriParserUtil.java:140-147`).

---

## 5. Excepciones de validación de parámetros

El procesamiento de parámetros utiliza tres excepciones especializadas:

| Excepción | Herencia | Cuándo se produce | Código de error asociado |
|---|---|---|---|
| `ParameterException` | `Exception` | Parámetros obligatorios ausentes, formatos inválidos, longitudes excedidas o caracteres no permitidos (`afirma-core/.../ParameterException.java:13`). | `SAF_03` (`ProtocolInvocationLauncherErrorManager.ERROR_PARAMS`) |
| `ParameterLocalAccessRequestedException` | `ParameterException` | Detección de host local (`localhost`, `127.0.0.1`) en las URL de servlets remotos (`afirma-core/.../ParameterLocalAccessRequestedException.java:14`). | `SAF_13` (`ProtocolInvocationLauncherErrorManager.ERROR_LOCAL_ACCESS_BLOCKED`) |
| `ParameterNeedsUpdatedVersionException` | `ParameterException` | Declarada para indicar versión obsoleta (`afirma-core/.../ParameterNeedsUpdatedVersionException.java:14`). Su constructor es de paquete y **nunca es instanciada ni lanzada** en el código (código muerto). | N/A (capturada de forma preventiva para `SAF_41` en `ProtocolInvocationLauncher.java:504, 616, 726, 811`). |

---

## 6. Catálogo de parámetros comunes

A continuación se detalla cada uno de los parámetros comunes del protocolo, su
constante en código, tipo, restricciones sintácticas y comportamiento.

### 6.1 `id` — Identificador de sesión de la operación

* **Constante:** `ID_PARAM = "id"` (`UrlParametersToSign.java:34`,
  `UrlParametersToSignAndSave.java:41`, `UrlParametersToSave.java:33`,
  `UrlParametersToSelectCert.java:24`, `UrlParametersForBatch.java:25`).
* **Tipo:** Cadena alfanumérica.
* **Longitud máxima:** 20 caracteres (`UrlParameters.MAX_ID_LENGTH = 20`,
  `UrlParameters.java:50`). Si supera 20 caracteres, lanza
  `ParameterException("La longitud del identificador de la operacion es mayor de 20 caracteres.")`
  (`UrlParametersToSign.java:220`).
* **Restricción de caracteres:** Se verifica carácter a carácter contra el
  alfabeto inglés en minúsculas y dígitos:
  `(c < 'a' || c > 'z') && (c < '0' || c > '9')`
  (`UrlParametersToSign.java:225`, `UrlParametersToSignAndSave.java:220`,
  `UrlParametersToSave.java:161`, `UrlParametersToSelectCert.java:139`,
  `UrlParametersForBatch.java:205`).
  Cualquier otro carácter (guiones, espacios, signos) lanza
  `ParameterException("El identificador de la firma debe ser alfanumerico.")`.
* **Comportamiento y alternativas:**
  Si el parámetro `id` no está presente, la aplicación comprueba si existe
  `fileid` y utiliza su valor como identificador de sesión:
  `else if (params.containsKey(FILE_ID_PARAM)) { sessionId = params.get(FILE_ID_PARAM); }`
  (`UrlParametersToSign.java:214-216`).
  En `UrlParametersToLoad.java`, el parámetro `id` **no se analiza**
  (su mapa de parámetros no incluye `ID_PARAM`).
* **Propósito:** Correlaciona la petición con el resultado subido al servlet
  de almacenamiento temporal (`StorageService`).

### 6.2 `ver` vs `v` — El doble sistema de versiones del protocolo

El protocolo utiliza dos nombres de parámetro distintos para la versión según el
canal de comunicación:

#### A) Parámetro `v` (en apertura de canales Socket y WebSocket)
* **Constante:** `PROTOCOL_VERSION_PARAM = "v"`
  (`ProtocolInvocationLauncher.java:75`).
* **Ámbito:** Únicamente en las invocaciones que abren servicios locales:
  `afirma://websocket?...` y `afirma://service?...`.
* **Extracción:** Se obtiene mediante `getVersion(urlParams)`
  (`ProtocolInvocationLauncher.java:228, 267, 923-939`).
* **Valor por defecto:** Si `v` no está definido o no es un número entero válido,
  devuelve `1` (`927, 935`).
* **Validación:**
  * Para WebSocket, se valida en `AfirmaWebSocketServerManager.startService`
    (`afirma-simple/.../AfirmaWebSocketServerManager.java:27-36`), que solo
    soporta versiones `3` y `4`.
  * Para Socket plano, se valida en `ServiceInvocationManager.startService`
    (`afirma-simple/.../ServiceInvocationManager.java:42-45`), que solo soporta
    versiones `1`, `2` y `3`.
  * Si la versión no está soportada, lanza `UnsupportedProtocolException`,
    mostrando el error `SAF_21` (`ERROR_UNSUPPORTED_PROCEDURE`).

#### B) Parámetro `ver` (en operaciones de datos y servidor intermedio)
* **Constante:** `VER_PARAM = "ver"` (`UrlParametersToSign.java:38`,
  `UrlParametersToSignAndSave.java:45`, `UrlParametersToSave.java:36`,
  `UrlParametersToLoad.java:18`, `UrlParametersToSelectCert.java:27`,
  `UrlParametersForBatch.java:31`).
* **Ámbito:** En todas las operaciones de procesamiento directo: `sign`,
  `signandsave`, `save`, `load`, `selectcert`, `batch`.
* **Extracción:** En `set<Op>Parameters`:
  ```java
  if (params.containsKey(VER_PARAM)) {
      setMinimumProtocolVersion(params.get(VER_PARAM));
  } else {
      setMinimumProtocolVersion(Integer.toString(ProtocolVersion.VERSION_0.getVersion())); // "0"
  }
  ```
* **Resolución en `ProtocolInvocationLauncher.launch`:**
  Cuando la aplicación se invoca directamente (desde línea de órdenes o URI del SO),
  el argumento `protocolVersion` es `-1`. El despachador resuelve entonces:
  `if (requestedProtocolVersion == -1) { requestedProtocolVersion = parseProtocolVersion(params.getMinimumProtocolVersion()); }`
  (`ProtocolInvocationLauncher.java:300-302, 377-379, 450-452, 539-541, 653-655, 760-762`).
* **Función `parseProtocolVersion(version)`:**
  Parsea la cadena con `Integer.parseInt(version)`. Si es nula o inválida,
  devuelve `1` (`907-915`).
* **Interacción clave:** Si la llamada proviene de un canal socket/websocket
  activo, `requestedProtocolVersion` ya contiene la versión fijada en la
  conexión inicial (`!= -1`), por lo que el parámetro `ver` de la operación
  **es completamente ignorado**.

### 6.3 `dat` y `gzip` — Datos inline y descompresión

* **Constante `dat`:** `DATA_PARAM = "dat"` (`UrlParameters.java:34`).
* **Constante `gzip`:** `GZIPPED_DATA_PARAM = "gzip"` (`UrlParameters.java:40`).
* **Procesamiento:** `UrlParameters.setCommonParameters` (`UrlParameters.java:298-320`).
* **Comprobación de ficheros locales:**
  Si el valor comienza por `file:/`, se rechaza con:
  `ParameterException("No se permite la lectura de ficheros locales: " + dataPrm)`
  (`299-304`).
* **Mecanismo de resolución (`DataDownloader.downloadData`):**
  Ubicado en `afirma-core/.../DataDownloader.java:57-140`, evalúa la fuente de
  datos en el siguiente orden estricto:
  1. Si `gzip=true` y los datos son Base64 válidos: decodifica el Base64
     (sustituyendo caracteres URL-safe `_` por `/` y `-` por `+`) y los
     descomprime mediante GZIP con `gunzipBytes()` (`63-69`).
  2. Si comienza por `http://` o `https://`: descarga los datos por HTTP GET
     usando `UrlHttpManagerFactory` (`74-100`).
  3. Si comienza por `ftp://`: abre un `InputStream` con `java.net.URL` (`102-108`).
  4. Si comienza por `file:/`: intenta cargarlo (aunque `UrlParameters` ya lo
     ha impedido previamente) (`110-122`).
  5. Si el contenido es Base64 reconocible por `Base64.isBase64()`: lo
     decodifica sustituyendo `_` por `/` y `-` por `+` (`127-137`).
  6. En cualquier otro caso: toma los bytes textuales en crudo con
     `dataSource.getBytes()` (`139`).

### 6.4 `fileid` — Identificador de datos remotos

* **Constante:** `FILE_ID_PARAM = "fileid"` (`UrlParameters.java:59`).
* **Tipo:** Cadena.
* **Propósito:** Indica que los datos o la configuración completa de la operación
  no viajan en la URI y deben descargarse del servidor intermedio mediante
  `rtservlet`.
* **Comportamiento en los parsers:**
  Si `fileid` está presente y `dat` no, `UrlParameters.setCommonParameters`
  asigna `fileId` y exige la presencia de `rtservlet` (`UrlParameters.java:269-277`).
  En los analizadores específicos (`UrlParametersToSign.java:251-253`,
  `UrlParametersToSignAndSave.java:242-244`, `UrlParametersToSelectCert.java:157-159`,
  `UrlParametersForBatch.java:227-229`), la presencia de `fileId` provoca un
  **retorno inmediato (`return`)**: se interrumpe la lectura del resto de
  parámetros de la URI (`format`, `algorithm`, `properties`, etc.), ya que se
  espera recuperarlos íntegramente del XML remoto.

### 6.5 `rtservlet` — Servlet remoto de recuperación (*RetrieveService*)

* **Constante:** `RETRIEVE_SERVLET_PARAM = "rtservlet"` (`UrlParameters.java:43`).
* **Tipo:** URL HTTP o HTTPS.
* **Validación (`UrlParameters.validateURL`, líneas 351-380):**
  1. Debe ser una URL válida según `new URL(URLDecoder.decode(url, "UTF-8"))`.
  2. Protocolo obligatorio `http` o `https`. Cualquier otro protocolo lanza
     `ParameterException("El protocolo de la URL proporcionada para el servlet no esta soportado: ...")`
     (`364-368`).
  3. **Protección contra acceso local:** Si el host es `localhost` o `127.0.0.1`,
     lanza `ParameterLocalAccessRequestedException("El host de la URL proporcionada para el Servlet es local")`
     (`370-374`), que en `setCommonParameters` se traduce a error `SAF_13`.
  4. **Prohibición de parámetros en la URL del servlet:** La URL no puede
     contener caracteres `?` ni `=`:
     `if (servletUrl.toString().indexOf('?') != -1 || servletUrl.toString().indexOf('=') != -1)`
     lanza `ParameterException("Se han encontrado parametros en la URL del servlet")`
     (`376-378`).

### 6.6 `stservlet` — Servlet remoto de almacenamiento (*StorageService*)

* **Constante:** `STORAGE_SERVLET_PARAM = "stservlet"` (`UrlParameters.java:46`).
* **Tipo:** URL HTTP o HTTPS.
* **Propósito:** URL de subida a la que AutoFirma envía el resultado de la
  operación (firma electrónica, certificado o mensaje de error) cuando opera
  por servidor intermedio.
* **Validación:** Se valida exactamente con las mismas reglas de
  `validateURL()` descritas para `rtservlet`.
* **Obligatoriedad:** En operaciones donde `servicesRequired == true`
  (comunicación sin socket):
  Si se proporciona `id` pero no se incluye `stservlet`, lanza
  `ParameterException("No se ha recibido la direccion del servlet para el guardado del resultado de la operacion")`
  (`UrlParametersToSign.java:273-275`, `UrlParametersToSignAndSave.java:266-268`,
  `UrlParametersToSelectCert.java:179-181`, `UrlParametersForBatch.java:280-282`).

### 6.7 `key` — Clave de cifrado simétrico para el servidor intermedio

* **Constante:** `KEY_PARAM = "key"` (`UrlParameters.java:56`).
* **Longitud fija obligatoria:** Exactamente 8 caracteres
  (`CIPHER_KEY_LENGTH = 8`, `UrlParameters.java:53`).
* **Validación (`UrlParameters.verifyCipherKey`, líneas 327-345):**
  * Si el parámetro no existe o su valor es vacío: devuelve `null` (comunicación
    sin cifrar).
  * Si `key.length() != 8`: lanza
    `ParameterException("La longitud de la clave de cifrado no es correcta")` (`342`).
  * Devuelve los bytes de la clave con `key.getBytes()`.
* **Algoritmo:** La clave de 8 bytes se emplea para cifrado DES (DES/ECB o
  DES/CBC según la versión del protocolo; ver capítulo 03).

### 6.8 `properties` — Diccionario de parámetros adicionales

* **Constante:** `PROPERTIES_PARAM = "properties"` (`UrlParameters.java:31`).
* **Tipo:** Cadena en Base64 (estándar o URL-safe).
* **Formato interno:** Al decodificarse en Base64, representa un archivo de texto
  de propiedades Java (`clave=valor` por línea).
* **Decodificación (`AOUtil.base642Properties`, `afirma-core/.../AOUtil.java:485-495`):**
  Sustituye caracteres URL-safe (`-` por `+`, `_` por `/`), decodifica el
  Base64 y carga las propiedades mediante:
  `p.load(new InputStreamReader(new ByteArrayInputStream(decodedBytes), "UTF-8"))`.
* **Tolerancia a fallos:** Si la decodificación o el análisis sintáctico del
  fichero de propiedades falla, `UrlParametersToSign` registra un error severo
  en el log pero **no interrumpe la ejecución**, asignando un objeto
  `new Properties()` vacío (`UrlParametersToSign.java:306-310`).

### 6.9 `keystore` y `ksb64` — Selección y biblioteca del almacén de claves

AutoFirma admite la configuración previa del almacén de claves a abrir:

* **Parámetro moderno:** `KEYSTORE_PARAM = "ksb64"` (`UrlParameters.java:66`).
  Su valor viene codificado en Base64.
* **Parámetro heredado:** `KEYSTORE_OLD_PARAM = "keystore"` (`UrlParameters.java:63`).
  Texto en plano codificado en URL (`//TODO: Eliminar para terminar la compatibilidad con Autofirma 1.4.X`).
* **Prioridad:** Si existe `keystore`, tiene precedencia sobre `ksb64`
  (`UrlParameters.java:386-396`).
* **Sintaxis del valor desreferenciado:**
  `<nombre_almacen>[:<ruta_biblioteca_opcional>]`
  El separador `:` divide el valor en dos partes:
  1. **Nombre del almacén (`defaultKeyStore`):**
     Extraído por `getKeyStoreName()` (`382-412`). Corresponde a la porción
     anterior al carácter `:` (o la cadena completa si no hay dos puntos).
     Si la cadena no tiene contenido válido antes del separador, devuelve `null`.
  2. **Ruta de la biblioteca nativa (`defaultKeyStoreLib`):**
     Extraído por `getDefaultKeyStoreLib()` (`415-440`). Corresponde a la
     porción posterior al carácter `:`.
     Pasa por el método de saneamiento `cleanupPath(path)` (`447-456`), que
     elimina comillas dobles y simples (`"`, `'`), recorta espacios en blanco y
     resuelve la ruta absoluta canónica con
     `new File(cleanedpath).getCanonicalPath()`. Si la resolución canónica falla,
     devuelve `null`.

### 6.10 `aw` — Espera activa (*Active Waiting*)

* **Constante:** `ACTIVE_WAITING_PARAM = "aw"` (`UrlParameters.java:70`).
* **Tipo:** Booleano. Se parsea con
  `Boolean.parseBoolean(params.get("aw"))` (`UrlParameters.java:258`).
* **Comportamiento:**
  Si es `true` y la invocación no es por socket (`!bySocket`), el despachador
  llama a `requestWait(storageServletUrl, id)`
  (`ProtocolInvocationLauncher.java:338, 408, 484, 574, 684, 791`).
  Arranca un hilo en segundo plano que sondea periódicamente el servlet de
  almacenamiento para notificar al servidor intermedio de que la aplicación
  sigue viva en el cliente y evitar la expiración prematura de la sesión.

### 6.11 `mcv` — Versión mínima requerida de AutoFirma

* **Constante:** `MINIMUM_CLIENT_VERSION_PARAM = "mcv"` (`UrlParameters.java:73`).
* **Tipo:** Cadena de versión (por ejemplo `"1.8.0"` o `"1.9.2"`).
* **Verificación:** Se ejecuta en cada lanzador específico
  (`ProtocolInvocationLauncherSign.java:143-154`,
  `ProtocolInvocationLauncherSignAndSave.java:140-151`,
  `ProtocolInvocationLauncherSave.java:62-73`,
  `ProtocolInvocationLauncherLoad.java:73-84`,
  `ProtocolInvocationLauncherSelectCert.java:89-100`,
  `ProtocolInvocationLauncherBatch.java:90-101`).
* **Lógica de comprobación:**
  ```java
  final Version requestedVersion = new Version(minimumRequestedVersion);
  if (requestedVersion.greaterThan(SimpleAfirma.getVersion())) {
      final String errorCode = ProtocolInvocationLauncherErrorManager.ERROR_MINIMUM_VERSION_NON_SATISTIED;
      ProtocolInvocationLauncherErrorManager.showError(errorCode);
      if (!bySocket) {
          throw new SocketOperationException(errorCode);
      }
      return ProtocolInvocationLauncherErrorManager.getErrorMessage(errorCode);
  }
  ```
  Si la versión de la aplicación en ejecución es inferior a `mcv`, se interrumpe
  la operación con el código de error `SAF_41`.

### 6.12 `appname` — Nombre o dominio de la aplicación solicitante

* **Constante:** `APP_NAME_PARAM = "appname"` (`UrlParameters.java:76`).
* **Tipo:** Cadena.
* **Procesamiento:** Se extrae y almacena únicamente en `UrlParametersToSign.java:241-243`
  y `UrlParametersForBatch.java:221-223`.
* **Uso real en el código:** Se guarda en la variable de instancia `this.appName`,
  accesible mediante el getter `getAppName()`. Sin embargo, **ninguna clase de
  la aplicación llama a `getAppName()`** en todo el repositorio. Se conserva
  por motivos de compatibilidad sintáctica con invocaciones externas.

### 6.13 `sticky` y `resetsticky` — Persistencia del certificado en sesión

Permiten recordar la clave privada y certificado elegidos por el usuario para
no volver a solicitar confirmación en firmas consecutivas:

* **Constantes:** `STICKY_PARAM = "sticky"` y `RESET_STICKY_PARAM = "resetsticky"`
  (`UrlParametersToSign.java:42, 46`, `UrlParametersToSignAndSave.java:48, 52`,
  `UrlParametersToSelectCert.java:30, 34`, `UrlParametersForBatch.java:34, 38`).
* **Tipo:** Booleanos (`Boolean.parseBoolean(...)`, defecto `false`).
* **Estado en memoria:** Reside en un campo estático de la JVM:
  `ProtocolInvocationLauncher.stickyKeyEntry`
  (`ProtocolInvocationLauncher.java:90, 110-120`).
* **Comportamiento durante la selección de certificado:**
  * Si `sticky=true`, `resetsticky=false` y `stickyKeyEntry != null`:
    se reutiliza directamente la entrada de clave existente sin mostrar diálogo
    al usuario (`ProtocolInvocationLauncherSign.java:518-521`).
  * Si la operación culmina con éxito:
    `ProtocolInvocationLauncher.setStickyKeyEntry(stickySignatory ? pke : null)`
    (`ProtocolInvocationLauncherSign.java:643`). Si `sticky=true`, se guarda la
    clave privada; si `sticky=false`, **se borra cualquier clave previamente
    fijada**.
  * Si `resetsticky=true`: se ignora cualquier clave prefijada y se obliga al
    usuario a seleccionar un nuevo certificado.
* **Alcance:** Dado que `stickyKeyEntry` es estático en el proceso de la JVM,
  la persistencia **solo funciona en los transportes de larga duración**
  (Socket local o WebSocket). En el transporte por servidor intermedio, donde
  cada invocación por URI ejecuta un proceso de sistema operativo nuevo, el valor
  no sobrevive entre ejecuciones.

### 6.14 `jvc` — Código de versión del JavaScript

* **Constante:** `JAVASCRIPT_VERSION_CODE_PARAM = "jvc"`
  (`ProtocolInvocationLauncher.java:68`).
* **Constantes de control:**
  `DEFAULT_JAVASCRIPT_VERSION_CODE = 1` (`64`).
  `MIN_JAVASCRIPT_VERSION_CODE_NEEDED = 1` (`66`).
* **Extracción y verificación:** En `ProtocolInvocationLauncher.java:196-214`.
  Si `jvc` no está presente o no es un entero, se establece a `1`.
  Si `jvc < MIN_JAVASCRIPT_VERSION_CODE_NEEDED` (es decir, menor estricto que 1),
  se muestra un diálogo modal de aviso al usuario con el texto de `ProtocolLauncher.51`
  (*«Se está utilizando una versión antigua del cliente web...»*).
  La versión actual de `autoscript.js` declara `VERSION_CODE = 3` (`autoscript.js:27`)
  y envía siempre `jvc=3` (`autoscript.js:4362`).

---

## 7. Gestión de parámetros no reconocidos

La clase `UrlParametersToSign` implementa un filtro de clasificación de parámetros:

* **Listado de parámetros conocidos (`KNOWN_PARAMETERS`):**
  Definido en `UrlParametersToSign.java:52-57`:
  `format`, `algorithm`, `id`, `ver`, `sticky`, `resetsticky`, `properties`,
  `dat`, `gzip`, `rtservlet`, `stservlet`, `key`, `fileid`, `keystore`, `ksb64`,
  `aw`, `mcv`, `appname`.
* **Colección `anotherParams`:**
  El método `setAnotherParams(params)` (`360-367`) itera sobre todas las claves
  recibidas en la URL; cualquier parámetro no incluido en `KNOWN_PARAMETERS` se
  almacena en el mapa `anotherParams`.
* **Propósito:** Permite que extensiones o *plugins* de AutoFirma
  (como los de validación o hash adicionales) recuperen configuraciones propias
  sin que el parser principal las rechace como inválidas.

---

## 8. Matriz de soporte de parámetros comunes por operación

La siguiente tabla resume qué parámetros comunes reconoce cada clase de parámetros:

| Parámetro | `UrlParametersToSign` | `UrlParametersToSignAndSave` | `UrlParametersToSave` | `UrlParametersToLoad` | `UrlParametersToSelectCert` | `UrlParametersForBatch` |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| `id` | Sí | Sí | Sí | **No** | Sí | Sí |
| `ver` | Sí | Sí | Sí | Sí | Sí | Sí |
| `dat` | Sí | Sí | Sí | **No** | **No** | Sí |
| `gzip` | Sí | Sí | Sí | **No** | **No** | Sí |
| `fileid` | Sí | Sí | Sí | Sí (en launcher) | Sí | Sí |
| `rtservlet` | Sí | Sí | Sí | Sí (en launcher) | Sí | Sí |
| `stservlet` | Sí | Sí | Sí | Sí (en launcher) | Sí | Sí |
| `key` | Sí | Sí | Sí | Sí (en launcher) | Sí | Sí |
| `properties` | Sí | Sí | **No** | **No** | Sí | Sí |
| `keystore`/`ksb64` | Sí | Sí | **No** | **No** | Sí | Sí |
| `aw` | Sí | Sí | Sí | Sí (en launcher) | Sí | Sí |
| `mcv` | Sí | Sí | Sí | Sí | Sí | Sí |
| `appname` | Sí | **No** | **No** | **No** | **No** | Sí |
| `sticky` | Sí | Sí | **No** | **No** | Sí | Sí |
| `resetsticky` | Sí | Sí | **No** | **No** | Sí | Sí |

---

## Lo que el código no aclara

**Incoherencia entre `ver` y `v`.** El parámetro de versión de la URI se llama
`v` en el arranque de sockets y websockets (`ProtocolInvocationLauncher.java:75`),
pero se llama `ver` en los parsers de operaciones (`UrlParametersToSign.java:38`).
Si un invocador envía `afirma://sign?v=3...`, el parser de firma busca `ver`,
no lo encuentra y asume `ProtocolVersion.VERSION_0` (versión 0)
(`UrlParametersToSign.java:238`), ignorando silenciosamente el parámetro `v`.

**Algoritmos soportados dispares entre `sign` y `signandsave`.**
El conjunto `SUPPORTED_SIGNATURE_ALGORITHMS` de `UrlParametersToSign`
(`UrlParametersToSign.java:60-74`) incluye algoritmos ECDSA (`SHA1withECDSA`,
`SHA256withECDSA`, `SHA384withECDSA`, `SHA512withECDSA`). En cambio, el conjunto
de `UrlParametersToSignAndSave` (`UrlParametersToSignAndSave.java:67-77`) **no los
contempla**: únicamente admite variantes RSA. Intentar ejecutar una operación
`signandsave` con ECDSA produce un error `ParameterException("Algoritmo de firma no soportado")`
(`285`). El código no justifica esta asimetría.

**`ParameterNeedsUpdatedVersionException` es código muerto.** La excepción
está declarada en `afirma-core` (`ParameterNeedsUpdatedVersionException.java:14`)
y se captura de forma explícita en varios bloques `catch` de
`ProtocolInvocationLauncher.java` (`504, 616, 726, 811`). Sin embargo, su
constructor es de visibilidad de paquete y ninguna clase del proyecto la lanza.
La verificación de `mcv` lanza en su lugar una `SocketOperationException` con
código `SAF_41`.

**El aviso de `jvc` es inalcanzable con parámetros ausentes.**
`DEFAULT_JAVASCRIPT_VERSION_CODE` y `MIN_JAVASCRIPT_VERSION_CODE_NEEDED` valen
ambos `1` (`ProtocolInvocationLauncher.java:64, 66`). Cuando `jvc` no se pasa en
la URI o contiene un valor no numérico, la excepción se captura y se asigna el
valor por defecto `1` (`202`). Por tanto, la condición `jvc < 1` solo puede
dispararse si el llamador envía intencionadamente un número igual o menor que 0
(como `jvc=0` o `jvc=-1`).

**`load` carece de análisis de servlets pero el launcher los utiliza.**
`UrlParametersToLoad.java` no analiza ni declara las constantes `id`, `fileid`,
`stservlet` ni `rtservlet`. Sin embargo, `ProtocolInvocationLauncher.java`
invoca `params.getFileId()` (`767`), `params.getStorageServletUrl()` (`791, 803`)
y `params.getId()` (`791, 804`). Al no haberse parseado estos campos en la
lectura de la URL, tales llamadas devuelven `null` a menos que los parámetros
hayan sido inyectados por otra vía.

**`appname` se analiza pero nunca se consulta.** Tanto `UrlParametersToSign`
como `UrlParametersForBatch` definen el parámetro `appname` y lo almacenan en
la propiedad `appName`, pero ningún componente de lógica de negocio o interfaz
de usuario consume el método `getAppName()`.

**`resetsticky` y `sticky` sin aislamiento multiusuario.**
La referencia al certificado y clave fija se almacena en el campo estático
`ProtocolInvocationLauncher.stickyKeyEntry` (`ProtocolInvocationLauncher.java:90`).
Al ser estático para toda la JVM, en caso de múltiples hilos concurrentes en
comunicación por socket o websocket no existe aislamiento de sesión: una
operación con `sticky=true` puede afectar a peticiones posteriores concurrentes.
