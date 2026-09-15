# 14. Versiones y compatibilidad

Este capítulo describe los mecanismos de versionado, negociación de protocolo y
control de compatibilidad presentes en la invocación de AutoFirma 1.9.2. En la
arquitectura de AutoFirma conviven cuatro nociones de versión distintas:

1. La **versión del protocolo** (`ProtocolVersion`, valores enteros 0–4), que
   gobierna la estructura del intercambio de mensajes, el cifrado hacia el
   servidor intermedio y el formato de las respuestas.
2. La **versión de la aplicación cliente** (cadena semántica `"1.9.2"`, comparada
   mediante la clase `Version`), requerida opcionalmente mediante el parámetro `mcv`.
3. El **código de versión del JavaScript de despliegue** (`jvc` / `VERSION_CODE`,
   entero), utilizado para advertir de scripts de invocación obsoletos o inseguros.
4. La **versión interna del aplicativo** (entero `currentVersion`, p. ej. `21`),
   usada exclusivamente por el comprobador de actualizaciones automáticas (`Updater`).

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. El enum `ProtocolVersion`

El núcleo de protocolo define las versiones del protocolo de comunicación en
el enum `ProtocolVersion`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolVersion.java:8-70`).
A diferencia de la versión de AutoFirma como aplicación, estos identificadores solo
se incrementan cuando un cambio rompe la compatibilidad binaria o estructural
del protocolo (`ProtocolVersion.java:5-7`).

### 1.1 Catálogo de versiones

| Versión | Identificador enum | Valor entero | Propósito y cambios introducidos | Dónde se evalúa en el código |
|---|---|---|---|---|
| 0 | `VERSION_0` | `0` | Versión inicial del protocolo de invocación por URI (`ProtocolVersion.java:11`). | Valor por defecto si falta `ver` en los parsers de operaciones (`UrlParametersToSign.java:238`, `UrlParametersToSave.java:174`, etc.). |
| 1 | `VERSION_1` | `1` | Introduce correcciones en el cifrado y descifrado de datos intercambiados con el servidor intermedio (`ProtocolVersion.java:12-13`). | Valor por defecto en `parseProtocolVersion()` (`ProtocolInvocationLauncher.java:912`) y en `getVersion()` de sockets (`ProtocolInvocationLauncher.java:927`). Soportado en sockets locales (`ServiceInvocationManager.java:45`). |
| 2 | `VERSION_2` | `2` | Realiza cambios en la codificación del parámetro de almacén de claves, introduciendo `ksb64` (`Base64`) junto al histórico `keystore` (`ProtocolVersion.java:14-15`, `UrlParameters.java:47-52, 308-320`). | Soportado en sockets locales (`ServiceInvocationManager.java:45`). |
| 3 | `VERSION_3` | `3` | Devuelve el nombre del fichero y metadatos (`extraData` en JSON) en las operaciones de firma (`ProtocolVersion.java:16-17`). Versión mínima de WebSocket sin comprobación de sesión (`AfirmaWebSocketServerManager.java:27`). | `NativeSignDataProcessor.java:77, 97`, `ServiceInvocationManager.java:42`, `AfirmaWebSocketServerManager.java:27`. |
| 4 | `VERSION_4` | `4` | Introduce seguridad adicional en la comunicación por WebSocket: comprobación estricta de `idsession` por conexión y soporte multiconexión (`ProtocolVersion.java:18-19`, `AfirmaWebSocketServerManager.java:30`). | Versión máxima soportada del protocolo (`ProtocolInvocationLauncher.java:62`). Despacho a `AfirmaWebSocketServerV4` (`AfirmaWebSocketServerManager.java:70-72`). |

### 1.2 La constante `MAX_PROTOCOL_VERSION_SUPPORTED`

En `ProtocolInvocationLauncher.java:62` se define la cota superior del protocolo:

```java
static final ProtocolVersion MAX_PROTOCOL_VERSION_SUPPORTED = ProtocolVersion.VERSION_4;
```

Esta constante gobierna la admisión de peticiones en todas las operaciones
(`batch`, `selectcert`, `save`, `signandsave`, `sign`, `load`).

### 1.3 Método de comprobación `ProtocolVersion.support()`

La comprobación de si una versión solicitada es admisible se realiza a través de
los métodos `support()` del enum
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolVersion.java:38-65`):

```java
public boolean support(final int protocolVersion) {
    return this.version >= protocolVersion;
}
```

La variante sobrecargada `support(final Object protocolVersion)` (`38-57`) admite
instancias de `Integer`, `ProtocolVersion` o cualquier objeto cuya representación
`toString()` sea analizable mediante `Integer.parseInt()`. Si el objeto es nulo o
no numérico, registra una advertencia en log y devuelve `false` (`51-56`).

