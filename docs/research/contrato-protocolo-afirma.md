# El contrato del protocolo `afirma://` según `autoscript.js`

Sondeo del [#224](https://github.com/sgomez/rfirma/issues/224), hijo del mapa
[#217](https://github.com/sgomez/rfirma/issues/217). Va un hito por delante: mide
lo que la **v0.5** tendrá que implementar, para que el hito no se pare esperando
la medición.

La especificación no hay que adivinarla. El cliente que ejecutan las sedes es
`autoscript.js`, 6.180 líneas, y el servidor que lo atiende es el paquete
`es.gob.afirma.standalone.protocol` de AutoFirma. Todo lo que sigue está citado
por fichero y línea contra el repositorio oficial, clonado fuera de este
repositorio; en las citas se abrevia su raíz como **`<clienteafirma>`** y la de
este repositorio como **`<rfirma>`**.

**Respuesta corta.** El navegador moderno **no** usa `afirma://sign`. Usa
`afirma://websocket?…` sólo para **arrancar** la aplicación, y a partir de ahí
todo viaja por **`wss://127.0.0.1:<puerto>`**, en mensajes de texto plano que son
URL `afirma://…` enviadas por el socket. El alcance mínimo de rfirma son cuatro
mensajes —eco, `selectcert`, `sign` y `cosign`—, un formato de error de una sola
línea y un idioma de filtros de certificado que rfirma **hoy no tiene en
absoluto**. Todo lo demás (servlets, cifrado, `batch`, `save`, `load`,
`countersign`) es prescindible sin romper a ninguna sede que hable v4.

---

## 1. El arranque: `afirma://websocket`, no `afirma://sign`

**Confirmado.** El transporte vigente es `AppAfirmaWebSocketClient`
(`<clienteafirma>/afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js:2047`).
La única URL de esquema `afirma://` que llega al sistema operativo en ese
transporte es la de arranque, construida en `openNativeApp`:

```
afirma://websocket?ports=<p1,p2,p3>&v=4&jvc=4&idsession=<20 chars>[&dlgload=false]
```

— `autoscript.js:2469`-`2475`. El `v` es `PROTOCOL_VERSION = 4`
(`autoscript.js:2049`), el `jvc` es `VERSION_CODE = 4` (`autoscript.js:27`, junto
a `VERSION = "1.10.1"`), y `dlgload` sólo aparece cuando la sede ha pedido que no
se muestre el diálogo de carga.

Esa URL se entrega al sistema con `document.location = url` en Chrome, iOS y
Firefox de Android, y con un iframe oculto de 1×1 en los demás
(`autoscript.js:982`-`1050`). Es decir: **la única función del esquema
`afirma://` es lanzar el proceso**. Ninguna operación real viaja por él.

A partir de ahí, el cliente abre `wss://127.0.0.1:<puerto>`
(`URL_REQUEST_PREFIX`, `autoscript.js:2051`-`2053`) y **envía la URL de la
operación como cuerpo del mensaje del socket**: `ws.send(currentOperationUrl)`
(`autoscript.js:2589`), donde `currentOperationUrl` es la cadena
`afirma://sign?op=sign&idsession=…&algorithm=…` construida por `buildUrl`
(`autoscript.js:2396`). Del lado de AutoFirma, ese mismo texto entra en
`ProtocolInvocationLauncher.launch(message, …)`
(`<clienteafirma>/afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/AfirmaWebSocketServerV4Sup.java:102`),
que exige que empiece por `afirma://` o por `getresult?`
(`ProtocolInvocationLauncher.java:211`).

> El propio código de AutoFirma reconoce que esto es una deuda: «*La comunicación
> por sockets/websockets no debería utilizar URLs*»
> (`ProtocolInvocationLauncher.java:254`-`256`). rfirma tiene que hablarlo igual,
> pero conviene saber que es un accidente histórico y no un diseño.

### Cómo se eligen los puertos

`AfirmaUtils.getRandomPorts` (`autoscript.js:1955`-`1978`):

- Rango válido absoluto: `MIN_PORT = 1024`, `MAX_PORT = 65535`
  (`autoscript.js:1905`, `1911`).
- Rango por defecto: `DEFAULT_MIN_PORT = 49152` .. `MAX_PORT`, es decir el rango
  efímero de IANA (`autoscript.js:1908`, `1960`-`1961`).
- Se sortean **tres** puertos distintos del rango (`autoscript.js:1963`-`1978`),
  con `Math.random` y un antibucle: si el rango es más estrecho que el número de
  aleatorios pedidos, `getUniqueRandom` devuelve `0` para no colgarse
  (`autoscript.js:1988`-`1990`).
- La sede puede estrechar el rango con `setServerRange`
  (`autoscript.js:962`-`971`), donde se recorta contra `MIN_PORT`/`MAX_PORT`.

Los tres van en `ports=` separados por comas (`autoscript.js:2460`-`2468`).
AutoFirma los parsea en `getChannelInfo` (`ProtocolInvocationLauncher.java:1078`)
y los guarda en un `ChannelInfo` que los va sirviendo de uno en uno con
`nextPortAvailable()` (`ChannelInfo.java:56`-`61`).

Si `ports` **no** viene, AutoFirma asume protocolo v3 y usa el puerto fijo
`DEFAULT_WEBSOCKET_PORT = 63117` (`ProtocolInvocationLauncher.java:97`,
`273`-`276`).

### Qué es `idsession`

Veinte caracteres alfanuméricos sorteados con `window.crypto.getRandomValues`
cuando existe, y con un PRNG casero cuando no
(`generateNewIdSession`, `autoscript.js:1914`-`1933`; el alfabeto está en
`VALID_CHARS_TO_ID`, `autoscript.js:1902`, y **le falta la `v` minúscula** —un
descuido del original, no una decisión).

Es una **credencial de canal**, no un identificador de transacción: viaja en la
URL de arranque (`autoscript.js:2472`) y se repite en **cada** mensaje posterior,
tanto en el eco (`echo=-idsession=<id>@EOF`, `autoscript.js:2608`) como en cada
petición de operación (`data.idsession`, p. ej. `autoscript.js:2265`) y en el
sondeo de resultado (`getresult?idsession=<id>`, `autoscript.js:2637`).

Del lado servidor es una comprobación **obligatoria**: si el socket se abrió con
un `idsession` y el mensaje no trae exactamente ese, la petición se descarta con
error y sin ejecutarse (`AfirmaWebSocketServerV4Sup.java:78`-`84`). El extractor
tolera que el valor termine en `@EOF` (`AfirmaWebSocketServerV4Sup.java:134`-`157`).
Junto a ella hay una segunda guardia: **la petición tiene que venir de
127.0.0.1** o se rechaza (`AfirmaWebSocketServerV4Sup.java:69`-`75`,
`isLocalAddress` en `:121`-`123`).

### Qué pasa si ningún puerto está libre

Dos mitades, y las dos hay que reproducirlas.

**En AutoFirma**: `AfirmaWebSocketServerManager.startService` recorre los puertos
hasta que uno abre (`AfirmaWebSocketServerManager.java:71`-`102`). Si se agotan,
lanza `SocketOperationException`
(`AfirmaWebSocketServerManager.java:104`-`106`) y el lanzador **cierra la
aplicación entera** tras mostrar el error `AF220001`
(`ProtocolInvocationLauncher.java:289`-`292`). Ojo al detalle: el
`BindingErrorListener` reintenta el arranque completo y, si eso también falla,
hace `Runtime.getRuntime().halt(-1)`
(`AfirmaWebSocketServerManager.java:125`-`133`).

**En el navegador**: `waitAppAndProcessRequest` intenta conectar a los tres
puertos, 15 veces (`AUTOFIRMA_CONNECTION_RETRIES = 15`, `autoscript.js:268`) con
2 s entre intentos (`AUTOFIRMA_LAUNCHING_TIME = 2000`, `autoscript.js:265`) y una
espera inicial de 3 s (`autoscript.js:2430`). Agotados, sortea tres puertos
nuevos y notifica al `errorCallback` de la sede con la excepción
`es.gob.afirma.standalone.ApplicationNotFoundException` y el código
**`AS620017`** (`autoscript.js:2493`-`2506`). Ese `AS…` lo pone el **navegador**,
no la aplicación: es el catálogo de errores del propio `autoscript.js`
(`ErrorCode`, `autoscript.js:337`-`433`), y rfirma **no** lo emite nunca.

---

## 2. La negociación de versión

Hay **tres** ejes de versión y conviene no confundirlos.

**a) `v` — versión de protocolo.** El JS manda `v=4`
(`PROTOCOL_VERSION = 4`, declarado tres veces, una por transporte:
`autoscript.js:2049`, `2989`, `4191`). AutoFirma admite las mayores **3 y 4**:

```java
private static final int PROTOCOL_VERSION_3 = 3;
private static final int PROTOCOL_VERSION_4 = 4;
private static final int CURRENT_PROTOCOL_VERSION = PROTOCOL_VERSION_4;
private static final int[] SUPPORTED_PROTOCOL_VERSIONS = { PROTOCOL_VERSION_3, PROTOCOL_VERSION_4 };
```

— `AfirmaWebSocketServerManager.java:29`-`38`. La clase `ProtocolVersion`
reconoce las cadenas `"0"`, `"1"`, `"2"`, `"3"`, `"4"` y `"4.1"`, con forma
`X` o `X.Y` (`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolVersion.java:23`-`33`,
`70`-`84`).

**Ante una versión que no conoce**, `checkSupportProtocol` lanza
`UnsupportedProtocolException` marcando si hace falta actualizar —cuando la
pedida es *mayor* que la actual—
(`AfirmaWebSocketServerManager.java:142`-`149`; `UnsupportedProtocolException.java:30`-`51`),
y el lanzador muestra el error `AF620011` («Version de protocolo de comunicacion
con el navegador no soportada»,
`<clienteafirma>/afirma-simple/src/main/java/es/gob/afirma/standalone/SimpleErrorCode.java:113`)
y **cierra la aplicación** (`ProtocolInvocationLauncher.java:284`-`288`).

**b) `jvc` — versión de código del JavaScript.** `VERSION_CODE = 4`
(`autoscript.js:27`), enviado en el arranque (`autoscript.js:2471`). Hace dos
cosas:

