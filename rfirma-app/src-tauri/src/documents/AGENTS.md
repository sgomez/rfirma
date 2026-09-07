# Mapa de `documents`: por dónde entra el documento y dónde cae

El contexto de **documentos**: destino, soltados, abiertos, recientes, rúbrica y
documento en curso (ADR-0011, ADR-0012). La memoria entre sesiones no vive aquí:
la sirve `signing/adapters/memory.rs` por el puerto `DocumentsMemory`. Rutas
relativas a `src/documents/`; para situarte en un fichero, `just outline <ruta>`.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz, `DocumentsRoot`, y la fachada que usan los vecinos: `chosen_folder`, `opened_document`, `is_remembered`, `open_unrecorded`, `deliver`, `note_signed`, `told_as_dropped`. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `ports.rs` | Los dos puertos: `DocumentsMemory` y `DocumentFiles`, el disco donde viven los documentos. |
| `adapters/files.rs` | `RealFiles`: el `DocumentFiles` de verdad, `std::fs` y nada más. |
| `application/tests.rs` | El andamio de grada A del contexto: `InMemoryFiles`, un disco de mentira con carpetas, ficheros, atajos y los que se niegan a abrirse. Solo en pruebas. |
| `adapters/rubric/mod.rs` | El reparto. |
| `adapters/rubric/normalize.rs` | La normalización de la rúbrica. Pruebas en `adapters/rubric/normalize/tests.rs`. |
| `adapters/rubric/store.rs` | Dónde queda guardada la rúbrica: se copia, no se referencia. Pruebas en `adapters/rubric/store/tests.rs`. |
| `adapters/failures.rs` | La única traducción de las situaciones de documentos a la vista de la ventana y al código de la sede (ADR-0009). Pruebas en `adapters/failures/tests.rs`. |
| `adapters/tauri.rs` | Las órdenes de documentos: abrir por el portal, leer, recientes, rúbrica, destino y abrir el PDF firmado o su carpeta. |
| `adapters/tauri_rubric.rs` | Lo que para el resto del contexto hacen `adapters/tauri.rs` y `views.rs`, solo para la rúbrica: aparte por tamaño, no porque sea otra cosa. Pruebas en `adapters/tauri_rubric/tests.rs`. |
| `adapters/views.rs` | Destino, PDF firmado, documento abierto, soltado y reciente, con su `From` desde `domain/told.rs` y `RecentRow`. Pruebas en `adapters/views/tests.rs`. |
| `application/documents.rs` | Por dónde entra el documento y dónde cae el firmado, y lo abierto en esta sesión: `OpenedDocuments`, que es un `Handles<Document>`. Pruebas en `application/documents/tests.rs`. |
| `application/recents.rs` | La bandeja, de la memoria a la ventana: `RecentRow` es la fila y `RecentsError` por qué no se anota. Pruebas en `application/recents/tests.rs`. |
| `domain/destination.rs` | `DestinationFolder`, el `FolderFact` que el disco contesta y los nombres candidatos del firmado. Pruebas en `domain/destination/tests.rs`. |
| `domain/dropped.rs` | Qué se decide de los ficheros que llegan de fuera: soltados en la ventana o nombrados en la línea de órdenes. Pruebas en `domain/dropped/tests.rs`. |
| `domain/error.rs` | Situaciones del destino y `DocumentError`: por qué un documento no se abre, no se lee o no se entrega (ADR-0009). Pruebas en `domain/error/tests.rs`. |
| `domain/document.rs` | **El documento en curso**, el único: `Document`, su `Origin` —portal o ruta del anfitrión— y si de él queda rastro (`Remembrance`). Pruebas en `domain/document/tests.rs`. |
| `domain/handles.rs` | `mint` y `Handles<T>`, el único mapa de asa a lo que nombra (ADR-0011): lo usan los documentos abiertos y los certificados de `identity`. Pruebas en `domain/handles/tests.rs`. |
| `domain/naming.rs` | Cómo se llama el firmado y qué pasa si el nombre existe. Pruebas en `domain/naming/tests.rs`. |
| `domain/told.rs` | Lo que el caso de uso cuenta de un documento antes de que la vista le ponga el formato del cable. Sin pruebas propias. |
| `domain/recents.rs` | La insignia y **la bandeja**: los diez recientes, genérica sobre lo que quien firma recuerde de cada uno (`Recents<Spot>`). No cruza a la ventana. Pruebas en `domain/recents/tests.rs`. |
| `domain/rubric.rs` | La rúbrica ya normalizada y sus situaciones (ADR-0009, ADR-0012). Pruebas en `domain/rubric/tests.rs`. |
