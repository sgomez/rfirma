# Mapa de `site`: el trámite de sede

El contexto de **sede**: el protocolo `afirma://` (`domain/protocol/`), la
confianza de la CA local (`domain/trust.rs`), el trámite y el arranque
(`application/`), y el canal `wss://` sobre el *loopback*, el material TLS, el
registro en NSS, el códec y el transporte (`adapters/`). `ports.rs` reúne los
tres puertos del contexto: `ProtocolCodec`, `Transport` y `TrustStores`.

Rutas relativas a `src/site/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera; lo que hoy se salta eso está en
`tests/module_directions_debt.txt` y solo mengua. Para situarte en un fichero,
`just outline <ruta>`; las pruebas de cada módulo viven en su hermano
`tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `mod.rs`, `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | | Solo `pub mod`: el reparto del contexto y el de cada capa. |
| `adapters/channel/bind.rs` | 36 | Ata uno de los puertos que sorteó la sede, siempre en `127.0.0.1` y **nunca el 63117** (ID-215). Pruebas en `adapters/channel/bind/tests.rs` (71). |
| `adapters/channel/conversation.rs` | 62 | Qué se contesta a cada mensaje, sin socket delante: las tres guardias del original, el `OK` del eco y la operación que **queda pendiente** (ID-320). Pruebas en `adapters/channel/conversation/tests.rs` (148). |
| `adapters/channel/mod.rs` | 12 | El reparto, y la tabla de las cuatro piezas. Léelo antes que sus hermanos. |
| `adapters/channel/reply.rs` | 27 | El asa por la que se le contesta a la sede cuando la respuesta llega mucho después (ID-321, ID-323). Pruebas en `adapters/channel/reply/tests.rs` (18). |
| `adapters/channel/server.rs` | 147 | El servidor: `async fn` que recibe el escuchador atado y devuelve puerto y asa de apagado (ID-213). **No existe escuchador en claro.** La operación que queda pendiente no se contesta aquí: se le entrega al puerto `SiteOperations` con su asa y la conexión se queda esperando (ID-320, ID-330). |
| `adapters/codec.rs` | 49 | **El códec de la versión 4**: `domain/protocol/` detrás del puerto, sin lógica nueva (RD-03). Lo instancia la negociación de arranque. Pruebas en `adapters/codec/tests.rs` (64). |
| `adapters/nss.rs` | 266 | El registro de verdad, por la API de NSS y **no** por `certutil`, que no está en el flatpak. Consume `NssHost` para la carga de NSS y el turno del token. No hay ni una llamada que borre: esa ausencia es el solape. Pruebas en `adapters/nss/tests.rs` (34). |
| `adapters/tauri.rs` | 96 | Las nueve órdenes del trámite de sede: desempaquetar el `State`, llamar a un verbo de `application/errand/` y traducir (RD-07). Ninguna decide ni guarda estado propio. |
| `adapters/tls/authority.rs` | 205 | La **CA local**: P-256, `nameConstraints` armada byte a byte, `keyUsage` de solo firmar certificados y 900 días (ID-221, ID-225). Pruebas en `adapters/tls/authority/tests.rs` (121). |
| `adapters/tls/mod.rs` | 10 | El reparto, y la tabla de las dos piezas con sus dos vidas (ID-220). |
| `adapters/tls/server.rs` | 123 | El **certificado del servidor local**: `CN=localhost`, las dos entradas de la SAN, y en memoria (ID-222). Pruebas en `adapters/tls/server/tests.rs` (103). |
| `adapters/tls/store.rs` | 155 | Las **dos ranuras** de la CA local —la que sirve y la siguiente del solape—, dos ficheros cada una; la clave nace `0600` (ID-223, ID-224). Pruebas en `adapters/tls/store/tests.rs` (124). |
| `adapters/transport.rs` | 47 | **El transporte de producción**: el `wss` sobre el *loopback* con puerto sorteado, `adapters/channel/` detrás del puerto (RD-04, ID-326). |
| `adapters/views.rs` | 248 | Los tipos que cruzan a la **ventana de sede** y su única conversión, del `Moment` del trámite a la vista (ID-338, ID-341). Aparte por ventana, como `rubric.rs` lo está por tamaño. Pruebas en `adapters/views/tests.rs` (72). |
| `adapters/window.rs` | 70 | **El adaptador de la ventana de sede**: la crea, le publica el momento del trámite y arma la mesa desde el `State` cuando el transporte entrega una operación (ID-330, ID-333, ID-338). Sin decisión dentro. |
| `application/errand/desk.rs` | 283 | La mesa del trámite (`ErrandDesk`) y los dos consentimientos que se deciden sobre ella: el orden de las cribas, la admisibilidad, la política y el recuadro (ID-258, ID-266, ID-272, ID-282). La de producción se arma aquí, no en la orden. |
| `application/errand/mod.rs` | 164 | Los verbos, y el reparto. **Léelo antes que sus hermanos**: es lo único que una orden llama. Pruebas en `application/errand/tests.rs` (1631). |
| `application/errand/outcome.rs` | 160 | El vocabulario de salida: `SiteOutcome` —lo que la sede recibe, sin versión— y lo que queda para la ventana: `ErrandStep`, `SigningConsent`, `NoCertificate`, `Moment`. |
| `application/errand/replies.rs` | 132 | Las respuestas finales, y **el único sitio que escribe en el cable** (ID-322): identidad entregada, firma entregada, la que no salió y la cancelación. |
| `application/errand/request.rs` | 14 | `SiteRequest`: lo que la sede quiere, sin versión (RD-02). |
| `application/errand/state.rs` | 205 | El estado del trámite con un solo dueño (`LiveErrand`): el trámite vivo, el códec negociado, el asa de respuesta, la petición apuntada, el fichero de paso, lo consentido y el último momento (ID-280, ID-321, ID-338, ID-341). Pruebas en `application/errand/state/tests.rs` (84). |
| `application/errand/tests.rs` | 1631 | Las pruebas del trámite entero, en grada A. Solo en pruebas; la guarda de dirección no lo lee. |
| `application/frontier.rs` | 118 | **La frontera de errores**: el único sitio donde una situación del ID-29 se convierte en el código `SAF_NN` que recibe la sede (ID-288, ID-292). Pruebas en `application/frontier/tests.rs` (201). |
| `application/session.rs` | 142 | La sesión **de sede**: la prefirma que vuelve a pasar el filtro de la sede (ID-259) y **la postfirma que no escribe nada** (ID-286, ID-264), con el código que le toca a cada negativa (ID-292). Pruebas en `application/session/tests.rs` (115). |
| `application/site.rs` | 111 | **La invocación de una sede**: **la negociación de arranque** —qué códec y qué transporte, en un solo sitio (RD-05)—, abre el canal en uno de los puertos sorteados, y decide si un rechazo sale por el socket o por la ventana (ID-214, ID-215, ID-248). Con un trámite vivo la segunda invocación se rechaza (ID-280). Pruebas en `application/site/tests.rs` (207). |
| `application/startup/channel.rs` | 60 | El canal abierto, sostenido en sus dos ranuras —el del trámite y el de un rechazo— y quién lo sostiene o cuenta por qué no lo hay (ID-325, ID-279, ID-280). Pruebas en `application/startup/channel/tests.rs` (74). |
| `application/startup/mod.rs` | 194 | Recibe la invocación y tres puertos —transporte, almacenes de confianza y abridor de ventana— y decide si se enseña la principal o se atiende un trámite de sede (ID-324, ID-328…ID-329, ID-334). Apunta **con qué momento se abre la ventana de sede**: el trámite o el callejón sin salida (ID-341). Pruebas en `application/startup/tests.rs` (370). |
| `application/startup/repair.rs` | 45 | La reparación de la CA local desde la ventana de sede, y en qué queda esa pantalla: dos preguntas, no una (ID-329, ID-341). Pruebas en `application/startup/repair/tests.rs` (20). |
| `application/trust.rs` | 220 | **La CA local en los almacenes NSS**: cuándo se instala, el solape —con la vigente **sirviendo** hasta que caduca— y el aviso que llega al terminar. Nunca se repara a mitad de un trámite (ID-224, ID-227). Pruebas en `application/trust/tests.rs` (358). |
| `domain/channel.rs` | 108 | El canal visto desde dentro: su cometido, sus situaciones (ADR-0009) y el asa del canal abierto, sin socket. Pruebas en `domain/channel/tests.rs` (11). |
| `domain/protocol/codes.rs` | 428 | **El catálogo publicado**: los cincuenta y tres `SAF_00`…`SAF_52` con frase nuestra, el parámetro que se nombra detrás, y las tres respuestas que no son códigos —`CANCEL`, `MEMORY_ERROR`, `NULL`— (ID-289, ID-290, ID-293). Pruebas en `domain/protocol/codes/tests.rs` (75). |
| `domain/protocol/filters.rs` | 136 | La expresión de filtro de la sede: la **lista blanca** que decide si se llama al motor, no qué se aplica (ID-256, ID-257, ID-260). Pruebas en `domain/protocol/filters/tests.rs` (166). |
| `domain/protocol/launch.rs` | 141 | La invocación de arranque: puertos, versión de protocolo y credencial de canal (ID-245…ID-249). Pruebas en `domain/protocol/launch/tests.rs` (159). |
| `domain/protocol/message.rs` | 72 | Lo que llega por el canal ya abierto —el eco, una operación o nada del protocolo— y con qué credencial viene. Puro (ID-244, TD-53). Pruebas en `domain/protocol/message/tests.rs` (72). |
| `domain/protocol/mod.rs` | 26 | El reparto, y las cinco cosas en las que rFirma se aparta del original a propósito. Léelo antes que sus hermanos. |
| `domain/protocol/operation.rs` | 325 | Lo que la sede pide por el canal ya abierto: el verbo, las dos guardias comunes, el `properties` del que salen los filtros y la petición de firma —`sign` y `cosign` en PAdES; `countersign`, `save` y `signandsave` con su rechazo propio— (ID-263, ID-264, ID-272, ID-276). Pruebas en `domain/protocol/operation/tests.rs` (297). |
| `domain/protocol/parameters.rs` | 52 | Las dos guardias comunes a toda operación: `mcv` y el `dat` que pide un fichero local (ID-250, ID-267). Pruebas en `domain/protocol/parameters/tests.rs` (64). |
| `domain/protocol/refusal.rs` | 105 | El rechazo del protocolo: el código que sale al cable, el detalle crudo que **no** sale (ID-291) y la situación con la que lo nombra la ventana (ID-341). Pruebas en `domain/protocol/refusal/tests.rs` (36). |
| `domain/protocol/url.rs` | 120 | Una URL `afirma://` partida en verbo y pares, con las rarezas de `extractParams`. Pruebas en `domain/protocol/url/tests.rs` (106). |
| `domain/protocol/version.rs` | 104 | El comparador de versiones del original, que **no es semver**, y sus cuatro trampas (ID-251, TD-54). Pruebas en `domain/protocol/version/tests.rs` (95). |
| `domain/protocol/visible.rs` | 90 | **El recuadro que pide la sede**: si lleva posición y página se honran **crudas** —el camino de la sede no comparte conversión con el local—, si no, se firma invisible o se cancela con `SAF_43`, y `signaturePages=append` se rechaza (ID-282…ID-284). Pruebas en `domain/protocol/visible/tests.rs` (199). |
| `domain/tls_error.rs` | 52 | Situaciones del material del canal (ADR-0009). Pruebas en `domain/tls_error/tests.rs` (11). |
| `domain/trust.rs` | 131 | El reparto, y las tres reglas **puras**: la etapa de la CA, el solape y que a mitad de un trámite no se toca nada. Aquí vive el puerto `TrustStores`. Léelo antes que sus hermanos. Pruebas en `domain/trust/tests.rs` (109). |
| `domain/trust_error.rs` | 52 | Situaciones de la confianza (ADR-0009). Pruebas en `domain/trust_error/tests.rs` (11). |
| `ports.rs` | 80 | **Los tres puertos**: `ProtocolCodec`, `Transport` y `TrustStores`, con el `ReplyHandle` por el que se contesta mucho después (RD-03, RD-04). Un cierre con la firma del transporte **es** un transporte. Pruebas en `ports/tests.rs` (33). |