- Si `jvc < MIN_JAVASCRIPT_VERSION_CODE_NEEDED`, AutoFirma enseña un aviso
  modal al usuario (`ProtocolInvocationLauncher.java:245`-`252`).
- **Sintetiza la versión 4.1**: si el protocolo es 4 y `jvc > 3`, la versión pasa
  a ser `4.1` (`reviewProtocolVersion`, `ProtocolInvocationLauncher.java:159`-`174`).
  El comentario del propio código explica por qué el truco existe: no se podía
  subir el `v` sin romper AutoFirma 1.9.

La 4.1 no es cosmética. Decide **dos** comportamientos:

1. **El formato de los errores** (§5): sólo desde 4.1 se emiten códigos `AF…`
   (`ProtocolInvocationLauncherErrorManager.java:277`, `326`).
2. **El procesado asíncrono**: `asynchronous = jvc > 3`
   (`ProtocolInvocationLauncher.java:281`), que es lo que habilita el par
   `#wait` / `getresult?` (§3).

**c) `mcv` — versión mínima de la aplicación cliente.** La sede la fija con
`setMinimumClientVersion` (`autoscript.js:555`-`557`) y el JS la antepone a los
parámetros de **cada** petición (`autoscript.js:2385`-`2388`). AutoFirma la lee
en `UrlParameters` (`MINIMUM_CLIENT_VERSION_PARAM = "mcv"`,
`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java:78`, `293`)
y la compara contra la versión de la aplicación al principio de la firma, con
`AF500005` si no se cumple
(`ProtocolInvocationLauncherSign.java:146`-`152`; `SimpleErrorCode.java:80`).

