# Mapa de `desktop`: el escritorio de la persona

El contexto del **escritorio**: en qué canal corre esto, quién atiende
`afirma://` y cómo se elige, la invocación desde fuera, la versión publicada y
las rutas de la máquina. `adapters/paths.rs` es el único fichero del repositorio
que conoce el sistema operativo (ADR-0010).

Rutas relativas a `src/desktop/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera; lo que hoy se salta eso está en
`tests/module_directions_debt.txt` y solo mengua. Para situarte en un fichero,
`just outline <ruta>`; las pruebas de cada módulo viven en su hermano
`tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `mod.rs`, `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | | Solo `pub mod`: el reparto del contexto y el de cada capa. |
| `adapters/channel.rs` | 99 | El canal de distribución (`/.flatpak-info`) y quién dice el escritorio que atiende `afirma://`, por GIO. Dentro del sandbox no llama a nada: no hay pregunta que valga (ID-240). Léelo antes que sus hermanos. Pruebas en `adapters/channel/tests.rs` (71). |
| `adapters/choice.rs` | 212 | Elegir manejador y leer al elegido: el `default` **explícito** en el `mimeapps.list` del `$HOME`, con todo lo demás intacto, y la advertencia de que Firefox guarda la suya aparte (ID-238, ID-241). Pruebas en `adapters/choice/tests.rs` (221). |
| `adapters/failures.rs` | 22 | La única traducción de las situaciones del escritorio a lo que ve la ventana (ADR-0009); ninguna llega a la sede. Pruebas en `adapters/failures/tests.rs` (30). |
| `adapters/paths.rs` | 269 | Las tres rutas de la memoria entre sesiones, más las cuatro de la CA local: dos ranuras, la que sirve y la siguiente. Único sitio que conoce el sistema operativo (ADR-0010), y el único que puede crear un fichero `0600` de nacimiento. Pruebas en `adapters/paths/tests.rs` (301). |
| `adapters/releases.rs` | 37 | El único sitio que abre una conexión: le pregunta a GitHub por la última publicación y devuelve el cuerpo tal cual (ID-178, ID-182). Pruebas en `adapters/releases/tests.rs` (9). |
| `adapters/tauri.rs` | 58 | Las cuatro órdenes del escritorio: invocación, versión publicada y manejadores de `afirma://`. |
| `adapters/views.rs` | 61 | Manejadores de `afirma://` y versión nueva, y su conversión desde `domain/handlers.rs`. Sin pruebas propias. |
| `application/handlers.rs` | 45 | Quién atiende `afirma://`, del escritorio a Preferencias y de vuelta: lo que se puede saber y lo que se escribe (ID-238…ID-240). Devuelve `domain/handlers.rs`, nunca una vista. Pruebas en `application/handlers/tests.rs` (49). |
| `application/invocation.rs` | 216 | La invocación desde fuera, `rfirma documento.pdf`: qué abre, qué hace la segunda y por dónde sale la URL `afirma://` que no es una ruta (ID-157…ID-160, ID-235, ID-236). Pruebas en `application/invocation/tests.rs` (265). |
| `application/version.rs` | 107 | Si hay una versión nueva publicada: el puerto de red doblable, la caché de 24 h y la comparación de versiones (ID-177, ID-178, ID-180, ID-182). Pruebas en `application/version/tests.rs` (192). |
| `domain/error.rs` | 52 | Situaciones de elegir manejador (ADR-0009). Pruebas en `domain/error/tests.rs` (11). |
| `domain/handlers.rs` | 23 | Quién atiende `afirma://`, tal como lo decide el caso de uso. Sin pruebas propias. |
