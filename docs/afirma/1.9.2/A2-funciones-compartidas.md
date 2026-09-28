# Anexo A2 — Revisión de las funciones compartidas del original en el envío

A diferencia del resto de este manual (ver [README.md](README.md), «no compara
con rFirma ni describe lo que rFirma implementa»), **este anexo sí compara**.
Nació de #1162: `BatchSigner` monta una URL con los parámetros en la *query*,
pero quien decide cómo viajan de verdad es `UrlHttpManagerImpl`, que la parte
en el `?` y los escribe en el cuerpo si el método es POST. Portar la llamada
sin portar la función llamada dejó un lote remoto que mandaba el cuerpo vacío
(#1161). Este anexo cataloga las demás funciones compartidas del original a
las que llama el código portado del protocolo, para que la próxima vez que se
porte un capítulo se revise también la capa que usa.

Cada fila dice **qué hace la función en el envío**, con su cita de 1.9.2, y
**si rFirma lo reproduce**. No decide si una diferencia hay que arreglarla:
eso se decide en el issue que la fila enlaza.

`UrlHttpManagerImpl` (cabeceras `Origin`, `Host`, `Accept`, `Connection`,
`Authorization`, partido de la URL en un POST) es la primera de la lista por
decisión de #1162, y su revisión completa es #1168. Este anexo cubre el
resto: `DataDownloader`, `Base64`, `SSLErrorProcessor` y la lectura de
`extraParams`.

## Catálogo

### `DataDownloader.downloadData` — el origen de `dat`

* **Qué hace en el envío:** decide cómo obtener los datos a partir del valor
  del parámetro `dat`, en este orden: si `gzip=true` y el valor es Base64, lo
  descomprime directamente; si empieza por `http://` o `https://`, hace un
  `GET` con `UrlHttpManagerFactory.getInstalledManager().readUrl` (la función
  de la fila siguiente) y, si `ignoreSSLSecurity` está activo, monta un
  `SSLConfig` con un `TrustManager` que acepta cualquier certificado; si
  empieza por `ftp://`, abre un `URL.openStream()` directo; si empieza por
  `file:/`, lee el fichero local; en cualquier otro caso, prueba si es Base64
  y si no lo es, devuelve el texto tal cual.
  Cita: `afirma-core/src/main/java/es/gob/afirma/core/misc/http/DataDownloader.java:57-141`.
* **Quién lo llama para `dat`:** `UrlParameters.java:306-317` invoca
  `downloadData(dataPrm, gzipped)`, es decir, **siempre con
  `ignoreSSLSecurity=false`** — la variante insegura no se usa desde el
  protocolo `afirma://`.
* **rFirma:** `site/domain/protocol/operation/document.rs::data_of` reproduce
  el orden gzip → URL `http(s)` → rechazo de `ftp://` → Base64 → texto, el
  mismo que el de `DataDownloader.java:63-139`, con `HttpDataSource`
  (`site/adapters/data_download.rs`) para el `GET`. Coincide en Base64, gzip y
  URL `http(s)`. **Rechaza `ftp://`** con un `Refusal` explícito en vez de
  descargarlo (`document.rs::data_of`, constante `FTP`): decisión ya tomada en
  el código, sin ADR que la respalde. **No llega nunca a ver `file:/`**: se
  corta antes, ver la fila siguiente.

### `UrlParameters.setDataFromUrlParam` — el corte de `file:/` antes de descargar

* **Qué hace en el envío:** antes de llamar a `DataDownloader`, si
  `dat` empieza por `file:/`, lanza `ParameterException("No se permite la
  lectura de ficheros locales: " + dataPrm)` y la operación se aborta con ese
  error.
  Cita: `afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java:300-304`.
  Ya citado en [06-operaciones-firma.md](06-operaciones-firma.md)
  §2.4, pero sin conectar con la función a la que se lo ahorra.
* **rFirma:** reproduce el rechazo explícito, antes de que `data_of` (fila
  anterior) llegue a ver el valor:
  `parameters.rs::check_local_access_is_not_requested` (constante
  `LOCAL_FILE_PREFIX = "file:/"`), llamado por `check_common_parameters` desde
  `operation.rs::check_the_parameters_the_original_parses` para todos los
  verbos que analiza el original. Pruebas:
  `parameters/tests.rs::data_that_asks_for_a_local_file_is_refused` y
  `operation/tests/document.rs::a_signature_that_asks_for_a_local_file_never_gets_read`.
  Sin diferencia — no se abre issue. rFirma es, eso sí, **más estricto** que
  el original: compara sin distinguir mayúsculas y tras quitar los espacios
  iniciales (`FILE:/x` y `  file:/x` se rechazan también), mientras que el
  original compara `startsWith("file:/")` al pie de la letra y con `FILE:/x`
  sigue adelante hasta Base64 o texto literal.

### `SSLErrorProcessor` — la confianza en el certificado del servidor remoto

* **Qué hace en el envío:** implementa `HttpErrorProcessor`. Cuando
  `UrlHttpManagerImpl` recibe una `SSLHandshakeException` al conectar (a
  `dat` por URL, al servidor intermedio, al servidor trifásico o a los
  servlets del lote), le pasa la excepción a `SSLErrorProcessor`, que —si hay
  entorno gráfico y no está en modo `headless`— muestra un diálogo nativo
  preguntando si se quiere confiar en el certificado del servidor, y si la
  persona acepta, lo importa a un almacén de confianza temporal
  (`TRUSTED_KS_PWD = "changeit"`) y reintenta la conexión con él.
  Cita: `afirma-core/src/main/java/es/gob/afirma/core/misc/http/SSLErrorProcessor.java:112-227`
  (`processHttpError`) y su uso en `DataDownloader.java:90-99`.
* **rFirma:** no reproduce ningún mecanismo de confianza puntual. Los
  clientes HTTP de `site/adapters/` (`data_download.rs`, `batch_services.rs`,
  `servlets.rs`, `triphase_server.rs`, `relay.rs`) usan la validación TLS del
  sistema sin excepción: un servidor remoto con un certificado no reconocido
  hace fallar la petición, sin diálogo y sin forma de continuar. No hay ADR
  que decida esto a propósito; ADR-0005 cubre el certificado que rFirma
  *presenta* como servidor local, no la confianza que rFirma *exige* de un
  servidor remoto. → #1174 (decisión con ADR).

### `Base64.isBase64` / `Base64.decode` — la codificación de `dat` y `properties`

* **Qué hace en el envío:** `isBase64` acepta el alfabeto
  `A-Za-z0-9=_-\t\n+/\r~` (incluido `~`, que no es Base64 estándar), exige que
  el `=` de relleno solo aparezca en los dos últimos caracteres y que la
  longitud sin saltos de línea sea múltiplo de cuatro. `decode` ignora
  espacios, corta el último grupo si es incompleto y no comprueba los bits
  sobrantes. Cita: `afirma-core/src/main/java/es/gob/afirma/core/misc/Base64.java:625-691`
  (`isBase64`) y `599-610` (`decode`).
* **rFirma:** ya revisado y reproducido carácter a carácter en
  `site/domain/protocol/operation/document.rs`
  (`is_base64_to_the_original`, `decode_like_the_original`), con la cita al
  pie de cada función. Sin diferencia — no se abre issue.

### `AOUtil.base642Properties` — la lectura de `properties`

* **Qué hace en el envío:** descodifica el Base64 URL-safe de `properties` y
  lo carga con `java.util.Properties.load` sobre un `InputStreamReader` en
  UTF-8: formato `.properties` completo — comentarios `#`/`!`, separador `=`,
  `:` o espacio en blanco, continuación de línea con `\` al final, escapes
  `\\`, `\n`, `\r`, `\t`, `\:`, `\=`, `\ ` y `\uXXXX`. Un fallo de lectura no
  aborta la operación: se registra como `Properties` vacío.
  Cita: `afirma-core/src/main/java/es/gob/afirma/core/misc/AOUtil.java:485-495`,
  llamado desde `UrlParametersToSign.java:298-313` y equivalentes en
  `UrlParametersToSignAndSave`, `UrlParametersForBatch` y
  `UrlParametersToSelectCert`.
* **rFirma:** `site/domain/protocol/operation/properties.rs::pairs_of`
  sigue `Properties.load`: UTF-8, comentarios `#`/`!`, continuación de línea
  con `\` final, separador `=`/`:`/blanco no escapado y los escapes `\t \r \n
  \f` y `\uXXXX`; un fallo de lectura tampoco aborta la operación (mismo
  comentario en el código, citando `UrlParametersToSign.java:207`). Sin
  diferencia conocida, salvo un suplente `\uXXXX` suelto, que Rust no puede
  representar.

## Diferencias abiertas como issues

| Issue | Diferencia | Tipo |
|---|---|---|
| #1174 | Sin mecanismo de confianza puntual en un certificado TLS remoto no reconocido (`SSLErrorProcessor`) al descargar `dat` o hablar con los servlets | Decisión (ADR) |

`Origin` y el resto de cabeceras de `UrlHttpManagerImpl` se investigan en
#1168, no en este anexo.