> Detalle que conviene registrar: la constante homónima del lanzador,
> `MIN_REQUESTED_VERSION_PARAM = "mcv"` (`ProtocolInvocationLauncher.java:91`),
> **no se usa en ningún sitio** —es la única aparición del símbolo en todo
> `afirma-simple`—. La comprobación real es la de `UrlParameters`. rfirma tiene
> que implementar la de `UrlParameters`, no la otra.

**Recomendación para rfirma.** Anunciarse como **4.1** (es decir: aceptar `v=4`
con `jvc>3` y comportarse en consecuencia), aceptar `v=3` degradando al formato
de error antiguo, y ante cualquier otra mayor devolver `AF620011` en lugar de
cerrarse: cerrar la aplicación deja al navegador dando vueltas 30 segundos hasta
el `AS620017`, que es peor experiencia y no aporta nada.

---

## 3. El contrato de las cuatro operaciones del alcance mínimo

Reglas comunes, antes del detalle:

- **Codificación**. Todo lo binario va en **Base64 URL-safe** (`-` y `_` en vez
  de `+` y `/`). El JS normaliza antes de enviar (`normalizeBase64Data`,
  `autoscript.js:2242`-`2244`) y deshace la sustitución al recibir (p. ej.
  `autoscript.js:2828`). Los pares que ya van en Base64 se marcan con
  `avoidEncoding` para que no se les aplique además `encodeURIComponent`
  (`createKeyValuePair`, `autoscript.js:2914`-`2920`; uso en `buildUrl`,
  `autoscript.js:2397`-`2405`).
- **Los pares con valor `null` no se envían**: `buildUrl` los salta
  (`autoscript.js:2398`). Un campo ausente y un campo vacío no son lo mismo.
- **Nada de `application/json`**. El mensaje del socket es una URL y la respuesta
  es una cadena con campos separados por `|`.

