# El contrato del protocolo `afirma://` en la versión publicada, la 1.9.2

**Medido contra el tag `v1.9.2` de [`ctt-gob-es/clienteafirma`](https://github.com/ctt-gob-es/clienteafirma),
commit `b4fe147c322932ebdd11e25db3134af934e0e832`**, que es lo que las sedes ejecutan hoy. En las
citas se abrevia la raíz de ese clon como **`<clienteafirma>`** y la de este repositorio como
**`<rfirma>`**. Toda línea citada es del tag; `master` **no** es fuente de nada de lo que sigue.

> **Por qué se rehízo.** La versión anterior de este informe se midió contra `master`, 219 commits
> por delante del último *release* y sin publicar. Se descubrió en el
> [#318](https://github.com/sgomez/rfirma/issues/318) y se rehizo en el
> [#329](https://github.com/sgomez/rfirma/issues/329). Lo que `master` tiene y la 1.9.2 **no**:
> el protocolo 4.1, el par `#wait`/`getresult?` por WebSocket, los códigos de error `AF…`, el
> parámetro `servicetimeout` y el parámetro de arranque `dlgload`. La 1.9.2 lleva `autoscript.js`
> **1.9.0** con `VERSION_CODE = 3` y el servidor `AfirmaWebSocketServerV4`.

Sondeo original del [#224](https://github.com/sgomez/rfirma/issues/224); rehecho para el mapa
[#308](https://github.com/sgomez/rfirma/issues/308).

**Respuesta corta.** El navegador moderno **no** usa `afirma://sign`. Usa `afirma://websocket?…`
sólo para **arrancar** la aplicación, y a partir de ahí todo viaja por
**`wss://127.0.0.1:<puerto>`**, en mensajes de texto plano que son URL `afirma://…` enviadas por
el socket. El alcance mínimo de rfirma son cuatro mensajes —eco, `selectcert`, `sign` y `cosign`—,
**un formato de error de una línea que empieza por `SAF_`** y un idioma de filtros de certificado
que rfirma **hoy no tiene en absoluto**. El intercambio es **estrictamente síncrono**: petición,
respuesta, sin sondeo intermedio y **sin ningún temporizador en el cliente**. Todo lo demás
(servlets, cifrado, `batch`, `save`, `load`, `countersign`) es prescindible sin romper a ninguna
sede que hable v4.

---

## 1. El arranque: `afirma://websocket`, no `afirma://sign`

El transporte vigente es el bloque `AppAfirmaWebSocketClient` de
`<clienteafirma>/afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js` (constantes del
transporte en `:1747`-`1751`). La única URL de esquema `afirma://` que llega al sistema operativo
es la de arranque, construida en `openNativeApp` (`autoscript.js:2138`-`2158`):

```
afirma://websocket?ports=<p1,p2,p3>&v=4&jvc=3&idsession=<20 caracteres>
```

— `autoscript.js:2153`-`2157`. El `v` es `PROTOCOL_VERSION = 4` (`autoscript.js:1747`) y el `jvc`
es `VERSION_CODE = 3` (`autoscript.js:27`, junto a `VERSION = "1.9.0"` en `:26`).

> **No hay `dlgload`.** El parámetro no existe en la 1.9.2: `grep dlgload` da cero ocurrencias
> tanto en `autoscript.js` como en `ProtocolInvocationLauncher.java`. Son **exactamente cuatro**
> parámetros de arranque.

Esa URL se entrega al sistema con `document.location = url` en Chrome, iOS y Firefox de Android, y
con un iframe oculto de 1×1 en los demás (`openUrl`, `autoscript.js:793`-`838`;
`openUrlWithIframe`, `:843`-`870`). Es decir: **la única función del esquema `afirma://` es lanzar
el proceso**. Ninguna operación real viaja por él.

A partir de ahí el cliente abre `wss://127.0.0.1:<puerto>` (`URL_REQUEST_PREFIX`,
`autoscript.js:1751`; `new WebSocket(...)` en `:2201`) y **envía la URL de la operación como cuerpo
del mensaje del socket**: `ws.send(currentOperationUrl)` (`autoscript.js:2267`), donde
`currentOperationUrl` es la cadena `afirma://sign?op=sign&idsession=…&algorithm=…` construida por
`buildUrl` (`autoscript.js:2065`-`2090`). Del lado de AutoFirma ese mismo texto entra en
`ProtocolInvocationLauncher.launch(message, 4, true)`
(`<clienteafirma>/afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/AfirmaWebSocketServerV4.java:91`),
que **exige que empiece por `afirma://`** y nada más (`ProtocolInvocationLauncher.java:172`-`178`).

> No hay ningún otro prefijo aceptado. En particular, `getresult?` **no existe en la 1.9.2**
> (§3.4).

> El propio código de AutoFirma reconoce que esto es una deuda: «*La comunicación por
> sockets/websockets no debería utilizar URLs*» (`ProtocolInvocationLauncher.java:216`-`222`).
> rfirma tiene que hablarlo igual, pero conviene saber que es un accidente histórico y no un
> diseño.

### Cómo se eligen los puertos

`AfirmaUtils.getRandomPorts` (`autoscript.js:1653`-`1676`):

- Rango válido absoluto: `MIN_PORT = 1024`, `MAX_PORT = 65535` (`autoscript.js:1603`, `1609`).
- Rango por defecto: `DEFAULT_MIN_PORT = 49152` .. `MAX_PORT`, es decir el rango efímero de IANA
  (`autoscript.js:1606`, `1658`-`1659`).
- Se sortean **tres** puertos distintos del rango (`autoscript.js:1662`-`1675`), con `Math.random`
  y un antibucle: si el rango es más estrecho que el número de aleatorios pedidos,
  `getUniqueRandom` devuelve `0` para no colgarse (`autoscript.js:1686`-`1688`).
- La sede puede estrechar el rango con **`setPortRange`** (`autoscript.js:771`-`787`; expuesta en
  la API pública), donde se recorta contra `MIN_PORT`/`MAX_PORT`. *(La versión anterior de este
  informe la llamaba `setServerRange`: ese nombre no existe en el tag.)*

Los tres van en `ports=` separados por comas (`autoscript.js:2144`-`2150`). AutoFirma los parsea en
`getChannelInfo` (`ProtocolInvocationLauncher.java:973`-`1009`; el `split(",")` en `:975`-`990`,
con `Math.abs` sobre cada valor y `IllegalArgumentException` si alguno no es numérico) y los guarda
en un `ChannelInfo` (`ChannelInfo.java:7`-`45`). Quien recorre la lista es
`AfirmaWebSocketServerManager.startService`, con un `do…while` que prueba puerto a puerto hasta que
uno abre (`AfirmaWebSocketServerManager.java:63`-`89`).

> `ChannelInfo` en el tag es un contenedor pelado: **no tiene `nextPortAvailable()`**. Esa API es
> de `master`.

Si `ports` **no** viene, AutoFirma asume protocolo v3 y usa el puerto fijo
`DEFAULT_WEBSOCKET_PORT = 63117` (`ProtocolInvocationLauncher.java:87`, `233`-`236`).

### Qué es `idsession`

Veinte caracteres alfanuméricos sorteados con `window.crypto.getRandomValues` cuando existe, y con
un PRNG casero cuando no (`generateNewIdSession`, `autoscript.js:1611`-`1631`; el alfabeto está en
`VALID_CHARS_TO_ID`, `autoscript.js:1600`, y **le falta la `v` minúscula** —un descuido del
original, no una decisión).

Es una **credencial de canal**, no un identificador de transacción: viaja en la URL de arranque
(`autoscript.js:2157`) y se repite en **cada** mensaje posterior, tanto en el eco
(`echo=-idsession=<id>@EOF`, `autoscript.js:2286`) como en cada petición de operación
(`data.idsession`, `autoscript.js:1944` y `:1959`).

Del lado servidor hay **tres** guardias, y en este orden
(`AfirmaWebSocketServerV4.onMessage`, `:57`-`93`):

1. **La petición tiene que venir de 127.0.0.1** o se responde `SAF_47`
   (`AfirmaWebSocketServerV4.java:61`-`68`; `isLocalAddress` en `:100`-`102`).
2. **El `idsession` del mensaje tiene que coincidir** con el del canal, o se responde `SAF_46`
   y la petición no se ejecuta (`:72`-`78`). El extractor tolera que el valor termine en `@EOF`
   (`getSessionId`, `:109`-`127`).
3. Sólo después se mira si es un eco (`:81`-`83`) o una operación (`:86`-`92`).

> **Guardia del lado del arranque que conviene copiar**: `getChannelInfo` valida que el `idsession`
> de la URL sea **sólo letras o dígitos**; si no, lo pone a `null`
> (`ProtocolInvocationLauncher.java:992`-`1008`). Un `idsession` a `null` desactiva la comprobación
> 2 entera (`AfirmaWebSocketServerV4.java:72`, `this.sessionId != null`). Es decir: en el original,
> **un `idsession` mal formado abre un canal sin credencial**, no un canal cerrado.

### Qué pasa si ningún puerto está libre

Dos mitades, y las dos hay que reproducirlas.

**En AutoFirma**: si el bucle de `startService` agota los puertos, lanza `SocketOperationException`
(`AfirmaWebSocketServerManager.java:91`-`93`); el lanzador muestra el error `SAF_45` en un diálogo
y **cierra la aplicación entera** con `forceCloseApplication(0)`
(`ProtocolInvocationLauncher.java:248`-`250`; `forceCloseApplication` en `:1022`-`1024`, que es un
`Runtime.getRuntime().halt(exitCode)`). No hay `BindingErrorListener` ni reintento de arranque:
eso es de `master`.

**En el navegador**: `waitAppAndProcessRequest` intenta conectar a los tres puertos, 15 veces
(`AUTOFIRMA_CONNECTION_RETRIES = 15`, `autoscript.js:152`) con 2 s entre intentos
(`AUTOFIRMA_LAUNCHING_TIME = 2000`, `autoscript.js:149`) y una espera inicial de 3 s
(`autoscript.js:2115`). Agotados, sortea tres puertos nuevos, **enseña un diálogo de reintento** y,
si la persona no reintenta, notifica al `errorCallback` de la sede con la excepción
`es.gob.afirma.standalone.ApplicationNotFoundException` y un mensaje de texto
(`autoscript.js:2172`-`2185`).

> **No hay códigos `AS…` en la 1.9.2.** `grep -c 'AS6200\|ErrorCode' autoscript.js` da **cero**: el
> catálogo `ErrorCode` del cliente, con `AS620017` y compañía, es de `master`. Lo que la 1.9.2 pasa
> al `errorCallback` es el par (nombre de excepción Java, texto). Cuando el socket se cierra sin
> más, lo que llega es `java.lang.InterruptedException` con «Autofirma se ha cerrado o ha cerrado
> el websocket de comunicacion» (`autoscript.js:2216`-`2223`); cuando el eco se agota,
> `java.util.concurrent.TimeoutException` (`:2274`-`2280`).

---

## 2. La negociación de versión

Hay **tres** ejes de versión y conviene no confundirlos. **Ninguno de los tres es una versión que
la aplicación anuncie**: el flujo es de una sola dirección, la sede declara y la aplicación acepta
o se cierra. No existe ningún mensaje en el que rfirma diga qué versión habla.

**a) `v` — versión de protocolo.** El JS manda `v=4` (`PROTOCOL_VERSION = 4`, declarado una vez por
transporte: `autoscript.js:1747` el WebSocket, `:2621` el de sockets HTTP en claro, `:3715` el de
servlets). AutoFirma admite **exactamente las versiones 3 y 4**:

```java
private static final int PROTOCOL_VERSION_3 = 3;
private static final int PROTOCOL_VERSION_4 = 4;
private static final int CURRENT_PROTOCOL_VERSION = PROTOCOL_VERSION_4;
private static final int[] SUPPORTED_PROTOCOL_VERSIONS = new int[] { PROTOCOL_VERSION_3, PROTOCOL_VERSION_4 };
```

— `AfirmaWebSocketServerManager.java:27`-`36`. El valor sale de `getVersion`, que **devuelve `1` si
el parámetro `v` no viene** (`ProtocolInvocationLauncher.java:923`-`944`).

**No existe la versión 4.1, y no existe forma de expresarla.** `ProtocolVersion` en el tag es un
`enum` de cinco valores enteros —`VERSION_0` … `VERSION_4`— y su `support(...)` compara enteros;
no hay forma `X.Y` ni parseo de cadenas con punto
(`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolVersion.java:8`-`19`,
`38`-`64`). Tampoco existe `reviewProtocolVersion` en el lanzador: la búsqueda no da resultados.

**Ante una versión que no conoce**, `checkSupportProtocol` lanza `UnsupportedProtocolException`
(`AfirmaWebSocketServerManager.java:100`-`107`) y el lanzador muestra `SAF_21` («La versión de
Autofirma instalada no es compatible con este trámite») **en un diálogo** y **cierra la aplicación**
(`ProtocolInvocationLauncher.java:242`-`244`). El navegador no se entera: eso ocurre al procesar la
URL de arranque, cuando todavía no hay socket por el que contestar.

**b) `jvc` — versión de código del JavaScript.** `VERSION_CODE = 3` (`autoscript.js:27`), enviado en
el arranque (`autoscript.js:2155`). En la 1.9.2 hace **una sola cosa**: si
`jvc < MIN_JAVASCRIPT_VERSION_CODE_NEEDED`, que vale **1**, se enseña un aviso modal
(`ProtocolInvocationLauncher.java:64`, `196`-`214`). Nada más. No sintetiza ninguna versión, no
cambia el formato de los errores y no habilita ningún procesado asíncrono. El valor por defecto
cuando no viene o no parsea es `DEFAULT_JAVASCRIPT_VERSION_CODE = 1` (`:66`, `:196`-`204`).

**c) `mcv` — versión mínima de la aplicación cliente.** La sede la fija con
`setMinimumClientVersion` (`autoscript.js:336`) y `buildUrl` la antepone a los parámetros de
**cada** petición (`autoscript.js:2069`-`2072`). AutoFirma la lee en `UrlParameters`
(`MINIMUM_CLIENT_VERSION_PARAM = "mcv"`,
`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java:73`,
`260`-`261`) y la compara contra la versión de la aplicación, devolviendo `SAF_41` si no se cumple.
La comprobación está en **los cuatro lanzadores** que la pueden recibir, con el mismo código
literal: `ProtocolInvocationLauncherSign.java:142`-`150`,
`ProtocolInvocationLauncherSelectCert.java:89`-`98`, `…Load.java:73`-`77` y `…Save.java:62`-`66`.

> **Corregido.** La versión anterior de este informe decía que se comprobaba «sólo en la firma» y
> que `selectcert` no la miraba. Es falso: `ProtocolInvocationLauncherSelectCert.java:89` la
> comprueba igual que la firma. Para rfirma la diferencia es que `mcv` afecta a **las dos**
> operaciones de su alcance mínimo, no a una.

**No es semver, y no se parece.** La comparación la hace una clase propia,
`<clienteafirma>/afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/Version.java`
(constructor en `:21`-`56`, `greaterThan` en `:120`-`…`), cuyas reglas contradicen semver justo
donde importa: **más partes separadas por punto es más nueva** (`1.7.0.0` > `1.7.0`), **un sufijo
de texto suma** (`1.7a` > `1.7`) salvo si empieza por espacio (`1.7 RC1` < `1.7`), y el sufijo se
compara **sin distinguir mayúsculas** (`1.7A` == `1.7a`). En semver un sufijo de *prerelease* hace
la versión **menor**: implementar semver aquí da veredictos distintos a los del original.

> Detalle que conviene registrar, y que sigue siendo cierto en el tag: la constante homónima del
> lanzador, `MIN_REQUESTED_VERSION_PARAM = "mcv"` (`ProtocolInvocationLauncher.java:81`), **no se
> usa en ningún sitio** —es la única aparición del símbolo en todo `afirma-simple`—. La
> comprobación real es la de `UrlParameters`. rfirma tiene que implementar la de `UrlParameters`,
> no la otra.

### Recomendación para rfirma

**Comportarse como protocolo 4, y no anunciarse como nada.** La recomendación anterior —«anunciarse
como 4.1»— era errónea por dos motivos independientes, los dos comprobables en el tag:

1. **La 4.1 no existe** en el código que ejecutan las sedes. `ProtocolVersion` sólo llega a
   `VERSION_4` (`ProtocolVersion.java:19`) y `SUPPORTED_PROTOCOL_VERSIONS` sólo contiene 3 y 4
   (`AfirmaWebSocketServerManager.java:36`).
2. **No hay canal por el que anunciarse.** La versión la declara la sede en la URL de arranque y
   nadie la devuelve. Un servidor no «se anuncia» en este protocolo.

Lo concreto que se deduce, y que sí es implementable:

- Se acepta `v=4` (lo que manda el cliente publicado) y `v=3` (puerto fijo `63117`, sin comprobación
  de `idsession`). Cualquier otro valor es una invocación que no se atiende.
- **El formato de error es siempre `SAF_NN`** (§5). No hay ninguna condición bajo la cual la 1.9.2
  entienda un `err-00:=AF…`: `processResponse` no contempla el prefijo `err-`
  (`autoscript.js:2304`-`2330`).
- **El tercer campo de la respuesta de firma se emite**, porque la condición es
  `getProtocolVersion() >= 3` y el servidor V4 pasa `4`
  (`NativeSignDataProcessor.java:97`; `AfirmaWebSocketServerV4.java:91`).
- Ante `v` no soportada, **rfirma tiene margen para mejorar el original**: abrir el socket igual y
  contestar `SAF_21` al primer mensaje, en vez de cerrarse en silencio como hace AutoFirma. Cerrar
  la aplicación deja al navegador reintentando 15 × 2 s hasta el diálogo de «no se encuentra la
  aplicación», que es peor experiencia y no aporta nada. **Esto es una propuesta, no una medición**:
  no se ha comprobado que el cliente publicado se comporte bien con un socket que abre y luego
  responde un error a la primera petición.
- **`jvc` se ignora.** En la 1.9.2 no decide nada que rfirma tenga que reproducir; el único efecto
  es un diálogo de aviso, y rfirma no tiene por qué reproducir un diálogo del original.

---

## 3. El contrato de las cuatro operaciones del alcance mínimo

Reglas comunes, antes del detalle:

- **Codificación**. Todo lo binario va en **Base64 URL-safe** (`-` y `_` en vez de `+` y `/`). El JS
  normaliza antes de enviar (`normalizeBase64Data`, `autoscript.js:1936`-`1938`) y deshace la
  sustitución al recibir (p. ej. `autoscript.js:2465`). Los pares que ya van en Base64 se marcan
  con `avoidEncoding` para que no se les aplique además `encodeURIComponent` (`createKeyValuePair`,
  `autoscript.js:2552`-`2558`; uso en `buildUrl`, `autoscript.js:2079`-`2087`).
- **Los pares con valor `null` no se envían**: `buildUrl` los salta (`autoscript.js:2081`). Un campo
  ausente y un campo vacío no son lo mismo.
- **Nada de `application/json`**. El mensaje del socket es una URL y la respuesta es una cadena con
  campos separados por `|`.
- **El intercambio es síncrono y sin temporizador en el cliente** (§3.4).

### 3.1 Eco

Hay dos «ecos» distintos y sólo uno es del protocolo:

- `AutoScript.echo()`, que devuelve una constante y sólo sirve para que la sede compruebe que el
  `.js` ha cargado. **No sale del navegador.**
- El eco **del socket**, que sí es el contrato.

**Petición** (`autoscript.js:2286`), texto plano, no una URL:

```
echo=-idsession=<idsession>@EOF
```

**Respuesta**: exactamente `OK` (`ECHO_OK_RESPONSE`, `AfirmaWebSocketServerV4.java:35`; se emite en
`:81`-`83`). El prefijo reconocido es `echo=` (`:27`) y el sufijo `@EOF` (`:30`).

Su papel es el arranque en frío: `processRequest` engancha `onMessageEchoFunction` y **sólo cuando
llega la respuesta al eco envía la operación real** (`autoscript.js:2242`-`2268`). Si el socket
todavía no está en `readyState === 1`, `sendEcho` reintenta cada 2 s hasta 15 veces
(`autoscript.js:2272`-`2299`).

Obligatorio: `idsession`. No hay más campos.

### 3.2 Selección de certificado

**Ojo al nombre.** En el JS la constante se llama `OPERATION_SELECT_CERTIFICATE = "certificate"`
(`autoscript.js:1761`), pero eso es sólo la etiqueta interna con la que el cliente recuerda qué
respuesta espera (`autoscript.js:1819`, `2339`). **El `op` que viaja por el cable es `selectcert`**
(`createSelectCertificateRequest`, `autoscript.js:1941`-`1953`, en concreto `:1943`) y la URL que
AutoFirma reconoce es `afirma://selectcert?…` (`ProtocolInvocationLauncher.java:370`). rfirma tiene
que registrar `selectcert`.

**Petición** (`autoscript.js:1941`-`1953`, más el `mcv` de `buildUrl`, `:2069`-`2072`):

| campo | obligatorio | contenido |
| --- | --- | --- |
| `op` | sí | `selectcert` |
| `idsession` | sí | el del canal |
| `properties` | no | `extraParams` en Base64 URL-safe (formato `.properties`) |
| `ksb64` | no | almacén por defecto, en Base64 URL-safe |
| `sticky` | no | `true`/`false`: reutilizar el certificado ya elegido |
| `resetsticky` | no | sólo si la sede lo pidió |
| `mcv` | no | versión mínima exigida, si la sede la fijó (**no se comprueba** aquí, §2c) |

**Respuesta**: el certificado en DER, **Base64 URL-safe, y nada más** —sin separadores ni prefijo—
(`Base64.encode(certEncoded, true)`, `ProtocolInvocationLauncherSelectCert.java:262`; se devuelve
en `:290`). El cliente sólo deshace la sustitución de caracteres (`processSelectCertificateResponse`,
`autoscript.js:2462`-`2471`).

**Cancelación**: la cadena desnuda `CANCEL` (`RESULT_CANCEL`,
`ProtocolInvocationLauncherSelectCert.java:43`; `getResultCancel()` en `:292`-`294`, devuelto en
`:206`).

**Sin certificados tras filtrar**: `SAF_19` (`ProtocolInvocationLauncherSelectCert.java:208`-`215`).

### 3.3 `sign` y `cosign`

Ambas comparten petición y respuesta; sólo cambia el `op` (`autoscript.js:1828`, `1835`, `1842`,
que delegan en `signOperation`, `:1853`). Los valores son `sign`, `cosign` y `countersign`, y el
`op` va tanto en el dominio de la URL (`afirma://cosign?`) como en un parámetro `op=` (`buildUrl`,
`autoscript.js:2077`; lado servidor, `ProtocolInvocationLauncher.java:643`-`645`).

**Petición** (`createSignRequest`, `autoscript.js:1956`-`1972`):

| campo | obligatorio | contenido |
| --- | --- | --- |
| `op` | sí | `sign` \| `cosign` \| `countersign` |
| `idsession` | sí | el del canal |
| `algorithm` | **sí** | ver lista abajo |
| `format` | **sí** | `PAdES`, `CAdES`, `XAdES`… |
| `dat` | **sí** en la práctica | el documento en Base64 URL-safe |
| `properties` | no | `extraParams` en Base64 URL-safe |
| `ksb64`, `sticky`, `resetsticky`, `appname` | no | |
| `mcv` | no | lo antepone `buildUrl` |

> **No hay `servicetimeout`.** `grep -c servicetimeout autoscript.js` da cero, y tampoco está en
> `KNOWN_PARAMETERS` de `UrlParametersToSign` (`UrlParametersToSign.java:52`-`57`). Es de `master`.

La obligatoriedad está en el parser, no en el JS
(`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSign.java`):

- Sin `format` → `ParameterException` («No se ha recibido el formato de firma»), `:279`-`281`.
- Sin `algorithm` → `ParameterException`, `:287`-`289`; con uno fuera de la lista → `:291`-`293`.
- Algoritmos admitidos, exactamente estos doce: `SHA1`, `SHA256`, `SHA384`, `SHA512`, y
  `SHA{1,256,384,512}with{RSA,ECDSA}` (`UrlParametersToSign.java:60`-`74`).
- Sin `dat` ni `fileid`, la operación no tiene de dónde sacar los datos (`UrlParameters.java:267`
  en adelante). El JS omite `dat` cuando la cadena es vacía (`autoscript.js:1970`).
- `properties` se decodifica con `AOUtil.base642Properties`, es decir un fichero `.properties` de
  Java en Base64; si no parsea, **se ignora en silencio** y se sigue con propiedades vacías
  (`UrlParametersToSign.java:298`-`315`).
- El valor de `dat` que empiece por `file:/` se rechaza explícitamente (`UrlParameters.java:300`-`303`):
  es la defensa contra leer ficheros locales por orden de la sede. rfirma debe replicarla.

> **Cómo llega ese fallo al navegador.** Todas esas `ParameterException` las recoge el lanzador y
> las convierte en **`SAF_03`** (`ProtocolInvocationLauncher.java:737`-`741` para la firma;
> `:430`-`434` para `selectcert`), y **el detalle no viaja**: `showErrorDetail` lo enseña en el
> diálogo local, pero lo que se devuelve por el socket es `getErrorMessage(ERROR_PARAMS)`, o sea
> `SAF_03` con su descripción genérica. La sede nunca sabe *qué* parámetro estaba mal. Es una
> pérdida de información del original, no una decisión de rfirma.

**Respuesta** (`NativeSignDataProcessor.postProcess`, `NativeSignDataProcessor.java:53`-`104`), con
`RESULT_SEPARATOR = '|'` (`:23`):

```
<certificado DER en B64 URL-safe> | <firma en B64 URL-safe> [ | <extraData en B64 URL-safe> ]
```

El tercer campo es un JSON con datos extra —el nombre del fichero cargado, por ejemplo— y se emite
cuando hay `extraData` y el protocolo es `>= 3` (`NativeSignDataProcessor.java:97`;
`buildExtraDataResult` en `:112`-`126`). El cliente lo parte por el primer y segundo `|`
(`processSignResponse`, `autoscript.js:2512`-`2549`); si no hay ningún `|`, trata el todo como una
firma y le hace un `Base64.decode` extra (`autoscript.js:2527`) — camino que sólo se da en los
transportes viejos.

**Cancelación**: `CANCEL` a pelo. El lanzador comprueba
`ProtocolInvocationLauncherSign.RESULT_CANCEL.equals(errorCode)` y devuelve el código tal cual
(`ProtocolInvocationLauncher.java:700`-`702`; `RESULT_CANCEL = "CANCEL"` en
`ProtocolInvocationLauncherSign.java:104`).

> **Un error de firma sí puede llevar detalle.** Cuando la `SocketOperationException` trae un
> mensaje distinto del código, lo que se devuelve es `"<código>: <mensaje>"`
> (`ProtocolInvocationLauncher.java:705`-`707`) — misma forma que `getErrorMessage`, así que el
> cliente lo reconoce igual por el prefijo `SAF_`.

**Sobre `cosign` en PAdES.** No es una cofirma CAdES: en PDF, cofirmar es **volver a firmar**.
`AOPDFSigner.cosign` delega literalmente en `sign`, en sus dos sobrecargas
(`<clienteafirma>/afirma-crypto-pdf/src/main/java/es/gob/afirma/signers/pades/AOPDFSigner.java:280`-`287`
y `:319`-`324`), y el javadoc lo dice: «*las multifirmas en los ficheros PDF se limitan a firmas
independientes «en serie»*» (`:250`, `:290`). Para rfirma, `cosign` con `format=PAdES` es **la misma
ruta de código que `sign`**; lo único que cambia es que los datos de entrada ya son un PDF firmado.

`countersign` sí es «operación no soportada para firmas PAdES»: lanza `UnsupportedOperationException`
(`AOPDFSigner.java:327`-`336`), que el lanzador de firma convierte en **`SAF_04`**
(`ProtocolInvocationLauncherSign.java:838`-`843`). *(La versión anterior de este informe decía
`AF600002`; ese código no existe en la 1.9.2.)*

Las cadenas de formato son `"CAdES"`, `"XAdES"`, `"PAdES"` y `"PAdEStri"`
(`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/signers/AOSignConstants.java:35`,
`72`, `111`, `114`).

### 3.4 No hay conversación asíncrona: ni `#wait` ni `getresult?`

**En la 1.9.2 el intercambio es síncrono y no hace falta que sea otra cosa.** Tres medidas:

1. **El cliente no tiene temporizador de respuesta.** Una vez contestado el eco,
   `onMessageEchoFunction` engancha `ws.onmessage` a `processResponse` y hace
   `ws.send(currentOperationUrl)` (`autoscript.js:2259`-`2267`). No hay `setTimeout`, ni contador,
   ni sondeo. Los 15 reintentos × 2 s (`autoscript.js:149`, `152`) son de la fase de **conexión** —
   `waitAppAndProcessRequest` (`:2162`-`2190`) y `sendEcho` (`:2272`-`2299`)—, no de la espera a la
   respuesta de la operación. **Una firma que pide PIN cabe de sobra en el camino síncrono.**
2. **El servidor tampoco espera de la persona.** `AfirmaWebSocketServerV4.onMessage` hace
   `setConnectionLostTimeout(batchOperation ? 240 : 60)` y a continuación llama a
   `ProtocolInvocationLauncher.launch(...)` en el mismo hilo (`AfirmaWebSocketServerV4.java:89`-`91`).
   Esos 60 s son el *keepalive* de conexión de `org.java-websocket` (pings), no un plazo para
   contestar.
3. **`getresult?` no existe.** `grep -c getresult autoscript.js` da **cero**, y
   `ProtocolInvocationLauncher.launch` sólo acepta cadenas que empiecen por `afirma://`
   (`:172`-`178`). Tampoco hay `WebSocketServerOperationHandler` en el paquete `protocol` del tag.

El único `#wait` que hay en `autoscript.js` está en el transporte de **servlets**
(`autoscript.js:4452`, dentro del bloque que arranca en `:3715`), donde es el servidor intermedio
quien dice «todavía no». No tiene nada que ver con el WebSocket.

**Consecuencia para rfirma**: `#wait` / `getresult?` **queda fuera del alcance mínimo**. Es la
corrección más importante respecto a la versión anterior de este informe, que lo declaraba «no
opcional».

---

## 4. Los filtros de certificado

> **El módulo `afirma-keystores-filters` es idéntico en `v1.9.2` y en `master`.**
> `git diff --stat v1.9.2 master -- afirma-keystores-filters` devuelve **un solo fichero cambiado,
> `pom.xml`, con una línea** (la versión del padre). Ni una línea de código difiere. Por tanto
> **todo lo medido en el [#314](https://github.com/sgomez/rfirma/issues/314) y decidido en el
> [#315](https://github.com/sgomez/rfirma/issues/315) sigue en pie sin matices**, incluidas las
> citas por número de línea.

### La sintaxis

Los filtros **no son un parámetro propio**: viajan dentro de `properties`, como claves del
`.properties` que la sede codifica en Base64. `CertFilterManager` los recoge en tres formas
alternativas, por orden de precedencia
(`<clienteafirma>/afirma-keystores-filters/src/main/java/es/gob/afirma/keystores/filters/CertFilterManager.java:165`-`182`):

1. `filter=<expresión>`
2. `filters=<expresión>`
3. `filters.1=<expresión>`, `filters.2=…`, numerados desde 1 y sin huecos.

Cada expresión es una **conjunción** de criterios separados por `;` (`FILTERS_SEPARATOR`, `:35`; el
`split` en `:188`). Cuando hay varias expresiones numeradas, entre ellas la relación es
**disyuntiva** («filtros disyuntivos», `:166`). Los criterios, con su prefijo literal
(`CertFilterManager.java:39`-`67`):

| prefijo | qué hace |
| --- | --- |
| `dnie:` | certificado de firma del DNIe |
| `ssl:` | certificado SSL |
| `qualified:` | cualificado, con el valor como argumento |
| `signingcert:` / `authcert:` | certificado de firma / de autenticación |
| `nonexpired:` | **al reves de lo que el nombre sugiere**: `false` muestra los caducados y `true` los oculta, porque el original construye `new ExpiredCertificateFilter(!parseBoolean(valor))` (`CertFilterManager.java:216`-`219`). Medido en el [#350](https://github.com/sgomez/rfirma/issues/350) |
| `sscd:` | dispositivo cualificado de creación de firma |
| `subject.rfc2254:` / `issuer.rfc2254:` | filtro LDAP RFC 2254 sobre el DN |
| `issuer.rfc2254.recurse:` | igual, recorriendo la cadena de emisores |
| `subject.contains:` / `issuer.contains:` | subcadena literal |
| `thumbprint:` | huella; admite `<algoritmo>:<huella>` (`:240`) |
| `policyid:` | lista de OID de política separados por comas (`:248`) |
| `pseudonym:` | seudónimo |
| `encodedcert:` | el certificado entero, en Base64 |
| `keyusage.<bit>:` | los nueve bits de `KeyUsage`, agrupados en un patrón (`:58`-`69`) |
| `disableopeningexternalstores` | prohíbe abrir otros almacenes desde el diálogo (`:39`) |

Hay dos claves más que viajan por el mismo canal y cambian el comportamiento del diálogo
(`CertFilterManager.java:29`-`30`, `146`-`153`):

- `headless=true` → selecciona automáticamente, sin diálogo.
- `mandatoryCertSelection=false` → lo mismo.

Y una regla por defecto que es una decisión, no un accidente: **si la sede no declara ningún filtro,
se añade uno que oculta los caducados**, citando ETSI TS 119 102-1
(`CertFilterManager.java:129`-`136`).

### Qué se espera que haga el cliente

Aplicar los filtros **al listado que se enseña**, no a la firma. El manager produce una lista de
`CertificateFilter` más dos banderas, y las tres cosas se le pasan al diálogo de selección:
`filters`, `mandatoryCertificate` y `allowOpenExternalStores`
(`ProtocolInvocationLauncherSelectCert.java:134`-`136`, `:181`-`185`;
`ProtocolInvocationLauncherSign.java:596`-`605`).

Si tras filtrar no queda ningún certificado, el error es el **mismo en las dos operaciones**:
`SAF_19`, desde `AOCertificatesNotFoundException`
(`ProtocolInvocationLauncherSelectCert.java:208`-`215`;
`ProtocolInvocationLauncherSign.java:619`-`623`). *(La versión anterior de este informe daba dos
códigos distintos, `AF502001` y `AF501001`. Ninguno existe en la 1.9.2 y la distinción tampoco.)*

### Contraste con `pkcs11::stores` de rfirma

**No se solapan: son ejes distintos, y el que hace falta es el que no existe.**

`<rfirma>/rfirma-app/src-tauri/src/pkcs11/stores.rs` responde a **dónde** buscar —qué módulo
PKCS#11, qué perfil NSS— y no filtra certificados en absoluto: es `CANDIDATE_MODULES`,
`CANDIDATE_SOFTOKENS`, `StoreClass` y la resolución de `profiles.ini`. Lo más parecido a un filtro
que hay hoy en rfirma vive en `<rfirma>/rfirma-app/src-tauri/src/pkcs11/mod.rs` y son dos criterios
fijos, ninguno configurable por nadie:

- **tener clave privada** (ID-07), que es lo que separa `signable_certificates` de
  `list_certificates_unfiltered_for_test`;
- **tener `CKA_LABEL` no vacía**, porque el ADR-0010 sólo deja persistir la etiqueta y ofrecer lo
  que no se puede reencontrar sería prometer de más.

El estado del certificado sí se calcula —`CertificateStatus` distingue vigente, caducado, aún no
válido y revocado (`pkcs11/certificate.rs`)—, así que el filtro por defecto de AutoFirma
(`nonexpired`) es **barato**: ya hay con qué. Todo lo demás —`subject.contains`, `issuer.rfc2254`,
`policyid`, `keyusage.*`, `thumbprint`— exige leer campos del X.509 que hoy no se leen.

Conclusión operativa: los filtros son **el trabajo de v0.5 que menos se parece a lo que ya hay** y
el que más fácil es subestimar. El [#315](https://github.com/sgomez/rfirma/issues/315) ya decidió
cómo se resuelve —motor prestado, `afirma-keystores-filters` al puente, con lista blanca de
prefijos en Rust para cerrar el *fail-open* del original—, y esa decisión **no la toca la 1.9.2**.

---

## 5. Los códigos de error

### El formato del mensaje

**Uno solo, de una línea, y siempre el viejo.** Lo produce
`ProtocolInvocationLauncherErrorManager.getErrorMessage` (`:183`-`185`):

```
SAF_NN: <descripción>
```

Y una variante para los errores que llevan detalle, construida por el lanzador
(`ProtocolInvocationLauncher.java:705`-`707`):

```
SAF_NN: <mensaje de la excepción>
```

La cancelación de la persona **no es un código**: es la cadena desnuda `CANCEL`
(`ProtocolInvocationLauncherSign.java:104`, `ProtocolInvocationLauncherSelectCert.java:43`).

> **No existe `err-00:=` ni `err-11:=`, ni ningún código `AF…`.** No hay
> `PROTOCOL_VERSION_WITH_ERROR_CODES`, no hay `OLD_ERRORS_ASSOCIATION`, y las clases
> `es.gob.afirma.core.ErrorCode` y `es.gob.afirma.standalone.SimpleErrorCode` **no existen en el
> tag** (`find . -name 'ErrorCode.java' -o -name 'SimpleErrorCode.java'` no devuelve nada). Todo
> eso es de `master`.

El cliente parsea así (`processResponse`, `autoscript.js:2304`-`2330`), y sólo así:

- `undefined`, `null` o `"CANCEL"` → cancelación
  (`es.gob.afirma.core.AOCancelledOperationException`), `:2306`-`2309`.
- `"MEMORY_ERROR"` → `es.gob.afirma.core.OutOfMemoryError`, `:2312`-`2315`.
- longitud > 4 y los cuatro primeros caracteres son `SAF_` → error, y **el mensaje que ve la sede es
  la respuesta entera**, con la excepción `java.lang.Exception`, `:2318`-`2321`.
- `"NULL"` → `java.lang.Exception` con «Error desconocido», `:2324`-`2327`.
- cualquier otra cosa → se trata como resultado de la operación en curso, `:2330`-`2358`.

> **Consecuencia práctica**: si rfirma responde algo que no empieza por `SAF_`, no es `CANCEL`, no
> es `MEMORY_ERROR` y no es `NULL`, el cliente lo entrega al `successCallback` como si fuera una
> firma. **No hay forma de señalar un error que no sea con el prefijo `SAF_`.**

### El catálogo entero

Cincuenta y tres códigos, `SAF_00` … `SAF_52`, declarados en
`ProtocolInvocationLauncherErrorManager.java:31`-`83`, con su descripción en
`<clienteafirma>/afirma-simple/src/main/resources/properties/protocolmessages.properties`
(la asociación código→clave está en el bloque estático, `:87`-`141`).

| código | constante | descripción |
| --- | --- | --- |
| `SAF_00` | `ERROR_CANNOT_READ_DATA` | No se han podido leer los datos a firmar |
| `SAF_01` | `ERROR_NULL_URI` | La URL recibida es nula |
| `SAF_02` | `ERROR_UNSUPPORTED_PROTOCOL` | Protocolo no soportado |
| `SAF_03` | `ERROR_PARAMS` | Error en los parámetros de entrada |
| `SAF_04` | `ERROR_UNSUPPORTED_OPERATION` | Operación no soportada |
| `SAF_05` | `ERROR_CANNOT_SAVE_DATA` | No se han podido guardar los datos |
| `SAF_06` | `ERROR_UNSUPPORTED_FORMAT` | Formato de firma no soportado |
| `SAF_07` | `ERROR_CANNOT_FIND_KEYSTORE` | No se ha podido determinar el almacén de claves |
| `SAF_08` | `ERROR_CANNOT_ACCESS_KEYSTORE` | Error accediendo al almacén de claves y certificados |
| `SAF_09` | `ERROR_SIGNATURE_FAILED` | Error realizando la firma electrónica |
| `SAF_10` | `ERROR_NO_CERTIFICATES_SYSTEM` | No hay certificados de firma instalados en el sistema |
| `SAF_11` | `ERROR_SENDING_RESULT` | Error en el envío del resultado de la operación |
| `SAF_12` | `ERROR_ENCRIPTING_DATA` | Error en el cifrado de los datos a enviar |
| `SAF_13` | `ERROR_LOCAL_ACCESS_BLOCKED` | Se ha pedido acceso a una dirección local y se ha bloqueado |
| `SAF_14` | `ERROR_OBSOLETE_APP` | La aplicación está obsoleta |
| `SAF_15` | `ERROR_DECRYPTING_DATA` | Error en el descifrado de los datos |
| `SAF_16` | `ERROR_RECOVERING_DATA` | Error al recuperar los datos del servidor intermedio |
| `SAF_17` | `ERROR_UNKNOWN_SIGNER` | Los datos no son una firma electrónica reconocida |
| `SAF_18` | `ERROR_DECODING_CERTIFICATE` | Error al descodificar el certificado de firma |
| `SAF_19` | `ERROR_NO_CERTIFICATES_KEYSTORE` | No hay ningún certificado válido en su almacén |
| `SAF_20` | `ERROR_LOCAL_BATCH_SIGN` | Error en el procesado del lote de firma |
| `SAF_21` | `ERROR_UNSUPPORTED_PROCEDURE` | La versión de Autofirma instalada no es compatible con este trámite |
| `SAF_22` | `ERROR_UNSOPPORTED_WEB_PROCEDURE` | El trámite web no es compatible con la versión instalada |
| `SAF_23` | `ERROR_INVALID_POLICY` | Política de firma no válida o parámetros incompatibles |
| `SAF_24` | `ERROR_RECOVERING_LOG` | Error al obtener el registro de log |
| `SAF_25` | `ERROR_CANNOT_LOAD_DATA` | Error en la lectura de los datos a cargar |
| `SAF_26` | `ERROR_CONTACT_BATCH_SERVICE` | Error en la comunicación con el servicio de firma de lotes |
| `SAF_27` | `ERROR_BATCH_SIGNATURE` | El servicio informó de un error durante la firma del lote |
| `SAF_28` | `ERROR_INVALID_PDF` | El fichero no es un PDF o es un PDF no soportado |
| `SAF_29` | `ERROR_INVALID_XML` | Las firmas XAdES Enveloped sólo pueden hacerse sobre datos XML |
| `SAF_30` | `ERROR_INVALID_DATA` | El formato de los datos no es adecuado para el tipo de firma |
| `SAF_31` | `ERROR_NO_SIGN_DATA` | Los datos introducidos no se corresponden con un objeto de firma |
| `SAF_32` | `ERROR_FACE_ALREADY_SIGNED` | La factura ya tiene firma y no admite firmas adicionales |
| `SAF_33` | `ERROR_PDF_WRONG_PASSWORD` | Contraseña del PDF no válida o ausente |
| `SAF_34` | `ERROR_PDF_UNREG_SIGN` | El PDF contiene firmas no registradas |
| `SAF_35` | `ERROR_PDF_CERTIFIED` | El PDF está certificado |
| `SAF_36` | `ERROR_CANNOT_FIND_SSL_KEYSTORE` | No se encuentra el almacén de claves SSL |
| `SAF_37` | `ERROR_CANNOT_ACCESS_SSL_KEYSTORE` | No se puede acceder al almacén de claves SSL |
| `SAF_38` | `ERROR_INVALID_FACTURAE` | El archivo no es una factura electrónica reconocida |
| `SAF_39` | `ERROR_INVALID_SIGNATURE` | La firma de entrada no es válida |
| `SAF_40` | `ERROR_RECOVER_SERVER_DOCUMENT` | Error al recuperar el documento |
| `SAF_41` | `ERROR_MINIMUM_VERSION_NON_SATISTIED` | El trámite requiere una versión más reciente de Autofirma |
| `SAF_42` | `ERROR_POSTPROCESSING_DATA` | Error al postprocesar una firma |
| `SAF_43` | `ERROR_VISIBLE_SIGNATURE` | Error durante la firma visible del PDF |
| `SAF_44` | `ERROR_SIGN_WITHOUT_DATA` | La firma no contiene los datos y no es compatible con la configuración |
| `SAF_45` | `ERROR_CANNOT_OPEN_SOCKET` | No se pudo abrir un socket para la comunicación |
| `SAF_46` | `ERROR_INVALID_SESSION_ID` | Id de sesión inválido |
| `SAF_47` | `ERROR_EXTERNAL_REQUEST_TO_SOCKET` | Petición al socket desde IP externa o sin identificar |
| `SAF_48` | `ERROR_PDF_SHADOW_ATTACK` | Posible PDF Shadow Attack |
| `SAF_49` | `ERROR_SIGNING_LTS_SIGNATURE` | Multifirma de firma de archivo |
| `SAF_50` | `ERROR_CONFIRMATION_NEEDED` | La operación puede generar firmas no válidas y necesita confirmación |
| `SAF_51` | `ERROR_INCOMPATIBLE_KEY_TYPE` | El tipo de clave del certificado no está soportado |
| `SAF_52` | `ERROR_LOCKED_KEYSTORE` | El almacén de claves está bloqueado |

### Los que rfirma tiene que saber producir

Los alcanzables desde las cuatro operaciones del mínimo, con el sitio exacto del original donde se
emiten:

| código | cuándo | dónde en el original |
| --- | --- | --- |
| `CANCEL` (sin código) | la persona cancela | `ProtocolInvocationLauncherSign.java:836`, `:850`; `…SelectCert.java:206` |
| `SAF_47` | petición desde una IP que no es 127.0.0.1 | `AfirmaWebSocketServerV4.java:61`-`68` |
| `SAF_46` | el `idsession` del mensaje no coincide | `AfirmaWebSocketServerV4.java:72`-`78` |
| `SAF_02` | el mensaje no empieza por `afirma://` | `ProtocolInvocationLauncher.java:172`-`178` |
| `SAF_04` | operación desconocida, y `countersign` en PAdES | `ProtocolInvocationLauncher.java:837`-`843`; `…Sign.java:838`-`843` |
| `SAF_03` | cualquier fallo de parámetros (falta `format`, falta `algorithm`, algoritmo no admitido, `dat` con `file:/`…) | `ProtocolInvocationLauncher.java:430`-`434`, `:737`-`741` |
| `SAF_13` | se pidió acceso a una dirección local | `ProtocolInvocationLauncher.java:732`-`736` |
| `SAF_14` | la petición exige una versión más nueva (`ParameterNeedsUpdatedVersionException`) | `ProtocolInvocationLauncher.java:726`-`731` |
| `SAF_41` | la sede exigió una versión de cliente mayor (`mcv`) | `ProtocolInvocationLauncherSign.java:142`-`150` |
| `SAF_19` | no queda ningún certificado tras filtrar | `…SelectCert.java:208`-`215`; `…Sign.java:619`-`623` |
| `SAF_08` | error accediendo al almacén | `…SelectCert.java:217`-`224`; `…Sign.java:584`, `:626` |
| `SAF_52` | el almacén está bloqueado | `ProtocolInvocationLauncherSign.java:650`-`657` |
| `SAF_51` | tipo de clave incompatible con el algoritmo | `ProtocolInvocationLauncherSign.java:634`-`639` |
| `SAF_06` | formato de firma no soportado | `ProtocolInvocationLauncherSign.java:266`-`270` |
| `SAF_28` | el fichero no es un PDF válido | `ProtocolInvocationLauncherSign.java:753`-`757` |
| `SAF_09` | error realizando la firma | `ProtocolInvocationLauncherSign.java:852`-`860` |
| `SAF_21` | versión de protocolo no soportada | `ProtocolInvocationLauncher.java:242`-`244` (**diálogo, no socket**) |
| `SAF_45` | no se pudo abrir el socket en ningún puerto | `ProtocolInvocationLauncher.java:248`-`250` (**diálogo, no socket**) |

Los dos últimos ocurren **antes de que exista socket**: el original los enseña en un diálogo y
cierra. rfirma tiene margen para hacerlo mejor (§2), pero eso es diseño nuevo, no imitación.

**Lo que rfirma NO emite nunca**: nada con prefijo `AF`, y nada con prefijo `AS`. Los `AF…` no
existen en la 1.9.2 y el cliente los entregaría como si fueran una firma; los `AS…` tampoco existen
en `autoscript.js` 1.9.0. Lo que la sede recibe en su `errorCallback` es siempre un par (nombre de
excepción Java, texto), y el nombre lo elige el propio `autoscript.js`.

---

## 6. Qué queda fuera del mínimo

Todo esto es de la **ficha 18b**, y ninguna sede que use el transporte websocket v4 con las cuatro
operaciones del mínimo lo necesita:

- **Los servlets del servidor intermedio.** El transporte que arranca en `autoscript.js:3715`
  (`PROTOCOL_VERSION = 3`) sube los datos a un servidor de la sede y AutoFirma se los descarga con
  `fileid`, `rtservlet` y `stservlet` (`ProtocolInvocationLauncher.java:660`-`679` para la firma).
  Arrastra consigo la espera activa (`ActiveWaitingThread`, `requestWait` en `:855`-`868`) y el
  `#wait` de `:4452`. Es el camino de los navegadores viejos; ninguno actual lo toma.
- **El cifrado extremo a extremo.** El parámetro `key` y el `NativeDataCipher`
  (`NativeSignDataProcessor.java:32`-`37`, `68`-`85`; `UrlParameters.java:342`). Sólo tiene sentido
  si hay servidor intermedio: la respuesta viaja por él y no por `wss://127.0.0.1`. Sin servlets,
  sobra.
- **`batch`** (`afirma://batch?`, `ProtocolInvocationLauncher.java:293`), con sus URL de prefirma y
  posfirma y el modo local. Respuesta con formato propio (`processBatchResponse`,
  `autoscript.js:2476`). Es además la única operación que sube el *keepalive* a 240 s
  (`AfirmaWebSocketServerV4.java:88`-`89`).
- **`save`** (`afirma://save?`, `:443`) y **`signandsave`** (`:532`).
- **`load` y `multiload`** (`afirma://load?`, `:753`), con su respuesta `nombre:datos|nombre:datos`
  (`processLoadResponse`, `autoscript.js:2377`).
- **`countersign`.** Está en el mismo `if` que `sign` y `cosign`
  (`ProtocolInvocationLauncher.java:643`-`645`) y es trivial de enrutar, pero en PAdES es «operación
  no soportada» por definición (`AOPDFSigner.java:327`-`336`), así que en el alcance PAdES de rfirma
  su implementación correcta es devolver **`SAF_04`**.
- **El transporte `afirma://service`** (sockets HTTP locales, `autoscript.js:2621`-`2626`;
  `ProtocolInvocationLauncher.java:264`). Es el predecesor del websocket y `autoscript.js` sólo lo
  elige en navegadores que ya no importan.

---

## Lo que esto deja decidido para el spec de la v0.5

1. rfirma registra el esquema `afirma://` y, al recibir `afirma://websocket?ports=…`, abre un
   **servidor WebSocket sobre TLS** en el primero de los tres puertos que consiga, atado a
   127.0.0.1.
2. El certificado TLS de ese `wss://` es exactamente el problema que el ADR-0005 y el instalador
   del hito v0.4 resuelven: sin CA en el almacén del navegador, el socket no abre. Esto **confirma**
   que la v0.4 es la puerta de la v0.5.
3. **Se habla protocolo 4 y no se anuncia ninguna versión.** Se acepta `v=4` y `v=3`; `jvc` se
   ignora. La 4.1, el `#wait`/`getresult?` y los códigos `AF…` **no entran en el alcance**: no
   existen en lo que ejecutan las sedes.
4. **El intercambio es síncrono.** Petición, respuesta, y la persona tarda lo que tarde: el cliente
   publicado no tiene temporizador de respuesta.
5. Se implementan cuatro mensajes: eco, `selectcert`, `sign` y `cosign` —y este último, en PAdES, es
   la misma ruta que `sign`—.
6. **Los errores salen siempre como `SAF_NN: <texto>`**, y la cancelación como `CANCEL` a pelo. Es
   el único formato que el cliente publicado reconoce.
7. Se implementan las dos guardias de seguridad del original —`idsession` en cada mensaje y origen
   127.0.0.1— y la tercera, la que prohíbe `dat` con `file:/`. La cuarta, validar que el `idsession`
   de la URL de arranque sea alfanumérico, se implementa **mejor que el original**: un `idsession`
   mal formado no puede abrir un canal sin credencial.
8. Los filtros de certificado son la pieza nueva de verdad, y su decisión —tomada en el #315 sobre
   un módulo que es idéntico en el tag— **no cambia**.

## Discoveries

- La versión anterior de este informe citaba `setServerRange` (`autoscript.js:962`-`971`); en el tag
  la función se llama **`setPortRange`** y está en `:771`-`787`.
- `createSignRequest` manda `appname` con la condición invertida —`!appName ? appName : DOMAIN_NAME`,
  `autoscript.js:1969`—, así que cuando la sede **no** fija nombre de aplicación se envía el valor
  vacío y cuando **sí** lo fija se envía el dominio. Es un fallo del original; rfirma no depende de
  ese campo, se anota por si algún día se usa.
- **No comprobado**: qué hace el cliente publicado si el servidor abre el socket y responde un error
  a la primera petición en vez de cerrarse (la propuesta de §2). Tampoco se ha comprobado el
  comportamiento con `v=3` extremo a extremo, porque el cliente publicado nunca lo manda por este
  transporte.
