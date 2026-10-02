# macOS en Apple Silicon como tercera plataforma

rFirma compila también en macOS para Apple Silicon (`aarch64-apple-darwin`). Entra igual que
Windows (ADR-0035): Linux no cambia, las dependencias van por target, los condicionales de
sistema viven en la lista cerrada de `tests/single_cfg_os_site.rs`, y cada pieza que solo existe
en un escritorio Linux tiene en macOS un **adaptador pendiente** que respeta el puerto, falla
como fallaría Linux sin el servicio y dice en su nombre qué falta.

## Lo que ya funciona y lo que queda pendiente

| Pieza en Linux | En macOS |
| --- | --- |
| OpenSSL del sistema | `openssl` con `vendored`, como en Windows: el binario no depende del de Homebrew y compila en cruzado. |
| Canal TLS local con `native-tls` | rustls, como en Windows (ADR-0036). |
| Token PKCS#11 y sus almacenes | El mismo `RealToken` de Linux; solo encuentra módulo si `RFIRMA_PKCS11_MODULE` lo nombra. |
| `mlock` del secreto | `mlock`; `MADV_DONTDUMP` es solo de Linux. |
| Cerrojos (Firefox, carpeta de paso) | Los de Unix, sin cambios. |
| Diálogo GTK del PIN | `PendingMacosPinDialog`: falla con un mensaje en castellano. |
| Llavero `oo7` (ADR-0034) | `PendingMacosKeychain`: responde que no hay llavero. |
| Manejadores de `afirma://` por GIO | `PendingMacosLaunchServices`: la lista no está disponible y elegir falla. |
| CA local en los almacenes NSS (ADR-0005) | `PendingMacosKeychainTrust`, sin perfiles: la CA consta como no instalada. |
| Diálogo GTK de fallo de arranque | El texto sale por la salida de error. |

La firma trifásica (ADR-0001) funciona de extremo a extremo con una clave en memoria: PAdES,
CAdES y XAdES pasan por el puente y el validador de AutoFirma da por válidas las PAdES. Lo que
falta para firmar con una tarjeta es el diálogo del PIN; para el almacén del sistema, un adaptador
del llavero de macOS como el de CNG en Windows.

## `afirma://` llega como evento, no como argumento

LaunchServices no pasa la URL en la línea de órdenes: abre la aplicación y se la entrega con un
Apple Event, que Tauri presenta como `RunEvent::Opened`. Para que siga valiendo un proceso por
trámite (ADR-0024), el proceso que recibe la URL lanza el mismo ejecutable con la URL como
argumento, y a partir de ahí el camino es el de Linux y Windows. El esquema se declara en el
`Info.plist` que añade `packaging/macos/Info.plist`.

## La biblioteca nativa va en `Contents/Frameworks`

`native-image` emite `librfirma_crypto.dylib` con las mismas banderas de
`native-image.properties`, y en macOS no emite auxiliares de AWT, solo cabeceras. Solo depende de
bibliotecas y *frameworks* del sistema. `just native` la instala con el nombre de instalación
`@rpath/librfirma_crypto.dylib` —si no, guarda la ruta absoluta de la compilación— y la vuelve a
firmar ad hoc, porque en arm64 cambiar el nombre invalida la firma y el cargador rechaza una
biblioteca sin firma válida.

Dentro del `.app` el ejecutable está en `Contents/MacOS` y la biblioteca en
`Contents/Frameworks`, que es donde la busca `Platform::native_library_directory` (`../Frameworks`).
`RFIRMA_LIB_DIR` la sigue sobreescribiendo en desarrollo.

## El paquete es un `.dmg` firmado ad hoc y sin notarizar