### 3.1 Eco

Hay dos «ecos» distintos y sólo uno es del protocolo:

- `AutoScript.echo()`, que devuelve la constante `"Cliente JavaScript"` y sólo
  sirve para que la sede compruebe que el `.js` ha cargado
  (`autoscript.js:2926`-`2930`). **No sale del navegador.**
- El eco **del socket**, que sí es el contrato.

**Petición** (`autoscript.js:2608`), texto plano, no una URL:

```
echo=-idsession=<idsession>@EOF
```

**Respuesta**: exactamente `OK` (`ECHO_OK_RESPONSE`,
`AfirmaWebSocketServerV4Sup.java:36`; se emite en `:90`-`91`). El prefijo
reconocido es `echo=` (`:28`) y el sufijo `@EOF` (`:31`).

Su papel es el arranque en frío: `processRequest` engancha
`onMessageEchoFunction` y **sólo cuando llega la respuesta al eco envía la
operación real** (`autoscript.js:2564`-`2590`). Si el socket todavía no está en
`readyState === 1`, reintenta cada 2 s hasta 15 veces
(`autoscript.js:2594`-`2620`).

Obligatorio: `idsession`. No hay más campos.

### 3.2 Selección de certificado

**Ojo al nombre.** En el JS la constante se llama
`OPERATION_SELECT_CERTIFICATE = "certificate"` (`autoscript.js:2063`), pero eso
es sólo la etiqueta interna con la que el cliente recuerda qué respuesta espera
(`autoscript.js:2125`). **El `op` que viaja por el cable es `selectcert`**
(`createSelectCertificateRequest`, `autoscript.js:2247`-`2249`) y la URL que AutoFirma
reconoce es `afirma://selectcert?…` (`ProtocolInvocationLauncher.java:427`).
rfirma tiene que registrar `selectcert`.

**Petición** (`autoscript.js:2247`-`2260`):

| campo | obligatorio | contenido |
| --- | --- | --- |
| `op` | sí | `selectcert` |
| `idsession` | sí | el del canal |
| `properties` | no | `extraParams` en Base64 URL-safe (formato `.properties`) |
| `ksb64` | no | almacén por defecto, en Base64 URL-safe |
| `sticky` | no | `true`/`false`: reutilizar el certificado ya elegido |
| `resetsticky` | no | sólo si la sede lo pidió |
| `mcv` | no | versión mínima exigida, si la sede la fijó |

**Respuesta**: el certificado en DER, **Base64 URL-safe, y nada más** —sin
separadores ni prefijo— (`ProtocolInvocationLauncherSelectCert.java:234`). El
cliente sólo deshace la sustitución de caracteres
(`processSelectCertificateResponse`, `autoscript.js:2824`-`2833`).

### 3.3 `sign` y `cosign`

Ambas comparten petición y respuesta; sólo cambia el `op`
(`autoscript.js:2133`-`2148`, que delegan en `signOperation`, `:2159`-`2164`).
Los valores son `sign`, `cosign` y `countersign`, y el `op` va tanto en el
dominio de la URL (`afirma://cosign?`) como en un parámetro `op=`
(`buildUrl`, `autoscript.js:2396`; lado servidor,
`ProtocolInvocationLauncher.java:713`-`715`).

**Petición** (`createSignRequest`, `autoscript.js:2262`-`2281`):

| campo | obligatorio | contenido |
| --- | --- | --- |
| `op` | sí | `sign` \| `cosign` \| `countersign` |
| `idsession` | sí | el del canal |
| `algorithm` | **sí** | ver lista abajo |
| `format` | **sí** | `PAdES`, `CAdES`, `XAdES`… |
| `dat` | **sí** en la práctica | el documento en Base64 URL-safe |
| `properties` | no | `extraParams` en Base64 URL-safe |
| `ksb64`, `sticky`, `resetsticky`, `appname`, `servicetimeout` | no | |

La obligatoriedad está en el parser, no en el JS:

- Sin `format` → `AF600104` (`UrlParametersToSign.java:294`-`296`).
- Sin `algorithm` → `AF600106`; con uno fuera de la lista → `AF600107`
  (`UrlParametersToSign.java:302`-`308`).
- Algoritmos admitidos, exactamente estos doce:
  `SHA1`, `SHA256`, `SHA384`, `SHA512`, y `SHA{1,256,384,512}with{RSA,ECDSA}`
  (`UrlParametersToSign.java:64`-`77`).
- Sin `dat` ni `fileid`, la operación no tiene de dónde sacar los datos
  (`AF600100`, `UrlParameters.java:311`). El JS omite `dat` cuando la cadena es
  vacía (`autoscript.js:2275`).
