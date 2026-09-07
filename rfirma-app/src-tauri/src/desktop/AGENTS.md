# Mapa de `desktop`: el escritorio de la persona

El contexto del **escritorio**: en qué canal corre esto, quién atiende
`afirma://`, la invocación desde fuera, la versión publicada y las rutas de la
máquina. Ni firma ni documentos. Rutas relativas a `src/desktop/`.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz: `DesktopRoot`, con las rutas, la invocación pendiente y la memoria de la versión. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `ports.rs` | **Los dos puertos**: `HandlerRegistry` y `VersionMemory`, que sirve `signing/adapters/memory.rs`. |
| `adapters/process.rs` | Lo que este proceso sabe de sí mismo: su línea de órdenes, su carpeta y el relanzamiento cuando los argumentos no son UTF-8. |
| `adapters/channel.rs` | El canal de distribución (`/.flatpak-info`) y quién dice el escritorio que atiende `afirma://`. Léelo antes que sus hermanos. Pruebas en `adapters/channel/tests.rs`. |
| `adapters/choice.rs` | Elegir manejador y leer al elegido, en el `mimeapps.list` del `$HOME` y con todo lo demás intacto. Firefox guarda la suya aparte. Pruebas en `adapters/choice/tests.rs`. |
| `adapters/failures.rs` | La única traducción de las situaciones del escritorio a lo que ve la ventana (ADR-0009); ninguna llega a la sede. Pruebas en `adapters/failures/tests.rs`. |
| `adapters/paths.rs` | Las rutas de la memoria entre sesiones y las de la CA local. Único sitio que conoce el sistema operativo (ADR-0010) y el único que crea un fichero `0600` de nacimiento. Pruebas en `adapters/paths/tests.rs`. |
| `adapters/registry.rs` | `DesktopRegistry`: el adaptador de `HandlerRegistry` sobre `channel.rs` y `choice.rs`. |
| `adapters/releases.rs` | El único sitio que abre una conexión: le pregunta a GitHub por la última publicación. Pruebas en `adapters/releases/tests.rs`. |
| `adapters/tauri.rs` | Las cuatro órdenes del escritorio: invocación —lo que trae lo cuenta `DocumentsRoot`—, versión publicada y manejadores de `afirma://`. |
| `adapters/views.rs` | Lo que cruza a la ventana: manejadores de `afirma://` y versión nueva, desde `domain/handlers.rs`. Sin pruebas propias. |
| `application/handlers.rs` | Quién atiende `afirma://`, del escritorio a Preferencias y de vuelta. Devuelve dominio, nunca una vista. Pruebas en `application/handlers/tests.rs`. |
| `application/invocation.rs` | La invocación desde fuera, `rfirma documento.pdf`: qué trae, qué hace la segunda y por dónde sale la URL `afirma://` que no es una ruta. Pruebas en `application/invocation/tests.rs`. |
| `application/version.rs` | Si hay una versión nueva publicada, con su caché de 24 h. Pruebas en `application/version/tests.rs`. |
| `domain/error.rs` | Las situaciones de elegir manejador (ADR-0009). Pruebas en `domain/error/tests.rs`. |
| `domain/handlers.rs` | Quién atiende `afirma://` tal como lo decide el caso de uso, y el nombre de nuestro `.desktop`. Sin pruebas propias. |
| `domain/version_check.rs` | La última comprobación de versión, tal como se recuerda entre sesiones. |
