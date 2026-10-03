# Mapa del backend (Rust / Tauri)

Este índice **sustituye a explorar el árbol**. El backend son cinco contextos,
cada uno con su mapa —`<contexto>/AGENTS.md`— y sus capas a la vista en la ruta
(ADR-0017): `domain/` (reglas puras), `ports.rs` (los `trait` del contexto),
`application/` (casos de uso) y `adapters/` (todo lo que toca el mundo,
incluidas las órdenes y las vistas de Tauri).

| Contexto | Qué es | Empieza por |
|---|---|---|
| `site/` | El trámite de sede: protocolo `afirma://`, canal, TLS, confianza, arranque. | `site/AGENTS.md` |
| `signing/` | La firma local: reglas, ciclo trifásico, sesión, puente FFI y aislado. | `signing/AGENTS.md` |
| `documents/` | Destino, soltados, abiertos, recientes, rúbrica, documento en curso. | `documents/AGENTS.md` |
| `identity/` | PKCS#11, certificados, listados, `.p12`. | `identity/AGENTS.md` |
| `desktop/` | Canal de distribución, manejadores, invocación, versión publicada, rutas. | `desktop/AGENTS.md` |

**La regla de la dirección se lee de la ruta y no tiene excepciones**: `domain/`
no importa nada del crate; `application/` importa `domain/` y `ports.rs` del
propio contexto y `domain/` de otros; `adapters/` importa lo que quiera del
propio contexto, y los casos de uso de otro solo por su raíz, `<contexto>/mod.rs`.
Lo vigila `tests/module_directions.rs`; una arista roja se mueve, no se anota.

Cada contexto tiene su **raíz de composición** en `<contexto>/mod.rs`: sus
adaptadores, su estado de proceso, sus puertos y la fachada que usan los vecinos.

## Lo que cuelga de la raíz

Cada módulo dice qué es en la primera línea `//!` de su fichero, y
`just outline rfirma-app/src-tauri/src/<carpeta>/` las junta en un índice: de un
contexto, de una capa o de `crossing/`. Lo que cuelga de la raíz es el armado
(`lib.rs`, `main.rs`, `event_loop.rs`), los dos errores sin contexto
(`memory_error.rs`, `startup_failure.rs`), `startup_dialog.rs` y
`compile_fail.rs`.

`tests/modules_open_with_a_header.rs` exige que todo `.rs` versionado bajo `src/`
que no sea de prueba abra con esa línea, sin partirla: **una frase, qué es y, si
ayuda, qué no es**, de 300 caracteres como mucho y sin citas a la spec ni a
issues. Las pruebas de un módulo van en su hermano `tests.rs`; los andamios de
grada A que comparten los contextos viven en `identity/`, `signing/` y
`site/application/tests.rs`.

## Al añadir o cambiar una orden de Tauri

El cuerpo va en el `adapters/tauri.rs` de su contexto y lo que decide, en
`application/`. `just contract` genera el contrato de las fuentes, así que no
hay nada que actualizar; pero un `#[tauri::command]` sin `async` sale como
bloqueante, y un tipo fuera de `crossing!` no cruza. Una prueba nueva ataca al
caso de uso, no a la orden.

**Bloqueante quiere decir el hilo del bucle de eventos.** Una orden sobre una
`fn` no `async` corre en `ExecutionContext::Blocking`; si dentro llama a un
`blocking_*` de un plugin (el de diálogo, por ejemplo), se cuelga para siempre
y sin error visible. La forma correcta es `#[tauri::command(async)]`, y
conviene fijarla con una prueba porque ninguna guarda la vigila.

**Un tipo nuevo en `crossing!` tiene que entrar en la guarda del ADR-0011.**
`the_portal_path_never_crosses_to_the_window` (`crossing/guards.rs`) construye
cada salida desde su caso de uso con un enlace del portal y recorre el JSON
campo a campo. Descubre sola el tipo, pero dónde entra se decide a mano: o se
construye en `crossings_from_a_portal_document`, o se declara en
`OUTPUTS_WITH_NO_DOCUMENT_BEHIND`, solo si detrás no puede haber ningún
documento. Las guardas hermanas se ponen rojas si se olvida cualquiera de las
dos cosas.

