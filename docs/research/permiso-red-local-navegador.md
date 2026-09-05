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
4. **AutoFirma sufre exactamente lo mismo**, y peor: tras **28 segundos** de reintentos en
   bucle, **no enseña ningún diálogo** —arranca sin ventana y se rinde en silencio— y quien
   pinta el error es la sede, con un mensaje genérico y **sin código**. Medido en el
   [#324](https://github.com/sgomez/rfirma/issues/324). Es un problema del ecosistema, no de
   rfirma.
5. **La distinción «denegado» vs «no hay servidor» no se puede hacer desde el evento del
   WebSocket** —la especificación de WebSocket lo prohíbe—, pero **sí** con la Permissions
   API antes de conectar. Ese es el remedio, y es del lado de la sede, no de rfirma.

Y un corolario incómodo: **rfirma no controla el lado que puede arreglarlo.** Quien pinta el
mensaje de error es `autoscript.js`, que sirve la sede.

> **Estado de las medidas.** Este informe se escribió sin ejecutar nada en un navegador, y
> dejaba la sección 6 como deuda. Esa deuda **está saldada**: la lista se pasó a mano el
> 2026-09-05 sobre Chrome 152, Firefox 155 y AutoFirma 1.9.0, y **los resultados están
> incorporados a lo largo de este documento**. La pasada completa, con el montaje y los
> literales, vive en el
> [#324](https://github.com/sgomez/rfirma/issues/324#issuecomment-5551318737).

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

**Medido** (Chrome 152, Firefox 155):

| Navegador | Texto del aviso (*loopback*) | Texto del aviso (red local) |
|---|---|---|
| Chrome 152 | *«`example.com` wants to access other apps and services on this device»* | — |
| Chrome 152, en castellano | **«`example.com` quiere acceder a otras aplicaciones y servicios de este dispositivo.»**, botones **Bloquear** / **Permitir** | — |
| Firefox 155 | *«`example.com` wants to access other apps and services on this device»* | *«…wants to access apps and services on devices connected to your local network»* |

Las dos redacciones de Firefox **existen y son distintas**, y los dos permisos se piden por
separado: con el de *loopback* ya concedido, una conexión a una IP `192.168.x.x` vuelve a
preguntar. El literal de Firefox **en castellano** sigue sin capturar: el paquete no traía el
idioma instalado en el equipo de la medida.

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
  **Block**, más una **✕** para descartar sin decidir. QZ Tray documenta una trampa
  importante:

  > *⚠️ WARNING: Ignoring this pop-up three time is equivalent to clicking 'block'!*

  Es la regla general de *permission fatigue* de Chrome, y **está confirmada a mano**: tres
  descartes con la ✕ y **a la cuarta ya no pregunta**, con la Permissions API devolviendo
  `denied`. Peor aún, Chrome presenta ese estado como **«Automatically blocked»**, así que
  quien cerró el aviso «por quitarlo de en medio» acaba donde quien pulsó *Block*, **sin
  recordar haber bloqueado nada**.
- **Firefox:** panel de permiso junto a la barra de direcciones, botones **Allow** / **Block**
  y **casilla «Remember my choice»**. **No hay ✕**, así que el camino de «ignorar tres veces»
  no existe aquí. Sin marcar la casilla, la denegación **no aparece en los ajustes** y aun así
  bloquea: medido, `local-network` devolvía `denied` mientras Preferencias no mostraba fila
  alguna. La persona no tiene dónde deshacerla salvo reiniciando el navegador.

  Y un plazo que no estaba previsto: **Firefox abandona la conexión a los ~20 s con el aviso
  todavía en pantalla** (21074 ms y 20072 ms en dos medidas), sin que la persona haya tocado
  nada. Chrome no lo hace. Es un requisito de diseño para la sede: **no puede dar por perdido
  el intento con el primer cierre**, o la primera visita de cada persona en Firefox falla
  siempre.

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
puerto de *loopback* que la sede intente después. **Confirmado a mano**: concedido en un
puerto, otros tres abrieron sin preguntar, en ~44 ms.

También está confirmada la persistencia en toda su extensión —recargar, segunda pestaña
simultánea, cerrar las pestañas, y **cerrar y reabrir el navegador entero**—, en los dos.

Y una asimetría que conviene saber antes de escribir instrucciones de reparación:

| Navegación privada | Firefox 155 | Chrome 152 |
|---|---|---|
| ¿Pregunta, pese a estar concedido en la ventana normal? | **Sí** | **No: hereda la concesión del perfil** |
| ¿Ofrece casilla de recordar? | **No** | n/a |
| ¿Recuerda dentro de la sesión privada? | Sí | Sí |
| ¿Se olvida al cerrarla? | **Sí** | **No** |

**Chrome en incógnito no aísla nada** de este permiso.

Cuánto dura la concesión de Chrome sin uso —Chrome caduca algunos permisos por inactividad—
**sigue sin determinar**: no hay fuente primaria y la medida exige semanas de espera. Es uno de
los dos únicos puntos de la sección 6 que quedan sin pasar.

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

**Medido contra una sede real** (`valide.redsara.es`, AutoFirma 1.9.0, permiso denegado en
Chrome 152):

- **28 segundos** hasta rendirse, del mismo orden que los ≈ 33 calculados aquí.
- **AutoFirma no enseña nada.** En modo WebSocket arranca sin ventana, se rinde en silencio y
  **no queda proceso suyo vivo** al terminar. Todo lo que ve la persona lo pinta la sede.
- El diálogo de la sede es **genérico y no muestra el código**: dice que no es posible conectar
  con AutoFirma, con botones *Reintentar* / *Cerrar*. El `AS620017` se queda dentro.
- La consola confirma la mecánica: **AutoFirma se ata a un solo puerto de los tres**, y ese da
  `ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS` mientras los otros dos dan
  `ERR_CONNECTION_REFUSED`, en bucle hasta agotar los intentos.

O sea: **el navegador tenía el diagnóstico exacto y la sede no lo usa**. El listón que rfirma
tiene que superar es aún más bajo de lo que este informe suponía.

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

**Matiz medido: los códigos de cierre no son iguales en los dos navegadores.**

| Situación | Chrome 152 | Firefox 155 |
|---|---|---|
| Permiso denegado | `1006` (consola: `ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS`) | `1006` |
| Fallo de TLS | `1006` (consola: `ERR_CERT_AUTHORITY_INVALID`) | **`1015`** |
| Plazo de ~20 s agotado con el aviso abierto | no aplica | `1006` |

Firefox **sí distingue el fallo de TLS con `1015`**; Chrome lo colapsa todo en `1006`. No
rompe la conclusión de esta sección —«denegado» y «no hay nadie» siguen siendo el mismo
`1006`—, pero deja una rendija útil: en Firefox, un `1015` **descarta** el problema de permiso
y **señala la CA**, que es el remedio opuesto.

### 4.2 El orden real de los fallos

**Los dos navegadores lo ordenan al revés**, y esto se midió a mano porque la predicción de
este informe era la contraria.

- **Firefox: el permiso va primero.** Con la CA no instalada y el permiso sin decidir, saca la
  burbuja y **espera**, sin adelantar ningún error de TLS.
- **Chrome: el TLS falla antes.** Mismo montaje: `net::ERR_CERT_AUTHORITY_INVALID` a los
  **7 ms**, con la burbuja del permiso todavía en pantalla y sin contestar.

La traza real de AutoFirma lo remacha: de los tres puertos que sortea la sede, **sólo el que
tenía a alguien escuchando** dio `ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS`; los otros dos
dieron `ERR_CONNECTION_REFUSED`. Si el gate se aplicara antes de conectar, los tres darían
bloqueo. **En Chrome el permiso se comprueba al final, cuando la conexión ya ha prosperado.**

Lo que sí se mantiene, y sostiene el ADR-0005, es que **la CA sigue siendo obligatoria**: sin
ella no hay eco aunque el permiso esté concedido. Lo que cambia es el diagnóstico: **en Chrome
el orden de los fallos no sirve para separarlos**, porque una CA sin instalar produce un fallo
inmediato e indistinguible aunque el permiso siga sin decidir. Separarlos exige la Permissions
API (sección 4.3), no la cronología.

Un mensaje que diga «instala la CA» cuando el fallo es el del permiso sigue mandando a la
persona por el camino equivocado, y es exactamente el remedio opuesto.

### 4.3 Lo que sí funciona: preguntar antes de conectar

La Permissions API expone el estado sin provocar el aviso, y devuelve `"granted"`,
`"prompt"` o `"denied"`. **Eso separa los dos casos limpiamente**:

- `"denied"` → *el navegador ha denegado*. Remedio: reabrir el permiso en los ajustes del
  navegador. Ni reinstalar rfirma, ni tocar la CA.
- `"granted"` y aun así el socket no abre → *no hay aplicación escuchando*, o la CA no está.
  Remedio: arrancar rfirma, o repararla desde Preferencias.
- `"prompt"` → todavía no ha preguntado: conectar y dejar que pregunte, avisando antes de
  qué va el aviso que va a salir.

**El descriptor está medido, y hay que usar `loopback-network`.**

| Descriptor | Chrome 152 | Firefox 155 |
|---|---|---|
| `loopback-network` | acepta; sigue el estado real | acepta; sigue el estado real |
| `local-network-access` | acepta; **alias** del anterior, cambia a la vez | **lanza `TypeError`** |
| `local-network` | acepta, pero es **otro** permiso: se queda en `prompt` | igual: otro permiso |

Estados observados con `loopback-network`: `prompt` de partida → `granted` tras *Allow* →
`denied` tras *Block*, en los dos navegadores. `local-network` **nunca se movió** al conceder
el de *loopback*: son dos permisos independientes, como decía la sección 1.1.

Y un aviso para quien lo implemente: **la detección de capacidades por excepción no vale**.
Chrome **no lanza** con nombres que no aplican y Firefox sí, así que la distinción hay que
hacerla por el **estado**, nunca por el `catch`.

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
**Resuelto a mano, y a favor de Chromium: el aviso sale.** Comprobado con un WebSocket a
`wss://127.0.0.1` en Chrome 152 y en Firefox 155, con perfil nuevo y desde un origen HTTPS
público. La guía de Sprinklr describe la fase de prueba para desarrolladores, no el
comportamiento actual. **El hito sigue en pie.**

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
- **Comprobar la conducta de «ignorar tres veces»** de Chrome. Medida ya a mano —tres ✕ y a la
  cuarta `denied`—, pero cada versión mayor obliga a repetirla, y no hay forma de guionizarla.
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

## 6. La lista de comprobación manual, y lo que dio

**Pasada entera el 2026-09-05** sobre Chrome 152.0.7977.64, Firefox 155.0 y AutoFirma 1.9.0,
con perfil de navegador nuevo en cada punto. Los resultados están incorporados a las secciones
1 a 5; aquí queda el índice y el montaje, para poder repetirla en cada versión mayor.

**Montaje.** Origen HTTPS público real (`https://example.com`), con el código en la consola de
DevTools. Al otro lado, un servidor WebSocket sobre TLS mínimo —sólo el *handshake* 101—
escuchando en `127.0.0.1` en varios puertos a la vez, con una cadena CA + hoja emitida al
efecto. Dos trampas del montaje, que cuestan una tarde si no se saben:

- **Firefox rechaza un autofirmado que sea a la vez CA y certificado de servidor**
  (`mozilla::pkix` no admite `CA:TRUE` en la hoja); Chrome sí lo traga. Hace falta cadena de
  dos niveles, con la CA en el perfil vía `certutil -A -d sql:<perfil> -t "C,,"`.
- Chrome acepta `--ignore-certificate-errors` (la banda amarilla de «marca no compatible» es
  normal); **Firefox no tiene equivalente**.

| # | Punto | Resultado | Dónde está |
|---|---|---|---|
| 1 | ¿Sale aviso con un WebSocket a *loopback*? | **Sí, en los dos.** El hito sigue en pie | § 4.4 |
| 2 | Texto literal del aviso | Capturado; el castellano de Chrome, en la tabla | § 1.2 |
| 3 | ¿Nombra aplicación, puerto o proceso? | **No.** Sólo el origen de la sede | § 1.2 |
| 4 | Persistencia (pestaña, ventana, reinicio) | **Total en los dos** | § 2 |
| 5 | Persistencia entre puertos | **Confirmada**: la clave es el espacio de direcciones | § 2 |
| 6 | Caducidad por inactividad en Chrome | **Sin medir** (exige semanas) | — |
| 7 | Ignorar tres veces en Chrome | **Confirmado**, y se presenta como «Automatically blocked» | § 1.3 |
| 8 | Navegación privada | Firefox aísla; **Chrome en incógnito no** | § 2 |
| 9 | Descriptor de la Permissions API | **`loopback-network`**, el único común | § 4.3 |
| 10 | Firefox: ¿dos redacciones? | **Sí**, y son dos permisos separados | § 1.2 |
| 11 | AutoFirma, el oráculo | **28 s, ningún diálogo propio, mensaje genérico sin código** | § 3.2 |
| 12 | El orden de los fallos | **Al revés en cada navegador**; el informe lo tenía mal | § 4.2 |
| 13 | Chrome: nombre y ruta del ajuste | **«Apps on device»**, `chrome://settings/content/loopbackNetwork` | abajo |
| 14 | El selector con dos manejadores de `afirma://` | Ni Chrome ni Firefox eligen; **manda `mimeapps.list`** | abajo |

La pasada completa, con literales y trazas, está en el
[#324](https://github.com/sgomez/rfirma/issues/324#issuecomment-5551318737).

### 6.1 Dónde se repara en Chrome (punto 13)

- Página global: `chrome://settings/content/loopbackNetwork`, donde el sitio figura como
  *«Not allowed to access other apps and services on this device»*.
- Por sitio: icono a la izquierda de la barra de direcciones → **Apps on device** →
  **«Automatically blocked»** + botón **Reset permission**.

En Firefox, *Settings → Privacy & Security*, sección **«Device apps and services»** — donde
**no** aparecen las denegaciones hechas sin marcar «Remember my choice», aunque bloqueen.

### 6.2 Qué pasa con dos aplicaciones registradas para `afirma://` (punto 14)

Medido con un `.desktop` de pruebas junto a la `afirma.desktop` de AutoFirma:

- **Basta instalar el `.desktop` en `~/.local/share/applications`** y correr
  `update-desktop-database` para que el `default` pase a ser el nuevo, sin escribir nada en
  `mimeapps.list`: el ámbito de usuario gana al del sistema. **Instalar rfirma le quitaría el
  `afirma://` a AutoFirma sin que la persona elija.**
- **Firefox no pinta un selector**: nombra la aplicación que dice el escritorio, con «*Choose a
  different application*» —que ahí sí lista las dos—, casilla «*Always allow*» y
  *Cancel* / *Open link*. La elección se guarda **dentro de Firefox**, en
  *Preferences → Applications*, **por encima de `mimeapps.list`**.
- **Chrome tampoco elige**: «*Open …?*», *Always allow*, abrir o cancelar. Pero delega en el
  escritorio, y **es GNOME quien saca su propio selector** con las dos aplicaciones, **sin
  casilla de recordar** y **volviendo a salir cada vez**.

Tres consecuencias para el registro del esquema:

1. `mimeapps.list` decide el primer escalón; el diseño que dependa de él se sostiene.
2. **Hay que escribir un `default` explícito en `[Default Applications]`**. Sin él, GNOME
   pregunta en cada invocación y no ofrece forma de callarlo.
3. **Firefox puede desautorizar a `mimeapps.list`**: si la persona marcó «always allow» sobre
   AutoFirma, rfirma reescribirá el fichero sin efecto alguno. Cualquier interfaz de rfirma que
   presuma de controlar el manejador tiene que decirlo, y mandar a *Preferences → Applications*.

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
- **AutoFirma no lo hace mejor**: **28 segundos medidos**, ningún diálogo propio, y un mensaje
  de la sede genérico y **sin código**. El listón que hay que superar es bajísimo.
- **El puerto no es fijo**: la sede sortea tres al azar y rfirma tiene que aceptar la lista
  que le llega en `afirma://websocket?ports=…`. Corrige la redacción del ADR-0005, sin
  cambiar su decisión.

## Lo que no se ha medido

- **La caducidad por inactividad de los permisos de Chrome**: exige semanas de espera y no hay
  fuente primaria.
- **El literal del aviso de Firefox en castellano**: el equipo de la medida no tenía el idioma
  instalado en el paquete.
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
