# Mapa de `site`: el trámite de sede

Este contexto es el trámite que abre una sede: el protocolo `afirma://`, el canal
por el que se habla con ella, el material TLS y la confianza de la CA local. La
firma en sí no vive aquí, sino en `signing/`. Las rutas son relativas a
`src/site/`, y las pruebas de cada módulo están en su hermano `tests.rs`.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz de composición del contexto: sus adaptadores, su estado de proceso y sus puertos instanciados. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `application/tests.rs` | Los dobles en memoria con los que la grada A no toca disco ni red. Solo en pruebas. |
| `adapters/channel/bind.rs` | Ata la escucha del canal a la ubicación que le llega. Pruebas en `adapters/channel/bind/tests.rs`. |
| `adapters/channel/conversation.rs` | Qué se contesta a cada mensaje del canal, sin socket delante. Pruebas en `adapters/channel/conversation/tests.rs`. |
| `adapters/channel/mod.rs` | El reparto, y la tabla de las cuatro piezas del canal. Léelo antes que sus hermanos. |
| `adapters/channel/reply.rs` | El asa por la que se le contesta a la sede cuando la respuesta llega mucho después. Pruebas en `adapters/channel/reply/tests.rs`. |
| `adapters/channel/server.rs` | El servidor del canal. **No existe escuchador en claro.** |
| `adapters/codec.rs` | El códec de la versión 4 del protocolo. Pruebas en `adapters/codec/tests.rs`. |
| `adapters/codec_v1.rs`, `adapters/codec_v3.rs` | Los códecs de las versiones 1 y 3, que delegan en el de la 4 en vez de repetirlo. Pruebas en `adapters/codec_v1/tests.rs` y `adapters/codec_v3/tests.rs`. |
| `adapters/codec_relay.rs` | El códec del servidor intermedio, el que cifra la respuesta con la clave negociada. Pruebas en `adapters/codec_relay/tests.rs`. |
| `adapters/desk.rs` | `Neighbours`: lo que el trámite pide a los contextos vecinos, servido sobre sus tres raíces. |
| `adapters/scratch.rs` | La carpeta de paso donde cae el documento de la sede mientras dura el trámite. |
| `adapters/service/mod.rs` | El transporte de producción de `service`: TLS crudo sobre el *loopback*, sin WebSocket. Pruebas en `adapters/service/tests.rs`. |
| `adapters/servlets.rs` | El cliente del servidor intermedio de producción, sobre `reqwest::blocking`. Pruebas en `adapters/servlets/tests.rs`. |
| `adapters/nss.rs` | El registro en los almacenes NSS por la API de NSS y **no** por `certutil`, que no está en el flatpak, sobre el `NssHost` de `identity/adapters/pkcs11/nss.rs`. Pruebas en `adapters/nss/tests.rs`. |
| `adapters/frontier.rs` | La única traducción de un rechazo del trámite al código `SAF_NN` de la sede y a la vista de la ventana (ADR-0009). Pruebas en `adapters/frontier/tests.rs`. |
| `adapters/tauri.rs` | Las órdenes de Tauri del trámite de sede. Ninguna decide ni guarda estado propio. |
| `adapters/tls/mod.rs` | El reparto de las dos piezas del material TLS; reexporta `LocalCa`. |
| `adapters/tls/server.rs` | El certificado del servidor local, en memoria. Pruebas en `adapters/tls/server/tests.rs`. |
| `adapters/tls/store.rs` | Las dos ranuras de la CA local en disco, detrás del puerto `LocalCaSlots`. Pruebas en `adapters/tls/store/tests.rs`. |
| `adapters/relay.rs` | El transporte del servidor intermedio: sin canal que sostener, la operación llega resuelta desde la propia invocación. Pruebas en `adapters/relay/tests.rs`. |
| `adapters/transport.rs` | El transporte de producción del `wss` sobre el *loopback*. |
| `adapters/views.rs` | Los tipos que cruzan a la ventana de sede y su única conversión. Pruebas en `adapters/views/tests.rs`. |
| `adapters/window.rs` | El adaptador de la ventana de sede: la crea y le publica lo que va pasando. Sin decisión dentro. |
| `application/errand/desk.rs` | La mesa del trámite (`ErrandDesk`) y los dos consentimientos que se deciden sobre ella. |
| `application/errand/mod.rs` | Los verbos, y el reparto. **Léelo antes que sus hermanos**: es lo único que una orden llama. Pruebas en `application/errand/tests.rs`. |
| `application/errand/outcome.rs` | El vocabulario de salida del trámite, y el puerto `ProtocolCodec`, que lo lee y lo escribe en el cable. |
| `application/errand/replies.rs` | Las respuestas finales, y **el único sitio que escribe en el cable**. |
| `application/errand/request.rs` | `SiteRequest`: lo que la sede quiere, sin versión. |
| `application/errand/state.rs` | El estado del trámite, con un solo dueño (`LiveErrand`). Pruebas en `application/errand/state/tests.rs`. |
| `application/errand/tests.rs` | Las pruebas del trámite entero, en grada A, con los vecinos doblados y el hilo de `fixtures.rs`. Solo en pruebas. |
| `application/filtering.rs` | El listado de certificados que la sede acepta. Pruebas en `application/filtering/tests.rs`. |
| `application/policies.rs` | **La política de firma que declara la sede.** Pruebas en `application/policies/tests.rs`. |
| `application/session.rs` | La sesión de firma **de sede**, y `SiteRefusal`, la situación de cada negativa sin traducir. Pruebas en `application/session/tests.rs`. |
| `application/site.rs` | **La invocación de una sede**: la negociación de arranque, que elige códec y decide si un rechazo sale por el socket o por la ventana. Pruebas en `application/site/tests.rs`. |
| `application/startup/channel.rs` | El canal abierto y sostenido, y quién lo sostiene o por qué no lo hay. Pruebas en `application/startup/channel/tests.rs`. |
| `application/startup/mod.rs` | El arranque: si se enseña la ventana principal o se atiende un trámite de sede, y con qué momento se abre la de sede. Pruebas en `application/startup/tests.rs`. |
| `application/startup/repair.rs` | La reparación de la CA local desde la ventana de sede. Pruebas en `application/startup/repair/tests.rs`. |
| `application/trust.rs` | Cuándo se instala la CA local en los almacenes y cómo se solapa con la siguiente. Pruebas en `application/trust/tests.rs`. |
| `domain/local_ca.rs` | La **CA local**, pura: la genera y la lee de PEM, y no toca el disco. Pruebas en `domain/local_ca/tests.rs`. |
| `domain/channel.rs` | El canal visto desde dentro: cometido, ubicación donde escucha, situaciones (ADR-0009) y asa, todo sin socket. Pruebas en `domain/channel/tests.rs`. |
| `domain/protocol/cipher.rs` | El cifrado DES del servidor intermedio, calcado del original. Pruebas en `domain/protocol/cipher/tests.rs`. |
| `domain/protocol/codes.rs` | **El catálogo publicado**: los `SAF_00`…`SAF_52` y las tres respuestas que no son códigos. Pruebas en `domain/protocol/codes/tests.rs`. |
| `domain/protocol/filters.rs` | La expresión de filtro de la sede: la **lista blanca** que decide si se llama al motor. Pruebas en `domain/protocol/filters/tests.rs`. |
| `domain/protocol/framing.rs` | **El framing artesanal del transporte `service`**, sin socket: lector y escritor puros. Pruebas en `domain/protocol/framing/tests.rs`. |
| `domain/protocol/launch.rs` | La invocación de arranque: verbo, versión de protocolo, ubicación de canal y credencial. Pruebas en `domain/protocol/launch/tests.rs`. |
| `domain/protocol/message.rs` | Lo que llega por el canal ya abierto y con qué credencial viene. Puro. Pruebas en `domain/protocol/message/tests.rs`. |
| `domain/protocol/mod.rs` | El reparto, y las cinco cosas en las que rFirma se aparta del original a propósito. Léelo antes que sus hermanos. |
| `domain/protocol/operation.rs` | Lo que la sede pide por el canal ya abierto: el verbo y la petición de firma. Pruebas en `domain/protocol/operation/tests.rs`. |
| `domain/protocol/parameters.rs` | Las dos guardias comunes a toda operación. Pruebas en `domain/protocol/parameters/tests.rs`. |
| `domain/protocol/refusal.rs` | El rechazo del protocolo: el código que sale al cable, el detalle crudo que **no** sale, y cómo lo nombra la ventana. Pruebas en `domain/protocol/refusal/tests.rs`. |
| `domain/protocol/url.rs` | Una URL `afirma://` partida en verbo y pares, con las rarezas del original. Pruebas en `domain/protocol/url/tests.rs`. |
| `domain/protocol/version.rs` | El comparador de versiones del original, que **no es semver**, y sus cuatro trampas. Pruebas en `domain/protocol/version/tests.rs`. |
| `domain/protocol/visible.rs` | **El recuadro que pide la sede**, que no comparte conversión con el camino local. Pruebas en `domain/protocol/visible/tests.rs`. |
| `domain/tls_error.rs`, `domain/trust_error.rs`, `domain/relay_error.rs` | Las situaciones (ADR-0009) del material del canal, de la confianza y del servidor intermedio. Pruebas en `domain/tls_error/tests.rs`, `domain/trust_error/tests.rs` y `domain/relay_error/tests.rs`. |
| `domain/signing.rs` | Lo que vuelve de la firma que pidió la sede: la firma en memoria, o el rechazo ya traducido por quien firmó. |
| `domain/trust.rs` | El reparto, y las tres reglas **puras** de la confianza. Aquí vive el puerto `TrustStores`. Léelo antes que sus hermanos. Pruebas en `domain/trust/tests.rs`. |
| `ports.rs` | **Los diez puertos**: los propios del contexto, los dos motores que presta el puente y lo que el trámite pide a los vecinos. Pruebas en `ports/tests.rs`. |

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
- Las pruebas del trámite van en `application/errand/tests.rs`, con el códec, el
  transporte y los dos motores del puente doblados. Dos oráculos siguen
  congelados: `just check-contract` compara `just contract` con
  `tests/contract.snapshot`, y la grada C del canal y el banco de conformidad no
  se tocan.
- El trámite escribe sus importaciones con `crate::…` y no con `super::super::…`:
  la guarda de dirección (`tests/module_directions.rs`) solo lee `use crate::`, y
  las aristas que hoy tolera por la lista de deuda se le escaparían si fueran
  relativas.