## La librería nativa en desarrollo

`adapters/ffi/location.rs` de `signing/` la busca por una ruta relativa al ejecutable,
`../lib/rfirma`, y es la misma en los tres canales de Linux (ADR-0004); el
instalador de Windows la deja junto al ejecutable, el `.app` de macOS en
`Contents/Frameworks`, y esa diferencia vive en
`Platform::native_library_directory` de `paths.rs` (ADR-0035): **no añadas
rutas en `ffi/location.rs`.** El nombre del fichero lo pone la plataforma con `DLL_PREFIX` y
`DLL_SUFFIX` de `std` (`library_file`): `librfirma_crypto.so` en Linux,
`rfirma_crypto.dll` en Windows, `librfirma_crypto.dylib` en macOS, sin ningún `cfg` (ADR-0035, ADR-0040). `RFIRMA_LIB_DIR` la sobreescribe, y eso es lo que ahorra
reconstruir la imagen desde un worktree: para la grada C,
`RFIRMA_LIB_DIR=<checkout principal>/rfirma-native-bridge/target/lib/rfirma`
reutiliza el `.so` ya compilado allí, unos tres minutos menos que `just native`.
El `.so` de `packaging/flatpak/build-dir/files/lib/rfirma/` **no sirve como
origen**: es el residuo de una compilación anterior del flatpak y envejece en
cuanto el puente añade un símbolo. El `MissingSymbol` que provoca llega al
trámite disfrazado de `SAF_08` («no se ha podido acceder al almacén de
claves»), que manda a investigar el almacén de certificados. Si la
reutilización falla así, reconstruye con `just native` desde el propio worktree.

Cualquier `cargo` necesita `rfirma-app/dist` ya construido, también
`cargo test --lib` y `cargo clippy --lib`: `lib.rs` llama a
`tauri::generate_context!()` y revienta con «The `frontendDist` configuration
is set to `"../dist"` but this path doesn't exist».

## Las pruebas que se leen a sí mismas

Leen el código **como texto**: `signing/application/cycle/tests.rs`,
`signing/application/session/tests.rs`, `signing/application/preview/tests.rs`,
`signing/adapters/tauri/tests.rs`, `site/application/session/tests.rs`,
`site/application/filtering/tests.rs`,
`site/application/errand/tests/token_and_launch.rs`,
`tests/site_frontier_guards.rs`, `crossing/guards.rs`,
`tests/module_directions.rs`, `tests/single_cfg_os_site.rs` y
`tests/adr_citations_resolve.rs`. Mover un fichero que una lee obliga a
reapuntarla y a comprobar con un cebo que sigue poniéndose roja. Entre ellas,
la guarda de `token_and_launch.rs` que comprueba que `headless` y
`mandatoryCertSelection` solo los lee el protocolo.

## Lo que no va en un comentario

La regla de los comentarios está en el `AGENTS.md` raíz; aquí, adónde va lo que
no cabe en ella:

* El porqué de una decisión va a un ADR; una advertencia a agentes de alcance
  general, a este mapa. Lo que ya dice la cabecera `//!` de un módulo no se repite, y un
  conteo, un número de PR o la interfaz del otro lado no van a ninguna parte.
* Una cita a un identificador de especificación (`ID-NN`, `TD-NN`, `RD-NN`,
  `RT-NN`) que ya exista se tolera mientras la poda no pase por su zona; al
  podar, pasa a citar el ADR que recoja la decisión o se borra. El código nuevo
  cita el ADR, nunca el identificador de spec. `tests/adr_citations_resolve.rs`
  vigila que cada `ADR-NNNN` citado tenga fichero en `docs/adr/`, y nada más.
* La regla aplica al código nuevo **y al movido**: mover un fichero es la
  ocasión de podarlo, no de trasladar su prosa.
* En revisión, un comentario nuevo de más de dos líneas pide justificación en la
  PR.