> [!IMPORTANT]
> `support(protocolVersion)` evalúa exclusivamente si `this.version >= protocolVersion`.
> Al ejecutarse sobre `MAX_PROTOCOL_VERSION_SUPPORTED` (`VERSION_4`, entero `4`),
> cualquier entero menor o igual que 4 (`... -1, 0, 1, 2, 3, 4`) devuelve `true`.
> La condición no impone cota inferior; solo rechaza valores estrictamente
> superiores a 4 (`protocolVersion > 4`).

Si `support()` devuelve `false`, la operación rechaza la solicitud arrojando una
`SocketOperationException` con código de error `SAF_21`
(`ERROR_UNSUPPORTED_PROCEDURE`, *"Versión de protocolo no soportada"*):
* `ProtocolInvocationLauncherSign.java:133-139`
* `ProtocolInvocationLauncherSignAndSave.java:130-137`
* `ProtocolInvocationLauncherBatch.java:77-87`
* `ProtocolInvocationLauncherSelectCert.java:75-86`
* `ProtocolInvocationLauncherSave.java:49-59`
* `ProtocolInvocationLauncherLoad.java:60-70`

---

## 2. Parámetros de versión en el protocolo

El protocolo utiliza cuatro parámetros distintos para gobernar las versiones:
`ver`, `v`, `mcv` y `jvc`. Su presencia y significado dependen del canal y del
tipo de invocación.

| Parámetro | Ámbito | Tipo | Valor por defecto | Función |
|---|---|---|---|---|
| `ver` | Query string de operaciones directas y elementos XML/JSON | Entero | `"0"` | Declara la versión mínima del protocolo requerida por la operación. |
| `v` | Query string de apertura de socket y WebSocket (`afirma://service`, `afirma://websocket`) | Entero | `1` | Declara la versión del protocolo del canal de transporte local. |
| `mcv` | Query string común de operaciones y payload XML/JSON | Cadena (p. ej. `"1.8.0"`) | `null` (no exigido) | Exige una versión mínima de la aplicación AutoFirma (*Minimum Client Version*). |
| `jvc` | Query string de arranque de socket, WebSocket y URL de comprobación | Entero | `1` | Código de versión del JavaScript cliente (*JavaScript Version Code*). |

### 2.1 Parámetro `ver` (Versión en operaciones)

En todas las operaciones de tratamiento de datos (`sign`, `cosign`, `countersign`,
`signandsave`, `save`, `load`, `selectcert`, `batch`), el parser de parámetros
busca la clave `ver` (`VER_PARAM`):
* `UrlParametersToSign.java:38, 234-239`
* `UrlParametersToSignAndSave.java:38, 229-234`
* `UrlParametersToSave.java:35, 170-175`
* `UrlParametersToLoad.java:23, 156-161`
* `UrlParametersToSelectCert.java:36, 148-153`
* `UrlParametersForBatch.java:42, 214-219`

Si el parámetro `ver` no figura en el mapa de entrada, se asigna como valor por
defecto la cadena correspondiente a la versión 0:

```java
setMinimumProtocolVersion(Integer.toString(ProtocolVersion.VERSION_0.getVersion())); // "0"
```

En el despachador central `ProtocolInvocationLauncher.java`, este valor se convierte
a entero mediante el método auxiliar `parseProtocolVersion(final String version)`
(`ProtocolInvocationLauncher.java:907-915`):

```java
private static int parseProtocolVersion(final String version) {
    int protocolVersion;
    try {
        protocolVersion = Integer.parseInt(version);
    } catch (final Exception e) {
        protocolVersion = 1;
    }
    return protocolVersion;
}
```

Si la cadena no es un entero válido, `parseProtocolVersion` adopta `1`. Si la cadena
es `"0"` (el valor por defecto asignado ante ausencia de `ver`), `Integer.parseInt("0")`
devuelve `0`.

### 2.2 Parámetro `v` (Versión en canales Socket y WebSocket)

En la apertura de canales de escucha local (`afirma://service` y `afirma://websocket`),
el parámetro de versión se denomina `v` (`PROTOCOL_VERSION_PARAM = "v"`,
`ProtocolInvocationLauncher.java:75`).

El valor se extrae mediante `getVersion(final Map<String, String> params)`
(`ProtocolInvocationLauncher.java:923-939`):

```java
private static int getVersion(final Map<String, String> params) {
    int protocolVersion = 1;
    final String protocolId = params.get(PROTOCOL_VERSION_PARAM);
    if (protocolId != null) {
        try {
            protocolVersion = Integer.parseInt(protocolId.trim());
        } catch (final Exception e) {
            LOGGER.info("El ID de protocolo indicado no es un numero entero (" + protocolId + "): " + e);
        }
    }
    return protocolVersion;
}
```

Si `v` no está presente o no es un entero analizable, se adopta `1`.

### 2.3 Fijación de versión entre transporte y operaciones: Desacoplamiento

El despachador `ProtocolInvocationLauncher.launch` admite dos signaturas:
1. `launch(final String urlString)` (`136-138`): invocación directa, que llama a
   `launch(urlString, -1, false)`.
2. `launch(final String urlString, final int protocolVersion, final boolean bySocket)`
   (`153-844`): invocación parametrizada, donde `protocolVersion` indica la versión
   previamente negociada por el canal de transporte.

