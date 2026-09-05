# El `Origin` del saludo WebSocket, y si se puede creer

Sondeo del issue [#312](https://github.com/sgomez/rfirma/issues/312), del mapa
[#308](https://github.com/sgomez/rfirma/issues/308). La pantalla de
consentimiento de la v0.5 —la del [#317](https://github.com/sgomez/rfirma/issues/317)—
mejora muchísimo si puede decir **qué sede pide la firma**, y empeora si enseña
un dato falsificable como si fuera un hecho. Esto mide qué llega de verdad en la
cabecera `Origin`, qué garantiza, y qué hace con ella el original.

Va detrás de `docs/research/contrato-protocolo-afirma.md`, que ya describe el
arranque por `afirma://websocket?ports=…`, el `idsession` y el resto del
contrato. Aquí sólo se mira el **saludo**.

Entorno de la medición: Ubuntu 26.04, Chromium **148.0.7778.280** (el del panel
de navegador de esta sesión), y un servidor WebSocket de veinte líneas escrito
para la ocasión que imprime el saludo crudo antes de aceptarlo. Código de
AutoFirma leído en `/home/sergio/Developer/SideProjects/clienteafirma`, rama de
trabajo local.

---

## Veredicto

**Sí llega, siempre, y con el nombre de la sede dentro. Y no vale como prueba de
nada.**

Las cuatro respuestas en una tabla:

| | Respuesta corta |
|---|---|
| 1. Qué manda el navegador | `Origin: https://<host>[:<puerto>]` — esquema, host y puerto de la página, sin ruta ni consulta. Nada más: **ni `Sec-Fetch-*`**, medido. |
| 2. Qué garantiza | Que **si quien llama es un navegador**, esa es la página. Un programa cualquiera de la máquina pone lo que quiera: dos líneas de Python, medido. |
| 3. Qué hace el original | **Nada.** `AfirmaWebSocketServer` recibe el `ClientHandshake` y no lo mira. En el transporte HTTP viejo hasta responde `Access-Control-Allow-Origin: *`. |
| 4. Qué cambia el permiso de red local | Sobre la **garantía**, nada: no añade ninguna señal al servidor local. Sobre el **valor**, sí: obliga a que el origen sea `https://`, y hace que el navegador enseñe el nombre de la sede a la persona **antes** que rfirma. |

La consecuencia para el #317: **la pantalla puede nombrar a la sede, pero
atribuyendo el nombre, no afirmándolo**. La recomendación completa, con la
redacción, está al final.

---

## 1. Qué manda el navegador

`autoscript.js` abre el socket con el constructor pelado, sin cabeceras —la API
no admite ninguna—:

```js
webSocket = new WebSocket(URL_REQUEST_PREFIX + port);   // autoscript.js:2523
```

con `URL_REQUEST_PREFIX = "wss://" + SERVER_HOST + ":"` y
`SERVER_HOST = "127.0.0.1"` (`autoscript.js:2051`-`2053`). El `Origin` no lo
pone el guion: lo pone el navegador.

### Lo medido

Una página servida en `http://127.0.0.1:8081` que hace
`new WebSocket("ws://127.0.0.1:8899")`. Esto es lo que llegó al servidor, tal
cual:

```
GET / HTTP/1.1
Host: 127.0.0.1:8899
Connection: Upgrade
Pragma: no-cache
Cache-Control: no-cache
User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.7778.280 Safari/537.36
Upgrade: websocket
Origin: http://127.0.0.1:8081
Sec-WebSocket-Version: 13
Accept-Encoding: gzip, deflate, br, zstd
Accept-Language: en-US
Sec-WebSocket-Key: VOvW19qKrBENDtokJFPzkw==
Sec-WebSocket-Extensions: permessage-deflate; client_max_window_bits
```

Tres cosas de aquí:

- **El `Origin` es el origen serializado de la página**: esquema, host y puerto,
  sin ruta y sin consulta. Es decir, la sede llega **por dominio**, que es
  exactamente lo que la pantalla necesitaría. En el caso real de una sede el
  valor será `https://sede.ejemplo.gob.es`, sin puerto por ser el 443.
- **No hay ninguna cabecera `Sec-Fetch-*`.** Esto contradice lo que se deduce de
  las especificaciones —Fetch Metadata enumera `websocket` entre los valores
  válidos de `Sec-Fetch-Mode`
  ([§2.2](https://w3c.github.io/webappsec-fetch-metadata/#sec-fetch-mode-header)),
  y Fetch las añade en *HTTP-network-or-cache fetch* justo detrás del `Origin`
  ([§4.5](https://fetch.spec.whatwg.org/#http-network-or-cache-fetch))—, pero la
  medición manda sobre la deducción: **Chromium 148 no las envía en un saludo
  WebSocket**. No hay señal extra que aprovechar.
- El `User-Agent` viaja, y es tan falsificable como el `Origin`. No sirve de
  refuerzo.

### Por qué llega siempre, y por qué el guion no puede tocarlo

Dos hechos independientes de la especificación, y los dos aplican:

- El `Origin` de un WebSocket se añade **incondicionalmente**. Fetch,
  [«append a request `Origin` header»](https://fetch.spec.whatwg.org/#append-a-request-origin-header):
  «If request's response tainting is "cors" or request's mode is either
  "websocket" or "webtransport", then append (`Origin`, serializedOrigin)». Y el
  modo `"websocket"` lo fija el propio estándar de WebSockets al construir la
  petición ([§2.2](https://websockets.spec.whatwg.org/#the-websocket-connection)).
- `Origin` es **forbidden request-header**
  ([Fetch §2.2.2](https://fetch.spec.whatwg.org/#forbidden-request-header)): el
  script no puede fijarla. Y la API `WebSocket()` ni siquiera acepta cabeceras:
  su constructor sólo toma `url` y `protocols`.

El origen que se serializa es el del *relevant settings object* del
`WebSocket`, o sea el del documento que lo crea
([WebSockets §3.1](https://websockets.spec.whatwg.org/#dom-websocket-websocket)).

### El caso `null`

Un origen opaco se serializa como la cadena `"null"`
([HTML, serialización de un origen](https://html.spec.whatwg.org/multipage/browsers.html#ascii-serialisation-of-an-origin)).
Se llega a él desde un `<iframe sandbox>` sin `allow-same-origin`
([HTML, *sandboxed origin browsing context flag*](https://html.spec.whatwg.org/multipage/browsers.html#sandboxed-origin-browsing-context-flag)),
desde un documento `data:` y, según la implementación, desde `file:`
([URL §4.5](https://url.spec.whatwg.org/#concept-url-origin), que para `file`
dice literalmente que se deja como ejercicio al lector y que ante la duda se
devuelva un origen opaco).

**Este caso no se ha podido medir.** El intento —un `<iframe sandbox="allow-scripts">`
apuntando a una página que abre el socket— murió antes de llegar al origen: el
navegador del panel devolvió `net::ERR_BLOCKED_BY_CLIENT` al cargar el propio
`iframe`, que es una restricción del arnés de esta sesión y no del modelo de
orígenes. Queda como **hueco de la medición**: se sabe por especificación que el
valor sería `null`, no se ha visto llegar.

---

## 2. Qué garantiza, y cómo de falsificable es

### Lo que garantiza

Lo que dice el RFC 6455, y no más. [§10.2 *Origin
Considerations*](https://www.rfc-editor.org/rfc/rfc6455#section-10.2):

> «The intent is not to prevent non-browsers from establishing connections but
> rather to ensure that trusted browsers under the control of potentially
> malicious JavaScript cannot fake a WebSocket handshake.»

Es decir: la cabecera existe para que **una página no pueda hacerse pasar por
otra dentro de un navegador**. No para autenticar a quien llama.

Y el propio RFC lo dice sin rodeos en [§10.1 *Non-Browser
Clients*](https://www.rfc-editor.org/rfc/rfc6455#section-10.1):

> «Such hosts are acting on their own behalf and can therefore send fake
> |Origin| header fields, misleading the server.»

El requisito de mandarla es **sólo para navegadores**
([§4.1](https://www.rfc-editor.org/rfc/rfc6455#section-4.1), requisito 8: «MUST
include […] if the request is coming from a browser client»), y del lado
servidor es **opcional** recibirla
([§4.2.1](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.1), requisito 7:
«Optionally, an |Origin| header field»), con la coletilla útil: «A connection
attempt lacking this header field SHOULD NOT be interpreted as coming from a
browser client».

Formulado como sirve para la pantalla: **el `Origin` es una afirmación de quien
llama sobre sí mismo, que sólo un navegador está obligado a hacer con
sinceridad.** Es la diferencia entre «esta es la sede» y «si quien llama es un
navegador, esta es la sede».

### Cómo de falsificable: medido

Trivialmente. Un cliente de doce líneas en Python, sin biblioteca, con el
`Origin` inventado:

```
GET / HTTP/1.1
Host: 127.0.0.1:8899
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: +6vzqwR0u9BDOU5j8WWxGQ==
Sec-WebSocket-Version: 13
Origin: https://sede.example-administracion.gob.es
```

→ `HTTP/1.1 101 Switching Protocols`. Y el mismo cliente **sin** la cabecera
también completa el saludo: no es obligatoria para nadie que no sea un
navegador.

### Y el atacante que importa ya está dentro

La guardia de AutoFirma que sí existe —y que el contrato ya da por copiada— es
que la petición venga de `127.0.0.1`
(`AfirmaWebSocketServerV4Sup.java:69`-`75`). O sea que el falsificador tiene que
ser **un programa de la misma máquina**. Eso no lo salva: un programa local
puede además invocar él mismo `afirma://websocket?ports=…&idsession=…`, con lo
que **conoce el `idsession`** y las dos guardias del original le dan igual.

De ahí sale el problema real de la pantalla: no es que el `Origin` sea inútil,
es que **enseñarlo como un hecho convierte la pantalla en el altavoz de la
mentira**. Un malware local pasa de pedir «algo quiere firmar» a pedir «la sede
de Hacienda quiere firmar», y lo firma la propia interfaz de rfirma.

---

## 3. Qué hace el original con ella

**Nada. En ninguno de los dos transportes.**

### El WebSocket

`AfirmaWebSocketServer.onOpen` recibe el saludo entero y no lo toca
(`AfirmaWebSocketServer.java:86`-`93`):

```java
@Override
public void onOpen(final WebSocket ws, final ClientHandshake handshake) {
    LOGGER.info("Apertura del socket del puerto " + getAddress().getPort());
    if (this.wsClient == null) {
        this.wsClient = ws;
    }
}
```

El parámetro `handshake` **no se usa**. Y no hay ninguna comprobación en otro
sitio: un `grep -rn 'Origin'` sobre `afirma-simple` devuelve una sola línea, y es
la del transporte HTTP (abajo). Tampoco se sobrescribe el gancho que la
biblioteca reserva para eso.

### Qué hace la biblioteca por su cuenta: tampoco nada

`org.java-websocket:Java-WebSocket` **1.6.0** (`afirma-simple/pom.xml:234`-`237`).
Un `grep -i origin` sobre las 78 fuentes de la 1.6.0 devuelve una única
coincidencia, y es la palabra «original» dentro de un comentario. La
implementación por defecto del gancho de saludo devuelve una respuesta vacía sin
mirar cabeceras
([`WebSocketAdapter.onWebsocketHandshakeReceivedAsServer`, v1.6.0](https://github.com/TooTallNate/Java-WebSocket/blob/v1.6.0/src/main/java/org/java_websocket/WebSocketAdapter.java#L47-L56)),
y `WebSocketServer` no la sobrescribe. `Draft_6455.acceptHandshakeAsServer`
comprueba versión, extensión y subprotocolo, y nada más
([v1.6.0](https://github.com/TooTallNate/Java-WebSocket/blob/v1.6.0/src/main/java/org/java_websocket/drafts/Draft_6455.java#L261-L285)).

O sea: **AutoFirma acepta el saludo de cualquier página del navegador de la
persona.** Si algún día rfirma quisiera mirar la cabecera, se lee con
`handshake.getFieldValue("Origin")` —búsqueda insensible a mayúsculas, y
**devuelve cadena vacía cuando falta**, no `null`
([`HandshakedataImpl1`, v1.6.0](https://github.com/TooTallNate/Java-WebSocket/blob/v1.6.0/src/main/java/org/java_websocket/handshake/HandshakedataImpl1.java#L45-L67))—,
pero eso es la biblioteca de Java, y rfirma no la usa.

### El transporte HTTP viejo

`ServiceInvocationManager` es el predecesor del WebSocket —sockets HTTPS locales,
`afirma://service`— y ahí la postura es **explícita**, no un olvido: cada
respuesta lleva

```java
sb.append("Access-Control-Allow-Origin: *\n");   // CommandProcessorThread.java:510
```

Es la declaración de que cualquier origen vale. Está fuera del mínimo de la v0.5
(§6 del contrato del protocolo), pero fija el criterio del original: **el origen
no es una credencial en AutoFirma, y nunca lo ha sido**.

### Que no lo compruebe no es un argumento para ignorarlo

Dos cosas distintas que conviene no mezclar:

- **Como control de acceso**, ignorarlo es lo mismo que hace el original, y es
  defendible: la lista blanca no existe. No hay ni puede haber un censo de sedes
  electrónicas españolas, y AutoFirma es genérico por diseño. El RFC 6455 §10.2
  recomienda verificar el origen a los servidores «not intended to process input
  from any web page but only for certain sites» —y rfirma **es** de los
  primeros—.
- **Como dato que se le enseña a la persona**, el original no tiene opinión
  porque su pantalla no lo enseña. Ahí no hay precedente que copiar: es una
  decisión nueva del #317.

---

## 4. Qué cambia el permiso de red local

Para el detalle de lo que ve la persona y de qué pasa cuando deniega está el
[#309](https://github.com/sgomez/rfirma/issues/309), que es su ticket. Aquí sólo
la pregunta de este sondeo: **si cambia algo de las garantías del origen.**

### Sobre la garantía: nada

*Local Network Access* es un modelo de **permiso**, no de *preflight*. El
borrador lo dice al presentarse: «This proposal builds on top of Chrome's
previously paused [PRIVATE-NETWORK-ACCESS] work but differs by gating access on a
permission rather than via preflight requests»
([WICG, Introduction](https://wicg.github.io/local-network-access/#introduction)).

Consecuencia directa: **el servidor local no recibe ninguna cabecera nueva**. Ni
la `Access-Control-Request-Private-Network` de la propuesta anterior ni ninguna
otra; el borrador no menciona ninguna. Lo que llega a rfirma es el mismo saludo
de la §1, con el mismo `Origin` y la misma credibilidad. **LNA decide *si* la
conexión sale del navegador, no *qué* le llega al servidor.**

Y aplica a WebSocket sin tocar nada del lado del socket
([§3.3](https://wicg.github.io/local-network-access/#integration-with-websockets)):
«WebSocket opening handshake directly applies fetch in step 11, and so no
modification to the WebSocket specification is required».

### Sobre el valor del dato: sí, dos cosas

Aunque la garantía no mejore, el contexto sí:

1. **El origen que llega de un navegador es forzosamente `https://`.** LNA
   [§2.4](https://wicg.github.io/local-network-access/#secure-context-restriction):
   «The capability to make local network requests is a powerful feature and must
   only be allowed from secure contexts». Sumado a que una página `https` no
   puede abrir un `ws://` en claro, el par sede pública + `wss://127.0.0.1` sólo
   existe con la sede en `https`. Traducido: **un `Origin` que no empiece por
   `https://` no lo ha podido producir el camino esperado.** Es lo único
   parecido a una comprobación que rfirma puede hacer sin lista blanca —y sigue
   sin distinguir a un navegador de un programa que finja bien—.
2. **El navegador enseña el nombre de la sede antes que rfirma.** El permiso se
   pide por origen y el usuario decide sobre ese nombre, así que cuando la
   ventana de rfirma aparece, la persona **ya ha visto** el dominio en un aviso
   del navegador, que sí es de fiar. El alcance exacto es de implementación
   («The exact scope of the permission is implementation-defined»,
   [§2.3](https://wicg.github.io/local-network-access/#local-network-request-permission-prompt)),
   pero el nombre está.

### Estado

En Chrome el permiso llegó a estable en la **142** (octubre de 2025), pero los
WebSocket quedaron **fuera** de forma explícita: «WebSockets
([crbug.com/421156866](https://crbug.com/421156866)), WebTransport […] and WebRTC
connections to the local network are not yet gated on the LNA permission»
([blog de Chrome, act. 29-09-2025](https://developer.chrome.com/blog/local-network-access)).

El cierre de ese hueco es el *Intent to Ship: Local network access restrictions
for WebSockets*
([blink-dev, 19-02-2026](https://groups.google.com/a/chromium.org/g/blink-dev/c/O6GMKt44Ups)),
con tres LGTM en marzo de 2026 y destino **Chrome 147**, escritorio y Android.
Así que a día de hoy el camino de la v0.5 **sí** pasa por el aviso de red local.
El navegador de esta medición es la 148; el sondeo no lo disparó porque una
página en `127.0.0.1` hablando con `127.0.0.1` no cruza ninguna frontera de
espacio de direcciones. El caso real —sede pública contra `wss://127.0.0.1`— es
el del #309.

---

## Lo que no sirve, y por qué

Antes de la recomendación, las salidas que se han mirado y descartado, para que
no se vuelvan a proponer:

- **Lista blanca de sedes.** No existe el censo, y AutoFirma es genérico. Sin
  lista, comprobar el origen no es comprobar nada.
- **La URL de arranque.** `afirma://websocket?ports=…&v=…&jvc=…&idsession=…`
  (`autoscript.js:2469`-`2474`) **no lleva ni un dato de la sede**. La cabecera
  del saludo es el único sitio del protocolo donde aparece el dominio.
- **`Sec-Fetch-*` o el `User-Agent` como refuerzo.** Las primeras no llegan
  (medido, §1); el segundo es igual de falsificable.
- **Preguntarle al sistema operativo quién tiene el otro extremo del socket.**
  En Linux se podría llegar del puerto al inodo y del inodo al proceso, y ver si
  es un navegador. **No se ha medido**, y en el canal flatpak no vale: el
  espacio de PID del *sandbox* no ve los procesos del anfitrión. Quedaría, como
  mucho, para el `.deb` y el `.rpm`, y una invariante que sólo se cumple en dos
  de los tres canales no es una invariante.

---

## Recomendación para el #317

**Sí: la pantalla nombra a la sede. Pero la atribuye, no la afirma.**

No enseñarla sería tirar el único dato que responde a la pregunta que la persona
se está haciendo —«¿esto lo he pedido yo?»—, y que además es correcto en el
99 % de los casos, que son los del navegador. Enseñarla como un hecho verificado
sería mentir, y prestarle a un malware local la credibilidad de la ventana de
firma. La salida no es elegir entre las dos: es **redactarla como lo que es**.

### Las tres reglas

1. **El dominio se muestra, con verbo de declaración.** Ni escudo, ni candado,
   ni «verificado», ni tipografía de sello. Del `Origin` se muestra el **host**;
   el esquema y el puerto se guardan para el registro.

2. **Debajo, la frase que la hace honesta y además accionable.** No basta con
   decir «no se puede comprobar»: hay que decir **quién sí puede**. La persona
   tiene delante la comprobación que a rfirma le falta —la pestaña desde la que
   acaba de pulsar «Firmar»—:

   > **sede.ejemplo.gob.es** pide firmar un documento.
   >
   > Ese nombre lo declara quien hace la petición y rfirma no puede
   > comprobarlo. Compruebe que coincide con la página desde la que lo ha
   > pedido.

   La segunda frase no es letra pequeña: es la que convierte un dato incompleto
   en una tarea que la persona sí sabe hacer.

3. **Sin origen es un caso distinto, y se ve distinto.** Si el `Origin` falta,
   vale `null`, o no empieza por `https://` —los tres son el mismo caso: *esto
   no lo ha producido el camino esperado*, §4—, no se inventa un nombre ni se
   deja el hueco en blanco:

   > La petición **no dice de qué página viene**.

   Con el peso visual de un aviso, no el de un campo vacío. La firma se sigue
   pudiendo consentir —cancelar es siempre la otra puerta—, pero la persona ve
   que le falta la mitad del dato.

### Lo que la recomendación *no* incluye

- **No rechazar el saludo por el `Origin`.** Ni cuando falta, ni cuando no es
  `https`. Contra un falsificador no sirve —pone `https://` y ya—, y contra una
  sede rara sí rompe. Su único efecto sería darle a quien lo escribe la
  sensación de haber puesto una defensa. Lo que sí se hace con el dato es
  **registrarlo entero** —esquema, host y puerto— para que un incidente se pueda
  reconstruir.
- **Ninguna invariante nueva.** La que sostiene esto ya está decidida y es más
  fuerte que cualquier cosa que el `Origin` pueda aportar: **una sede nunca
  provoca una firma silenciosa**. El nombre de la sede informa la decisión; no
  la sustituye. Que el dato sea parcial es exactamente por lo que la pantalla no
  puede desaparecer nunca —ni con `headless`, ni con un solo certificado—.

---

## Reproducirlo

Los tres guiones son de usar y tirar y no se versionan; están descritos aquí lo
bastante para reescribirlos en cinco minutos.

1. **El servidor.** Un `socket` que escucha en `127.0.0.1:8899`, lee hasta el
   `\r\n\r\n`, **imprime el saludo crudo**, y responde `101` calculando el
   `Sec-WebSocket-Accept` como `base64(sha1(clave + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))`.
   Aceptar sin mirar el `Origin` es justamente el punto.
2. **La falsificación.** Un `socket.create_connection` contra ese puerto que
   manda a mano `GET / HTTP/1.1` con `Upgrade`, `Connection`,
   `Sec-WebSocket-Key` (16 bytes al azar en base64), `Sec-WebSocket-Version: 13`
   y el `Origin` que se quiera. Y una segunda pasada **sin** `Origin`.
3. **El navegador.** Una página estática servida con
   `python3 -m http.server 8081 --bind 127.0.0.1` que hace
   `new WebSocket("ws://127.0.0.1:8899")`, abierta en el navegador. Lo que
   imprime el servidor es la §1.

El caso del origen opaco (`<iframe sandbox="allow-scripts">`) **no salió** en
este entorno: el arnés bloquea la carga del propio `iframe` con
`net::ERR_BLOCKED_BY_CLIENT`. Hace falta un navegador de verdad para cerrarlo.

## Discoveries

- `docs/AGENTS.md` no listaba este informe; se añade en la misma rama.
