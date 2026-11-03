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
| Token PKCS#11 y sus almacenes | `MacosToken`: el llavero por Security.framework para el Almacén de macOS, que firma el resumen con la clave del llavero; el DNIe por rdnie; y el `RealToken` de Linux para los módulos, que solo encuentra si `RFIRMA_PKCS11_MODULE` lo nombra. |
| `mlock` del secreto | `mlock`; `MADV_DONTDUMP` es solo de Linux. |
| Cerrojos (Firefox, carpeta de paso) | Los de Unix, sin cambios. |
| Llavero `oo7` (ADR-0034) | `PendingMacosKeychain`: responde que no hay llavero. |

La firma trifásica (ADR-0001) funciona de extremo a extremo con una clave en memoria: PAdES,
CAdES y XAdES pasan por el puente y el validador de AutoFirma da por válidas las PAdES. El PIN
de una tarjeta por PKCS#11 lo pide una alerta de AppKit con campo seguro, modal sobre la ventana
que lo pidió (ADR-0047).

## En macOS, la API de macOS siempre que se pueda

Como en Windows (ADR-0035), para certificados, PIN y confianza rFirma usa lo que macOS ofrece
(Security.framework y LaunchServices): es lo que la persona ya conoce y evita mantener un camino
propio. De la regla salen:

- **Los certificados con clave de software del llavero** salen como «Almacén de macOS» y se firman
  por la API del sistema: Rust calcula el resumen y macOS lo firma (ADR-0001). La contraseña o el
  permiso los pide macOS con su diálogo. Una identidad con la clave en un token del sistema no sale.
- **«Instalar certificado» importa el `.p12` al llavero de inicio de sesión** (ADR-0034); en macOS
  no hay Almacén de rFirma.
- **La CA local va a la confianza de la persona usuaria** (ADR-0005), con el diálogo de contraseña
  del sistema, y `afirma://` se gestiona por LaunchServices.

Las tarjetas quedan fuera de la regla: macOS no trae controlador del DNIe, y ni el módulo de la
DGP ni OpenSC son una vía fiable en Apple Silicon. Por eso:

- **El DNIe 3.0 y 4.0 por contacto lo lee rFirma por PC/SC con rdnie**, el driver propio del
  proyecto: certificados sin PIN, intentos restantes antes de pedirlo y firma del resumen en la
  tarjeta, todo en Rust (ADR-0001). Su convivencia con los módulos PKCS#11, en el ADR-0048.
- **Cualquier otra tarjeta solo entra por PKCS#11 si `RFIRMA_PKCS11_MODULE` nombra su módulo.**
  Sin él, la línea del lector da el mensaje genérico de tarjeta ilegible, y la variable se
  explica en la documentación.

Los lectores se vigilan por PC/SC y la línea del lector aparece como en Linux y Windows.

El orden de almacenes pasa a ser: tarjeta, Almacén de Windows o de macOS, Almacén de rFirma,
NSS del sistema, familia Firefox, familia Chromium.

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

## Sin cuenta de Apple Developer: tap de Homebrew y actualizador propio

No hay cuenta de Apple Developer: el paquete no se firma con Developer ID ni se notariza, y
mantiene la firma ad hoc y el *hardened runtime* apagado, que además deja cargar módulos PKCS#11
de terceros. Con firma ad hoc, cada versión es otra identidad de código para macOS, y el
«Permitir siempre» del llavero puede volver a pedirse tras cada actualización.

- **Distribución:** un tap de Homebrew propio del proyecto, con un cask que instala el `.app`
  publicado en la Release; se actualiza en cada versión. Las instrucciones de instalación,
  incluido `xattr -c /Applications/rFirma.app` para abrir la aplicación ante el aviso de
  Gatekeeper la primera vez, van en la documentación del tap y en la de rFirma.
- **Actualización:** rFirma se actualiza sola con el canal de Windows: la entrega publica el
  paquete de actualización firmado con minisign y su entrada en `latest.json`. El cask declara
  `auto_updates: true`, así que `brew upgrade` no lo toca, y rFirma no trata una instalación por
  Homebrew como gestionada por paquete. La descarga del actualizador no lleva la marca de
  cuarentena, así que el aviso de Gatekeeper solo sale en la primera instalación; está por
  verificar con la firma ad hoc.

## GraalVM: la 25.0.2 en el CI, la 25.0.1 en un Mac Intel

El CI construye con la 25.0.2 fijada (ADR-0035) en `macos-15`, que es Apple Silicon. La 25.0.2 no
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
Apple y secretos de firma en el CI. No hay cuenta de Apple Developer; el instalador de Windows tampoco
está firmado.

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
- El CI tiene un job `macos` en `platforms.yml` que compila la `.dylib`, pasa `rustfmt`, `clippy`
  y `just test-macos`. Corre en un PR solo si toca algo que solo compila macOS o que cambia cómo
  se compila allí, o lleva `ci-macos` o `ci-full`, y en `main` en la nocturna (ADR-0014).
  La entrega (`build.yml`) construye el `.dmg` con firma ad hoc y el *hardened runtime* desactivado.
- `lib.rs` genera el contexto de Tauri una sola vez en `run` y `event_loop.rs` lo construye: en macOS `generate_context!()` define
  el `Info.plist` embebido, y llamarla dos veces falla al enlazar.
- Los códigos con que macOS rechaza un certificado de sede no reconocido (ADR-0039) se añaden a
  los de OpenSSL y Schannel, sin condicional de sistema.
- La CA local (ADR-0005) se confía en el dominio de la persona usuaria, con el diálogo de
  contraseña de macOS; Safari, Chrome y Firefox 120 o posterior la leen de ahí, sin perfiles NSS.