En la línea 189 de `ProtocolInvocationLauncher.java`, el campo estático
`requestedProtocolVersion` se inicializa con el argumento recibido:

```java
requestedProtocolVersion = protocolVersion;
```

Esto introduce dos regímenes de resolución completamente distintos:

#### Régimen A: Invocación directa (servidor intermedio o línea de comandos)
`protocolVersion` entra con valor `-1`. Al alcanzar el bloque de la operación
solicitada (p. ej. `afirma://sign?`), se ejecuta:

```java
if (requestedProtocolVersion == -1) {
    requestedProtocolVersion = parseProtocolVersion(params.getMinimumProtocolVersion());
}
```
(`ProtocolInvocationLauncher.java:300-302, 377-379, 450-452, 539-541, 653-655, 760-762`).
La versión de protocolo se toma del parámetro `ver` de la operación analizada.

#### Régimen B: Invocación a través de Socket o WebSocket
1. Al abrir el canal (`afirma://service?...` o `afirma://websocket?...`), la versión
   se lee del parámetro `v` (`ProtocolInvocationLauncher.java:228, 267`).
2. El hilo procesador (`CommandProcessorThread` en socket, `AfirmaWebSocketServer[V4]`
   en WebSocket) almacena esa versión en su campo `this.protocolVersion`.
3. Cuando el navegador envía una orden (p. ej. `cmd=afirma://sign?...`), el hilo
   ejecuta:
   `ProtocolInvocationLauncher.launch(cmdUri.toString(), this.protocolVersion, true)`
   (`CommandProcessorThread.java:293, 308, 334, 350`, `AfirmaWebSocketServer.java:113`).
4. Al entrar en `launch()`, `requestedProtocolVersion` se asigna a `this.protocolVersion`
   (que ya no es `-1`, sino `1`, `2`, `3` o `4`).
5. Cuando el despachador llega a la operación, la condición `if (requestedProtocolVersion == -1)`
   es **falsa**.

> [!IMPORTANT]
> En la comunicación por socket o WebSocket, el parámetro `ver` incluido dentro de la
> URI del comando (`cmd=afirma://sign?...&ver=X`) es **completamente ignorado**.
> La versión de ejecución de la operación queda irremediablemente ligada al parámetro `v`
> transmitido en el apretón de manos inicial del canal.

### 2.4 Parámetro `jvc` (JavaScript Version Code)

El parámetro `jvc` identifica la revisión técnica del script cliente JavaScript
(`afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js`).
En `ProtocolInvocationLauncher.java:64-78`:

```java
private static final int MIN_JAVASCRIPT_VERSION_CODE_NEEDED = 1;
private static final int DEFAULT_JAVASCRIPT_VERSION_CODE = 1;
private static final String JAVASCRIPT_VERSION_CODE_PARAM = "jvc";
```

En las líneas 196-214, `launch()` extrae `jvc`. Si el parámetro falta o no es numérico,
asigna `DEFAULT_JAVASCRIPT_VERSION_CODE` (`1`). A continuación comprueba:

```java
if (jvc < MIN_JAVASCRIPT_VERSION_CODE_NEEDED) {
    JOptionPane.showMessageDialog(
            null,
            ProtocolMessages.getString("ProtocolLauncher.51"),
            ProtocolMessages.getString("ProtocolLauncher.52"),
            JOptionPane.WARNING_MESSAGE);
}
```

* **Mensaje (`ProtocolLauncher.51`):** *"Se ha identificado una versión no segura o
  con errores del JavaScript de despliegue de Autofirma. Solicite al responsable
  de la aplicación que actualice a la última versión."*
* **Título (`ProtocolLauncher.52`):** *"Versión de JavaScript no segura"*

Esta comprobación es meramente informativa (muestra un diálogo modal Swing de tipo
`WARNING_MESSAGE`), pero **no interrumpe ni cancela** la ejecución de la petición.

El script oficial `autoscript.js` define en su cabecera:
```javascript
var VERSION_CODE = 3;
```
(`autoscript.js:27`) y lo concatena sistemáticamente en las peticiones de apertura de
WebSocket (`2155`), Socket (`2932`) y URL de comprobación (`4362`).

### 2.5 Parámetro `mcv` (Minimum Client Version)

El parámetro `mcv` permite a una sede electrónica exigir que el usuario disponga de
una versión de AutoFirma igual o superior a una determinada cadena de versión
(p. ej. `mcv=1.8.0` o `mcv=1.9.2`).

* Definido en `UrlParameters.java:73`:
  `protected static final String MINIMUM_CLIENT_VERSION_PARAM = "mcv";`
* Definido en `ProtocolInvocationLauncher.java:81`:
  `static final String MIN_REQUESTED_VERSION_PARAM = "mcv";`
* Analizado en `UrlParameters.setCommonParameters(params)` (`UrlParameters.java:260-262`),
  método compartido por todas las operaciones a través de `ProtocolInvocationUriParserUtil`
  (`ProtocolInvocationUriParserUtil.java:50, 67, 81, 143, 159, 174`).