- `properties` se decodifica con `AOUtil.base642Properties`, es decir un fichero
  `.properties` de Java en Base64; si no parsea, **se ignora en silencio** y se
  sigue con propiedades vacías (`UrlParametersToSign.java:312`-`329`).
- El valor de `dat` que empiece por `file:/` se rechaza explícitamente
  (`UrlParameters.java:346`-`351`): es la defensa contra leer ficheros locales
  por orden de la sede. rfirma debe replicarla.

**Respuesta** (`NativeSignDataProcessor.postProcess`,
`NativeSignDataProcessor.java:114`-`128`), con `RESULT_SEPARATOR = '|'`
(`:24`):

```
<certificado DER en B64 URL-safe> | <firma en B64 URL-safe> [ | <extraData en B64 URL-safe> ]
```

El tercer campo es un JSON con datos extra —el nombre del fichero cargado, por
ejemplo— y **sólo se emite desde protocolo 3** (`NativeSignDataProcessor.java:124`).
El cliente lo parte por el primer y segundo `|`
(`processSignResponse`, `autoscript.js:2874`-`2908`); si no hay ningún `|`, trata
el todo como una firma y le hace un `Base64.decode` extra
(`autoscript.js:2891`) — camino que sólo se da en los transportes viejos.

**Sobre `cosign` en PAdES.** No es una cofirma CAdES: en PDF, cofirmar es
**volver a firmar**. `AOPDFSigner.cosign` delega literalmente en `sign`, en sus
dos sobrecargas (`<clienteafirma>/afirma-crypto-pdf/src/main/java/es/gob/afirma/signers/pades/AOPDFSigner.java:289`-`296`
y `:330`-`336`), y el javadoc lo dice: «*las multifirmas en los ficheros PDF se
limitan a firmas independientes «en serie»*» (`:298`-`301`). Para rfirma, `cosign`
con `format=PAdES` es **la misma ruta de código que `sign`**; lo único que cambia
es que los datos de entrada ya son un PDF firmado. `countersign` sí es
«operación no soportada para firmas PAdES» (`AOPDFSigner.java:338`-`342`).

Las cadenas de formato son `"PAdES"`, `"CAdES"`, `"XAdES"` y `"PAdEStri"`
(`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/signers/AOSignConstants.java:35`,
`72`, `111`, `114`).

### 3.4 La conversación asíncrona: `#wait` y `getresult?`

Es parte del contrato mínimo, no un extra, porque una firma con PIN tarda más de
lo que aguanta un websocket.

Con `jvc > 3`, AutoFirma **no** contesta la firma: lanza un hilo, guarda el
resultado indexado por `idsession` y responde inmediatamente `#wait`
(`WebSocketServerOperationHandler.java:35`-`52`; `WAIT_RESPONSE` en `:18`). El
cliente, al ver `#wait`, espera `AUTOFIRMA_GETRESULT_TIME = 2000` ms
(`autoscript.js:2068`) y manda `getresult?idsession=<id>`
(`autoscript.js:2634`-`2639`). Cada sondeo devuelve `#wait` mientras el hilo
siga vivo —y también si no hay hilo para ese id— y el resultado real cuando
termina (`WebSocketServerOperationHandler.java:66`-`81`).

En el camino síncrono (`jvc <= 3`) el servidor sube el *timeout* del websocket a
60 s, o 240 s si la petición es de lote (`AfirmaWebSocketServerV4Sup.java:100`-`102`).

---

## 4. Los filtros de certificado

### La sintaxis

Los filtros **no son un parámetro propio**: viajan dentro de `properties`, como
claves del `.properties` que la sede codifica en Base64. `CertFilterManager` los
recoge en tres formas alternativas, por orden de precedencia
(`<clienteafirma>/afirma-keystores-filters/src/main/java/es/gob/afirma/keystores/filters/CertFilterManager.java:165`-`182`):

1. `filter=<expresión>`
2. `filters=<expresión>`
3. `filters.1=<expresión>`, `filters.2=…`, numerados desde 1 y sin huecos.

Cada expresión es una **conjunción** de criterios separados por `;`
(`FILTERS_SEPARATOR`, `:35`; el `split` en `:189`). Cuando hay varias expresiones
numeradas, entre ellas la relación es **disyuntiva** («filtros disyuntivos», `:159`).
Los criterios, con su prefijo literal (`CertFilterManager.java:39`-`68`):

