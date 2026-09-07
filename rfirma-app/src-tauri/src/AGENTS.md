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

Cada contexto tiene su **raíz de composición** en `<contexto>/mod.rs` —
`IdentityRoot`, `DocumentsRoot`, `DesktopRoot`, `SigningRoot`, `SiteRoot`—: sus
adaptadores, su estado de proceso y sus puertos instanciados, más la fachada
que usan los vecinos. `lib.rs` las construye en ese orden sobre la misma
`Memory` y las registra con cinco `manage()`.

## Lo que cuelga de la raíz

| Módulo | Líneas | Qué es |
|---|---|---|
| `lib.rs` | 304 | `roots()`, que construye las cinco raíces, y `run()`: complementos, órdenes por su ruta entera en `generate_handler!`, instancia única (ADR-0010) y el arranque, que obedece a `site/application/startup/`. Sin pruebas propias. |
| `main.rs` | 6 | El binario. No hay nada dentro. |
| `crossing.rs` | 365 | `WindowCrossing`, el rasgo de lo que cruza a la ventana, y `crossing!`, el macro que lo declara y deja su forma en un registro de `inventory`. De ahí salen los tipos del contrato y la guarda de rutas. Pruebas en `crossing/tests.rs` (174). |
| `crossing/failure.rs` | 31 | `Failure`, lo que cruza cuando algo salió mal (ADR-0009); cada contexto traduce lo suyo en su `adapters/failures.rs`. Pruebas en `crossing/failure/tests.rs` (11). |
| `crossing/guards.rs` | 610 | Las guardas que ven todas las órdenes a la vez: la lista cerrada de órdenes, la guarda de rutas del ADR-0011, que todo lo que nombra una orden esté en el registro, y que toda orden que toque el portal sea `async`. Solo en pruebas. |
| `memory_error.rs` | 56 | `MemoryError` y su `Situation` (ADR-0009): la memoria entre sesiones es una sola (ADR-0010) y los puertos de cuatro contextos hablan de ella, así que no es de ninguno. Pruebas en `memory_error/tests.rs` (16). |
| `compile_fail.rs` | 97 | Lo que ya no compila: un doctest `compile_fail` por cada invariante que sostiene el sistema de tipos (`SealedPreSignature`, `CompletedCycle`, `SafCode`, `WindowCrossing`), y uno positivo por las mismas rutas. |

`tests/agents_map_is_complete.rs` exige que todo `.rs` versionado bajo `src/`
esté nombrado aquí o en el mapa de su contexto: **un módulo nuevo se añade en la
misma PR que lo crea.** Las pruebas de un módulo van en su hermano `tests.rs`;
los andamios de grada A que comparten los contextos viven en
`identity/`, `signing/` y `site/application/tests.rs`.

## Al añadir o cambiar una orden de Tauri

El cuerpo va en el `adapters/tauri.rs` de su contexto y lo que decide en
`application/`: la orden saca del `State` su raíz, resuelve las asas por las
fachadas, llama al caso de uso con dominio o `&dyn Puerto` y traduce el
resultado. `just contract` genera el contrato de las fuentes: no hay nada que
actualizar, pero un `#[tauri::command]` sin `async` sale como bloqueante, y un
tipo fuera de `crossing!` no cruza. Una prueba nueva ataca al caso de uso, no a
la orden.

## Las pruebas que se leen a sí mismas

Leen el código **como texto**: `signing/application/cycle/tests.rs`,
`signing/application/session/tests.rs`, `site/application/session/tests.rs`,
`site/application/filtering/tests.rs`, `tests/site_frontier_guards.rs`,
`crossing/guards.rs`, `tests/module_directions.rs`, `tests/single_cfg_os_site.rs`
y `tests/adr_citations_resolve.rs`. Mover un fichero que una lee obliga a
reapuntarla y a comprobar con un cebo que sigue poniéndose roja.