* Comprobado al inicio del procesamiento de cada operación:
  * `ProtocolInvocationLauncherSign.java:143-150`
  * `ProtocolInvocationLauncherSignAndSave.java:140-147`
  * `ProtocolInvocationLauncherBatch.java:90-100`
  * `ProtocolInvocationLauncherSelectCert.java:89-100`
  * `ProtocolInvocationLauncherSave.java:62-73`
  * `ProtocolInvocationLauncherLoad.java:73-84`

El mecanismo de comprobación compara la versión solicitada contra la versión
reportada por la propia aplicación:

```java
if (options.getMinimumClientVersion() != null) {
    final String minimumRequestedVersion = options.getMinimumClientVersion();
    final Version requestedVersion = new Version(minimumRequestedVersion);
    if (requestedVersion.greaterThan(SimpleAfirma.getVersion())) {
        final String errorCode = ProtocolInvocationLauncherErrorManager.ERROR_MINIMUM_VERSION_NON_SATISTIED;
        throw new SocketOperationException(errorCode);
    }
}
```

El error `ERROR_MINIMUM_VERSION_NON_SATISTIED` corresponde al código `SAF_41`
(`ProtocolInvocationLauncherErrorManager.java:72`), cuyo texto asociado es:
*"El uso de este trámite web requiere una versión más reciente de Autofirma.
Actualice a la última versión disponible."* (`protocolmessages.properties:53`).

---

## 3. Requisitos de versión por canal de transporte

Cada mecanismo de comunicación impone restricciones estrictas sobre qué valores
de protocolo tolera.

### 3.1 Canal WebSocket (`afirma://websocket`)

El gestor `AfirmaWebSocketServerManager`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/AfirmaWebSocketServerManager.java`)
restringe las versiones válidas a un conjunto cerrado:

```java
private static final int PROTOCOL_VERSION_3 = 3;
private static final int PROTOCOL_VERSION_4 = 4;
private static final int CURRENT_PROTOCOL_VERSION = PROTOCOL_VERSION_4;
private static final int[] SUPPORTED_PROTOCOL_VERSIONS = new int[] { PROTOCOL_VERSION_3, PROTOCOL_VERSION_4 };
```
(`AfirmaWebSocketServerManager.java:27-36`).

En `startService()` (`52-56`), se invoca `checkSupportProtocol(requestedProtocolVersion)`:

```java
private static void checkSupportProtocol(final int version) throws UnsupportedProtocolException {
    for (final int supportedVersion : SUPPORTED_PROTOCOL_VERSIONS) {
        if (supportedVersion == version) {
            return;
        }
    }
    throw new UnsupportedProtocolException(version, version > CURRENT_PROTOCOL_VERSION);
}
```
(`100-107`).

#### Comportamiento ante versión no soportada:
Si se solicita una versión distinta de 3 o 4 (p. ej. `v=1`, `v=2` o si se omite `v`,
que por defecto devuelve `1`), `checkSupportProtocol` lanza `UnsupportedProtocolException`.
En `ProtocolInvocationLauncher.java:240-245`, la excepción se captura, se muestra el
error `SAF_21` en la interfaz gráfica y se fuerza el cierre fulminante del proceso:

```java
ProtocolInvocationLauncherErrorManager.showError(errorCode, e); // SAF_21
forceCloseApplication(0);
```

#### Diferencias entre versión 3 y versión 4 en WebSocket:
* **Versión 3:** Instancia `AfirmaWebSocketServer` (`AfirmaWebSocketServerManager.java:75`).
  No valida identificador de sesión en la conexión WebSocket. Si no se especifica
  el parámetro `ports` en la URI, recurre al puerto fijo por defecto `63117`
  (`ProtocolInvocationLauncher.java:87, 233-236`).
* **Versión 4:** Instancia `AfirmaWebSocketServerV4` (`AfirmaWebSocketServerManager.java:71`).
  Exige que el cliente WebSocket incluya el parámetro `idsession` en la query string
  del handshake HTTP de WebSocket (`ws.getResourceDescriptor()`, `AfirmaWebSocketServerV4.java:49-74`).
  Si el `idsession` no coincide con el negociado en la URI de lanzamiento, rechaza la
  conexión con código de cierre WebSocket `1008` (Violación de política,
  `AfirmaWebSocketServerV4.java:70`). Permite que múltiples pestañas del navegador
  se conecten simultáneamente siempre que compartan el mismo `idsession`.

### 3.2 Canal Socket local HTTP (`afirma://service`)

