# Mapa de `desktop`: el escritorio de la persona

El contexto del **escritorio**: en qué canal corre esto, quién atiende
`afirma://` y cómo se elige, la invocación desde fuera, la versión publicada y
las rutas de la máquina. `adapters/paths.rs` es el único fichero del repositorio
que conoce el sistema operativo (ADR-0010). `ports.rs` declara lo que los casos
de uso piden de fuera: el registro de manejadores del escritorio
(`HandlerRegistry`, adaptador en `adapters/registry.rs`) y la memoria de la
versión publicada (`VersionMemory`, que sirve `signing/adapters/memory.rs`).

Rutas relativas a `src/desktop/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera, y los casos de uso de otro contexto solo por su raíz
(`<contexto>/mod.rs`); lo vigila `tests/module_directions.rs`. Para situarte
en un fichero, `just outline <ruta>`; las pruebas de cada módulo viven en su
hermano `tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz: `DesktopRoot`, con las rutas, la invocación pendiente y la memoria de la versión. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `ports.rs` | **Los dos puertos**: `HandlerRegistry` y `VersionMemory`. |
| `adapters/process.rs` | Lo que este proceso sabe de sí mismo: su línea de órdenes, su carpeta y el relanzamiento cuando los argumentos no son UTF-8. |
| `adapters/channel.rs` | El canal de distribución (`/.flatpak-info`) y quién dice el escritorio que atiende `afirma://`, por GIO. Dentro del sandbox no llama a nada: no hay pregunta que valga (ID-240). Léelo antes que sus hermanos. Pruebas en `adapters/channel/tests.rs`. |
| `adapters/choice.rs` | Elegir manejador y leer al elegido: el `default` **explícito** en el `mimeapps.list` del `$HOME`, con todo lo demás intacto, y la advertencia de que Firefox guarda la suya aparte (ID-238, ID-241). Pruebas en `adapters/choice/tests.rs`. |
| `adapters/failures.rs` | La única traducción de las situaciones del escritorio a lo que ve la ventana (ADR-0009); ninguna llega a la sede. Pruebas en `adapters/failures/tests.rs`. |
| `adapters/paths.rs` | Las tres rutas de la memoria entre sesiones, más las cuatro de la CA local: dos ranuras, la que sirve y la siguiente. Único sitio que conoce el sistema operativo (ADR-0010), y el único que puede crear un fichero `0600` de nacimiento. Pruebas en `adapters/paths/tests.rs`. |
| `adapters/registry.rs` | `DesktopRegistry`: el adaptador de `HandlerRegistry` sobre `channel.rs` y `choice.rs`, con el canal detectado y el `mimeapps.list` del `$HOME`. |
| `adapters/releases.rs` | El único sitio que abre una conexión: le pregunta a GitHub por la última publicación y devuelve el cuerpo tal cual (ID-178, ID-182). Pruebas en `adapters/releases/tests.rs`. |
| `adapters/tauri.rs` | Las cuatro órdenes del escritorio: invocación —lo que trae se cuenta por `DocumentsRoot`—, versión publicada y manejadores de `afirma://`. |
| `adapters/views.rs` | Manejadores de `afirma://` y versión nueva, y su conversión desde `domain/handlers.rs`. Sin pruebas propias. |
| `application/handlers.rs` | Quién atiende `afirma://`, del escritorio a Preferencias y de vuelta, sobre el puerto `HandlerRegistry` (ID-238…ID-240). Devuelve `domain/handlers.rs`, nunca una vista. Pruebas en `application/handlers/tests.rs`. |
| `application/invocation.rs` | La invocación desde fuera, `rfirma documento.pdf`: qué trae —las rutas que propone, que la raíz de documentos clasifica y cuenta a la ventana—, qué hace la segunda y por dónde sale la URL `afirma://` que no es una ruta (ID-157…ID-160, ID-235, ID-236). Pruebas en `application/invocation/tests.rs`. |
| `application/version.rs` | Si hay una versión nueva publicada: el puerto de red doblable, la caché de 24 h sobre `VersionMemory` y la comparación de versiones (ID-177, ID-178, ID-180, ID-182). Pruebas en `application/version/tests.rs`. |
| `domain/error.rs` | Situaciones de elegir manejador (ADR-0009). Pruebas en `domain/error/tests.rs`. |
| `domain/handlers.rs` | Quién atiende `afirma://`, tal como lo decide el caso de uso, y el nombre de nuestro `.desktop`. Sin pruebas propias. |
| `domain/version_check.rs` | La última comprobación de versión, tal como se recuerda entre sesiones. |
