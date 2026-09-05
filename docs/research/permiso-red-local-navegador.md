# El permiso de red local del navegador: qué ve la persona, y si la concesión sobrevive

Sondeo para el issue [#309](https://github.com/sgomez/rfirma/issues/309), del mapa
[#308](https://github.com/sgomez/rfirma/issues/308). Es **la puerta del hito v0.5**: el
[ADR-0005](../adr/0005-servidor-local-https-y-ca-en-los-almacenes-nss.md) deja escrito que
*Local Network Access* (LNA) se aplica por dirección de destino y no por esquema, y aplaza a
este sondeo el diseño del aviso.

**Registra hechos y deja una lista de comprobación manual; no decide** qué promete la v0.5:
eso son el [#316](https://github.com/sgomez/rfirma/issues/316) y el
[#317](https://github.com/sgomez/rfirma/issues/317).

Fecha del sondeo: **5 de septiembre de 2026**. En este entorno **no se pueden ejecutar
navegadores**, así que todo lo que sigue está leído de fuente documental —la especificación
de la WICG, las notas de versión y los sistemas de seguimiento de Chromium y Mozilla— o del
código fuente de AutoFirma. **Lo que no se puede leer de una fuente primaria está marcado
como no determinado y aparece en la lista de comprobación manual de la sección 6.**

## Veredicto

**El permiso existe, es ineludible, y sí se recuerda; pero el aviso no nombra a la
aplicación que escucha, y desde JavaScript no se distingue de «no hay nada escuchando».**

Cinco hechos gobiernan el hito:

1. El permiso lo pide **el origen de la sede**, no rfirma. rfirma no aparece en el aviso: el
   texto habla de «otras aplicaciones y servicios **de este dispositivo**», sin nombrar
   cuál.
2. **Son dos permisos, no uno.** La especificación separa `loopback-network` de
   `local-network`, y `127.0.0.1` cae en el primero. El aviso de *loopback* es el que nos
   toca, y su redacción es distinta —y menos alarmante— que la del de red local.
3. **La concesión se recuerda**, por origen de la sede, entre pestañas y entre sesiones, en
   los dos navegadores. En Firefox hay que marcar una casilla; en Chrome es automática.
4. **AutoFirma sufre exactamente lo mismo**, y peor: su `autoscript.js` colapsa *todos* los
   fallos de conexión en un único código `AS620017` «no se ha podido invocar a la aplicación
   nativa», tras **33 segundos** de reintentos. Es un problema del ecosistema, no de rfirma.
5. **La distinción «denegado» vs «no hay servidor» no se puede hacer desde el evento del
   WebSocket** —la especificación de WebSocket lo prohíbe—, pero **sí** con la Permissions
   API antes de conectar. Ese es el remedio, y es del lado de la sede, no de rfirma.

Y un corolario incómodo: **rfirma no controla el lado que puede arreglarlo.** Quien pinta el
mensaje de error es `autoscript.js`, que sirve la sede.

## 0. Qué había en el ADR-0005, y qué cambia

| Lo que dice el ADR-0005 | Estado tras este sondeo |
|---|---|
| LNA se aplica por dirección de destino, no por esquema | **Confirmado.** `wss://` con la CA instalada recibe el mismo trato que `ws://` |
| En vigor en Chrome 147 (7/4/2026) y Firefox 154 (18/8/2026), por defecto, cubriendo WebSocket | **Confirmado**, con matiz: la restricción general es de Chrome 142 y Firefox 153; **147 y 154 son las versiones que la extienden a WebSocket** |
| Sin la CA el saludo TLS ni siquiera completa | **Confirmado**, y es *anterior* al permiso: el orden real está en la sección 4 |
| «El puerto y el protocolo no son variables de diseño: `autoscript.js` tiene cableados `SERVER_HOST` y `URL_REQUEST_PREFIX`» | **El esquema y el host sí están cableados; el puerto no.** Ver sección 3: la sede sortea **tres puertos aleatorios** en 49152–65535 y se los pasa a la aplicación. `63117` es el valor por defecto **del lado Java**, no lo que pide el navegador |

Ese último es el único punto del ADR-0005 que este sondeo desmiente, y **no cambia ninguna
decisión** del ADR: sigue sin haber variable de diseño, sólo que la fija la sede en tiempo de
ejecución y rfirma tiene que aceptar una lista.

## 1. Qué ve la persona, literalmente

### 1.1 Son dos permisos distintos, y el nuestro es el de *loopback*

La especificación de la WICG lo dice en su algoritmo de comprobación:

> *Let permissionName be `"local-network"` if addressSpace is local, or `"loopback-network"`
> if addressSpace is loopback.*
> — [WICG, Local Network Access](https://wicg.github.io/local-network-access/), § 3.1

`127.0.0.1` es *loopback* (`127.0.0.0/8`, `::1/128`), así que **rfirma cae siempre en
`loopback-network`**. Los dos son *policy-controlled features* con lista blanca `["self"]`,
lo que importa para los `iframe` (sección 5.3).

Que Chrome implementa los dos por separado se comprueba fuera de la especificación: el
remedio documentado para las extensiones de Webflow bloqueadas en Chrome 147 es añadir
`allow="loopback-network"` al `iframe`
([webflow/mcp-server#124](https://github.com/webflow/mcp-server/issues/124)), y el mensaje de
consola que citan nombra el espacio:

> *has been blocked by CORS policy: Permission was denied for this request to access the
> `loopback` address space.*

### 1.2 El texto del aviso

| Navegador | Texto del aviso (*loopback*) | Texto del aviso (red local) |
|---|---|---|
| Chrome / Edge 147+ | *«`example.com` wants to… Access other apps and services on this device»* | *«Access other devices on your local network»* |
| Firefox 153+ | *«…wants access to other apps and services on your device»* | *«…wants access to local network devices»* |

Fuentes: la [documentación de LNA de QZ Tray](https://github.com/qzind/tray/wiki/LNA) —una
aplicación local con WebSocket seguro a *loopback*, el análogo técnico más cercano a
AutoFirma que hay documentado— y la
[guía de Box para sus dominios](https://docs.box.com/en/box-tools/using-box-tools/allow-box-domains-local-network-access-in-chrome-edge-and-firefox-to-avoid-box-tools-disruption).
El [blog de Chrome](https://developer.chrome.com/blog/local-network-access) recoge la
redacción del permiso **de red local**, *«Look for and connect to any device on your local
network»*, que es la que casi toda la prensa reprodujo y **no** es la que nos toca.

**El aviso no nombra la aplicación que escucha.** Ni «rfirma», ni «AutoFirma», ni el puerto,
ni el proceso. Nombra **el origen de la sede** —quien pide— y describe la capacidad en
abstracto. Esto es consecuencia directa del modelo: el permiso es del solicitante, y el
navegador no sabe, ni puede saber antes de conectar, qué hay al otro lado del puerto.

### 1.3 Dónde sale, y con qué botones

- **Chrome / Edge:** burbuja modal anclada bajo la barra de direcciones, botones **Allow** /
  **Block**. QZ Tray documenta una trampa importante:

  > *⚠️ WARNING: Ignoring this pop-up three time is equivalent to clicking 'block'!*

  Es la regla general de *permission fatigue* de Chrome, y significa que **cerrar el aviso
  tres veces deniega de forma persistente** sin que la persona haya dicho que no.
- **Firefox:** panel de permiso junto a la barra de direcciones, botones **Allow** / **Block**
  y **casilla «Remember my choice for this site»**. Sin marcarla, la denegación queda como
  **«Blocked Temporarily»** y el aviso vuelve a salir.

### 1.4 El caso que no sale ninguno

Chrome sólo pregunta desde **contexto seguro**. Una sede servida por HTTP en claro no recibe
aviso: la conexión **falla en silencio y la persona no tiene forma de recuperarla**
([intent to ship](https://groups.google.com/a/chromium.org/g/blink-dev/c/O6GMKt44Ups):
*«requires a Secure Context»*). Para rfirma esto es benigno —las sedes son HTTPS— pero es
la razón por la que el aviso es ineludible y no se puede esquivar bajando de esquema.

## 2. Si la concesión se recuerda

**Sí, y se guarda por origen de la sede.**

La especificación deja la persistencia como potestad del navegador:

> *A user agent may persist this decision to reduce permission fatigue.*
> — [WICG, Local Network Access](https://wicg.github.io/local-network-access/), § 2.3

Los dos navegadores la ejercen:

| | Chrome / Edge | Firefox |
|---|---|---|
| ¿Se recuerda? | Sí, automáticamente al pulsar *Allow* | Sí, **si se marca** «Remember my choice for this site» |
| Clave de la concesión | Origen de la sede | Origen de la sede |
| ¿Depende de la dirección de destino? | No: es el par (origen, espacio de direcciones), no (origen, IP:puerto) | Igual |
| Entre pestañas y sesiones | Sí | Sí |
| Dónde se gestiona | *Settings → Privacy and security → Site settings → Permissions → Additional permissions → Local network access*, renombrado a **«Apps on device»** en versiones posteriores a la 144 | `about:preferences#permissionsData`, entrada **«Local network devices»** |
| Cómo se retira o repara | Icono de la izquierda de la barra de direcciones | Icono de ajustes de la barra de direcciones; borrar la entrada *Blocked Temporarily* |

Que la clave sea **el espacio de direcciones y no la IP concreta** es el hecho más útil de
esta sección: significa que **la concesión no se pierde porque `autoscript.js` sortee otros
tres puertos en la siguiente visita** (sección 3), y que un solo *Allow* cubre cualquier
puerto de *loopback* que la sede intente después.

Cuánto dura la concesión de Chrome sin uso —Chrome caduca algunos permisos por inactividad—
**no está determinado** por fuente primaria. Va a la lista de comprobación.

## 3. Qué le pasa hoy a AutoFirma

Medido sobre el código, no sobre el navegador: checkout de `clienteafirma` en `master`,
`pom.xml` raíz en la versión **1.9.1**, HEAD diez commits por delante de `v1.9.1`
(22 de agosto de 2026). El JS de sede lleva versionado propio,
`autoscript.js:26` → `var VERSION = "1.10.1";`.

### 3.1 Lo que hace la sede

`afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js`:

```js
var SERVER_HOST = "127.0.0.1";                          // :2051
var URL_REQUEST_PREFIX = "wss://" + SERVER_HOST + ":";  // :2053
...
webSocket = new WebSocket(URL_REQUEST_PREFIX + port);   // :2523
```

- **Tres puertos aleatorios** del rango 49152–65535 (`getRandomPorts`, `:1955-1978`; límites
  en `:1905-1911`), acotables por la sede con `setPortRange`.
- Se le pasan a la aplicación por el esquema propio:
  `afirma://websocket?ports=a,b,c&v=4&jvc=4&idsession=…` (`:2469-2472`).
- Espera **3000 ms** fijos (`:2430`) y luego sondea **los tres puertos** en cada intento
  (`:2487-2489`), **15 intentos cada 2000 ms** (`:264-267`). Ventana total ≈ **33 segundos**.
- El apretón de manos es un eco: `ws.send("echo=-idsession=" + idSession + "@EOF")` (`:2608`),
  que el lado Java contesta `"OK"` (`AfirmaWebSocketServer.java:144-147`). **La operación
  real no se envía hasta que vuelve el eco.**

### 3.2 Lo que ve la sede cuando falla

Y aquí está el oráculo:

```js
webSocket.onerror = function(e) {                       // :2551-2553
    console.log("Procesado por defecto del error");
};
```

**`onerror` no propaga nada.** La única señal de éxito es el flag `connected`, que sólo pone
`onopen`. Todo lo demás —puerto cerrado, TLS rechazado por CA no confiada, **permiso de LNA
denegado**, AutoFirma no instalada— colapsa, 33 segundos después, en el mismo error:

```js
errorCB("es.gob.afirma.standalone.ApplicationNotFoundException",   // :2499-2506
        ErrorCode.Request.WEBSOCKET_INVOICE_APP.message, errorCode);
```

con el código **`AS620017`**, *«Ha ocurrido un error al intentar invocar a la aplicación
nativa mediante websocket»* (`:337-420`). Antes de rendirse muestra un diálogo modal propio
con «reintentar / cancelar» (`Dialog.showErrorDialog(ERROR_CONNECTING_AFIRMA, …)`,
`:1525-1565`).

Sólo distingue fallos **posteriores** a tener conexión: cierre del socket (`AS420002`),
memoria (`AS620018`), cancelación, y los `err-XX:` del protocolo.

**Conclusión del punto 3: a AutoFirma le pasa exactamente lo mismo, y su diagnóstico es
peor que el mínimo posible.** Con Chrome 147 y Firefox 154, la persona que deniegue —o que
ignore el aviso tres veces en Chrome— verá, medio minuto después, un diálogo que le dice que
AutoFirma no está instalada. La v0.5 **hereda** el problema; no lo crea. Y tiene margen de
sobra para hacerlo mejor, porque el listón está en el suelo.

### 3.3 Un camino de AutoFirma que sí provoca el aviso antes

`autoscript.js` tiene un tercer transporte, `AppAfirmaJSSocket` (`:1111-1116`), que habla
**HTTPS plano por XHR** contra `https://127.0.0.1:<puerto>/afirma` (`:3005`, `:3413`) con el
mismo eco (`:3470`). Ese transporte **es `fetch`/XHR**, cubierto por LNA desde Chrome 142, y
por tanto **sí provoca el aviso de permiso con el mensaje de CORS explícito** de la sección
1.1. Es la observación que abre el remedio de la sección 4.3.

## 4. Cómo se distingue «no hay aplicación» de «el navegador ha denegado»

### 4.1 Desde el evento del WebSocket: no se puede

Es una limitación de la propia API, no de LNA. El evento `error` de un `WebSocket` es
deliberadamente opaco —sin código, sin razón— para no filtrar información de la red local, y
un cierre antes del apretón de manos llega como `1006` sin motivo. Chrome anota el error de
verdad sólo en su registro interno:

> `ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS` (net error **-385**)

visible en `chrome://net-export`, y confirmado en el propio Chromium
([`net/base/net_error_list.h`](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/net/base/net_error_list.h)):

```c
NET_ERROR(BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS, -385)
NET_ERROR(CACHED_IP_ADDRESS_SPACE_BLOCKED_BY_LOCAL_NETWORK_ACCESS_POLICY, -384)
```

`chrome://net-export` **no es una fuente que la aplicación web pueda leer**. Sirve para
soporte, no para diagnóstico automático.

### 4.2 El orden real de los fallos

Importa para redactar los mensajes, porque **los tres modos de fallo son secuenciales y sólo
uno es visible cada vez**:

1. **El permiso de LNA se evalúa primero**, antes de abrir el socket. Denegado, **no hay
   saludo TLS**: la CA es irrelevante en este escalón.
2. **Concedido el permiso, va el saludo TLS.** Sin la CA de rfirma en el almacén NSS, falla
   aquí. Esto es lo que sostiene el ADR-0005: la CA sigue siendo obligatoria, pero **por
   detrás** del permiso.
3. **Superado el TLS, va el eco.** Si no vuelve `"OK"`, es que hay *algo* escuchando en ese
   puerto que no es rfirma.

Un mensaje que diga «instala la CA» cuando el fallo es (1) manda a la persona por el camino
equivocado, y es exactamente el remedio opuesto.

### 4.3 Lo que sí funciona: preguntar antes de conectar

La Permissions API expone el estado sin provocar el aviso, y devuelve `"granted"`,
`"prompt"` o `"denied"`. **Eso separa los dos casos limpiamente**:

- `"denied"` → *el navegador ha denegado*. Remedio: reabrir el permiso en los ajustes del
  navegador. Ni reinstalar rfirma, ni tocar la CA.
- `"granted"` y aun así el socket no abre → *no hay aplicación escuchando*, o la CA no está.
  Remedio: arrancar rfirma, o repararla desde Preferencias.
- `"prompt"` → todavía no ha preguntado: conectar y dejar que pregunte, avisando antes de
  qué va el aviso que va a salir.

**Cuál es el nombre exacto del descriptor no está determinado.** La especificación nombra
`local-network` y `loopback-network`; varias guías de terceros usan
`navigator.permissions.query({ name: "local-network-access" })`; y Playwright registró
`local-network-access` como nombre de permiso concedible
([microsoft/playwright#37861](https://github.com/microsoft/playwright/issues/37861)). Son
tres nombres candidatos y **hay que comprobarlos con `navigator.permissions.query` en cada
navegador** —una `TypeError` distingue el inválido del válido—. Va a la lista de
comprobación.

Y la limitación de fondo: **la Permissions API la llama la sede, no rfirma**. Todo este
remedio vive en `autoscript.js`, código que no controlamos. Lo que rfirma **sí** puede
hacer, y es lo que le queda:

- Explicar el aviso **antes** de que salga, en la pantalla de la propia aplicación y en su
  documentación, con el texto literal de cada navegador (sección 1.2) y una captura.
- Dejar en Preferencias un **camino de reparación** que diga, en el mismo sitio donde se
  gestiona la CA, cómo se reabre el permiso de red local en cada navegador.
- No prometer que rfirma puede diagnosticarlo por sí sola. **No puede**: el fallo ocurre
  entero dentro del navegador y rfirma nunca recibe ni un paquete.

### 4.4 Un dato contradictorio que hay que resolver a mano

Hay fuentes de terceros que afirman lo contrario de lo que dicen Chromium y la
especificación:

> *Unlike fetch, subresource requests, or subframe requests, WebSockets do not display a
> user-facing permission prompt. End users cannot "click Allow" to resolve the issue.*
> — [Sprinklr, guía de LNA para WebSockets](https://www.sprinklr.com/help/articles/resolving-browser-local-network-access-restrictions-for-sprinklr-websockets/resolving-browser-local-network-access-restrictions-for-sprinklr-websockets/69fa06f8c8cbd953dd028264)

Contra eso, el *intent to ship* de Chromium dice *«A new permission will be shown to
users»*, la nota de la versión 147 dice *«WebSocket connections to local addresses now
trigger permission prompts»*, y tanto QZ Tray como Citrix describen el aviso apareciendo con
un WebSocket. **La lectura más probable es que la guía de Sprinklr esté escrita contra la
fase de prueba para desarrolladores (Chrome 142–146), donde el bloqueo existía sin aviso.**
Pero es un hecho que decide el hito, así que **no se da por resuelto aquí**: es el punto 1 de
la lista de comprobación.

## 5. Si es automatizable

**Parcialmente, y de forma asimétrica: Chrome sí, Firefox sí, pero por caminos distintos y
ninguno de los dos es «pulsar el botón».**

### 5.1 Preconcesión por política de empresa

Es el camino que ya usa la industria para no depender de la persona.

| Navegador | Política | Efecto |
|---|---|---|
| Chrome / Edge | `LocalNetworkAccessAllowedForUrls` | Concede el permiso a una lista de orígenes, sin aviso |
| Chrome / Edge | `LocalNetworkAccessBlockedForUrls` | Lo deniega |
| Chrome / Edge | `LocalNetworkAccessRestrictionsTemporaryOptOut` | Desactiva la restricción entera, **temporal** |
| Firefox | `LocalNetworkAccess` → `SkipDomains` | Lista de dominios exentos de la comprobación, con comodín `*.` |
| Firefox | `LocalNetworkAccess` → `EnablePrompting` | Desactiva el aviso |
| Firefox | `LocalNetworkAccess` → `Enabled` | Desactiva LNA por completo |

La política de Firefox está disponible desde **Firefox 145** y **ESR 153**
([referencia de administración de Mozilla](https://firefox-admin-docs.mozilla.org/reference/policies/localnetworkaccess/)).

**Esto sirve para las gradas de prueba; no sirve como remedio para la persona usuaria.**
Escribir `policies.json` en `/etc` exige `root`, y el ADR-0005 ya rechazó por escrito ese
camino para la CA. Un `SkipDomains` con la lista de sedes electrónicas españolas sería,
además, una lista que se desincroniza sola.

### 5.2 Interruptores para las gradas

- **Firefox**, `about:config` — se pueden fijar desde el perfil de prueba, sin `root`:
  `network.lna.enabled`, `network.lna.blocking`, `network.lna.websocket.enabled`
  (el que activó la cobertura de WebSocket,
  [bug 2042339](https://bugzilla.mozilla.org/show_bug.cgi?id=2042339)),
  `network.lna.block_trackers`, `network.lna.skip-domains`.
- **Chrome**, banderas: `chrome://flags/#local-network-access-check` y
  `#local-network-access-check-websockets`. Y el argumento
  `--ip-address-space-overrides=<ip:puerto>=public`, que reclasifica una dirección y así la
  saca del ámbito de LNA.
- **Playwright** acepta el permiso en `context.grantPermissions()` desde la **1.57**
  ([#37861](https://github.com/microsoft/playwright/issues/37861), cerrada con
  [#37871](https://github.com/microsoft/playwright/pull/37871)); en **Puppeteer** sigue
  abierta la petición ([#14405](https://github.com/puppeteer/puppeteer/issues/14405)). Hay
  además una queja documentada de que la concesión **no es fiable en integración continua**
  con `launchPersistentContext` ([#38670](https://github.com/microsoft/playwright/issues/38670)).

### 5.3 Lo que no se puede automatizar

- **Leer el texto del aviso.** Es UI nativa del navegador, fuera del DOM. Comprobar la
  redacción exacta es inevitablemente manual, y hay que rehacerlo en cada versión.
- **Comprobar la conducta de «ignorar tres veces»** de Chrome.
- **El `iframe`.** Si la sede embebe el cliente en un `iframe` de otro origen, hace falta
  `allow="loopback-network"` en el elemento, porque el permiso es *policy-controlled* con
  lista blanca `["self"]`. Eso **lo escribe la sede**; si no lo hace, no hay nada que rfirma
  pueda aportar.

### 5.4 Recomendación de gradas

Una grada de navegador **no bloqueante**, sobre Playwright, con perfiles preconcedidos, que
compruebe lo automatizable: que el estado de la Permissions API es el esperado, que el
WebSocket abre con permiso concedido y no abre sin él, y que la aplicación no se cuelga en
ninguno de los dos casos. Todo lo demás —redacciones, ubicación del aviso, botones— es la
lista de la sección 6, ejecutada a mano una vez por cada versión mayor de cada navegador.

## 6. Lista de comprobación manual

Lo que **no** se ha podido documentar desde fuente primaria. Cada punto dice qué mirar, en
qué navegador y con qué montaje.

**Montaje común.** Una página HTTPS servida desde un origen público real (no `localhost`),
que abra `new WebSocket("wss://127.0.0.1:<puerto>")` contra rfirma —o contra AutoFirma— ya
escuchando y con su CA confiada. Perfil de navegador **nuevo** en cada pasada; si no, la
concesión de la pasada anterior enmascara el resultado.

1. **[Decide el hito] ¿Sale aviso al abrir un WebSocket a *loopback*?**
   Chrome/Edge **147+** y Firefox **154+**. Resuelve la contradicción de la sección 4.4.
   Si la respuesta fuese que no sale ninguno, **la v0.5 no puede prometer el flujo de sede
   por WebSocket sin política de empresa**, y hay que volver al #308.
2. **El texto literal del aviso de *loopback*, con captura**, en Chrome 147+ y Firefox 154+,
   **en castellano**, que es lo que verá quien lo use. Los textos de la sección 1.2 son la
   redacción inglesa; la traducción no se ha podido leer.
3. **¿Nombra el aviso a la aplicación, el puerto o el proceso?** La respuesta esperada es que
   no. Confirmarlo, porque de ello depende toda la redacción de nuestra documentación.
4. **Persistencia.** Conceder, cerrar la pestaña, volver: ¿pregunta otra vez? Cerrar el
   navegador entero y volver: ¿pregunta? Abrir la misma sede en otra pestaña a la vez: ¿una
   sola concesión?
5. **Persistencia entre puertos.** Conceder con un puerto y forzar que `autoscript.js` sortee
   otros tres: ¿vuelve a preguntar? La sección 2 dice que no debería, porque la clave es el
   espacio de direcciones. **Confirmarlo es lo que sostiene el diseño entero.**
6. **Caducidad por inactividad en Chrome.** Conceder y no volver en varias semanas. No hay
   fuente primaria sobre si Chrome revoca los permisos de LNA por desuso.
7. **Ignorar tres veces en Chrome** (cerrar la burbuja con la ✕, no pulsar *Block*):
   comprobar que la cuarta vez ya no pregunta y el estado es `"denied"`.
8. **Navegación privada** en los dos navegadores: ¿pregunta?, ¿recuerda dentro de la sesión
   privada?, ¿se olvida al cerrarla?
9. **El descriptor de la Permissions API.** En la consola de cada navegador:
   ```js
   for (const n of ["loopback-network", "local-network", "local-network-access"]) {
     try { console.log(n, (await navigator.permissions.query({name: n})).state); }
     catch (e) { console.log(n, "INVÁLIDO", e.name); }
   }
   ```
   Anotar cuál acepta cada navegador y qué estado devuelve antes de conceder, después de
   conceder y después de denegar. **Es el punto que decide si la sección 4.3 es
   implementable.**
10. **Firefox: ¿de verdad hay dos redacciones?** Provocar el aviso contra `127.0.0.1` y
    contra una IP `192.168.x.x` y comprobar que el texto cambia, y que las entradas de
    `about:preferences#permissionsData` son distintas.
11. **AutoFirma de verdad, el oráculo.** Con AutoFirma 1.9.x instalada y una sede real, en
    Chrome 147+ y Firefox 154+: denegar el permiso y **cronometrar** lo que tarda en salir el
    diálogo de error y anotar su texto exacto. La predicción de la sección 3.2 es ≈ 33
    segundos y un mensaje que dice que la aplicación no está instalada.
12. **El orden de los fallos de la sección 4.2.** Con la CA **no** instalada: comprobar que
    el aviso de permiso sale igualmente y que el fallo de TLS es posterior. Es lo que
    confirma que un mensaje sobre la CA sería el remedio equivocado en el escalón (1).
13. **Chrome: nombre y ubicación del ajuste.** Confirmar si en la versión instalada se llama
    «Local network access» o «Apps on device», y anotar la ruta exacta en castellano, que es
    lo que hay que escribir en nuestra documentación de reparación.

## Lo que este informe deja decidido

- **El permiso es ineludible y no se puede esquivar.** No hay esquema, puerto ni truco que lo
  evite desde el lado de la aplicación. Cualquier diseño de la v0.5 que suponga lo contrario
  está mal.
- **El aviso no nos nombra**, así que **la explicación tiene que darla rfirma antes**, no el
  navegador. Es trabajo de documentación y de la pantalla de primer arranque, junto al de la
  CA, y en el mismo momento (ADR-0005: primer arranque, no primera invocación).
- **La CA sigue siendo obligatoria y queda por detrás del permiso** (sección 4.2). El
  ADR-0005 no cambia.
- **rfirma no puede diagnosticar la denegación**: el fallo ocurre entero dentro del navegador
  y no llega ni un paquete. Lo único que puede es **describir los dos casos y sus dos
  remedios opuestos** en Preferencias y en la documentación, y no prometer más.
- **AutoFirma no lo hace mejor**: 33 segundos y un único `AS620017` para todo. El listón que
  hay que superar es bajísimo.
- **El puerto no es fijo**: la sede sortea tres al azar y rfirma tiene que aceptar la lista
  que le llega en `afirma://websocket?ports=…`. Corrige la redacción del ADR-0005, sin
  cambiar su decisión.

## Lo que no se ha medido

- **Nada se ha ejecutado en un navegador.** Todo lo de arriba es lectura de fuentes; la
  sección 6 es la deuda.
- **WebRTC y WebTransport** no se han mirado: no están en el camino de `autoscript.js`.
- **La conducta de Safari** —no hay canal de rfirma para macOS, y LNA no está anunciado allí.
- **Edge, Brave, Opera y Vivaldi** heredan el comportamiento de Chromium, pero cada uno pinta
  su propia burbuja y ninguno se ha comprobado.
- **Chrome en Android** y **Firefox para Android** tienen artículos de ayuda propios y avisos
  distintos. Fuera del alcance de la v0.5, que es de escritorio.

## Fuentes

Primarias:

- [WICG, *Local Network Access* (especificación)](https://wicg.github.io/local-network-access/) — nombres de permiso, definición de *loopback*, integración con `fetch` y con el apretón de manos de WebSocket, persistencia opcional.
- [Chrome, notas de la versión 147](https://developer.chrome.com/release-notes/147) — 7 de abril de 2026, extensión de LNA a WebSocket y WebTransport.
- [Chromium, *Intent to Ship: Local network access restrictions for WebSockets*](https://groups.google.com/a/chromium.org/g/blink-dev/c/O6GMKt44Ups) — contexto seguro, políticas de empresa, banderas.
- [Chromium, *Ready for Developer Testing*](https://groups.google.com/a/chromium.org/g/blink-dev/c/4gx2y5jPGbU) — *«A new permission will be shown to users»*.
- [Chromium, `net/base/net_error_list.h`](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/net/base/net_error_list.h) — errores −384 y −385.
- [Chrome for Developers, *New permission prompt for Local Network Access*](https://developer.chrome.com/blog/local-network-access) — redacción del permiso de red local, bandera de la 138.
- [Firefox, notas de la versión 154.0](https://www.firefox.com/en-US/firefox/154.0/releasenotes/) — 18 de agosto de 2026, cobertura de WebSocket.
- [Bugzilla 2042339, *[LNA] Enable LNA Restrictions for Websockets*](https://bugzilla.mozilla.org/show_bug.cgi?id=2042339) — `network.lna.websocket.enabled`.
- [Bugzilla 2059274](https://bugzilla.mozilla.org/show_bug.cgi?id=2059274) — dónde guarda Firefox la concesión (`about:preferences#permissionsData`, «Local network devices»).
- [Mozilla, referencia de administración: política `LocalNetworkAccess`](https://firefox-admin-docs.mozilla.org/reference/policies/localnetworkaccess/) — campos y versiones.
- Código de AutoFirma, checkout de `clienteafirma` en `master` sobre `v1.9.1`: `afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js`, y en `afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/`: `AfirmaWebSocketServer.java`, `AfirmaWebSocketServerV4Sup.java`, `AfirmaWebSocketServerManager.java`, `ServiceInvocationManager.java`, `ProtocolInvocationLauncher.java`, `SecureSocketUtils.java`, `ChannelInfo.java`, `CommandProcessorThread.java`.

Secundarias, usadas sólo para el texto de los avisos y marcadas como tales en el cuerpo:

- [QZ Tray, wiki *LNA*](https://github.com/qzind/tray/wiki/LNA) — el análogo técnico más cercano: redacciones de Chrome y Firefox, «ignorar tres veces».
- [Box, *Allow Box domains local network access*](https://docs.box.com/en/box-tools/using-box-tools/allow-box-domains-local-network-access-in-chrome-edge-and-firefox-to-avoid-box-tools-disruption) — redacción de Firefox, «Remember my choice», «Blocked Temporarily».
- [Citrix CTX696569](https://support.citrix.com/external/article/CTX696569/chrome-147-and-edge-147-local-network-ac.html) — aviso con WebSocket en Chrome/Edge 147.
- [Sprinklr, guía de LNA para WebSockets](https://www.sprinklr.com/help/articles/resolving-browser-local-network-access-restrictions-for-sprinklr-websockets/resolving-browser-local-network-access-restrictions-for-sprinklr-websockets/69fa06f8c8cbd953dd028264) — **la fuente contradictoria** de la sección 4.4.
- [webflow/mcp-server#124](https://github.com/webflow/mcp-server/issues/124) — `allow="loopback-network"` y el mensaje de CORS con el espacio `loopback`.
- [microsoft/playwright#37861](https://github.com/microsoft/playwright/issues/37861), [#38670](https://github.com/microsoft/playwright/issues/38670), [puppeteer/puppeteer#14405](https://github.com/puppeteer/puppeteer/issues/14405) — automatización.
