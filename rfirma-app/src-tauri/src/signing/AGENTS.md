# Mapa de `signing`: la firma local

El contexto de la **firma local**: las reglas puras, el ciclo trifásico, la
sesión, la frontera nativa —puente FFI e hilo del aislado— y la memoria entre
sesiones (ADR-0010). La clave privada no cruza al puente (ADR-0001). Rutas
relativas a `src/signing/`; para situarte en un fichero, `just outline <ruta>`.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz, `SigningRoot`, y la fachada que usan los vecinos: `configuration`, `is_live`, `signed_document`, `signed_folder`, `begin_for_the_site`, `finish`. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `ports.rs` | Los cuatro puertos: `Bridge`, `IsolateHost`, `Signer` y `DocumentBytes`. El puente no tiene entrada que firme (ADR-0001). |
| `application/tests.rs` | Los andamios de grada A que comparten todos los contextos: `NoIsolate`, `a_memory()`, `an_order()`, `a_completed_cycle()` y `DocumentsInMemory`. Solo en pruebas. |
| `adapters/engines.rs` | Los adaptadores de `Bridge` y de los tres motores que la sede declara en `site/ports.rs`, `FilterEngine`, `PolicyEngine` y `ValidationEngine`. Pruebas en `adapters/engines/tests.rs`. |
| `adapters/ffi.rs` | La frontera FFI: cargar `librfirma_crypto.so` y volver sin fugas. Diez entradas, y ninguna firma. Pruebas en `adapters/ffi/tests.rs`. |
| `adapters/isolate.rs` | El hilo dueño del isolate de GraalVM, y el adaptador de `IsolateHost`. Pruebas en `adapters/isolate/tests.rs`. |
| `adapters/memory.rs` | `Memory`, la memoria entre sesiones (ADR-0010), y las rebanadas que cada vecino pide por su puerto: `DocumentsMemory`, `CertificateMemory` y `VersionMemory`. Pruebas en `adapters/memory/tests.rs`. |
| `adapters/failures.rs` | La única traducción de las situaciones de la firma local a la vista de la ventana y al código de la sede (ADR-0009). Pruebas en `adapters/failures/tests.rs`. |
| `adapters/orders.rs` | Lo que la ventana manda, ya deserializado, y `SigningChoice`, lo decidido sin asas; lo que no vale es un `PlacementError`. Pruebas en `adapters/orders/tests.rs`. |
| `adapters/state.rs` | `State`, el fichero de estado tal como se guarda. Pruebas en `adapters/state/tests.rs`. |
| `adapters/store.rs` | El fichero JSON versionado que soporta las dos memorias. Pruebas en `adapters/store/tests.rs`. |
| `adapters/tauri.rs` | Las órdenes de firma local: el ciclo, la previsualización, la esquina PAdES, la configuración y las firmas no registradas. Pruebas en `adapters/tauri/tests.rs`. |
| `adapters/views.rs` | `PlacementView` y `ConfigurationView`, con sus conversiones desde `VisibleBox` y `Preferences` y de vuelta. Sin pruebas propias. |
| `adapters/files.rs` | `RealDocumentBytes`: leer del disco el PDF que se va a firmar. |
| `application/configuration.rs` | Los ajustes como `Preferences`, puros: cómo se enseñan y cómo se combinan. Quien los guarda es la orden. Pruebas en `application/configuration/tests.rs`. |
| `application/configuration_memory.rs` | Lo que el usuario elige y la aplicación obedece. Pruebas en `application/configuration_memory/tests.rs`. |
| `application/cycle.rs` | El ciclo trifásico, parametrizado por formato, sobre los puertos `Bridge` y `Token`. El único caso de uso que cruza la FFI **para firmar**. Pruebas en `application/cycle/tests.rs`. |
| `application/preview.rs` | La prefirma en seco: el ciclo entero con un `PK1` inventado, sin PIN y sin escribir, para pintar el sello de verdad. Pruebas en `application/preview/tests.rs`. |
| `application/session.rs` | La sesión: las dos prefirmas —la local y la de sede—, la firma y la postfirma; `CycleFailure` es lo que puede salir mal. Pruebas en `application/session/tests.rs`. |
| `domain/admissibility.rs` | Lo que no se puede firmar en PAdES, decidido antes del PIN, y lo que sí se puede pero no entendemos: el `/SubFilter` que el puente no lee. No es de ningún otro formato. Pruebas en `domain/admissibility/tests.rs`. |
| `domain/bridge.rs` | El vocabulario del puente —`Format` entre otros— y las tres etapas del ciclo: `PreSignature` → `SealedPreSignature` → `CompletedCycle`. La carga es de `adapters/ffi.rs`. Pruebas en `domain/bridge/tests.rs`. |
| `domain/config.rs` | Los siete ajustes de firma y ni uno más, y `SigningChoice`, lo que la persona decidió de esta firma; aquí viven `Placement` y `PadesRect`. Pruebas en `domain/config/tests.rs`. |
| `domain/isolate_gone.rs` | El marcador de que el hilo del isolate murió, sin el hilo. |
| `domain/language.rs` | Los cinco idiomas (ADR-0009). Pruebas en `domain/language/tests.rs`. |
| `domain/layer2_text.rs` | El texto del recuadro visible: un párrafo, y la máscara sobre el `CN`. Pruebas en `domain/layer2_text/tests.rs`. |
| `domain/placement.rs` | Del recuadro arrastrado en el visor al `/Rect` del PDF: `PageSet`, `VisibleBox`, `Spot`, `BoxSize` y `PlacementError`. Pruebas en `domain/placement/tests.rs`. |
| `domain/properties.rs` | Los `extraParams` en el formato del puente, y `merged_with`: quién manda cuando la sede y rFirma tocan la misma clave. Pruebas en `domain/properties/tests.rs`. |
| `domain/session_seal.rs` | El sello de sesión: una invariante entre prefirma y postfirma (ADR-0016). Pruebas en `domain/session_seal/tests.rs`. |
