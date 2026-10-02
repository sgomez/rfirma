# Mapa de `desktop`: el escritorio de la persona

El contexto del **escritorio**: en qué canal corre esto, quién atiende
`afirma://`, la invocación desde fuera, la versión publicada y las rutas de la
máquina. Ni firma ni documentos. Rutas relativas a `src/desktop/`.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz: `DesktopRoot`, con las rutas, la invocación pendiente y la memoria de la versión. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `ports.rs` | **Los puertos**: `HandlerRegistry`, `VersionMemory`, que sirve `signing/adapters/memory.rs`, `UpdateInstaller`, y los de la línea de órdenes: `CertificateStores`, `CertificateFilter`, `Terminal`, `DesktopHandover`, `CommandLineFiles`, `SignatureVerifier` y `DocumentSigner`. |
| `adapters/process.rs` | Lo que este proceso sabe de sí mismo: su línea de órdenes, su carpeta, el relanzamiento cuando los argumentos no son UTF-8 y el proceso de sede de cada URL que macOS entrega por Apple Event. No decide el rol. |
| `adapters/channel.rs` | El canal de distribución (`/.flatpak-info`) y quién dice el escritorio que atiende `afirma://`. Léelo antes que sus hermanos. Pruebas en `adapters/channel/tests.rs`. |
| `adapters/choice.rs` | Elegir, leer o retirar el manejador, en el `mimeapps.list` del `$HOME` y con todo lo demás intacto. Firefox guarda la suya aparte. Pruebas en `adapters/choice/tests.rs`. |
| `adapters/command_line_ports.rs` | Los adaptadores de los puertos de la línea de órdenes: `DiskFiles`, el disco, y `NativeVerifier` y `NativeFilter`, el validador y el motor de filtros del original en la librería nativa, que no se cargan hasta que se les pregunta. Sin pruebas propias. |
| `adapters/failures.rs` | La única traducción de las situaciones del escritorio a lo que ve la ventana (ADR-0009); ninguna llega a la sede. Pruebas en `adapters/failures/tests.rs`. |
| `adapters/firefox_lock.rs` | Si Firefox tiene abierto un perfil: el bloqueo POSIX de `.parentlock`, o `parent.lock` sin compartir en Windows. Pruebas en `adapters/firefox_lock/tests.rs` y `adapters/firefox_lock/windows_tests.rs`. |
| `adapters/handover.rs` | `SpawnedDesktop`: entrega un fichero al escritorio lanzando otro proceso de rFirma, que la instancia única reenvía si ya hay uno. Sin pruebas propias. |
| `adapters/installer.rs` | El adaptador de `UpdateInstaller`: el plugin updater en Windows, que solo se registra con su configuración, y «no disponible» en el resto (ADR-0035). Sin pruebas propias. |
| `adapters/paths.rs` | Las rutas de la memoria entre sesiones y las de la CA local. Único sitio que conoce el sistema operativo (ADR-0010) y el único que crea un fichero `0600` de nacimiento. Pruebas en `adapters/paths/tests.rs`. |
| `adapters/registry.rs` | `DesktopRegistry`: el adaptador de `HandlerRegistry` sobre `channel.rs` y `choice.rs` en Linux, sobre `registry/windows_classes.rs` en Windows, y el pendiente de Launch Services en macOS. |
| `adapters/registry/windows_classes.rs` | Quién abre `afirma://` en Windows: `HKCU\Software\Classes` sobre `HKLM`, y la rama de rFirma en la del usuario (ADR-0035). Pruebas en `adapters/registry/windows_classes/tests.rs`. |
| `adapters/releases.rs` | El único sitio que abre una conexión: le pregunta a GitHub por la última publicación. Pruebas en `adapters/releases/tests.rs`. |
| `adapters/terminal.rs` | La entrada de la línea de órdenes, `run_the_command_line`, y los adaptadores de sus puertos: `SeenStores`, `ProcessTerminal` y `RootsSigner`, que firma por el camino de la sede. Ni Tauri ni ventana; sus pruebas, con el binario, en `tests/command_line.rs`. |
| `adapters/terminal/tty.rs` | El PIN tecleado sin eco en `/dev/tty`, que no toca stdin ni stdout. Solo Unix. Pruebas en `adapters/terminal/tty/tests.rs`. |
| `adapters/tauri.rs` | Las órdenes del escritorio: invocación, estado de la barra de título nativa, versión publicada, manejadores de `afirma://` y su elección, destino externo, estado y retirada. Pruebas en `adapters/tauri/tests.rs`. |
| `adapters/titlebar.rs` | La barra de título nativa de GTK de la ventana principal en Linux, y la composición de WebKitGTK que piden sus popovers: sus eventos, cuándo aplicar el estado de la ventana y qué dice cada reciente. Nada en Windows ni macOS. Pruebas en `adapters/titlebar/tests.rs`. |
| `adapters/titlebar/gtk_titlebar.rs` | Los widgets GTK de esa barra: sus acciones `hdr`, el popover propio de los recientes y el ☰. Solo Linux, sin pruebas propias. |
| `adapters/views.rs` | Lo que cruza a la ventana: manejadores de `afirma://`, versión nueva, señales de estado, resultado de la retirada y la barra de título nativa. Sin pruebas propias. |
| `adapters/webkit_renderer.rs` | Si WebKitGTK debe componer sin la GPU en esta sesión (ADR-0007): solo la decisión; la fija `titlebar.rs`. Pruebas en `adapters/webkit_renderer/tests.rs`. |
| `application/command_line.rs` | El caso de uso de la línea de órdenes (ADR-0041): de los argumentos al código de salida, los bytes de stdout y las líneas de stderr, sin escribir en ningún flujo. Pruebas en `application/command_line/tests.rs`. |
| `application/command_line/config.rs` | El `-config` de `sign`: sus propiedades, con las reglas de las `properties` de una sede. No las expande. Pruebas en `application/command_line/config/tests.rs`. |
| `application/command_line/tests/filter_and_xml.rs` | Las pruebas de `-filter` y de la respuesta de `-xml`, partidas de `application/command_line/tests.rs`. |
| `application/command_line/tests/sign_config.rs` | Las pruebas de `sign -config` en el caso de uso, partidas de `application/command_line/tests.rs`. |
| `application/command_line/verify.rs` | La orden `verify`: el formato que detecta `-format auto` y un resultado de validez por línea, como el original. No el XML de `-xml`. Pruebas en `application/command_line/verify/tests.rs`. |
| `application/destination.rs` | Abrir un destino externo conocido en el navegador. Pruebas en `application/destination/tests.rs`. |
| `application/handlers.rs` | Quién atiende `afirma://`, del escritorio a Preferencias y de vuelta. Devuelve dominio, nunca una vista. Pruebas en `application/handlers/tests.rs`. |
| `application/invocation.rs` | La invocación desde fuera, `rfirma documento.pdf`: qué trae, qué hace la segunda —solo del escritorio (ADR-0024)— el rol de proceso que decide `role_of`, terminal incluido, y el de las URL entregadas por Apple Event. Pruebas en `application/invocation/tests.rs`. |
| `application/store_scope.rs` | Los certificados que deja `-store` (ADR-0022): la familia NSS o un módulo ya descubierto; nunca carga uno por nombrarlo la orden. |
| `application/status.rs` | Evaluación y medición de las señales del panel de estado. Pruebas en `application/status/tests.rs`. |
| `application/version.rs` | Si hay una versión nueva publicada: pregunta siempre y, si falla, usa la última conocida; e instalarla, solo si es mayor que la que corre. Pruebas en `application/version/tests.rs`. |
| `application/withdrawal.rs` | Qué reintentar y cómo fusionar el resultado al retirar rFirma, sin puertos: decisión pura. Pruebas en `application/withdrawal/tests.rs`. |
| `domain/channel.rs` | El canal de distribución en el que corre el proceso (ADR-0015). Sin pruebas propias. |
| `domain/command_line.rs` | Las órdenes de AutoFirma que se reconocen, su sintaxis y lo que se rechaza de ellas (ADR-0041); no las ejecuta. Pruebas en `domain/command_line/tests.rs`. |
| `domain/sign_arguments.rs` | Los argumentos de `sign` y `cosign` analizados y lo que se rechaza de ellos (ADR-0041); no firma ni abre almacenes. Pruebas en `domain/sign_arguments/tests.rs`. |
| `domain/store_scope.rs` | A qué almacenes acota el valor de `-store`, con los nombres de la línea de órdenes, y lo que se rechaza de él (ADR-0041); no abre ninguno. Pruebas en `domain/store_scope/tests.rs`. |
| `domain/destination.rs` | Destino externo reconocido por la aplicación y su dirección web. Pruebas en `domain/destination/tests.rs`. |
| `domain/error.rs` | Las situaciones de elegir manejador (ADR-0009). Pruebas en `domain/error/tests.rs`. |
| `domain/handlers.rs` | Quién atiende `afirma://` tal como lo decide el caso de uso, y el nombre de nuestro `.desktop`. Sin pruebas propias. |
| `domain/installation.rs` | El resultado cerrado de instalar la versión anunciada y los fallos del instalador. Sin pruebas propias. |
| `domain/status.rs` | Las cuatro señales, cinco veredictos y tres tipos de acción del panel de estado. Sin pruebas propias. |
| `domain/version_check.rs` | La última comprobación de versión, tal como se recuerda entre sesiones. |
| `domain/withdrawal.rs` | El resultado de retirar algo propio de rFirma: por manejador o por almacén NSS. Sin pruebas propias. |
