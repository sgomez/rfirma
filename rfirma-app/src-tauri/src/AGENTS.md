# Mapa del backend (Rust / Tauri)

Este índice **sustituye a explorar el árbol**. El backend son cinco contextos,
cada uno con su mapa —`<contexto>/AGENTS.md`— y sus capas a la vista en la ruta
(ADR-0017, #408): `domain/` (reglas puras), `ports.rs` (los `trait` del
contexto), `application/` (casos de uso) y `adapters/` (todo lo que toca el
mundo, incluidas las órdenes y las vistas de Tauri).

| Contexto | Qué es | Empieza por |
|---|---|---|
| `site/` | El trámite de sede: protocolo `afirma://`, canal, TLS, confianza, arranque. | `site/AGENTS.md` |
| `signing/` | La firma local: reglas, ciclo trifásico, sesión, puente FFI y aislado. | `signing/AGENTS.md` |
| `documents/` | Destino, soltados, abiertos, recientes, rúbrica, documento en curso. | `documents/AGENTS.md` |
| `identity/` | PKCS#11, certificados, listados, `.p12`. | `identity/AGENTS.md` |
| `desktop/` | Canal de distribución, manejadores, invocación, versión publicada, rutas. | `desktop/AGENTS.md` |

`tests/agents_map_is_complete.rs` comprueba que todo `.rs` versionado bajo
`src/` está nombrado, por su ruta, aquí o en el mapa de su contexto. **Un
módulo nuevo se añade a la tabla que le toque en la misma PR que lo crea.**

## La regla de la dirección, y la deuda

Se lee de la ruta y no tiene excepciones escritas (RD-03): `domain/` de un
contexto no importa nada del crate; `application/` importa `domain/` y
`ports.rs` del propio contexto y `domain/` de otros; `adapters/` importa lo que
quiera del propio contexto, y los casos de uso de otro solo a través de
`lib.rs`, la única raíz que junta contextos. Lo vigila
`tests/module_directions.rs`.

Lo que hoy va contra eso está declarado, arista por arista, en
`tests/module_directions_debt.txt`. **La lista solo mengua**: una arista nueva
fuera de ella pone la guarda en rojo, y una línea que deje de ser infracción
también. Quién la vacía: #439 (los casos de uso hablan al puente y a los
motores por puertos), #440 (`Failure` deja de ser un tipo de `commands/`) y #443
(las raíces de composición por contexto, que reparten `Environment` y `Memory`
y sacan de `lib.rs` lo que no es cableado). Lo que no caiga en ninguno se anota
en #443. Para regenerarla, `MODULE_DIRECTIONS_DUMP=1 cargo test --test
module_directions -- --nocapture` la vuelca línea a línea.

## Lo que cuelga de la raíz

| Módulo | Líneas | Qué es |
|---|---|---|
| `commands/failure.rs` | 181 | Cómo se le cuenta a la ventana que algo salió mal (ID-29). Pruebas en `commands/failure/tests.rs` (40). |
| `commands/guards.rs` | 581 | Las cuatro guardas que ven todas las órdenes a la vez (ID-85), y las pruebas del descubrimiento de tipos. Descubren sus fuentes por ruta: `commands/` y, en cada `<contexto>/adapters/`, los `tauri*`, `views*` y `orders*`. Solo en pruebas. |
| `fixtures.rs` | 80 | Los andamios que comparten las pruebas de los casos de uso de todos los contextos. Solo en pruebas. |
| `lib.rs` | 405 | Registro de comandos, complementos y estados de Tauri, la instancia única (ID-160) y el arranque, que **obedece a `site/application/startup/` y no decide nada**: compone el transporte de producción (`site/adapters/transport.rs`), le pasa los tres puertos y obedece lo que devuelve (ID-324…ID-334). Absorbe hasta el #443 los dos repartos que desaparecieron: `Environment` —la raíz de composición— con la carpeta de destino elegida, y `Memory`, las dos memorias y sus dos soportes (ADR-0010). Empieza aquí para ver el cableado. Pruebas en `tests.rs` (59). |
| `main.rs` | 6 | El binario. No hay nada dentro. |
| `tests/memory.rs` | 336 | Las pruebas de `Memory`: los dos interruptores y lo exento. Las declara `tests.rs`. |

## Presupuesto de lectura

- **Para situarte, `just outline <ruta>`; nunca `cat` de un módulo de más de 300
  líneas.** Desde el esqueleto, `sed -n 'A,Bp;C,Dp'` con **todos** los tramos en
  una sola llamada. Cada mapa da el tamaño de cada módulo antes de que lo abras.
- **Las pruebas de un módulo van en su fichero hermano `tests.rs`, nunca dentro
  del módulo.** Para `x.rs`, el hermano es `x/tests.rs`; para `mod.rs`, es
  `tests.rs` en el mismo directorio. No lo leas salvo que vayas a tocarlo; para
  saber qué cubre: `grep -n 'fn ' <hermano>`.
- El primer bloque `//!` de cada módulo es su contrato: `head -40 <fichero>` casi
  siempre basta para decidir si es el fichero que buscas.

