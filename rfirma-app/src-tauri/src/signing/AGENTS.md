# Mapa de `signing`: la firma local

El contexto de la **firma local**: las reglas puras (`domain/`), el ciclo
trifásico y la sesión (`application/`), y la frontera nativa —el puente FFI y
el hilo del aislado— en `adapters/`. `ports.rs` declara los dos motores que
presta el puente, `FilterEngine` y `PolicyEngine`; su adaptador es
`adapters/engines.rs`. La clave privada no cruza al puente (ADR-0001): lo
vigilan las pruebas de `application/cycle/tests.rs`.

Rutas relativas a `src/signing/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera; lo que hoy se salta eso está en
`tests/module_directions_debt.txt` y solo mengua. Para situarte en un fichero,
`just outline <ruta>`; las pruebas de cada módulo viven en su hermano
`tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `mod.rs`, `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | | Solo `pub mod`: el reparto del contexto y el de cada capa. |
| `adapters/engines.rs` | 62 | Los adaptadores de `FilterEngine` y `PolicyEngine` sobre el puente y sobre el hilo del aislado, donde se resuelve la doble `Result` (RD-06). Pruebas en `adapters/engines/tests.rs` (12). |
| `adapters/ffi.rs` | 640 | La frontera FFI: cargar `librfirma_crypto.so` y volver sin fugas. **Cinco entradas**, y ninguna firma. Un solo fallo del puente tiene nombre propio: el PDF con firmas no registradas (ID-296). Pruebas en `adapters/ffi/tests.rs` (358). |
| `adapters/isolate.rs` | 84 | El hilo dueño del isolate de GraalVM. Pruebas en `adapters/isolate/tests.rs` (44). |
| `adapters/memory_error.rs` | 56 | Situaciones de la memoria (ADR-0009). Pruebas en `adapters/memory_error/tests.rs` (16). |
| `adapters/orders.rs` | 97 | Lo que la ventana manda, ya deserializado, y **la validación del destino** antes de llamar al puente (ID-94). Pruebas en `adapters/orders/tests.rs` (73). |
| `adapters/store.rs` | 210 | El fichero JSON versionado que soporta las dos memorias. Pruebas en `adapters/store/tests.rs` (181). |
| `adapters/tauri.rs` | 120 | Las diez órdenes de firma local: el ciclo (prefirma, PIN, postfirma, cancelar), la previsualización, la esquina PAdES, la configuración y las firmas no registradas. Pruebas en `adapters/tauri/tests.rs` (37). |
| `adapters/views.rs` | 81 | `StatusView`, `PlacementView` y `ConfigurationView`. Pruebas en `adapters/views/tests.rs`. |
| `application/configuration.rs` | 97 | Los ajustes, del disco a la ventana y de vuelta. Pruebas en `application/configuration/tests.rs` (226). |
| `application/configuration_memory.rs` | 59 | Lo que el usuario elige y la aplicación obedece. Pruebas en `application/configuration_memory/tests.rs` (87). |
| `application/cycle.rs` | 218 | El ciclo trifásico: prefirma Java, firma Rust, postfirma Java. El único caso de uso que cruza la FFI **para firmar** (ID-82); el otro que la cruza es `application/filtering.rs`, y no firma. Pruebas en `application/cycle/tests.rs` (108). |
| `application/filtering.rs` | 87 | El listado que la sede acepta: los criterios de rFirma primero y la expresión de la sede después, aplicada por el motor prestado del puente (ID-252, ID-258, ID-259). El puerto `FilterEngine` está en `ports.rs`; su adaptador, en `adapters/engines.rs`. Pruebas en `application/filtering/tests.rs` (156). |
| `application/policies.rs` | 31 | **La política de firma que declara la sede**: `expPolicy` expandido por `ExtraParamsProcessor` del original, y quién manda cuando la sede y rFirma tocan la misma clave (ID-266). El puerto `PolicyEngine` está en `ports.rs`; su adaptador, en `adapters/engines.rs`. Pruebas en `application/policies/tests.rs` (122). |
| `application/preview.rs` | 44 | La prefirma en seco: el ciclo entero con un `PK1` inventado, sin PIN y sin escribir, para pintar el sello de verdad (ID-136, ID-110). Pruebas en `application/preview/tests.rs` (95). |
| `application/session.rs` | 360 | La sesión **local**: prefirma, firma en el token y la postfirma que entrega el documento; la sesión a medias es la misma para los dos recorridos (ADR-0001). Pruebas en `application/session/tests.rs` (348). |
| `application/state.rs` | 85 | El estado que la aplicación acumula por su cuenta (ID-31), y lo **global** de la firma visible (ID-74). Pruebas en `application/state/tests.rs` (255). |
| `domain/admissibility.rs` | 235 | Lo que no se puede firmar, decidido antes del PIN, y lo que **sí se puede pero no entendemos**: el `/SubFilter` que el puente no lee (ID-297, ID-299). Pruebas en `domain/admissibility/tests.rs` (151). |
| `domain/config.rs` | 169 | Los siete ajustes de firma y ni uno más (ID-18). Aquí viven `Placement` y `PadesRect` (ID-90). Pruebas en `domain/config/tests.rs` (256). |
| `domain/language.rs` | 48 | Los cinco idiomas (ADR-0009 enmendado; el valencià salió en el ID-124). Pruebas en `domain/language/tests.rs` (34). |
| `domain/layer2_text.rs` | 245 | El texto del recuadro visible: un párrafo, y la máscara sobre el `CN`. Pruebas en `domain/layer2_text/tests.rs` (193). |
| `domain/mod.rs` | 21 | El reparto. Qué se le pide al puente y qué se le exige de vuelta. |
| `domain/placement.rs` | 376 | Del recuadro arrastrado en el visor al `/Rect` del PDF (ID-21), y `PageSet`: en qué páginas se estampa y si el documento las tiene (ID-91, ID-94). Pruebas en `domain/placement/tests.rs` (375). |
| `domain/properties.rs` | 45 | Los `extraParams` en el formato del puente. Pruebas en `domain/properties/tests.rs` (92). |
| `domain/session_seal.rs` | 50 | El sello de sesión: una invariante entre prefirma y postfirma (ADR-0016). Pruebas en `domain/session_seal/tests.rs` (75). |
