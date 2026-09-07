# Mapa de `site`: el trámite de sede

El contexto de **sede**: el protocolo `afirma://` (`domain/protocol/`), la
confianza de la CA local (`domain/trust.rs`), el trámite y el arranque
(`application/`), y el canal `wss://` sobre el *loopback*, el material TLS, el
registro en NSS, el códec y el transporte (`adapters/`), y el filtrado y la
política de firma que declara la sede (`application/filtering.rs`,
`application/policies.rs`). `ports.rs` reúne los puertos del contexto: los
propios —`Transport`, `TrustStores`, `LocalCaSlots`—, los dos motores que el
puente presta —`FilterEngine`, `PolicyEngine`— y lo que el trámite pide a los
vecinos —`Certificates`, `ScratchDocuments`, `SiteSigning`—, que sirve
`adapters/desk.rs` sobre las tres raíces. El códec, `ProtocolCodec`, vive con
el vocabulario que habla, en `application/errand/outcome.rs`. Los casos de uso
no nombran ningún adaptador ni ningún caso de uso de otro contexto (#453,
#443).

Rutas relativas a `src/site/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera, y los casos de uso de otro contexto solo por su raíz
(`<contexto>/mod.rs`); lo vigila `tests/module_directions.rs`. Para situarte
en un fichero, `just outline <ruta>`; las pruebas de cada módulo viven en su
hermano `tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `mod.rs` | 33 | La raíz: `SiteRoot`, con el trámite vivo, el canal sostenido, la confianza de la CA local, sus ranuras en disco, la tabla de códecs (`CodecTable`) y la carpeta de paso. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | | Solo `pub mod`: el reparto de cada capa. |
| `application/tests.rs` | 105 | Los dobles de `LocalCaSlots` (`InMemoryCaSlots`) y de `Certificates` (`Directory`, los certificados que ve un trámite), con los que la grada A no toca disco ni token. Solo en pruebas. |
| `adapters/channel/bind.rs` | 47 | Ata la ubicación del canal: uno de los puertos que sorteó la sede —siempre en `127.0.0.1` y **nunca el 63117 cuando la sede sorteó puertos**— o un puerto fijo, tal cual, aunque sea el 63117 (ID-215); reexporta la constante del puerto fijo desde `domain/protocol/launch.rs`. Pruebas en `adapters/channel/bind/tests.rs` (100). |
| `adapters/channel/conversation.rs` | 66 | Qué se contesta a cada mensaje, sin socket delante: las tres guardias del original, el `OK` del eco y la operación que **queda pendiente** (ID-320); la guardia de credencial se salta cuando la sede no la negoció. Pruebas en `adapters/channel/conversation/tests.rs` (172). |
| `adapters/channel/mod.rs` | 12 | El reparto, y la tabla de las cuatro piezas. Léelo antes que sus hermanos. |
| `adapters/channel/reply.rs` | 27 | El asa por la que se le contesta a la sede cuando la respuesta llega mucho después (ID-321, ID-323). Pruebas en `adapters/channel/reply/tests.rs` (18). |
| `adapters/channel/server.rs` | 147 | El servidor: `async fn` que recibe el escuchador atado y devuelve puerto y asa de apagado (ID-213). **No existe escuchador en claro.** La operación que queda pendiente no se contesta aquí: se le entrega al puerto `SiteOperations` con su asa y la conexión se queda esperando (ID-320, ID-330). |
| `adapters/codec.rs` | 51 | **El códec de la versión 4**: `domain/protocol/` detrás del puerto, sin lógica nueva (RD-03). Lo instancia la raíz (`lib.rs`) y la negociación lo recibe ya decidido. Pruebas en `adapters/codec/tests.rs` (64). |
| `adapters/codec_v3.rs` | 25 | **El códec de la versión 3**: mismo catálogo y misma forma de respuesta que la 4, medido contra el original; delega en `V4Codec` en vez de repetirlo. La negociación lo elige cuando la ubicación de canal es el puerto fijo. Pruebas en `adapters/codec_v3/tests.rs` (42). |
| `adapters/desk.rs` | 93 | `Neighbours`: las tres raíces vecinas detrás de `Certificates`, `ScratchDocuments` y `SiteSigning`; y `signing_refusal_of`, que guarda en el rechazo lo que quien firmó ya tradujo. |
| `adapters/scratch.rs` | 23 | `RealScratch`: crear la carpeta de paso, dejar ahí el documento de la sede y borrarlo al acabar el trámite. |
| `adapters/nss.rs` | 266 | El registro de verdad, por la API de NSS y **no** por `certutil`, que no está en el flatpak. Consume el `NssHost` de `identity/adapters/pkcs11/nss.rs` para la carga de NSS y el turno del token. No hay ni una llamada que borre: esa ausencia es el solape. Pruebas en `adapters/nss/tests.rs` (34). |
| `adapters/frontier.rs` | 120 | **La única traducción de un rechazo del trámite** (`SiteRefusal`) a la vista de la ventana y al código `SAF_NN` de la sede, en un solo `match` (ADR-0009), incluido el del filtrado; el rechazo de la firma llega ya traducido por quien firmó. Y el código del canal y la cancelación. Pruebas en `adapters/frontier/tests.rs` (186). |
| `adapters/tauri.rs` | 101 | Las nueve órdenes del trámite de sede: sacar `SiteRoot` del `State`, llamar a un verbo de `application/errand/` y traducir (RD-07). Ninguna decide ni guarda estado propio. |
| `adapters/tls/mod.rs` | 9 | El reparto de las dos piezas con sus dos vidas (ID-220); reexporta `LocalCa` desde `domain/local_ca.rs`. |
| `adapters/tls/server.rs` | 123 | El **certificado del servidor local**: `CN=localhost`, las dos entradas de la SAN, y en memoria (ID-222). Pruebas en `adapters/tls/server/tests.rs` (103). |
| `adapters/tls/store.rs` | 182 | Las **dos ranuras** de la CA local en disco —la que sirve y la siguiente del solape—, dos ficheros cada una; la clave nace `0600` (ID-223, ID-224). El adaptador del puerto `LocalCaSlots`. Pruebas en `adapters/tls/store/tests.rs` (124). |
| `adapters/transport.rs` | 56 | **El transporte de producción**: el `wss` sobre el *loopback*, en la ubicación de canal que le llega —sorteo o puerto fijo—, con `adapters/channel/` detrás del puerto (RD-04, ID-326). |
| `adapters/views.rs` | 257 | Los tipos que cruzan a la **ventana de sede** y su única conversión, del `Moment` del trámite a la vista (ID-338, ID-341). Aparte por ventana, como `rubric.rs` lo está por tamaño. Pruebas en `adapters/views/tests.rs` (72). |
| `adapters/window.rs` | 80 | **El adaptador de la ventana de sede**: la crea, le publica el momento del trámite y arma la mesa —los dos motores del aislado y `Neighbours` sobre las tres raíces— cuando el transporte entrega una operación (ID-330, ID-333, ID-338). Sin decisión dentro. |
| `application/errand/desk.rs` | 206 | La mesa del trámite (`ErrandDesk`: los dos motores y los `Neighbours`, que son los tres puertos de fuera juntos) y los dos consentimientos que se deciden sobre ella: el orden de las cribas, la admisibilidad, la política y el recuadro (ID-258, ID-266, ID-272, ID-282). La de producción la arma `adapters/window.rs`. |
| `application/errand/mod.rs` | 150 | Los verbos, y el reparto. **Léelo antes que sus hermanos**: es lo único que una orden llama. Pruebas en `application/errand/tests.rs` (1905). |
| `application/errand/outcome.rs` | 168 | El vocabulario de salida: `SiteOutcome` —lo que la sede recibe, sin versión ni código— y lo que queda para la ventana: `ErrandStep`, `SigningConsent`, `NoCertificate`, `Moment`; y el puerto `ProtocolCodec`, que lee la petición y escribe ese vocabulario en el cable. Vive aquí y no en `ports.rs` porque un rechazo lleva dentro lo que dijeron los vecinos, y un puerto solo habla en dominio. |
| `application/errand/replies.rs` | 110 | Las respuestas finales, y **el único sitio que escribe en el cable** (ID-322): identidad entregada, firma entregada, la que no salió y la cancelación. |
| `application/errand/request.rs` | 14 | `SiteRequest`: lo que la sede quiere, sin versión (RD-02). |
| `application/errand/state.rs` | 212 | El estado del trámite con un solo dueño (`LiveErrand`): el trámite vivo, el códec negociado, el asa de respuesta, la petición apuntada, el fichero de paso, lo consentido y el último momento (ID-280, ID-321, ID-338, ID-341). Pruebas en `application/errand/state/tests.rs` (84). |
| `application/errand/tests.rs` | 1905 | Las pruebas del trámite entero, en grada A, con los vecinos doblados sobre el token y el hilo de `fixtures.rs`. Solo en pruebas; la guarda de dirección no lo lee. |
| `application/filtering.rs` | 104 | El listado que la sede acepta: los criterios de rFirma primero y la expresión de la sede después, aplicada por el motor prestado del puente (ID-252, ID-258, ID-259); el certificado tras el asa lo resuelve `Certificates`. Pruebas en `application/filtering/tests.rs` (166). |
| `application/policies.rs` | 21 | **La política de firma que declara la sede**: `expPolicy` expandido por `ExtraParamsProcessor` del original a través de `PolicyEngine` (ID-266). Pruebas en `application/policies/tests.rs` (123). |
| `application/session.rs` | 86 | La sesión **de sede**: la prefirma que vuelve a pasar el filtro de la sede antes de pedir el secreto (ID-259) y **la postfirma que no escribe nada** (ID-286, ID-264), las dos por el puerto `SiteSigning`; y `SiteRefusal`, la situación de cada negativa sin traducir. Pruebas en `application/session/tests.rs` (158). |
| `application/site.rs` | 133 | **La invocación de una sede**: **la negociación de arranque** —recibe la tabla de códecs (`CodecTable`) y el transporte de la raíz, en un solo sitio (RD-05)—, elige el códec por la versión de protocolo que declaró `LaunchRequest`, y decide si un rechazo sale por el socket —sorteado o fijo— o por la ventana (ID-214, ID-215, ID-248). Con un trámite vivo la segunda invocación se rechaza (ID-280). Pruebas en `application/site/tests.rs` (400), con la tabla de negociación como prueba central. |
| `application/startup/channel.rs` | 60 | El canal abierto, sostenido en sus dos ranuras —el del trámite y el de un rechazo— y quién lo sostiene o cuenta por qué no lo hay (ID-325, ID-279, ID-280). Pruebas en `application/startup/channel/tests.rs` (74). |
| `application/startup/mod.rs` | 196 | Recibe la llamada de sede de la invocación, si la trae, y los puertos —códec, transporte, ranuras de la CA local, almacenes de confianza y abridor de ventana— y decide si se enseña la principal o se atiende un trámite de sede (ID-324, ID-328…ID-329, ID-334). Apunta **con qué momento se abre la ventana de sede**: el trámite o el callejón sin salida (ID-341). Pruebas en `application/startup/tests.rs` (388). |
| `application/startup/repair.rs` | 46 | La reparación de la CA local desde la ventana de sede sobre `LocalCaTrust` —las ranuras y los almacenes ya decididos por la raíz—, y en qué queda esa pantalla: dos preguntas, no una (ID-329, ID-341). Pruebas en `application/startup/repair/tests.rs` (20). |
| `application/trust.rs` | 220 | **La CA local en los almacenes NSS**, sobre los puertos `LocalCaSlots` y `TrustStores`: cuándo se instala, el solape —con la vigente **sirviendo** hasta que caduca— y el aviso que llega al terminar. Nunca se repara a mitad de un trámite (ID-224, ID-227). Pruebas en `application/trust/tests.rs` (365), con las ranuras en memoria. |
| `domain/local_ca.rs` | 205 | La **CA local**: P-256, `nameConstraints` armada byte a byte, `keyUsage` de solo firmar certificados y 900 días (ID-221, ID-225). Pura: la genera y la lee de PEM, y no toca el disco. Pruebas en `domain/local_ca/tests.rs` (121). |
| `domain/channel.rs` | 118 | El canal visto desde dentro: su cometido —con la credencial negociada, que puede ser ausente—, la ubicación donde escucha —sorteo o puerto fijo—, sus situaciones (ADR-0009) y el asa del canal abierto, sin socket. Pruebas en `domain/channel/tests.rs` (11). |
| `domain/protocol/codes.rs` | 428 | **El catálogo publicado**: los cincuenta y tres `SAF_00`…`SAF_52` con frase nuestra, el parámetro que se nombra detrás, y las tres respuestas que no son códigos —`CANCEL`, `MEMORY_ERROR`, `NULL`— (ID-289, ID-290, ID-293). Pruebas en `domain/protocol/codes/tests.rs` (75). |
| `domain/protocol/filters.rs` | 136 | La expresión de filtro de la sede: la **lista blanca** que decide si se llama al motor, no qué se aplica (ID-256, ID-257, ID-260). Pruebas en `domain/protocol/filters/tests.rs` (166). |
| `domain/protocol/launch.rs` | 203 | La invocación de arranque: verbo, versión de protocolo, ubicación de canal (`ChannelLocation`, sorteada en la v4 o fija en la v3) y credencial (`NegotiatedCredential`, ausente cuando la v3 no la trae) (ID-245…ID-249). Aquí vive `THE_PORT_OF_THE_THIRD_PROTOCOL`, que `adapters/channel/bind.rs` reexporta. Pruebas en `domain/protocol/launch/tests.rs` (246). |
| `domain/protocol/message.rs` | 72 | Lo que llega por el canal ya abierto —el eco, una operación o nada del protocolo— y con qué credencial viene. Puro (ID-244, TD-53). Pruebas en `domain/protocol/message/tests.rs` (72). |
| `domain/protocol/mod.rs` | 26 | El reparto, y las cinco cosas en las que rFirma se aparta del original a propósito. Léelo antes que sus hermanos. |
| `domain/protocol/operation.rs` | 325 | Lo que la sede pide por el canal ya abierto: el verbo, las dos guardias comunes, el `properties` del que salen los filtros y la petición de firma —`sign` y `cosign` en PAdES; `countersign`, `save` y `signandsave` con su rechazo propio— (ID-263, ID-264, ID-272, ID-276). Pruebas en `domain/protocol/operation/tests.rs` (297). |
| `domain/protocol/parameters.rs` | 52 | Las dos guardias comunes a toda operación: `mcv` y el `dat` que pide un fichero local (ID-250, ID-267). Pruebas en `domain/protocol/parameters/tests.rs` (64). |
| `domain/protocol/refusal.rs` | 105 | El rechazo del protocolo: el código que sale al cable, el detalle crudo que **no** sale (ID-291) y la situación con la que lo nombra la ventana (ID-341). Pruebas en `domain/protocol/refusal/tests.rs` (36). |
| `domain/protocol/url.rs` | 120 | Una URL `afirma://` partida en verbo y pares, con las rarezas de `extractParams`. Pruebas en `domain/protocol/url/tests.rs` (106). |
| `domain/protocol/version.rs` | 104 | El comparador de versiones del original, que **no es semver**, y sus cuatro trampas (ID-251, TD-54). Pruebas en `domain/protocol/version/tests.rs` (95). |
| `domain/protocol/visible.rs` | 90 | **El recuadro que pide la sede**: si lleva posición y página se honran **crudas** —el camino de la sede no comparte conversión con el local—, si no, se firma invisible o se cancela con `SAF_43`, y `signaturePages=append` se rechaza (ID-282…ID-284). Pruebas en `domain/protocol/visible/tests.rs` (199). |
| `domain/signing.rs` | 25 | Lo que vuelve de la firma que pidió la sede: `SiteSignature` en memoria, o `SigningRefusal` con el código y la vista ya decididos por quien firmó. |
| `domain/tls_error.rs` | 52 | Situaciones del material del canal (ADR-0009). Pruebas en `domain/tls_error/tests.rs` (11). |
| `domain/trust.rs` | 131 | El reparto, y las tres reglas **puras**: la etapa de la CA, el solape y que a mitad de un trámite no se toca nada. Aquí vive el puerto `TrustStores`. Léelo antes que sus hermanos. Pruebas en `domain/trust/tests.rs` (109). |
| `domain/trust_error.rs` | 52 | Situaciones de la confianza (ADR-0009). Pruebas en `domain/trust_error/tests.rs` (11). |
| `ports.rs` | 177 | **Los nueve puertos**: `Transport`, `TrustStores` y `LocalCaSlots`; `FilterEngine` y `PolicyEngine`, los motores del puente; `Scratch`, la carpeta de paso donde cae el documento de la sede; y `Certificates`, `ScratchDocuments` y `SiteSigning`, lo que el trámite pide a los vecinos, con `SiteSigningRequest`. Con el `ReplyHandle` por el que se contesta mucho después (RD-03, RD-04). `Transport::open` recibe la ubicación del canal —sorteo o puerto fijo—, no la lista de puertos: un cierre con esa firma **es** un transporte. Pruebas en `ports/tests.rs` (36). |

## Al tocar lo que sale hacia la sede

Todo lo que la sede recibe cuando no sale una firma pasa por
`domain/protocol/codes.rs` —el catálogo cerrado— y se decide en
`adapters/frontier.rs` —la única traducción de un `SiteRefusal` a un código y
a una vista, a la vez—. El trámite (`application/`) no conoce ni `Failure` ni
`SafCode`: devuelve la situación, y la traduce el códec para el cable y la
orden de Tauri para la ventana. Dos cosas que salen mal si se olvidan:

- **Un código no se escribe a mano.** Nadie compone una cadena `SAF_…`: se
  construye un `WireAnswer` y se llama a `on_the_wire()`.
  `tests/site_frontier_guards.rs` compara lo que sale contra las líneas que el
  `enum` puede producir, así que un código acuñado sale en rojo; y `SafCode`
  no se construye desde texto, con el cebo en `compile_fail.rs` de la raíz.
- **Una situación nueva no compila** hasta que se le decide código y vista:
  `told` en `adapters/frontier.rs` es un `match` cerrado sobre `SiteRefusal`, y
  cada contexto tiene el suyo en su `adapters/failures.rs`. Lo que la prueba añade encima es que el código elegido
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
  la raíz (`lib.rs`), y `application/site.rs::negotiate` los recibe ya
  decididos. No hay nada por si acaso (RD-10): un adaptador nuevo es un
  fichero nuevo, no un `if`.
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