El gestor `ServiceInvocationManager`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ServiceInvocationManager.java`)
soporta las versiones 1, 2 y 3:

```java
private static final int CURRENT_PROTOCOL_VERSION = 3;
private static final int[] SUPPORTED_PROTOCOL_VERSIONS = new int[] { 1, 2, CURRENT_PROTOCOL_VERSION };
```
(`ServiceInvocationManager.java:42-45`).

La comprobación en `checkSupportProtocol(protocolVersion)` (`212-220`) valida que
el número coincida exactamente con uno de los elementos de `SUPPORTED_PROTOCOL_VERSIONS`.

#### Comportamiento ante versión no soportada:
Si se solicita una versión que no sea 1, 2 o 3 (p. ej. `v=0` o `v=4`), lanza
`UnsupportedProtocolException`. En `ProtocolInvocationLauncher.java:281-288`, se captura,
se muestra el error `SAF_21` en pantalla y se devuelve la cadena de error `SAF_21`.
A diferencia de WebSocket, la versión 4 **no está soportada** en socket local HTTP.

### 3.3 Canal por Servidor Intermedio

El transporte por servidor intermedio no utiliza una URI de apertura de servicio,
sino que invoca directamente la operación (`afirma://sign?`, `afirma://batch?`, etc.).

* Admite cualquier versión `protocolVersion <= 4` (`MAX_PROTOCOL_VERSION_SUPPORTED.support()`).
* Si `ver` se omite en la URI, se asume `ProtocolVersion.VERSION_0` (entero `0`).
* Si se proporciona una versión superior a 4 (p. ej. `ver=5`), arroja `SAF_21`.

---

## 4. Requisitos de versión por operación

La siguiente tabla resume el soporte de versión en cada una de las operaciones
admitidas por el protocolo:

| Operación | Rango de protocolo admitido | Impacto funcional de `protocolVersion` | Validación de `mcv` | Código de error por versión |
|---|---|---|---|---|
| `sign` / `cosign` / `countersign` | `0 .. 4` | Si `protocolVersion >= 3`, la respuesta incluye un tercer bloque con `extraData` en JSON (`NativeSignDataProcessor.java:77, 97`). Si `< 3`, solo devuelve certificado y firma. | Sí (`ProtocolInvocationLauncherSign.java:143`) | Protocolo: `SAF_21`. Cliente: `SAF_41`. |
| `signandsave` | `0 .. 4` | Idéntico a `sign`: delega el proceso de firma en `processSign()` con el mismo procesador de datos (`ProtocolInvocationLauncherSignAndSave.java:582`). | Sí (`ProtocolInvocationLauncherSignAndSave.java:140`) | Protocolo: `SAF_21`. Cliente: `SAF_41`. |
| `batch` | `0 .. 4` | Valida cota máxima; transfiere la versión al motor de lotes (`ProtocolInvocationLauncherBatch.java:77, 345`). | Sí (`ProtocolInvocationLauncherBatch.java:90`) | Protocolo: `SAF_21`. Cliente: `SAF_41`. |
| `selectcert` | `0 .. 4` | Valida cota máxima; no altera el formato de salida según la versión (`ProtocolInvocationLauncherSelectCert.java:75`). | Sí (`ProtocolInvocationLauncherSelectCert.java:89`) | Protocolo: `SAF_21`. Cliente: `SAF_41`. |
| `save` | `0 .. 4` | Valida cota máxima; no altera el formato de salida según la versión (`ProtocolInvocationLauncherSave.java:49`). | Sí (`ProtocolInvocationLauncherSave.java:62`) | Protocolo: `SAF_21`. Cliente: `SAF_41`. |
| `load` | `0 .. 4` | Valida cota máxima; no altera el formato de salida según la versión (`ProtocolInvocationLauncherLoad.java:60`). | Sí (`ProtocolInvocationLauncherLoad.java:73`) | Protocolo: `SAF_21`. Cliente: `SAF_41`. |

---

## 5. Algoritmo de comparación de versión de la aplicación (`Version.java`)

