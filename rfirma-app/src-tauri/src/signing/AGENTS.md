# Mapa de `signing`: la firma local

El contexto de la **firma local**: las reglas puras (`domain/`), el ciclo
trifásico y la sesión (`application/`), y la frontera nativa —el puente FFI y
el hilo del aislado— y **la memoria entre sesiones** (ADR-0010) en
`adapters/`. `ports.rs` declara el puente que ve el ciclo, `Bridge`; el hilo
que lo aloja, `IsolateHost`, cuyo adaptador es `adapters/isolate.rs` (#453); y
lo que el ciclo le pide al token, `Signer`, que presta la raíz de identidad.
Los dos motores que el puente presta a la sede —`FilterEngine` y
`PolicyEngine`— son puertos de `site/ports.rs`; su adaptador sigue siendo
`adapters/engines.rs`. Los casos de uso reciben el hilo como `impl IsolateHost`
porque su método es genérico, y **no resuelven asas**: el documento y el
certificado les llegan ya resueltos por la orden. La clave privada no
cruza al puente (ADR-0001): el puerto no tiene entrada que firme, la postfirma
solo acepta una `SealedPreSignature` (el cebo está en `compile_fail.rs`, en la
raíz), y los puntos de entrada de Java los vigila `application/cycle/tests.rs`.

Rutas relativas a `src/signing/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera, y los casos de uso de otro contexto solo por su raíz
(`<contexto>/mod.rs`). No hay deuda: `tests/module_directions_debt.txt` está
vacío. Para situarte en un fichero,
`just outline <ruta>`; las pruebas de cada módulo viven en su hermano
`tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `mod.rs` | 80 | La raíz: `SigningRoot`, con la memoria, el hilo del aislado y la sesión; y la fachada que usan los vecinos: `configuration`, `is_live`, `signed_document`, `signed_folder`, `begin_for_the_site`, `finish`. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | | Solo `pub mod`: el reparto de cada capa. |
| `adapters/engines.rs` | 76 | Los adaptadores de `Bridge` y de los dos puertos de sede, `FilterEngine` y `PolicyEngine`, sobre el puente, y de los dos motores sobre el hilo del aislado, donde se resuelve la doble `Result` (RD-06). Pruebas en `adapters/engines/tests.rs` (12). |
| `adapters/ffi.rs` | 421 | La frontera FFI: cargar `librfirma_crypto.so` y volver sin fugas. **Cinco entradas**, y ninguna firma. Un solo fallo del puente tiene nombre propio: el PDF con firmas no registradas (ID-296). Pruebas en `adapters/ffi/tests.rs` (358). |
| `adapters/isolate.rs` | 85 | El hilo dueño del isolate de GraalVM, y el adaptador de `IsolateHost`. Pruebas en `adapters/isolate/tests.rs` (44). |
| `adapters/memory.rs` | 198 | `Memory`: las dos memorias, sus dos soportes y la configuración viva, con los dos interruptores (ADR-0010); y los adaptadores de la rebanada que cada vecino pide por su puerto: `DocumentsMemory`, `CertificateMemory` y `VersionMemory`. Pruebas en `adapters/memory/tests.rs` (346). |
| `adapters/failures.rs` | 183 | La única traducción de las situaciones de la firma local —memoria, admisibilidad, sello, puente, colocación, ciclo, filtrado— a la vista de la ventana y al código de la sede (ADR-0009). Pruebas en `adapters/failures/tests.rs` (100). |
| `adapters/orders.rs` | 111 | Lo que la ventana manda, ya deserializado, **la validación del destino** antes de llamar al puente (ID-94) y `choice()`, lo decidido sin asas como `SigningChoice`; lo que no vale es un `PlacementError`. Pruebas en `adapters/orders/tests.rs` (74). |
| `adapters/state.rs` | 69 | `State`, el fichero de estado tal como se guarda (ID-31): la bandeja, lo **global** de la firma visible (ID-74), el certificado, la última carpeta y la comprobación de versión. Pruebas en `adapters/state/tests.rs` (255). |
| `adapters/store.rs` | 210 | El fichero JSON versionado que soporta las dos memorias. Pruebas en `adapters/store/tests.rs` (181). |
| `adapters/tauri.rs` | 146 | Las diez órdenes de firma local: el ciclo (prefirma, PIN, postfirma, cancelar), la previsualización, la esquina PAdES, la configuración y las firmas no registradas. Resuelven las asas por `DocumentsRoot` e `IdentityRoot`, y `finish_signing` **orquesta la postfirma**: entrega, recuerda el certificado y anota la fila si el documento se recuerda. Pruebas en `adapters/tauri/tests.rs` (37). |
| `adapters/views.rs` | 132 | `StatusView`, `PlacementView` y `ConfigurationView`, con sus conversiones desde `VisibleBox` y `Preferences` y de vuelta. Pruebas en `adapters/views/tests.rs`. |
| `application/configuration.rs` | 91 | Los ajustes como `Preferences`, puros: cómo se enseñan, cómo se combinan con lo que la ventana devuelve y con la carpeta concedida. Quien los guarda es la orden, sobre `adapters/memory.rs`. Pruebas en `application/configuration/tests.rs` (230). |
| `application/configuration_memory.rs` | 59 | Lo que el usuario elige y la aplicación obedece. Pruebas en `application/configuration_memory/tests.rs` (87). |
| `application/cycle.rs` | 192 | El ciclo trifásico sobre los puertos `Bridge` y `Token`: prefirma Java, firma Rust, postfirma Java, que sella la prefirma con la firma del token y devuelve un `CompletedCycle`. El único caso de uso que cruza la FFI **para firmar** (ID-82); el otro que la cruza es `application/filtering.rs`, y no firma. Pruebas en `application/cycle/tests.rs` (64). |
| `application/preview.rs` | 44 | La prefirma en seco: el ciclo entero con un `PK1` inventado, sin PIN y sin escribir, para pintar el sello de verdad (ID-136, ID-110). Pruebas en `application/preview/tests.rs` (95). |
| `application/session.rs` | 392 | La sesión: las dos prefirmas —la local sobre una `SigningChoice` y la de sede sobre lo que declaró la sede—, la firma con el `Signer` y la postfirma, que devuelve el `CompletedCycle` con el documento y el certificado (`Signed`) y **no entrega nada**: eso lo decide la orden. `CycleFailure` es todo lo que puede salir mal de abrir el documento a entregarlo. Pruebas en `application/session/tests.rs` (271). |
| `domain/admissibility.rs` | 235 | Lo que no se puede firmar, decidido antes del PIN, y lo que **sí se puede pero no entendemos**: el `/SubFilter` que el puente no lee (ID-297, ID-299). Pruebas en `domain/admissibility/tests.rs` (151). |
| `domain/bridge.rs` | 322 | El vocabulario con el que se habla al puente: las cuatro peticiones, `BridgeError` y dónde se busca la librería; y las tres etapas del ciclo como tipos, `PreSignature` → `SealedPreSignature` (con `TokenSignature` y el sello intacto) → `CompletedCycle`, con los campos cerrados para que nadie salte una. Sin la carga: eso es `adapters/ffi.rs`. Pruebas en `domain/bridge/tests.rs` (57). |
| `domain/config.rs` | 213 | Los siete ajustes de firma y ni uno más (ID-18), y `SigningChoice`: lo que la persona decidió de esta firma, ya validado y sin asas; `for_the_site` es la invisible que pide una sede. Aquí viven `Placement` y `PadesRect` (ID-90). Pruebas en `domain/config/tests.rs` (256). |
| `domain/isolate_gone.rs` | 13 | El marcador de que el hilo del isolate murió, sin el hilo. |
| `domain/language.rs` | 48 | Los cinco idiomas (ADR-0009 enmendado; el valencià salió en el ID-124). Pruebas en `domain/language/tests.rs` (34). |
| `domain/layer2_text.rs` | 245 | El texto del recuadro visible: un párrafo, y la máscara sobre el `CN`. Pruebas en `domain/layer2_text/tests.rs` (193). |
| `domain/memory_error.rs` | 56 | Situaciones de la memoria (ADR-0009). Pruebas en `domain/memory_error/tests.rs` (16). |
| `domain/mod.rs` | 25 | El reparto. Qué se le pide al puente y qué se le exige de vuelta. |
| `domain/placement.rs` | 464 | Del recuadro arrastrado en el visor al `/Rect` del PDF (ID-21), `PageSet`: en qué páginas se estampa y si el documento las tiene (ID-91, ID-94), `VisibleBox` —el recuadro que recuerda la bandeja—, `Spot` y `BoxSize` —cómo lo guarda: la esquina de cada documento y el tamaño global— y `PlacementError`. Pruebas en `domain/placement/tests.rs` (375). |
| `domain/properties.rs` | 55 | Los `extraParams` en el formato del puente, y `merged_with`: quién manda cuando la sede y rFirma tocan la misma clave (ID-266). Pruebas en `domain/properties/tests.rs` (92). |
| `domain/session_seal.rs` | 50 | El sello de sesión: una invariante entre prefirma y postfirma (ADR-0016). Pruebas en `domain/session_seal/tests.rs` (75). |
| `ports.rs` | 39 | **Los tres puertos**: `Bridge`, `IsolateHost` y `Signer`. El puente no tiene entrada que firme (ADR-0001); el `Signer` firma unos bytes con el token y nunca entrega la clave. |
