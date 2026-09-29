# El saludo TLS del canal local en Windows es de rustls

En Windows, el servidor TLS del canal local (`wss://127.0.0.1` y el transporte `service`,
ADR-0005) usa **rustls** con el proveedor criptográfico de **ring**. En Linux no cambia nada:
sigue siendo `native-tls` sobre el OpenSSL del sistema. La diferencia vive en un único fichero,
`site/adapters/channel/acceptor.rs`, que da a los dos transportes el mismo `LocalTlsAcceptor` y
el mismo tipo de flujo cifrado.

## Por qué schannel no sirve aquí

En Windows, `native-tls` es schannel, y schannel no firma con una clave que tenga en memoria:
necesita que la clave esté en un proveedor del sistema.

- `Identity::from_pkcs8` importa la clave en un contenedor **persistente** del proveedor
  RSA antiguo (`PROV_RSA_FULL`) con un nombre aleatorio. La clave de la CA local y del
  certificado del servidor es de curva elíptica P-256 (ADR-0005), y ese proveedor la rechaza
  («valor de etiqueta ASN1 incorrecto»): el canal no llega a abrirse.
- `Identity::from_pkcs12` sí acepta curva elíptica, pero `native-tls` llama a
  `PFXImportCertStore` sin `PKCS12_NO_PERSIST_KEY`: cada apertura del canal dejaría en
  `%APPDATA%\Microsoft\Crypto\Keys` una clave nueva que nadie borra. El certificado del
  servidor se genera en memoria en cada arranque y no se guarda en disco (ADR-0005); con
  schannel, su clave acabaría guardada igual, una por trámite.

rustls firma con la clave en memoria del proceso, como OpenSSL en Linux, y no toca ningún
almacén del sistema.

## Lo que ve el navegador

El navegador, que es el cliente, recibe lo mismo que en Linux: el certificado del servidor
local (`CN=localhost`, `DNS:localhost` e `IP:127.0.0.1`), firmado con ECDSA P-256 por la CA
local, sobre TLS 1.2 o 1.3. Edge y Chrome lo validan contra `CurrentUser\Root`, donde rFirma
instala la CA local (ADR-0035). Las pruebas del canal (`channel_client`, `channel_operations`,
`service_acknowledgement` y la de inactividad) usan como cliente el `native-tls` de schannel y
pasan contra este servidor.

## Considered Options

**`Identity::from_pkcs12` con `native-tls`**: una sola pila TLS, sin dependencias nuevas.
Descartada por la clave persistida en cada apertura del canal, que además no se puede borrar
porque `native-tls` no expone el contexto del certificado importado.

**Una CA local RSA solo en Windows**: `from_pkcs8` la aceptaría, pero el dominio (`local_ca.rs`)
tendría un condicional de sistema, la CA sería distinta en cada plataforma, y la clave seguiría
quedando en un contenedor persistente con nombre aleatorio.

**schannel directo** (crate `schannel`) con un almacén en memoria y `PKCS12_NO_PERSIST_KEY`:
evita la clave persistida, pero obliga a reescribir el acoplamiento con `tokio` que ya resuelve
`tokio-native-tls`, y schannel no garantiza servir con una clave efímera en todas las versiones
de Windows 10.

**rustls también en Linux**: una sola implementación para las dos plataformas, pero cambia la
pila TLS que ya funciona en Linux y en el flatpak. Queda fuera del port.

## Consequences

- En Windows hay dos pilas TLS en el binario: schannel para las conexiones salientes
  (`reqwest`) y rustls para el servidor local. Es la excepción, solo en Windows, a no meter
  `rustls` en el árbol que dice el `Cargo.toml`.
- `tokio-rustls` entra en `[target.'cfg(windows)'.dependencies]` sin sus características por
  omisión (`aws-lc-rs` exige cmake y nasm) y con `ring` y `tls12`. El `Cargo.lock` gana
  `rustls`, `rustls-webpki`, `ring`, `untrusted`, `tokio-rustls` y un `windows-sys` 0.52; el
  flatpak los descarga para resolver el grafo, pero no los compila.
- En Linux el aceptador envuelve el mismo `tokio_native_tls::TlsAcceptor` de antes; el fallo
  del saludo llega como `io::Error` en vez de `native_tls::Error`, y los dos transportes lo
  tratan igual: cierran esa conexión.
