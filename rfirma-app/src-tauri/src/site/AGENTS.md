# Mapa de `site`: el trámite de sede

Este contexto es el trámite que abre una sede: el protocolo `afirma://`, el canal
por el que se habla con ella, el material TLS y la confianza de la CA local. La
firma en sí no vive aquí, sino en `signing/`. Las rutas son relativas a
`src/site/`, y las pruebas de cada módulo están en su hermano `tests.rs`.

## Dónde vive qué

Cada módulo dice qué es en su cabecera `//!`, y `just outline
rfirma-app/src-tauri/src/site/` las junta en un índice; acótalo a un
subdirectorio (`site/domain/protocol/`) si ya sabes la capa.

## Al tocar lo que sale hacia la sede

Todo lo que la sede recibe cuando no sale una firma pasa por
`domain/protocol/codes.rs` —el catálogo cerrado— y se decide en
`adapters/frontier.rs`, la única traducción de un `SiteRefusal` a un código y a
una vista a la vez. El trámite no conoce ni `Failure` ni `SafCode`: devuelve la
situación. Dos cosas que salen mal si se olvidan:

- **Un código no se escribe a mano.** Se construye un `WireAnswer` y se llama a
  `on_the_wire()`. `tests/site_frontier_guards.rs` pone en rojo cualquier código
  acuñado, comprueba que el elegido esté en el catálogo y que no sea nunca
  `SAF_48`, que la 1.9.2 no puede producir; y `SafCode` no se construye desde
  texto, con el cebo en `compile_fail.rs` de la raíz.
- **Una situación nueva no compila** hasta que se le decide código y vista:
  `told` en `adapters/frontier.rs` es un `match` cerrado sobre `SiteRefusal`, y
  cada contexto tiene el suyo en su `adapters/failures.rs`.

## Al tocar el trámite

- **Una decisión del trámite** —qué se enseña, qué se contesta, qué se
  recuerda— va en `application/errand/`: en `desk.rs` si se toma sobre la mesa,
  en `replies.rs` si es lo que la sede recibe, en `state.rs` si es memoria. Los
  verbos de `mod.rs` son la única puerta, y una orden de `adapters/tauri.rs` no
  hace más que llamar a uno.
- **Cómo se escribe algo en el cable** va en `adapters/codec.rs`, detrás de
  `ProtocolCodec`; **por dónde entra y sale** va en `adapters/transport.rs`,
  detrás de `Transport`. Qué códec y qué transporte se instancian lo decide la
  raíz (`lib.rs`). No hay nada por si acaso: un adaptador nuevo es un fichero
  nuevo, no un `if`.
- **Lo que la ventana de sede ve** se traduce en `adapters/views.rs`; quién lo
  publica es `adapters/window.rs`.
- Las pruebas del trámite van en `application/errand/tests/`, repartidas por
  comportamiento, con el códec, el transporte y los dos motores del puente
  doblados. Dos oráculos siguen
  congelados: `tests/contract_discovers_adapters.rs` compara `just contract`
  con `tests/contract.snapshot`, y la grada C del canal y el banco de conformidad no
  se tocan.
- **Todo `Format` que nombra la sede llega al consentimiento**: el trámite no
  filtra por `Format::bridged()`, y el que no cruza al puente, `NONE`, lo firma
  `signing/application/bare_pkcs1.rs` (ADR-0001).
- **El recuadro y la rúbrica son de PAdES**: con cualquier otro formato el
  trámite ni los lee ni los rechaza, y los olvida antes del consentimiento
  (`forget_the_box`), de modo que ninguna de sus claves llega al puente.
- El trámite escribe sus importaciones con `crate::…` y no con `super::super::…`:
  la guarda de dirección (`tests/module_directions.rs`) solo lee `use crate::`, y
  las aristas que hoy tolera por la lista de deuda se le escaparían si fueran
  relativas.
