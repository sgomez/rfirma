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

| Módulo | Qué es |
|---|---|
| `lib.rs` | El armado de la aplicación: las cinco raíces, los complementos, el registro de órdenes, la instancia única (ADR-0010) y el arranque de `site/application/startup/`. Sin pruebas propias. |
| `main.rs` | El binario. No hay nada dentro. |
| `crossing.rs` | El rasgo `WindowCrossing` y el macro `crossing!`, con los que se declara todo lo que cruza a la ventana. Pruebas en `crossing/tests.rs`. |
| `crossing/failure.rs` | `Failure`, lo que cruza cuando algo salió mal (ADR-0009); cada contexto traduce lo suyo en su `adapters/failures.rs`. Pruebas en `crossing/failure/tests.rs`. |
| `crossing/guards.rs` | Las guardas que ven todas las órdenes a la vez, entre ellas la de rutas del ADR-0011. Solo en pruebas. |
| `memory_error.rs` | `MemoryError` y su `Situation` (ADR-0009): la memoria entre sesiones es una sola (ADR-0010) y no es de ningún contexto. Pruebas en `memory_error/tests.rs`. |
| `compile_fail.rs` | Lo que no debe compilar: un doctest `compile_fail` por invariante que sostiene el sistema de tipos, y uno positivo por la misma ruta. |

`tests/agents_map_is_complete.rs` exige que todo `.rs` versionado bajo `src/`
esté nombrado aquí o en el mapa de su contexto: **un módulo nuevo se añade en la
misma PR que lo crea.** Las pruebas de un módulo van en su hermano `tests.rs`;
los andamios de grada A que comparten los contextos viven en
`identity/`, `signing/` y `site/application/tests.rs`.

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

`adapters/ffi.rs` de `signing/` la carga por una ruta relativa al ejecutable,
`../lib/rfirma`, y es la misma en los tres canales (ADR-0004): **no añadas
rutas ahí.** `RFIRMA_LIB_DIR` la sobreescribe, y eso es lo que ahorra
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
`signing/application/session/tests.rs`, `site/application/session/tests.rs`,
`site/application/filtering/tests.rs`, `tests/site_frontier_guards.rs`,
`crossing/guards.rs`, `tests/module_directions.rs`, `tests/single_cfg_os_site.rs`
y `tests/adr_citations_resolve.rs`. Mover un fichero que una lee obliga a
reapuntarla y a comprobar con un cebo que sigue poniéndose roja.
