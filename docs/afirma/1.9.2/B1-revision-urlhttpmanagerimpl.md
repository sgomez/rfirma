# Anexo B1: revisión de `UrlHttpManagerImpl`

Primera entrega de la revisión de funciones compartidas del original a las que
llama el código portado (Implementation Decision de #1162). Describe **qué
hace** `es.gob.afirma.core.misc.http.UrlHttpManagerImpl` (1.9.2, tag `v1.9.2`,
commit `b4fe147c3`) en cada envío HTTP, con su cita, y compara cada
comportamiento con los cuatro clientes HTTP de rFirma. No forma parte de la
especificación del protocolo `afirma://` (como el Anexo A1, esto es
comparación con rFirma, y el resto del manual no lo hace).

`UrlHttpManagerImpl` es la única implementación de `UrlHttpManager` en 1.9.2 y
el punto de paso de **todo** el tráfico HTTP saliente: la prefirma y la
postfirma del lote remoto (`BatchSigner`), los servlets del servidor
intermedio (`HttpManager` → `IntermediateServerUtil`), el servidor trifásico
(`AOPkcs1TriPhaseSigner`) y la descarga del `dat` por URL (`DataDownloader`).
Ninguno de esos cuatro llamantes pasa un `timeout` explícito: todos usan la
sobrecarga que deja `DEFAULT_TIMEOUT` (`-1`, sin `connectTimeout`).

## Los cuatro clientes HTTP de rFirma

| Servicio remoto | Cliente de rFirma | Fichero |
|---|---|---|
| Prefirma y postfirma del lote remoto | `RelayBatchServices` | `rfirma-app/src-tauri/src/site/adapters/batch_services.rs` |
| Servidor intermedio (`stservlet`/`rtservlet`) | `RelayServlets` | `rfirma-app/src-tauri/src/site/adapters/servlets.rs` |
| Servidor trifásico (`serverUrl`) | `HttpTriphaseServer` | `rfirma-app/src-tauri/src/site/adapters/triphase_server.rs` |
| Descarga del `dat` por URL | `HttpDataSource` | `rfirma-app/src-tauri/src/site/adapters/data_download.rs` |

Los cuatro usan `reqwest::blocking` con la validación TLS del sistema y un
`Duration` de 30 s fijo en la construcción del cliente.

## Comportamiento por comportamiento

### 1. El cuerpo del POST/PUT va partido por `?`, no en la *query*

`UrlHttpManagerImpl.java:188-196` parte la URL por el primer `?` cuando el
método es POST o PUT: conecta solo con lo que hay antes, y lo que hay después
se escribe como cuerpo `application/x-www-form-urlencoded`
(`UrlHttpManagerImpl.java:267-277`).

**rFirma:** reproducido en los tres que hacen POST. `RelayBatchServices::post`
compone el cuerpo aparte y lo manda con `.body(body)` y `Content-Type`
explícito (`batch_services.rs:31-63`); `RelayServlets` y `HttpTriphaseServer`
usan `.form(&params)`, que hace lo mismo. Arreglado para el lote en #1161; los
otros dos ya lo hacían así (Further Notes de #1162). La descarga del `dat` es
un GET en los dos lados —`DataDownloader.java:92` y `HttpDataSource`—, así
que el troceado por `?` no se le aplica.

### 2. Cabeceras por defecto que añade antes de escribir el cuerpo

`UrlHttpManagerImpl.java:238-265`: sobre las cabeceras que trae la llamada,
llama a `conn.addRequestProperty` (solo si esa cabecera no viene ya puesta)
con `Authorization`, `Accept`, `Connection`, `Host` y `Origin`, en ese orden:

| Cabecera | Cita | Valor pedido |
|---|---|---|
| `Authorization` | `UrlHttpManagerImpl.java:158-186,246-248` | `Basic <base64(usuario:contraseña)>`, si la URL trae credenciales (`usuario:contraseña@host`); solo usuario o solo contraseña si falta el otro |
| `Accept` | `UrlHttpManagerImpl.java:249-251` | `*/*` |
| `Connection` | `UrlHttpManagerImpl.java:252-254` | `keep-alive` |
| `Host` | `UrlHttpManagerImpl.java:255-257` | El host de la URL, sin puerto |
| `Origin` | `UrlHttpManagerImpl.java:258-260` | `<esquema>://<host>` de la URL de destino, sin puerto |

De las cinco, solo `Authorization` y `Accept` llegan de verdad a la red:
`Connection`, `Host` y `Origin` están en la lista `restrictedHeaders` de
`sun.net.www.protocol.http.HttpURLConnection` (JDK 21,
`HttpURLConnection.java:198-213`), y `addRequestProperty` las descarta en
silencio —ni excepción ni registro— salvo con la propiedad de sistema
`-Dsun.net.http.allowRestrictedHeaders=true` (`:279-285,483-498`), que
`clienteafirma` no fija en ningún punto de `v1.9.2`. El JDK pone su propio
`Host` (con puerto si no es el de por defecto, `HttpURLConnection.java:2328`)
y su propio `Connection: keep-alive` (`:661`); ninguno de los dos sale con el
valor de la tabla.

**rFirma, cabecera a cabecera:**

- **`Host`: no aplica.** Ningún cliente de rFirma la fija a mano, pero
  `reqwest`/`hyper` la componen siempre a partir de la autoridad de la URL en
  toda petición HTTP/1.1 — igual que en el original, la pone el motor HTTP,
  no la aplicación. No hay diferencia de envío que abrir.
- **`Connection`: no aplica en la práctica.** Ningún cliente la fija a mano,
  pero `hyper` mantiene conexiones persistentes por defecto en HTTP/1.1 con o
  sin la cabecera explícita — de nuevo, el motor HTTP en los dos lados. Se
  anota, no se abre issue.
- **`Origin`: no aplica.** El original tampoco la envía: es una cabecera
  restringida que el JDK descarta sin avisar (arriba). Ninguno de los cuatro
  clientes de rFirma la envía tampoco (confirmado leyendo `batch_services.rs`,
  `servlets.rs`, `triphase_server.rs` y `data_download.rs` enteros): los dos
  lados se comportan igual, cada uno por su motivo. No hay diferencia de
  envío que abrir (ADR-0023: seguir al original en los casos felices).
- **`Accept`: reproducida (cabecera por defecto de `reqwest`).** `reqwest`
  0.13.4 (`rfirma-app/src-tauri/Cargo.lock`) inserta `Accept: */*` por
  defecto en `async_impl::ClientBuilder::new()`
  (`src/async_impl/client.rs:283-284`), y `blocking::ClientBuilder::new()`
  envuelve ese builder (`src/blocking/client.rs:97`); ninguno de los cuatro
  clientes llama a `default_headers` para pisarla. No hay diferencia de
  envío que abrir.
- **`Authorization` por credenciales en la URL: no reproducida.** El tipo
  `reqwest::Url` que usan los cuatro clientes sí sabe extraer usuario y
  contraseña de una URL con la forma `usuario:contraseña@host` (es
  `Url::username()`/`Url::password()` de la crate `url`), pero ninguno de los
  cuatro lee esos campos ni compone la cabecera. → **#1175**.

### 3. Caché de la conexión deshabilitada

`UrlHttpManagerImpl.java:233-234`: `conn.setUseCaches(false)` y
`conn.setDefaultUseCaches(false)`, para que `HttpURLConnection` no sirva una
respuesta cacheada de una petición anterior a la misma URL.

**rFirma: no aplica.** `reqwest::blocking` no implementa ninguna caché HTTP
propia (ni de respuesta ni de conexión reutilizada más allá del *keep-alive*
del punto 2); no hay nada que deshabilitar ni comportamiento que reproducir.

### 4. Tiempo de espera

`UrlHttpManagerImpl.java:279-282`: el `connectTimeout` de la conexión solo se
fija si `timeout != DEFAULT_TIMEOUT` (`-1`); no hay `readTimeout` en ningún
punto del fichero. Los cuatro llamantes de 1.9.2 revisados (`BatchSigner`,
`HttpManager`/`IntermediateServerUtil`, `AOPkcs1TriPhaseSigner`,
`DataDownloader`) usan la sobrecarga sin `timeout`, así que en la práctica
**ninguno de los cuatro envíos que compara este anexo tiene tiempo de espera
explícito** en el original: depende solo del `connectTimeout` por defecto de
la JVM.

**rFirma:** los cuatro clientes fijan `Duration::from_secs(30)` como
`.timeout()` del `reqwest::blocking::Client`, que en `reqwest` cubre la
petición entera (conexión y lectura de la respuesta), no solo la conexión. Es
una diferencia sin decidir — un lote remoto largo o un servidor trifásico
lento fallarían en rFirma a los 30 s donde el original seguiría esperando —,
candidata a decidirse en un ADR: acotar la espera es una red de seguridad
razonable para una aplicación de escritorio sin proceso supervisor, pero
30 s puede ser corto para un lote grande o un documento grande en trifásica.
→ **#1176**.

### 5. Cuerpo de la respuesta de error (4xx/5xx)

`UrlHttpManagerImpl.java:285-296`: ante un código 4xx o 5xx, lee el cuerpo del
error (`conn.getErrorStream()`) y lo mete en la excepción `HttpError`, que
`BatchSigner` registra con `e.getResponseDescription()` y `HttpManager`
propaga tal cual.

**rFirma:** ninguno de los cuatro conserva el cuerpo de la respuesta de
error. `RelayBatchServices::post` pasa `status.to_string()` —la línea
canónica del código de estado, p. ej. `"404 Not Found"`, no el cuerpo— a
`BatchError::answered` (`batch_services.rs:49-56`) sin leer la respuesta.
`RelayServlets`, `HttpTriphaseServer` y `HttpDataSource` usan
`.error_for_status()` de `reqwest`, que también descarta el cuerpo y solo
conserva el código de estado (`servlets.rs:41-46,60-64`,
`triphase_server.rs:42-47`, `data_download.rs:23-26`). Un servidor que
explica el 4xx en el cuerpo (el caso más común: un mensaje de error del
servlet) pierde ese detalle en el registro de rFirma para los cuatro
clientes. → **#1177**.

### 6. Cookies

`UrlHttpManagerImpl.java:60-64`: un bloque estático instala un
`CookieManager` con `CookiePolicy.ACCEPT_ALL` como `CookieHandler` por
defecto de la JVM, así que **toda** conexión `HttpURLConnection` del proceso
—incluidas las de estos cuatro envíos— guarda y reenvía cookies de sesión
entre peticiones.

**rFirma: no reproducida.** Ninguno de los cuatro `reqwest::blocking::Client`
se construye con `.cookie_store(true)` (comprobado en las cuatro
construcciones: `batch_services.rs:22-24`, `servlets.rs:22-24`,
`triphase_server.rs:18-20`, `data_download.rs:17-19`), así que rFirma no
guarda ni reenvía cookies entre la prefirma y la postfirma del lote, ni entre
un `store`/`wait`/`retrieve` del servidor intermedio. Un servicio remoto que
dependa de una cookie de sesión entre esas llamadas (p. ej. afinidad de
sesión de un balanceador que no reescribe la URL) fallaría con rFirma y
funcionaría con AutoFirma. Candidata a decisión deliberada: guardar cookies
por proceso es una superficie nueva (persisten entre trámites de sedes
distintas si el cliente se reutiliza) que el original no acota tampoco, así
que no es un "seguir al original sin más". → **#1180**.

### 7. Confianza SSL configurable y bucle local

`UrlHttpManagerImpl.java:210-231` permite desactivar la validación del
certificado SSL por propiedad de sistema (`disableSslChecks`) o por lista de
dominios seguros (`secureDomainsList`, vía `HttpManager`, que la conecta a una
preferencia de la aplicación con un diálogo de confirmación interactivo por
dominio). `UrlHttpManagerImpl.java:203-208,320-331` evita el proxy del
sistema (`Proxy.NO_PROXY`) en Android o cuando la URL es del bucle local.

**rFirma: no reproducida, y no se abre issue.** Los cuatro clientes validan
siempre con la confianza TLS del sistema (así lo dice el comentario de cada
uno: «con la validación TLS del sistema») y no ofrecen forma de desactivarla
por dominio ni de forma interactiva. Es una decisión ya tomada en el propio
código, no un olvido: bajar la barra de confianza TLS de un cliente que habla
con el servidor de una sede (donde puede haber un certificado en juego) es un
riesgo que el original acepta y rFirma no reproduce a propósito (ADR-0023, el
caso "contradicción o riesgo grave"). La rama de bucle local
(`Proxy.NO_PROXY`) tampoco se reproduce, pero el servidor intermedio, el
trifásico y la descarga del `dat` de una sede real no son direcciones locales
en la práctica: se anota, no se abre issue por falta de caso real que lo
motive.

## Resumen

| Comportamiento | Cita | ¿rFirma lo reproduce? | Issue |
|---|---|---|---|
| Cuerpo del POST/PUT partido por `?` | `UrlHttpManagerImpl.java:188-196,267-277` | Sí, en los tres que hacen POST; GET en los otros dos | (#1161) |
| `Host` | `UrlHttpManagerImpl.java:255-257` | No aplica (automático) | — |
| `Connection: keep-alive` | `UrlHttpManagerImpl.java:252-254` | No aplica en la práctica | — |
| `Origin` | `UrlHttpManagerImpl.java:258-260` | No aplica (el original tampoco la envía) | — |
| `Accept: */*` | `UrlHttpManagerImpl.java:249-251` | Sí, en los cuatro (por defecto de `reqwest`) | — |
| `Authorization` desde credenciales de la URL | `UrlHttpManagerImpl.java:158-186,246-248` | No, en ninguno | #1175 |
| Caché de conexión deshabilitada | `UrlHttpManagerImpl.java:233-234` | No aplica (`reqwest` no cachea) | — |
| Tiempo de espera | `UrlHttpManagerImpl.java:279-282` | No: 30 s fijo frente a sin tiempo de espera explícito | #1176 |
| Cuerpo de la respuesta de error (4xx/5xx) | `UrlHttpManagerImpl.java:285-296` | No, en ninguno | #1177 |
| Cookies entre llamadas | `UrlHttpManagerImpl.java:60-64` | No, en ninguno | #1180 |
| Confianza SSL configurable / bucle local sin proxy | `UrlHttpManagerImpl.java:203-231,320-331` | No, a propósito (ADR-0023) | — |