El análisis y comparación de versiones semánticas solicitadas mediante el parámetro
`mcv` se implementa en la clase `Version`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/Version.java:9-206`).

### 5.1 Obtención de la versión de AutoFirma

La versión actual de la aplicación se recupera mediante `SimpleAfirma.getVersion()`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/SimpleAfirma.java:1237-1242`),
que a su vez consulta `Updater.getCurrentVersionText()`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/updater/Updater.java:101-103`).

Este valor se lee del fichero de propiedades `updater.properties`
(`afirma-simple/src/main/resources/properties/updater.properties:2-4`):
```properties
currentVersionText.WINDOWS=1.9.2
currentVersionText.LINUX=1.9.2
currentVersionText.MACOSX=1.9.2
```
Si la propiedad no existe para el sistema operativo en curso, se devuelve `"0"`.

### 5.2 Estructura interna de `Version`

Una instancia de `Version` descompone una cadena de texto en dos componentes
(`Version.java:11-13`):
* `List<Integer> versionParts`: lista ordenada de componentes numéricos enteros
  separados por puntos.
* `String aditionalText`: texto alfanumérico o etiquetas adicionales adjuntas al
  último componente (p. ej. en `"1.7 RC1"`, la última parte numérica es `7` y el
  texto adicional es `" RC1"`).

### 5.3 Algoritmo del constructor `Version(final String legibleVersion)`

Implementado en `Version.java:21-56`:

1. **Caso nulo:** Si `legibleVersion == null`, asigna una única parte entera `0` y
   `aditionalText = ""` (`25-29`).
2. **División por puntos:** Se divide la cadena con la expresión regular `\\.`:
   `final String[] parts = legibleVersion.split("\\.");` (`32`).
3. **Partes intermedias:** Para todas las partes excepto la última (`0 .. parts.length - 2`),
   se convierten directamente a entero: `this.versionParts.add(Integer.valueOf(part));` (`35-38`).
   Si alguna contiene caracteres no numéricos, se lanza `NumberFormatException`.
4. **Última parte:** Se recorre carácter a carácter buscando el primer índice `limit`
   que no sea un dígito (`!Character.isDigit(lastPart.charAt(i))`) (`41-47`).
   * Si `limit > 0`: la subcadena `0..limit` se convierte a entero y se añade a
     `versionParts`, mientras que la subcadena a partir de `limit` se almacena en
     `this.aditionalText` (`49-52`).
   * Si `limit <= 0`: toda la última parte se convierte a entero y `aditionalText`
     queda vacío (`52-55`). Si la última parte comenzaba directamente por un carácter
     no numérico (`limit == 0`, p. ej. `"1.7.beta"`), el bloque ejecuta
     `Integer.valueOf(lastPart)` y lanza `NumberFormatException`.

### 5.4 Algoritmo de comparación `greaterThan(final Version version)`

Implementado en `Version.java:120-169`, la comparación sigue seis reglas estrictas:

```
                  ┌───────────────────────────────────────────────┐
                  │ Comparar versionParts posición a posición     │
                  │ de izquierda a derecha                        │
                  └───────────────────────┬───────────────────────┘
                                          │
                        ¿Alguna parte es distinta?
                       /                          \
                     SÍ                            NO
                    /                                \
     ┌────────────────────────────┐    ┌─────────────────────────────────────────┐
     │ Devuelve (thisPart > other)│    │ ¿Alguna versión tiene más partes?       │
     └────────────────────────────┘    └────────────────────┬────────────────────┘
                                                            │
                                              ¿Mismo número de partes?
                                             /                        \
                                           NO                          SÍ
                                          /                              \
                           ┌────────────────────────────┐    ┌───────────────────────────┐
                           │ Devuelve                   │    │ Comparar aditionalText    │
                           │ (this.size > other.size)   │    │ según reglas de sufijo    │
                           └────────────────────────────┘    └───────────────────────────┘