La variante `[macos]` de `just bundle` construye con el *bundler* de Tauri un `.app` y un `.dmg` para
`aarch64-apple-darwin`, con la configuración de `packaging/macos/tauri.macos.json` pasada por
`--config`, como la de Windows, para que `tauri.conf.json` no cambie. La firma es ad hoc
(`signingIdentity: "-"`): en Apple Silicon un binario sin ninguna firma no arranca. El *hardened
runtime* va apagado porque activa la validación de bibliotecas, que rechazaría una `.dylib`
firmada ad hoc sin identificador de equipo. Sin notarizar, Gatekeeper avisa al abrirlo la primera
vez y hay que elegir «Abrir igualmente».

El mínimo es macOS 15.4 (`minimumSystemVersion`): pdf.js usa el global `Iterator`, que el WebKit
del sistema no trae hasta esa versión, y sin él la ventana se queda en negro. No deja fuera ningún
equipo, porque todo Mac con Apple Silicon puede actualizar a ella.

El `.dmg` pasa por la misma puerta del contenido que los demás paquetes
(`packaging/verifica-contenido.sh`): una sola `librfirma_crypto.dylib` y ningún auxiliar de AWT.

## GraalVM: la 25.0.2 en el CI, la 25.0.1 en un Mac Intel

El CI construye con la 25.0.2 fijada (ADR-0035) en `macos-14`, que es Apple Silicon. La 25.0.2 no
publica versión para macOS x64, así que en un Mac Intel se desarrolla con la 25.0.1, que sí la
publica. `native-image` no compila en cruzado: la `.dylib` de un Mac Intel es x86_64 y solo sirve
para probar allí; el paquete arm64 sale siempre del CI o de un Mac con Apple Silicon. El código de
Rust es el mismo en las dos arquitecturas y compila en cruzado con
`--target aarch64-apple-darwin`.

GraalVM se busca en `GRAALVM_HOME`, después en `JAVA_HOME` y por último con
`/usr/libexec/java_home -v 25`.

## Considered Options

**Un binario universal (arm64 y x86_64)**: exige dos `.dylib`, y la de x86_64 solo sale de la
25.0.1 en un runner Intel. Los Mac Intel son una plataforma que Apple ya no vende.

**Notarizar con un Developer ID**: quita el aviso de Gatekeeper, pero exige una cuenta de pago de
Apple y secretos de firma en el CI. Queda para cuando haya quien la mantenga; el instalador de
Windows tampoco está firmado.

**Instalar la `.dylib` en `Contents/Resources`**: funciona igual para `dlopen`, pero
`Frameworks` es donde macOS espera el código cargable y donde `codesign` lo firma al sellar el
`.app`.

**OpenSSL de Homebrew**: obligaría a distribuir su `libssl` dentro del `.app` y a tenerlo para
cada arquitectura.

**Registrar `afirma://` con `tauri-plugin-deep-link`**: da el mismo evento, pero añade una
dependencia en todas las plataformas para algo que en Linux y Windows ya resuelve el sistema.

## Consequences

- `just tools`, `just bootstrap`, `just native`, `just dev`, `just fmt`, `just test-macos` y
  `just bundle` funcionan en macOS; las gradas B y C que necesitan softhsm o NSS no corren.
- El CI tiene un job `macos` en `ci.yml` que compila la `.dylib`, pasa `rustfmt`, `clippy` y
  `just test-macos`.
- El `.dmg` no se publica en la entrega: `build.yml` no lo construye mientras los adaptadores de
  firma estén pendientes, porque un paquete que no puede firmar no sirve a quien lo descarga.
- `lib.rs` genera el contexto de Tauri una sola vez en `run` y `event_loop.rs` lo construye: en macOS `generate_context!()` define
  el `Info.plist` embebido, y llamarla dos veces falla al enlazar.
- Los códigos con que macOS rechaza un certificado de sede no reconocido (ADR-0039) se añaden a
  los de OpenSSL y Schannel, sin condicional de sistema.
- Sin diálogo del PIN, llavero ni CA local, un trámite de sede en macOS no puede todavía firmar
  con tarjeta ni el navegador confía en el canal local sin instalar la CA a mano.