## Al tocar lo que sale hacia la sede

Todo lo que la sede recibe cuando no sale una firma pasa por
`domain/protocol/codes.rs` —el catálogo cerrado— y se decide en
`application/frontier.rs` —la única traducción de una situación del ID-29 a un
código—. Dos cosas que salen mal si se olvidan:

- **Un código no se escribe a mano.** Nadie compone una cadena `SAF_…`: se
  construye un `WireAnswer` y se llama a `on_the_wire()`.
  `tests/site_frontier_guards.rs` compara lo que sale contra las líneas que el
  `enum` puede producir, así que un código acuñado sale en rojo.
- **Una situación nueva del ID-29 no compila** hasta que se le decide código:
  cada traducción de `application/frontier.rs` es un `match` cerrado sobre el
  enumerado de su módulo. Lo que la prueba añade encima es que el código elegido
  esté en el catálogo, y que no sea nunca `SAF_48`, que la 1.9.2 no puede
  producir (ID-295).

## Al tocar el trámite

- **Una decisión del trámite** —qué se enseña, qué se contesta, qué se
  recuerda— va en `application/errand/`: en `desk.rs` si se toma sobre la mesa,
  en `replies.rs` si es lo que la sede recibe, en `state.rs` si es memoria. Los
  verbos de `mod.rs` son la única puerta, y una orden de `adapters/tauri.rs` no
  hace más que llamar a uno.
- **Cómo se escribe algo en el cable** va en `adapters/codec.rs`, detrás de
  `ProtocolCodec`; **por dónde entra y sale** va en `adapters/transport.rs`,
  detrás de `Transport`. Qué códec y qué transporte se instancian lo decide
  `application/site.rs::negotiate`, y nadie más. No hay nada por si acaso
  (RD-10): un adaptador nuevo es un fichero nuevo, no un `if`.
- **Lo que la ventana de sede ve** es un `Moment` del trámite traducido en
  `adapters/views.rs`; quién lo publica es `adapters/window.rs`.
- Las pruebas del trámite van en `application/errand/tests.rs`, con el códec de
  la versión 4 o uno en memoria, el transporte doblado por un cierre y los dos
  motores del puente doblados (TD-51, TD-52). Dos oráculos siguen congelados:
  `just check-contract` compara `just contract` con `tests/contract.snapshot`,
  y la grada C del canal y el banco de conformidad no se tocan.
- El trámite escribe sus importaciones con `crate::…` y no con `super::super::…`:
  la guarda de dirección solo lee `use crate::`, y las aristas que hoy tolera
  por la lista de deuda se le escaparían si fueran relativas.