```

1. **Comparación numérica por componentes:** Se recorren las partes enteras de
   izquierda a derecha mientras ambas versiones contengan elementos en ese índice
   (`124-133`). En cuanto un número difiere, se determina el resultado:
   * Si `thisPart > anotherPart`: devuelve `true`.
   * Si `thisPart < anotherPart`: devuelve `false`.
2. **Longitud de componentes:** Si todas las partes comunes son numéricamente iguales
   pero una versión tiene más componentes que la otra, **la versión con más componentes
   es mayor** (`137-141`):
   * `"1.7.0.1" > "1.7.0"` -> `true`.
   * `"1.7.0.0" > "1.7.0"` -> `true` (la presencia de ceros extra cuenta como versión mayor).
3. **Igualdad de sufijo adicional:** Si ambas versiones tienen idénticos componentes
   numéricos y sus cadenas `aditionalText` son iguales ignorando mayúsculas/minúsculas
   (`equalsIgnoreCase`), se consideran iguales y devuelve `false` (`146-148`).
4. **Sufijo que empieza por espacio (Versiones preliminares / Release Candidates):**
   Una cadena de texto adicional que empieza por espacio (p. ej. `" RC1"`) se considera
   menor que una versión final vacía o que un sufijo que no empiece por espacio:
   * Si `this.aditionalText` no empieza por espacio (o está vacío) y `other` sí empieza
     por espacio: `this` es mayor -> devuelve `true` (`149-152`). P. ej., `"1.7" > "1.7 RC1"`
     y `"1.7a" > "1.7 RC1"`.
   * Si `this.aditionalText` empieza por espacio y `other` no empieza por espacio (o está
     vacío): `this` es menor -> devuelve `false` (`153-156`).
5. **Sufijo vacío frente a sufijo no espaciado:**
   * Si `this.aditionalText` está vacío y `other` tiene contenido que no empieza por
     espacio: `other` es mayor, por lo que `this` no es mayor -> devuelve `false` (`157-160`).
     P. ej., `"1.7" > "1.7a"` es `false`.
   * Si `other` está vacío y `this` tiene contenido que no empieza por espacio: `this`
     es mayor -> devuelve `true` (`161-164`). P. ej., `"1.7a" > "1.7"` es `true`.
6. **Comparación lexicográfica binaria:** En cualquier otro caso, se comparan ambas
   cadenas en minúsculas lexicográficamente (`165-168`):
   `this.aditionalText.toLowerCase().compareTo(version.getAditionalText().toLowerCase()) > 0`.
   P. ej., `"1.7b" > "1.7a"`, `"1.7a2" > "1.7a1"`, mientras que `"1.7A"` y `"1.7a"` son iguales.

---

## 6. Perspectiva del cliente JavaScript (`autoscript.js`)

El cliente oficial de referencia `autoscript.js`
(`afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js`)
implementa la negociación y envío de versiones de forma diferenciada según el
transporte elegido.

### 6.1 Constantes globales de versión
```javascript
var VERSION = "1.9.0";
var VERSION_CODE = 3;
```
(`autoscript.js:26-27`).
* `VERSION`: Cadena expuesta públicamente en la API JavaScript como `AutoScript.VERSION` (`4893`).
* `VERSION_CODE`: Código entero transmitido en el parámetro `jvc`.

### 6.2 Versiones de protocolo por transporte en `autoscript.js`

| Transporte en `autoscript.js` | Objeto JavaScript | Constante interna `PROTOCOL_VERSION` | Parámetro transmitido | Líneas de cita |
|---|---|---|---|---|
| WebSocket seguro | `AppAfirmaWebSocketClient` | `4` | `v=4`, `jvc=3` | `autoscript.js:1747, 2154-2155` |
| Socket HTTP local | `AppAfirmaSocketClient` | `1` | `v=1`, `jvc=3` | `autoscript.js:2621, 2931-2932` |
| Servidor intermedio | `AppAfirmaIntentClient` | `3` | `ver=3`, `jvc=3` | `autoscript.js:3715, 3785, 4362` |

### 6.3 Configuración de `mcv` desde la página web
La página web puede invocar:
```javascript
AutoScript.setMinimumClientVersion("1.9.2");
```
(`autoscript.js:336-338`). Esto asigna la variable global `minimumClientVersion` (`165`),
que posteriormente se inyecta en:
* WebSocket: `params.push(createKeyValuePair("mcv", minimumClientVersion));` (`2072`).
* Socket: `params.push(generateDataKeyValue("mcv", minimumClientVersion));` (`2870`).
* Servidor intermedio en XML: `params[params.length] = { key:"mcv", value:minimumClientVersion };` (`4310`).
* URL de verificación de instalación: `urlParams += "&mcv=" + encodeURIComponent(minimumClientVersion);` (`4364`).

---

## 7. Comprobación de actualizaciones automáticas (`Updater`)

Durante el arranque en modo escritorio o por protocolo, `SimpleAfirma.main` evalúa si
deben buscarse actualizaciones (`SimpleAfirma.java:948-953`):

```java
if (updatesEnabled && PreferencesManager.getBoolean(PreferencesManager.PREFERENCE_GENERAL_UPDATECHECK)) {
    Updater.checkForUpdates(null);
}
```

La comprobación se puede omitir configurando la propiedad del sistema Java
`-Des.gob.afirma.avoidUpdateCheck=true` o la variable de entorno
`NO_UPDATE_CHECK=true` (`SimpleAfirma.java:87, 89, 940-941`).

En `Updater.java:152-171`, la comparación de actualización **no utiliza** la clase
`Version` ni la versión de protocolo, sino un número entero interno denominado
`currentVersion` (`updater.properties:9-11`, p. ej. `currentVersion.LINUX=21`).
Descarga por HTTP un fichero remoto con el nuevo entero y evalúa:

```java
Integer.parseInt(newVersion) > Integer.parseInt(getCurrentVersion())
```
(`Updater.java:163`).

---

## Lo que el código no aclara

**Fallo crítico de actualización de versión en servidor intermedio.**
Cuando una petición hacia el servidor intermedio supera la longitud máxima de URL
(`isURLTooLong()`, `autoscript.js:3803`), los parámetros completos se suben al
servlet `StorageService` en un XML cifrado y en la URL de invocación solo se envían
`fileid`, `rtservlet` y `key` (`autoscript.js:4413-4424`). La función `buildUrlWithoutData`
**no incluye el parámetro `ver`** en la query string de la URI.
En `ProtocolInvocationLauncher.java:653-655`:
```java
if (requestedProtocolVersion == -1) {
    requestedProtocolVersion = parseProtocolVersion(params.getMinimumProtocolVersion());
}
```
Dado que la URL no contiene `ver`, `params.getMinimumProtocolVersion()` devuelve `"0"`
y `requestedProtocolVersion` se fija en `0`. Posteriormente (`660-679`), se descarga el
XML desde el servlet y se reasigna `params` (`params = ProtocolInvocationUriParser.getParametersToSign(xmlData, true)`),
el cual sí contiene `minimumProtocolVersion = "3"`. Sin embargo, `requestedProtocolVersion`
**nunca se recalcula ni se actualiza** tras parsear el XML.
Como consecuencia, la firma se procesa con `requestedProtocolVersion = 0`. Al llegar
a `NativeSignDataProcessor.java:77, 97`, la comprobación `getProtocolVersion() >= 3`
evalúa `0 >= 3` (**falso**), provocando que los metadatos `extraData` (nombre del
fichero) **no se devuelvan nunca** cuando los datos viajan por servidor intermedio
subidos previamente por exceso de tamaño.

**Incoherencia de nombre entre `ver` y `v`.**
En canales de socket y WebSocket, el parámetro de versión de la URI se llama `v`
(`ProtocolInvocationLauncher.java:75`). En las operaciones directas por URI o en
servidor intermedio, se llama `ver` (`UrlParametersToSign.java:38`). Si un cliente
externo envía `afirma://sign?v=3...`, el parser de firma busca `ver`, no lo encuentra,
adopta `"0"` por defecto e ignora `v=3` en silencio.

