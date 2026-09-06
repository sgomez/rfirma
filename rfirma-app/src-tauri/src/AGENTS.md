# Mapa del backend (Rust / Tauri)

Este índice **sustituye a explorar el árbol**. Localiza el módulo por su línea,
abre **solo** ese fichero, y solo el tramo que necesitas.

`tests/agents_map_is_complete.rs` comprueba que aquí está listado, por su ruta,
todo `.rs` versionado bajo `src/`. **Un módulo nuevo se añade a esta tabla en la
misma PR que lo crea**, o el PR sale en rojo.

## Presupuesto de lectura

- **Para situarte, `just outline <ruta>`; nunca `cat` de un módulo de más de 300
  líneas.** El esqueleto trae cada `fn`, `struct` y prueba con su número de línea
  y la primera línea de su `///`; desde ahí, `sed -n 'A,Bp;C,Dp'` con **todos**
  los tramos en una sola llamada — un turno por tramo sale más caro que haber
  leído el módulo entero.
  La tabla de abajo da el tamaño de cada módulo antes de que lo abras.
- **Los tests van al final de cada módulo**, tras `#[cfg(test)]`. No los leas
  salvo que vayas a tocarlos. Para saber qué cubren sin leerlos:
  `awk '/#\[cfg\(test\)\]/,0' <fichero> | grep -n '    fn '` — los nombres son
  frases en inglés y dicen la invariante entera.
- El fichero más grande del backend es `app/errand/tests.rs`, con 1660 líneas,
  y son todo pruebas; de producción, `ffi.rs` (1204), `app/documents.rs` (986),
  `signing/placement.rs` (942) y `app/signing/mod.rs` (937). El mayor de
  `commands/` es `commands/mod.rs` (927), y detrás va `commands/views.rs`
  (539); lo que los hace crecer es **prosa**: los cuerpos siguen siendo
  desempaquetar, llamar y traducir. Si lo que crece es un cuerpo, lo que ha
  entrado casi siempre es una decisión, y una decisión va en `app/`.
- El primer bloque `//!` de cada módulo es su contrato. `head -40 <fichero>` casi
  siempre basta para decidir si es el fichero que buscas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `main.rs` | 8 | El binario. No hay nada dentro. |