| prefijo | qué hace |
| --- | --- |
| `dnie:` | certificado de firma del DNIe |
| `ssl:` | certificado SSL |
| `qualified:` | cualificado, con el valor como argumento |
| `signingcert:` / `authcert:` | certificado de firma / de autenticación |
| `nonexpired:` | `true` muestra los caducados, `false` los oculta |
| `sscd:` | dispositivo cualificado de creación de firma |
| `subject.rfc2254:` / `issuer.rfc2254:` | filtro LDAP RFC 2254 sobre el DN |
| `issuer.rfc2254.recurse:` | igual, recorriendo la cadena de emisores |
| `subject.contains:` / `issuer.contains:` | subcadena literal |
| `thumbprint:` | huella; admite `<algoritmo>:<huella>` |
| `policyid:` | lista de OID de política separados por comas |
| `pseudonym:` | seudónimo |
| `encodedcert:` | el certificado entero, en Base64 |
| `keyusage.<bit>:` | los nueve bits de `KeyUsage`, agrupados en un patrón |
| `disableopeningexternalstores` | prohíbe abrir otros almacenes desde el diálogo |

Hay dos claves más que viajan por el mismo canal y cambian el comportamiento del
diálogo (`CertFilterManager.java:29`-`30`, `146`-`156`):

- `headless=true` → selecciona automáticamente, sin diálogo.
- `mandatoryCertSelection=false` → lo mismo.

Y una regla por defecto que es una decisión, no un accidente: **si la sede no
declara ningún filtro, se añade uno que oculta los caducados**, citando
ETSI TS 119 102-1 (`CertFilterManager.java:129`-`136`).

### Qué se espera que haga el cliente

Aplicar los filtros **al listado que se enseña**, no a la firma. El manager
produce una lista de `CertificateFilter` más dos banderas, y las tres cosas se le
pasan al diálogo de selección: `filters`, `mandatoryCertificate` y
`allowOpenExternalStores`
(`ProtocolInvocationLauncherSelectCert.java:125`-`127`, `165`-`170`;
`ProtocolInvocationLauncherSign.java:506`, `640`-`644`). Si tras filtrar no queda
ningún certificado, el error es `AF502001` en `selectcert` y `AF501001` en `sign`
(`SimpleErrorCode.java:84`, `88`).

### Contraste con `pkcs11::stores` de rfirma

**No se solapan: son ejes distintos, y el que hace falta es el que no existe.**

`<rfirma>/rfirma-app/src-tauri/src/pkcs11/stores.rs` responde a **dónde** buscar
—qué módulo PKCS#11, qué perfil NSS— y no filtra certificados en absoluto: es
`CANDIDATE_MODULES`, `CANDIDATE_SOFTOKENS`, `StoreClass` y la resolución de
`profiles.ini`. Lo más parecido a un filtro que hay hoy en rfirma vive en
`<rfirma>/rfirma-app/src-tauri/src/pkcs11/mod.rs` y son dos criterios fijos,
ninguno configurable por nadie:

- **tener clave privada** (ID-07), que es lo que separa `signable_certificates`
  de `list_certificates_unfiltered_for_test` (`mod.rs:274`-`285`);
- **tener `CKA_LABEL` no vacía**, porque el ADR-0010 sólo deja persistir la
  etiqueta y ofrecer lo que no se puede reencontrar sería prometer de más
  (`mod.rs:258`-`262`).

El estado del certificado sí se calcula —`CertificateStatus` distingue vigente,
caducado, aún no válido y revocado (`pkcs11/certificate.rs:147`-`173`)—, así que
el filtro por defecto de AutoFirma (`nonexpired`) es **barato**: ya hay con qué.
Todo lo demás —`subject.contains`, `issuer.rfc2254`, `policyid`, `keyusage.*`,
`thumbprint`— exige leer campos del X.509 que hoy no se leen, y `headless` /
`mandatoryCertSelection` exigen un camino de selección **sin diálogo** que hoy no
existe.

Conclusión operativa: los filtros son **el trabajo de v0.5 que menos se parece a
lo que ya hay** y el que más fácil es subestimar. Un subconjunto honesto para el
mínimo sería `nonexpired`, `subject.contains`, `issuer.contains`, `thumbprint` y
`encodedcert`, más `headless`/`mandatoryCertSelection`; el resto puede
degradar a «no filtro» **siempre que se registre**, porque un filtro ignorado
enseña certificados que la sede no quería y el usuario firmará con el que no
debe.

---

## 5. Los códigos de error

### El formato del mensaje

Uno solo, de una línea, y es lo que va por el socket
(`ProtocolInvocationLauncherErrorManager.getErrorMessage`, `:320`-`351`).

**Desde protocolo 4.1** (`PROTOCOL_VERSION_WITH_ERROR_CODES`, `:277`):

```
err-00:=AF<código de 6 dígitos> - <descripción>
```