**Parámetro `ver` en comandos de socket es código inoperante.**
Cuando se utiliza el transporte por socket local o WebSocket, la versión de protocolo
queda irrevocablemente congelada en la variable `this.protocolVersion` del hilo
durante el apretón de manos inicial de `afirma://service?v=...` o `afirma://websocket?v=...`.
Cualquier parámetro `ver` o `v` que el cliente web envíe dentro del comando
`cmd=afirma://sign?...&ver=X` es completamente ignorado, pues `requestedProtocolVersion`
ya no vale `-1` y la asignación condicional no se ejecuta (`ProtocolInvocationLauncher.java:653`).

**`ParameterNeedsUpdatedVersionException` es código muerto.**
La excepción `ParameterNeedsUpdatedVersionException`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ParameterNeedsUpdatedVersionException.java:14`)
posee un constructor con visibilidad de paquete (`18`) y no es instanciada ni lanzada
por ninguna clase de toda la base de código. Se trata de un vestigio de la versión 1.4
(año 2014) que permanece en los bloques `catch` de `ProtocolInvocationLauncher.java`
(`504, 616, 726, 811`) sin posibilidad alguna de ejecutarse. Las verificaciones
de versión insuficiente lanzan en su lugar `SocketOperationException` con código `SAF_41`.

**`Updater.isOldVersion()` es código muerto.**
El método `Updater.isOldVersion(final String neededVersion)` (`Updater.java:177-187`),
que comparaba cadenas contra el entero de compilación `currentVersion`, no es llamado
desde ningún punto del proyecto.

**La verificación de `jvc` es inalcanzable con parámetros ausentes.**
Tanto `DEFAULT_JAVASCRIPT_VERSION_CODE` como `MIN_JAVASCRIPT_VERSION_CODE_NEEDED`
valen exactamente `1` (`ProtocolInvocationLauncher.java:64, 66`). Cuando `jvc` falta o
no es numérico, se captura la excepción y se asigna el valor por defecto `1` (`202`).
Por ende, la condición `jvc < 1` únicamente puede dispararse si el invocador envía
de forma expresa un número menor o igual que cero (`jvc=0` o `jvc=-1`).

**`ProtocolVersion.support()` carece de cota inferior.**
El método `support(final int protocolVersion)` (`ProtocolVersion.java:62-64`) evalúa
`this.version >= protocolVersion`. Al llamarse sobre `MAX_PROTOCOL_VERSION_SUPPORTED` (`4`),
cualquier entero negativo (p. ej. `-5` o `-99`) es evaluado como soportado (`4 >= -5` es `true`).
La cota inferior solo se comprueba de forma explícita en `ServiceInvocationManager` (`{1, 2, 3}`)
y en `AfirmaWebSocketServerManager` (`{3, 4}`). En operaciones directas por servidor
intermedio, no existe validación de cota inferior.

**Asimetría de versiones entre Socket HTTP y WebSocket.**
`ServiceInvocationManager` soporta las versiones 1, 2 y 3 pero rechaza la 4 (`SAF_21`).
En contrapartida, `AfirmaWebSocketServerManager` solo soporta las versiones 3 y 4,
rechazando las versiones 1 y 2 (`SAF_21`). No existe un canal de transporte local
que admita de manera unificada todas las versiones del protocolo.

**Desalineación de versión de despliegue en `autoscript.js`.**
En la distribución de AutoFirma 1.9.2 (tag `v1.9.2`), el archivo `autoscript.js` declara
en su cabecera `var VERSION = "1.9.0";` (`autoscript.js:26`), a pesar de que el aplicativo
empaquetado reporta `1.9.2` en `updater.properties`.

**Errata de formato en mensajes de log de error.**
En `ProtocolInvocationLauncherBatch.java:78`, `ProtocolInvocationLauncherSelectCert.java:77`,
`ProtocolInvocationLauncherSave.java:50` y `ProtocolInvocationLauncherLoad.java:61`,
el mensaje de error contiene un error tipográfico en el especificador de formato:
`"Version de protocolo no soportada (%1s). Version actual: %s2. Hay que actualizar la aplicacion."`.
El literal `%s2` no es un especificador posicional válido de `String.format` (debería ser
`%2$s` o `%2s`), por lo que la versión máxima soportada no se imprime correctamente en
las trazas de registro de esas cuatro operaciones.