## Al añadir o cambiar una orden de Tauri

El cuerpo de la orden va en el `adapters/tauri.rs` de su contexto, `lib.rs` la
registra por su ruta entera en `generate_handler!`, y **lo que decide, en
`application/`**: si lo que estás escribiendo dentro de la orden no es
desempaquetar el `State` ni traducir el resultado, está en el fichero
equivocado.

`just contract` genera el contrato de las dos partes leyendo `commands/` y, en
cada `adapters/`, los `tauri*.rs`, `views*.rs` y `orders*.rs` —el adaptador de
Tauri de cada contexto, y nada más—. **No hay nada que actualizar** —una orden
nueva aparece por existir—, pero un tipo de salida sin `Serialize` no cruza y no
se publica, y un `#[tauri::command]` sin `async` sale publicado como
bloqueante, que es justo la trampa que cuelga la ventana. El extractor y la
guarda de rutas reconocen el tipo por el **macro** `derive(Serialize)` escrito
en el código, no por que implemente el rasgo.

Las guardas de conjunto están en `commands/guards.rs` y leen los mismos
ficheros: la lista cerrada de órdenes hay que renumerarla en
`the_list_of_commands_is_closed_and_this_is_how_long_it_is`; la guarda de rutas
(`the_portal_path_never_crosses_to_the_window`) descubre sola cada tipo que
derive `Serialize`, pero hay que decidir dónde entra —en
`crossings_from_a_portal_document` si detrás hay un documento, o en
`OUTPUTS_WITH_NO_DOCUMENT_BEHIND`—; y un comando que llame a un `blocking_*` de
un plugin necesita `#[tauri::command(async)]`. Los permisos de
`capabilities/*.json` solo filtran lo que la ventana pide y escucha, no lo que
Rust llama desde dentro de una orden. Una prueba nueva no se escribe contra la
orden, sino contra el caso de uso al que llama.

## Las pruebas que se leen a sí mismas

Vigilan invariantes leyendo el código **como texto**:
`signing/application/cycle/tests.rs` (la clave privada no cruza al puente,
ADR-0001, contando `bridge.`), `signing/application/session/tests.rs` (quién
escribe el sello de firmado, con una **lista fija** de ficheros que hay que
ampliar a mano), `site/application/session/tests.rs` (la postfirma de sede no
escribe nada), `tests/site_frontier_guards.rs`, `commands/guards.rs`,
`tests/module_directions.rs`, `tests/single_cfg_os_site.rs` y
`tests/adr_citations_resolve.rs`. Abren el `.rs` con `include_str!` o lo leen
del disco: mover un fichero que una de ellas lee obliga a reapuntarla, y a
comprobar con un cebo que sigue poniéndose roja.

## Al escribir un comentario

La regla es la restricción 6 del `AGENTS.md` raíz. La cabecera `//!` de un
módulo no repite lo que la tabla de su mapa ya dice de él. Las citas
`ID-NN`/`TD-NN` que ya hay se toleran hasta que la poda pase por su zona; no se
añaden nuevas. `tests/adr_citations_resolve.rs` vigila que cada `ADR-NNNN`
citado exista.