con el prefijo `err-11:=` en lugar de `err-00:=` **sólo** para la cancelación del
usuario (`:38`-`41`, `330`-`334`).

**Antes de 4.1**, el mensaje es el formato viejo: `SAF_NN: <texto>`, sacado de una
tabla de correspondencias que colapsa decenas de códigos nuevos en unos pocos
antiguos (`OLD_ERRORS_ASSOCIATION`, `:150` en adelante; los `SAF_` en `:49`-`94`),
y la cancelación es la cadena desnuda `CANCEL` (`:47`, `:344`-`346`).

El cliente parsea así (`processResponse`, `autoscript.js:2626`-`2685`):

- `CANCEL`, `null` o vacío → cancelación
  (`es.gob.afirma.core.AOCancelledOperationException`, `AS500001`).
- `#wait` → sondear con `getresult?`.
- empieza por `err-` (minúsculas) **y contiene `:=`** → error con código. **El
  código son exactamente 8 caracteres** y el separador es ` - ` en la posición 8;
  si no cuadra, todo se toma como mensaje y el código se pierde
  (`autoscript.js:2643`-`2660`). `err-11:` se traduce a
  `AOCancelledOperationException`.
- `MEMORY_ERROR` → `AS620018`.
- empieza por `SAF_` → error del formato viejo.
- `NULL` → `AS620019`.

> Los ocho caracteres son `AF` + seis dígitos. Es una restricción de formato
> **rígida** en el cliente: un código de otra longitud se descarta en silencio.

### El catálogo

Los códigos `AF` son la concatenación del prefijo `AF`
(`ProtocolInvocationLauncherErrorManager.java:38`) con el código numérico de
`ErrorCode` (`<clienteafirma>/afirma-core/src/main/java/es/gob/afirma/core/ErrorCode.java`)
o de `SimpleErrorCode` (`<clienteafirma>/afirma-simple/src/main/java/es/gob/afirma/standalone/SimpleErrorCode.java`).
El primer dígito clasifica la familia: 1 hardware, 2 interno, 3 terceros,
4 comunicación, 5 funcional, 6 petición (`ProtocolInvocationLauncherErrorManager.java:359`-`377`).

**Los que rfirma tiene que saber producir** —los alcanzables desde las cuatro
operaciones del mínimo—:

| código | cuándo | fuente |
| --- | --- | --- |
| `AF500001` | el usuario cancela (con prefijo `err-11:=`) | `ErrorCode.java:174` |
| `AF501001` | no hay certificados tras filtrar, al firmar | `SimpleErrorCode.java:84` |
| `AF502001` | no hay certificados tras filtrar, en `selectcert` | `SimpleErrorCode.java:88` |
| `AF500005` | la sede exigió una versión de cliente mayor (`mcv`) | `SimpleErrorCode.java:80` |
| `AF600002` | operación no soportada | `ErrorCode.java:195` |
| `AF600100` | ni `dat` ni `fileid` | `ErrorCode.java:198` |
| `AF600104` / `AF600105` | falta el formato / formato no soportado | `ErrorCode.java:202`-`203` |
| `AF600106` / `AF600107` | falta el algoritmo / algoritmo no soportado | `ErrorCode.java:204`-`205` |
| `AF600120` | `idsession` de la firma inválido | `ErrorCode.java:216` |
| `AF600121` | alguna propiedad de firma es incorrecta | `ErrorCode.java:196` |
| `AF620009` | el esquema de la URL no es `afirma://` | `SimpleErrorCode.java:112` |
| `AF620010` | el `idsession` del websocket no coincide | `SimpleErrorCode.java:113` |
| `AF620011` | versión de protocolo no soportada | `SimpleErrorCode.java:114` |
| `AF620012` | no llegó URI de invocación | `SimpleErrorCode.java:115` |
| `AF620013` | no llegaron puertos (sólo `afirma://service`) | `SimpleErrorCode.java:116` |
| `AF420001` | petición desde una IP que no es 127.0.0.1 | `SimpleErrorCode.java:70` |
| `AF220001` | no se pudo abrir el socket en ningún puerto | `SimpleErrorCode.java:40` |
| `AF220008` | no se pudo cargar el almacén SSL del websocket | `SimpleErrorCode.java:44` |
| `AF200108` | no se pudo acceder a la clave de firma (PIN) | `ErrorCode.java:89` |
| `AF200110` | clave incompatible con el algoritmo | `ErrorCode.java:91` |
| `AF200111` | error generando el PKCS#1 | `ErrorCode.java:92` |
| `AF200115` | error desconocido firmando | `ErrorCode.java:96` |