| `lib.rs` | 381 | Registro de comandos, complementos y estados de Tauri, la instancia única (ID-160) y el arranque, que **obedece a `app/startup/` y no decide nada**: compone el transporte de producción (`app/transport.rs`), le pasa los tres puertos y obedece lo que devuelve (ID-324…ID-334). Empieza aquí para ver el cableado. |
| `isolate.rs` | 179 | El hilo dueño del isolate de GraalVM. |
| `ffi.rs` | 1204 | La frontera FFI: cargar `librfirma_crypto.so` y volver sin fugas. **Cinco entradas**, y ninguna firma. Un solo fallo del puente tiene nombre propio: el PDF con firmas no registradas (ID-296). |
| **`commands/`** | | El adaptador de Tauri: desempaqueta, llama a `app/` y traduce (ID-79). |
| `commands/mod.rs` | 927 | **Las treinta y siete órdenes de Tauri**, y nada más que sus cuerpos. Las nueve de sede son desempaquetar el `State`, llamar a un verbo de `app/errand/` y traducir (RD-07): ninguna decide ni guarda estado propio. |
| `commands/views.rs` | 539 | Los tipos que cruzan a la ventana principal y las conversiones que los producen (ID-80). Los de la ventana de sede están aparte, en `views_site.rs`. |
| `commands/views_site.rs` | 435 | Los tipos que cruzan a la **ventana de sede** y su única conversión, del `Moment` del trámite a la vista (ID-338, ID-341). Aparte por ventana, como `rubric.rs` lo está por tamaño. |
| `commands/site_window.rs` | 129 | **El adaptador de la ventana de sede**: la crea, le publica el momento del trámite y arma la mesa desde el `State` cuando el transporte entrega una operación (ID-330, ID-333, ID-338). Sin decisión dentro. |
| `commands/rubric.rs` | 151 | Los mismos dos papeles que `views.rs`, solo para la rúbrica: aparte por tamaño, no porque sea otra cosa (ID-82). |
| `commands/failure.rs` | 248 | Cómo se le cuenta a la ventana que algo salió mal (ID-29). |
| `commands/orders.rs` | 253 | Lo que la ventana manda, ya deserializado, y **la validación del destino** antes de llamar al puente (ID-94). |
| `commands/guards.rs` | 745 | Las cuatro guardas que ven todas las órdenes a la vez (ID-85), y las pruebas del descubrimiento de tipos. Solo en pruebas. |
| **`app/`** | | Los casos de uso. Es la interfaz por la que se prueba (ID-77, TD-20). |
| `app/mod.rs` | 221 | El reparto, `Environment` —la raíz de composición— y la carpeta de destino elegida (ID-83). Léelo antes que sus hermanos. |
| `app/cycle.rs` | 477 | El ciclo trifásico: prefirma Java, firma Rust, postfirma Java. El único caso de uso que cruza la FFI **para firmar** (ID-82); el otro que la cruza es `app/filtering.rs`, y no firma. |
| `app/certificates.rs` | 756 | Qué certificados hay, cuál eligió la ventana, cuál se recordó, qué estampa el recuadro, y instalar o quitar un `.p12` (ID-192, ID-197). |
| **`app/signing/`** | | El recorrido de la firma en tres pasos, y su gemelo de sede. |
| `app/signing/mod.rs` | 937 | La sesión **local**: prefirma, firma en el token y la postfirma que entrega el documento; la sesión a medias es la misma para los dos recorridos (ADR-0001). |
| `app/signing/site.rs` | 350 | La sesión **de sede**: la prefirma que vuelve a pasar el filtro de la sede (ID-259) y **la postfirma que no escribe nada** (ID-286, ID-264), con el código que le toca a cada negativa (ID-292). |
| `app/frontier.rs` | 388 | **La frontera de errores**: el único sitio donde una situación del ID-29 se convierte en el código `SAF_NN` que recibe la sede (ID-288, ID-292). |
| `app/documents.rs` | 986 | Por dónde entra el documento y dónde cae el firmado, y las dos puertas de entrada: la que recuerda y la que no (ID-286). |
| **`app/errand/`** | | **El trámite de sede** (#406): un módulo profundo con interfaz de tres verbos —`attend`, `consent`, `decline`, más `finish` porque el PIN va en medio— que posee **toda** la memoria del trámite y toma el códec y el transporte por dos puertos. No nombra a ningún adaptador concreto: lo vigila `tests/module_directions.rs` (RD-12). |
| `app/errand/mod.rs` | 303 | Los verbos, y el reparto. **Léelo antes que sus hermanos**: es lo único que una orden llama. |
| `app/errand/state.rs` | 432 | El estado del trámite con un solo dueño (`LiveErrand`): el trámite vivo, el códec negociado, el asa de respuesta, la petición apuntada, el fichero de paso, lo consentido y el último momento (ID-280, ID-321, ID-338, ID-341). |
| `app/errand/request.rs` | 31 | `SiteRequest`: lo que la sede quiere, sin versión (RD-02). |
| `app/errand/outcome.rs` | 246 | El vocabulario de salida: `SiteOutcome` —lo que la sede recibe, sin versión— y lo que queda para la ventana: `ErrandStep`, `SigningConsent`, `NoCertificate`, `Moment`. |
| `app/errand/ports.rs` | 169 | **Los dos puertos**: `ProtocolCodec` y `Transport`, con el `ReplyHandle` por el que se contesta mucho después (RD-03, RD-04). Un cierre con la firma del transporte **es** un transporte. |
| `app/errand/desk.rs` | 375 | La mesa del trámite (`ErrandDesk`) y los dos consentimientos que se deciden sobre ella: el orden de las cribas, la admisibilidad, la política y el recuadro (ID-258, ID-266, ID-272, ID-282). La de producción se arma aquí, no en la orden. |
| `app/errand/replies.rs` | 193 | Las respuestas finales, y **el único sitio que escribe en el cable** (ID-322): identidad entregada, firma entregada, la que no salió y la cancelación. |
| `app/errand/tests.rs` | 1660 | Las pruebas del trámite entero, en grada A. Solo en pruebas; la guarda de dirección no lo lee. |
| `app/codec.rs` | 143 | **El códec de la versión 4**: `protocol/` detrás del puerto, sin lógica nueva (RD-03). Lo instancia la negociación de arranque. |
| `app/transport.rs` | 63 | **El transporte de producción**: el `wss` sobre el *loopback* con puerto sorteado, `channel/` detrás del puerto (RD-04, ID-326). Único sitio de `app/` que nombra a `channel` por el trámite. |
| `app/engines.rs` | 97 | Los adaptadores de `FilterEngine` y `PolicyEngine` sobre el puente y sobre el hilo del aislado, donde se resuelve la doble `Result` (RD-06). |
| `app/filtering.rs` | 327 | El listado que la sede acepta: los criterios de rFirma primero y la expresión de la sede después, aplicada por el motor prestado del puente (ID-252, ID-258, ID-259). Aquí se **declara** el puerto `FilterEngine`; su adaptador está en `app/engines.rs`. |
| `app/in_hand.rs` | 227 | **El documento en curso**, que no es la fila que se guarda: quién lo tiene delante, si de él queda rastro y quién decide que la bandeja escriba (ID-286, ID-287). |
| `app/invocation.rs` | 496 | La invocación desde fuera, `rfirma documento.pdf`: qué abre, qué hace la segunda y por dónde sale la URL `afirma://` que no es una ruta (ID-157…ID-160, ID-235, ID-236). |
| `app/policies.rs` | 221 | **La política de firma que declara la sede**: `expPolicy` expandido por `ExtraParamsProcessor` del original, y quién manda cuando la sede y rFirma tocan la misma clave (ID-266). Aquí se **declara** el puerto `PolicyEngine`; su adaptador está en `app/engines.rs`. |
| `app/preview.rs` | 231 | La prefirma en seco: el ciclo entero con un `PK1` inventado, sin PIN y sin escribir, para pintar el sello de verdad (ID-136, ID-110). |
| `app/recents.rs` | 603 | La bandeja, del disco a la ventana: quién la lee, quién la escribe y el reparto del recuadro (ID-74, ID-75). |
| `app/rubric.rs` | 113 | Adopta en el almacén lo que el diálogo del portal concede, y lee lo que ya había: envoltorio fino sobre `RubricStore` que solo existe por la regla de dirección (ID-79, TD-21). |
| `app/configuration.rs` | 394 | Los ajustes, del disco a la ventana y de vuelta. |
| `app/trust.rs` | 702 | **La CA local en los almacenes NSS**: cuándo se instala, el solape —con la vigente **sirviendo** hasta que caduca— y el aviso que llega al terminar. Nunca se repara a mitad de un trámite (ID-224, ID-227). |
| **`app/startup/`** | | **Qué se abre al arrancar** (TD-70), y lo que el arranque de sede sostiene después. |
| `app/startup/mod.rs` | 731 | Recibe la invocación y tres puertos —transporte, almacenes de confianza y abridor de ventana— y decide si se enseña la principal o se atiende un trámite de sede (ID-324, ID-328…ID-329, ID-334). Apunta **con qué momento se abre la ventana de sede**: el trámite o el callejón sin salida (ID-341). |
| `app/startup/channel.rs` | 209 | El canal abierto, sostenido en sus dos ranuras —el del trámite y el de un rechazo— y quién lo sostiene o cuenta por qué no lo hay (ID-325, ID-279, ID-280). |
| `app/startup/repair.rs` | 115 | La reparación de la CA local desde la ventana de sede, y en qué queda esa pantalla: dos preguntas, no una (ID-329, ID-341). |
| `app/site.rs` | 403 | **La invocación de una sede**: **la negociación de arranque** —qué códec y qué transporte, en un solo sitio (RD-05)—, abre el canal en uno de los puertos sorteados, y decide si un rechazo sale por el socket o por la ventana (ID-214, ID-215, ID-248). Con un trámite vivo la segunda invocación se rechaza (ID-280). |
| `app/handlers.rs` | 157 | Quién atiende `afirma://`, del escritorio a Preferencias y de vuelta: lo que se puede saber, lo que se escribe y el nombre de catálogo de cada situación (ID-238…ID-240). |
| `app/version.rs` | 382 | Si hay una versión nueva publicada: el puerto de red doblable, la caché de 24 h y la comparación de versiones (ID-177, ID-178, ID-180, ID-182). |
| `app/fixtures.rs` | 97 | Los andamios que comparten las pruebas de `app/`. Solo en pruebas. |
| `releases.rs` | 88 | El único sitio que abre una conexión: le pregunta a GitHub por la última publicación y devuelve el cuerpo tal cual (ID-178, ID-182). |
| `paths.rs` | 696 | Las tres rutas de la memoria entre sesiones, más las cuatro de la CA local: dos ranuras, la que sirve y la siguiente. Único sitio que conoce el sistema operativo (ADR-0010), y el único que puede crear un fichero `0600` de nacimiento. |
| **`desktop/`** | | El escritorio de la persona: en qué canal corre esto, quién atiende `afirma://` y cómo se elige (ID-237…ID-242). |
| `desktop/mod.rs` | 261 | El canal de distribución (`/.flatpak-info`) y quién dice el escritorio que atiende `afirma://`, por GIO. Dentro del sandbox no llama a nada: no hay pregunta que valga (ID-240). Léelo antes que sus hermanos. |
| `desktop/choice.rs` | 552 | Elegir manejador y leer al elegido: el `default` **explícito** en el `mimeapps.list` del `$HOME`, con todo lo demás intacto, y la advertencia de que Firefox guarda la suya aparte (ID-238, ID-241). |
| `desktop/error.rs` | 73 | Situaciones de elegir manejador (ADR-0009). |
| `dropped.rs` | 438 | Qué se decide de los ficheros que llegan de fuera: soltados en la ventana —uno solo o varios, incluida una carpeta recorrida— o nombrados en la línea de órdenes (ID-67, ID-68, ID-70, ID-157, ID-306). |
| **`memory/`** | | Lo que rFirma recuerda: seis memorias en dos mitades, y la caché de la comprobación de versión, que no es una memoria del usuario y es lo único exento de los dos interruptores (ADR-0010, ID-180). |
| `memory/mod.rs` | 542 | El reparto de las seis memorias. Léelo antes que sus hermanos. |
| `memory/state.rs` | 444 | El estado que la aplicación acumula por su cuenta (ID-31), y lo **global** de la firma visible (ID-74). |
| `memory/configuration.rs` | 205 | Lo que el usuario elige y la aplicación obedece. |
| `memory/recents.rs` | 599 | Los diez recientes, por ruta canónica, con el conjunto de páginas y la posición del recuadro de cada uno (ID-74, ID-95). Lee las filas de v0.2 y descarta la que no entienda. |
| `memory/store.rs` | 463 | El fichero JSON versionado que soporta las dos memorias. |
| `memory/opened.rs` | 262 | Los documentos abiertos en esta sesión: del identificador opaco al fichero, y si de cada concesión se guarda rastro (`Remembrance`, ID-286). |
| `memory/listed.rs` | 168 | Los certificados listados en esta sesión: del asa opaca a la referencia. |
| `memory/handles.rs` | 90 | Cómo se acuña un asa opaca (ID-61, ADR-0011). |
| `memory/error.rs` | 89 | Situaciones de la memoria (ADR-0009). |
| **`signing/`** | | Las reglas puras de la firma. |
| `signing/mod.rs` | 34 | El reparto. Qué se le pide al puente y qué se le exige de vuelta. **No importa `ffi`** (ID-82). |
| `signing/config.rs` | 508 | Los siete ajustes de firma y ni uno más (ID-18). Aquí viven `Placement` y `PadesRect` (ID-90). |
| `signing/placement.rs` | 942 | Del recuadro arrastrado en el visor al `/Rect` del PDF (ID-21), y `PageSet`: en qué páginas se estampa y si el documento las tiene (ID-91, ID-94). |
| `signing/admissibility.rs` | 429 | Lo que no se puede firmar, decidido antes del PIN, y lo que **sí se puede pero no entendemos**: el `/SubFilter` que el puente no lee (ID-297, ID-299). |
| `signing/layer2_text.rs` | 533 | El texto del recuadro visible: un párrafo, y la máscara sobre el `CN`. |
| `signing/properties.rs` | 179 | Los `extraParams` en el formato del puente. |
| `signing/session_seal.rs` | 152 | El sello de sesión: una invariante entre prefirma y postfirma (ADR-0016). |
| `signing/language.rs` | 105 | Los cinco idiomas (ADR-0009 enmendado; el valencià salió en el ID-124). |
| **`protocol/`** | | Lo que pide la sede, leído de una URL `afirma://` y nada más. Puro, sin sockets ni puente (ID-244, TD-53). |
| `protocol/mod.rs` | 61 | El reparto, y las cinco cosas en las que rFirma se aparta del original a propósito. Léelo antes que sus hermanos. |
| `protocol/url.rs` | 277 | Una URL `afirma://` partida en verbo y pares, con las rarezas de `extractParams`. |
| `protocol/launch.rs` | 366 | La invocación de arranque: puertos, versión de protocolo y credencial de canal (ID-245…ID-249). |
| `protocol/version.rs` | 226 | El comparador de versiones del original, que **no es semver**, y sus cuatro trampas (ID-251, TD-54). |
| `protocol/filters.rs` | 430 | La expresión de filtro de la sede: la **lista blanca** que decide si se llama al motor, no qué se aplica (ID-256, ID-257, ID-260). |
| `protocol/operation.rs` | 751 | Lo que la sede pide por el canal ya abierto: el verbo, las dos guardias comunes, el `properties` del que salen los filtros y la petición de firma —`sign` y `cosign` en PAdES; `countersign`, `save` y `signandsave` con su rechazo propio— (ID-263, ID-264, ID-272, ID-276). |
| `protocol/parameters.rs` | 145 | Las dos guardias comunes a toda operación: `mcv` y el `dat` que pide un fichero local (ID-250, ID-267). |
| `protocol/message.rs` | 179 | Lo que llega por el canal ya abierto —el eco, una operación o nada del protocolo— y con qué credencial viene. Puro (ID-244, TD-53). |
| `protocol/codes.rs` | 567 | **El catálogo publicado**: los cincuenta y tres `SAF_00`…`SAF_52` con frase nuestra, el parámetro que se nombra detrás, y las tres respuestas que no son códigos —`CANCEL`, `MEMORY_ERROR`, `NULL`— (ID-289, ID-290, ID-293). |
| `protocol/refusal.rs` | 170 | El rechazo del protocolo: el código que sale al cable, el detalle crudo que **no** sale (ID-291) y la situación con la que lo nombra la ventana (ID-341). |
| `protocol/visible.rs` | 365 | **El recuadro que pide la sede**: si lleva posición y página se honran **crudas** —el camino de la sede no comparte conversión con el local—, si no, se firma invisible o se cancela con `SAF_43`, y `signaturePages=append` se rechaza (ID-282…ID-284). |
| **`channel/`** | | **El canal**: el servidor `wss://` sobre el *loopback* y qué se contesta a cada mensaje (ID-212…ID-219). No sabe por qué se abre: eso es de `app/site.rs`. |
| `channel/mod.rs` | 31 | El reparto, y la tabla de las cuatro piezas. Léelo antes que sus hermanos. |
| `channel/bind.rs` | 131 | Ata uno de los puertos que sorteó la sede, siempre en `127.0.0.1` y **nunca el 63117** (ID-215). |
| `channel/server.rs` | 266 | El servidor: `async fn` que recibe el escuchador atado y devuelve puerto y asa de apagado (ID-213). **No existe escuchador en claro.** La operación que queda pendiente no se contesta aquí: se le entrega al puerto `SiteOperations` con su asa y la conexión se queda esperando (ID-320, ID-330). |
| `channel/conversation.rs` | 298 | Qué se contesta a cada mensaje, sin socket delante: las tres guardias del original, el `OK` del eco y la operación que **queda pendiente** (ID-320). |
| `channel/reply.rs` | 78 | El asa por la que se le contesta a la sede cuando la respuesta llega mucho después (ID-321, ID-323). |
| `channel/error.rs` | 78 | Situaciones del canal (ADR-0009). |
| **`pkcs11/`** | | La única parte que habla con el token. |
| `pkcs11/mod.rs` | 718 | La capa PKCS#11. |
| `pkcs11/stores.rs` | 675 | Dónde se buscan los certificados, incluidos los `.p12` instalados (ID-192). |
| `pkcs11/certificate.rs` | 411 | El certificado tal y como sale del token. |
| `pkcs11/error.rs` | 241 | Situaciones del token (ID-29, ADR-0009). |
| `pkcs11/nss.rs` | 402 | Cómo entra un `.p12` en un almacén NSS propio: el descodificador de PKCS#12 de `libsmime3` por FFI, sin criptografía propia y dentro del turno del token (ID-192, ID-193, ID-194). |
| `pkcs11/secret.rs` | 194 | Cómo se le pide el secreto a cada almacén: sin sesión, por pantalla o en el teclado del lector, que se rechaza (ID-189, ID-191). |
| **`destination/`** | | Dónde cae el firmado y por dónde entra el original (ADR-0011). |
| `destination/mod.rs` | 353 | El reparto, y `DestinationFolder`. **No importa `memory`** (ID-83). |
| `destination/naming.rs` | 190 | Cómo se llama el firmado y qué pasa si el nombre existe. |
| `destination/portal.rs` | 268 | El documento tal y como entra por el portal (ID-37). |
| `destination/error.rs` | 114 | Situaciones del destino (ADR-0009). |
| **`tls/`** | | El material criptográfico del canal, y **solo la fábrica**: aquí no se registra nada en ningún almacén NSS ni se levanta ningún servidor (ADR-0005). |
| `tls/mod.rs` | 27 | El reparto, y la tabla de las dos piezas con sus dos vidas (ID-220). |
| `tls/authority.rs` | 425 | La **CA local**: P-256, `nameConstraints` armada byte a byte, `keyUsage` de solo firmar certificados y 900 días (ID-221, ID-225). |
| `tls/server.rs` | 259 | El **certificado del servidor local**: `CN=localhost`, las dos entradas de la SAN, y en memoria (ID-222). |
| `tls/store.rs` | 336 | Las **dos ranuras** de la CA local —la que sirve y la siguiente del solape—, dos ficheros cada una; la clave nace `0600` (ID-223, ID-224). |
| `tls/error.rs` | 79 | Situaciones del material del canal (ADR-0009). |
| **`trust/`** | | **La confianza**: cómo entra la CA local en los almacenes NSS de la persona y cuándo toca renovarla (ADR-0005, ID-224, ID-227, ID-228). `tls/` fabrica y no registra; aquí se registra y no se fabrica. |
| `trust/mod.rs` | 353 | El reparto, y las tres reglas **puras**: la etapa de la CA, el solape y que a mitad de un trámite no se toca nada. Aquí vive el puerto `TrustStores`. Léelo antes que sus hermanos. |
| `trust/nss.rs` | 417 | El registro de verdad, por la API de NSS y **no** por `certutil`, que no está en el flatpak. No hay ni una llamada que borre: esa ausencia es el solape. |
| `trust/error.rs` | 82 | Situaciones de la confianza (ADR-0009). |
| **`rubric/`** | | De lo que aporta el usuario al JPEG que acepta el puente (ADR-0012). |
| `rubric/mod.rs` | 33 | El reparto. |
| `rubric/normalize.rs` | 586 | La normalización. |
| `rubric/store.rs` | 314 | Se copia, no se referencia (ID-33). |
| `rubric/error.rs` | 92 | Situaciones de la rúbrica (ADR-0009). |

## Al tocar lo que sale hacia la sede

Todo lo que la sede recibe cuando no sale una firma pasa por `protocol/codes.rs`
—el catálogo cerrado— y se decide en `app/frontier.rs` —la única traducción de
una situación del ID-29 a un código—. Dos cosas que salen mal si se olvidan:

- **Un código no se escribe a mano.** Nadie compone una cadena `SAF_…`: se
  construye un `WireAnswer` y se llama a `on_the_wire()`.
  `tests/site_frontier_guards.rs` compara lo que sale contra las líneas que el
  `enum` puede producir, así que un código acuñado sale en rojo. Esa guarda vive
  en `tests/` y no en `app/` porque mira a la vez lo que se queda —tipos de
  `commands/`— y lo que sale, y un módulo de `app/` no puede nombrar al
  adaptador sin ir contra la flecha del ID-81.
- **Una situación nueva del ID-29 no compila** hasta que se le decide código:
  cada traducción de `app/frontier.rs` es un `match` cerrado sobre el enumerado
  de su módulo. Lo que la prueba añade encima es que el código elegido esté en el
  catálogo, y que no sea nunca `SAF_48`, que la 1.9.2 no puede producir (ID-295).

## La regla de la dirección

Las dependencias van **hacia dentro**: `commands/` → `app/` → dominio e
infraestructura. Ningún módulo de dominio nombra a `app/` ni a `commands/`, y
entre hermanos el que sabe menos no nombra al que sabe más (`ffi` importa
`signing`, no al revés). Si vas a añadir capacidad nueva, el orden es: la regla
pura en su módulo de dominio → el caso de uso en `app/` → el cuerpo de la orden
en `commands/`. Lo vigila `tests/module_directions.rs` (ADR-0017).

## Al tocar el trámite de sede

La sede tiene la forma «adaptadores, casos de uso, puertos» que el resto del
backend todavía no tiene (#406), y la pregunta «¿dónde pongo esto?» tiene
respuesta por su forma:

- **Una decisión del trámite** —qué se enseña, qué se contesta, qué se
  recuerda— va en `app/errand/`: en `desk.rs` si se toma sobre la mesa, en
  `replies.rs` si es lo que la sede recibe, en `state.rs` si es memoria. Los
  verbos de `mod.rs` son la única puerta, y una orden de `commands/mod.rs` no
  hace más que llamar a uno.
- **Cómo se escribe algo en el cable** va en `app/codec.rs`, detrás de
  `ProtocolCodec`; **por dónde entra y sale** va en `app/transport.rs`, detrás de
  `Transport`. Qué códec y qué transporte se instancian lo decide
  `app/site.rs::negotiate`, y nadie más. No hay nada por si acaso (RD-10): un
  adaptador nuevo es un fichero nuevo, no un `if`.
- **Lo que la ventana de sede ve** es un `Moment` del trámite traducido en
  `commands/views_site.rs`; quién lo publica es `commands/site_window.rs`.
- Las pruebas del trámite van en `app/errand/tests.rs`, con el códec de la
  versión 4 o uno en memoria, el transporte doblado por un cierre y los dos
  motores del puente doblados (TD-51, TD-52). Dos oráculos están congelados
  mientras dure el #406: `just check-contract` compara `just contract` con
  `tests/contract.snapshot`, y la grada C del canal y el banco de conformidad
  no se tocan.

## Al añadir o cambiar una orden de Tauri

El cuerpo de la orden va en `commands/mod.rs` y **lo que decide, en `app/`**: si
lo que estás escribiendo dentro de la orden no es desempaquetar el `State` ni
traducir el resultado, está en el fichero equivocado (ID-79).

Lo que escribas aquí es lo que verá quien trabaje en la interfaz: `just contract`
genera el contrato de las dos partes leyendo `commands/mod.rs` y los tipos que
derivan `Serialize`. **No hay nada que actualizar** —una orden nueva aparece por
existir—, pero sí dos cosas que salen mal si te descuidas: un tipo de salida sin
`Serialize` no cruza y no se publica, y un `#[tauri::command]` sin `async` sale
publicado como bloqueante, que es justo la trampa que cuelga la ventana.

Y una tercera, más callada: tanto el extractor del `justfile` como la guarda de
rutas del ADR-0011 reconocen el tipo por el **macro** `derive(Serialize)` escrito
en el código, no por que implemente el rasgo. Un `impl Serialize` a mano en un
tipo de cruce se queda fuera del contrato **y** fuera de la guarda sin que nada
se ponga rojo; si necesitas uno, publícalo por otra vía.

Las guardas de conjunto están juntas en `commands/guards.rs`, y tres piden algo
de ti:

- **La lista cerrada de órdenes** hay que renumerarla. El nombre de la prueba
  **ya no lleva el número dentro** (TD-11): el conteo vive en la aserción de
  `the_list_of_commands_is_closed_and_this_is_how_long_it_is`, porque cambiar el
  número es la información y renombrar la prueba en cada sub-issue no dice nada.
- **La lista de ficheros del módulo** (`SOURCES`) hay que ampliarla si creas un
  fichero nuevo dentro de `commands/`; una guarda propia se pone roja si se te
  olvida.
- **La guarda de rutas** (`the_portal_path_never_crosses_to_the_window`)
  descubre sola todo tipo que derive `Serialize`, esté en el fichero de
  `commands/` que esté (ID-84), pero **sí** hay que decidir dónde entra cada
  uno: o se construye desde su caso de uso en `crossings_from_a_portal_document`
  —lo normal, si detrás hay un documento—, o se declara en
  `OUTPUTS_WITH_NO_DOCUMENT_BEHIND`. Su guarda hermana se pone roja mientras no
  esté en una de las dos.
- La del **hilo del portal** tampoco: un comando que llame a un `blocking_*` de
  un plugin necesita `#[tauri::command(async)]`, y ella lo vigila.

Y un aviso que no es una guarda, pero equivocarse cuesta lo mismo: los permisos
de `capabilities/*.json` **solo filtran el IPC de la ventana** (JS → Rust). Que
Rust llame a la API de un plugin —`tauri-plugin-dialog`, `tauri-plugin-opener`—
desde dentro de una orden no necesita permiso ninguno: `default.json` lleva solo
`core:default` y las dos funcionan. Confirmado dos veces, ID-63 con `dialog` y
el #131 con `opener`, la segunda desmintiendo la suposición contraria.

Lo que **sí** filtran es lo que la ventana escucha: `listen()` es
`core:event|listen` y pasa por la ACL. Por eso cada ventana necesita una
capacidad que la nombre —`capabilities/default.json` para `main`,
`capabilities/site.json` para `site` (ID-333)—: una ventana que no esté en
ninguna nace sin permisos y no oye ni sus propios eventos (`site-errand`,
ID-338).

Y una prueba nueva no se escribe contra la orden, sino contra el caso de uso de
`app/` al que llama (TD-21).

## Las pruebas que se leen a sí mismas

Siete ficheros vigilan invariantes leyendo el código **como texto**, no
ejecutándolo: `app/cycle.rs`, `app/signing/mod.rs`, `app/signing/site.rs`,
`tests/site_frontier_guards.rs`, `commands/guards.rs`,
`tests/module_directions.rs` y `tests/adr_citations_resolve.rs`. Abren el `.rs`
con `include_str!` y buscan cadenas dentro; la última pide a git la lista de
todos los `.rs` versionados y falla, con fichero y línea, por cada `ADR-NNNN`
citado sin fichero en `docs/adr/`. Las de `app/signing/site.rs` comprueban **ausencias** —que la postfirma
de un trámite de sede no entregue el documento, no anote fila y no recuerde el
certificado (ID-286, ID-264)—, y una ausencia no la vigila ninguna prueba de
comportamiento. Antes de tocar uno de esos módulos, o las guardas mismas, dos cosas:

- **`production_half` corta por `"\nmod tests {"`**, así que todo lo que quede
  **después** del módulo de pruebas es invisible para la guarda. Si estás
  comprobando a mano que una guarda falla como debe, coloca el tipo o la
  llamada de mentira **antes** del `mod tests`: detrás pasa en verde sin que
  nadie lo mire, que es justo el fallo que estas pruebas existen para evitar.
- **Algunas aserciones cuentan apariciones literales** —
  `assert_eq!(cycle.matches("bridge.").count(), 2)` en `app/cycle.rs` es la que
  sostiene el ADR-0001: una tercera llamada al puente sería la clave privada
  cruzando a Java. Mover código entre módulos que llevan una de estas pruebas
  obliga a elegir entre reescribir sus aserciones o mudar el fichero entero.
  Decídelo al planificar el cambio, no al final.
- **No todas descubren solas, y la de al lado sí.** La guarda de rutas de
  `commands/guards.rs` encuentra por su cuenta cada tipo nuevo, y eso enseña a
  confiar; la de `app/signing/mod.rs` que vigila quién escribe el sello firmado
  lleva una **lista fija** de ficheros de `app/` y de `commands/`. Un módulo
  nuevo de `app/` que pueda escribir en la bandeja hay que añadirlo a mano a esa
  lista, o queda sin vigilar y en verde.
- **La guarda de dirección no lee un `tests.rs`**: es la mitad de pruebas de su
  carpeta en un fichero aparte (`app/errand/tests.rs`), y se salta igual que un
  `mod tests` al pie. Lo que sí lee son las líneas `use crate::` de producción,
  y por eso el trámite escribe sus importaciones de `app/` con `crate::app::…`
  y no con `super::super::…`: las dos aristas prohibidas del RD-12 —el trámite
  nombrando a `channel`, a `app::codec` o a `app::transport`— se le escaparían
  si fueran relativas.

## Al escribir un comentario

La regla es la restricción 6 del `AGENTS.md` raíz. Lo que solo vale aquí: la
cabecera `//!` de un módulo no repite lo que la tabla «Dónde vive qué» ya dice
de él; si las dos cuentan lo mismo, se borra de la cabecera y se conserva la
tabla. Las citas `ID-NN`/`TD-NN` que ya hay se toleran hasta que la poda pase
por su zona, y entonces pasan a citar un ADR o se borran; no se añaden nuevas.
`tests/adr_citations_resolve.rs` vigila que cada `ADR-NNNN` citado exista.

Dos ejemplos de este backend, tal como quedan tras la poda:

- **Se queda.** En `rubric/normalize.rs`, antes de fijar `max_alloc` en el
  decodificador, hay un tope de memoria que parece redundante con el tope de
  10 MB de la entrada. No lo es —un PNG uniforme comprime 1000:1 y una reserva
  que falla aborta el proceso—, así que ahí queda una línea:
  `// El tope de entrada no acota lo decodificado (ADR-0012).` Las seis líneas
  que hoy lo explican se van al ADR-0012, que aún no lo cuenta: la poda lo
  enmienda en la misma PR.
- **Se va.** La cabecera de `commands/mod.rs` dice «Son veintidós» y enumera
  los puertos de TypeScript que rellena cada orden, con el número de PR de cada
  uno. El conteo es falso (el que lo sabe es la aserción de
  `the_list_of_commands_is_closed_and_this_is_how_long_it_is`), los puertos son
  la interfaz del otro lado y los PR son histórico. Nada de eso va a ninguna
  parte; queda: `//! Las órdenes de Tauri: lo único que la ventana puede pedir.
  No deciden nada.`