Los `SAF_NN` sólo hacen falta si rfirma decide aceptar `v=3`. La equivalencia
está tabulada en `OLD_ERRORS_ASSOCIATION` y el desconocido es `SAF_53`
(`ProtocolInvocationLauncherErrorManager.java:94`, `:339`-`342`).

**Lo que rfirma NO emite nunca**: los códigos `AS…`. Son del catálogo del propio
`autoscript.js` (`autoscript.js:337`-`433`) y los produce el navegador cuando la
aplicación no responde, se cierra el socket o la respuesta no es interpretable.
Aparecen en el `errorCallback` de la sede como si fueran errores del cliente,
pero rfirma sólo los provoca **por omisión**: `AS420002` si cierra el socket
(`autoscript.js:2538`-`2545`), `AS620017` si no llega a abrirlo.

---

## 6. Qué queda fuera del mínimo

Todo esto es de la **ficha 18b**, y ninguna sede que use el transporte websocket
v4 con las cuatro operaciones del mínimo lo necesita:

- **Los servlets del servidor intermedio.** El transporte `AppAfirmaJSSocket` /
  `AppAfirmaJSWebService` (`autoscript.js:2987` en adelante, `:4189` en adelante)
  sube los datos a un servidor de la sede y AutoFirma se los descarga con
  `fileid`, `rtservlet` y `stservlet` (`ProtocolInvocationLauncher.java:726`-`754`).
  Arrastra consigo `setServlets`, la espera activa (`ActiveWaitingThread`) y el
  troceado de peticiones largas (`autoscript.js:3612`, `3736`). Es el camino de
  los navegadores viejos; ninguno actual lo toma.
- **El cifrado extremo a extremo.** El parámetro `key`/`cipherconfig` y el
  `ServerCipher` (`NativeSignDataProcessor.java:72`-`112`;
  `UrlParameters.java:270`). Sólo tiene sentido si hay servidor intermedio: la
  respuesta viaja por él y no por `wss://127.0.0.1`. Sin servlets, sobra.
- **`batch`** (`afirma://batch?`, `ProtocolInvocationLauncher.java:336`), en sus
  dos sabores XML y JSON (`autoscript.js:2178`-`2196`), con sus URL de prefirma y
  posfirma y el modo `localBatchProcess`. Respuesta con formato propio
  (`processBatchResponse`, `autoscript.js:2838`).
- **`save`** (`afirma://save?`, `:521`) y **`signandsave`** (`:616`).
- **`load` y `multiload`** (`afirma://load?`, `:813`), con su respuesta
  `nombre:datos|nombre:datos` (`processLoadResponse`, `autoscript.js:2739`).
- **`countersign`.** Está en el mismo `if` que `sign` y `cosign`
  (`ProtocolInvocationLauncher.java:715`) y es trivial de enrutar, pero en PAdES
  es «operación no soportada» por definición (`AOPDFSigner.java:338`-`342`), así
  que en el alcance PAdES de rfirma su implementación correcta es devolver
  `AF600002`.
- **El transporte `afirma://service`** (sockets HTTP locales en claro,
  `autoscript.js:3310`; `ProtocolInvocationLauncher.java:301`). Es el predecesor
  del websocket y `autoscript.js` sólo lo elige en navegadores que ya no
  importan.

---

## Lo que esto deja decidido para el spec de la v0.5

1. rfirma registra el esquema `afirma://` y, al recibir
   `afirma://websocket?ports=…`, abre un **servidor WebSocket sobre TLS** en el
   primero de los tres puertos que consiga, atado a 127.0.0.1.
2. El certificado TLS de ese `wss://` es exactamente el problema que el ADR-0005
   y el instalador del hito v0.4 resuelven: sin CA en el almacén del sistema, el
   navegador no abre el socket. Esto **confirma** que la v0.4 es la puerta de la
   v0.5.
3. Se anuncia como protocolo **4.1** y se implementa el par `#wait` /
   `getresult?`, que no es opcional: una firma con PIN no cabe en el *timeout*
   síncrono.
4. Se implementan cuatro mensajes: eco, `selectcert`, `sign` y `cosign` —y este
   último, en PAdES, es la misma ruta que `sign`—.
5. Se implementan las dos guardias de seguridad del original —`idsession` en
   cada mensaje y origen 127.0.0.1— y la tercera, la que prohíbe `dat` con
   `file:/`.
6. Los filtros de certificado son la pieza nueva de verdad: rfirma hoy no filtra
   por ningún criterio que la sede pueda expresar, y lo que ya tiene
   (`CertificateStatus`) sólo cubre el filtro por defecto.

## Discoveries

- `docs/AGENTS.md` no listaba este informe; se añade en la misma rama.
