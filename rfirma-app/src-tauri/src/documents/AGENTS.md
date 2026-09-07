# Mapa de `documents`: por dónde entra el documento y dónde cae

El contexto de **documentos**: el destino, lo que se suelta o se nombra en la
línea de órdenes, lo abierto en esta sesión, los recientes, la rúbrica y el
documento en curso (ADR-0011, ADR-0012). `ports.rs` declara lo único que los
casos de uso piden de fuera, `DocumentsMemory`: el destino elegido, la última
carpeta abierta y la bandeja, que sirve `signing/adapters/memory.rs`.
`domain/recents.rs` deriva `Serialize` para el fichero de estado y **no** cruza
a la ventana, por eso ni el contrato ni la guarda de rutas lo leen.

Rutas relativas a `src/documents/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera, y los casos de uso de otro contexto solo por su raíz
(`<contexto>/mod.rs`); lo vigila `tests/module_directions.rs`. Para situarte
en un fichero, `just outline <ruta>`; las pruebas de cada módulo viven en su
hermano `tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz: `DocumentsRoot`, con la carpeta por omisión, la rúbrica, lo abierto y la memoria; y la fachada que usan los vecinos: `chosen_folder`, `opened_document`, `is_remembered`, `open_unrecorded`, `deliver`, `note_signed`, `told_as_dropped`. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `ports.rs` | **Los dos puertos**: `DocumentsMemory` y `DocumentFiles`, el disco donde viven los documentos. |
| `adapters/files.rs` | `RealFiles`: el `DocumentFiles` de verdad, `std::fs` y nada más. |
| `application/tests.rs` | El andamio de la grada A del contexto: `InMemoryFiles`, un disco de mentira con carpetas, ficheros, atajos y los que se niegan a abrirse. Solo en pruebas. |
| `adapters/rubric/mod.rs` | El reparto. |
| `adapters/rubric/normalize.rs` | La normalización. Pruebas en `adapters/rubric/normalize/tests.rs`. |
| `adapters/rubric/store.rs` | Se copia, no se referencia (ID-33). Pruebas en `adapters/rubric/store/tests.rs`. |
| `adapters/failures.rs` | La única traducción de las situaciones de documentos —destino, rúbrica, documento, bandeja, soltado— a la vista de la ventana y al código de la sede (ADR-0009). Pruebas en `adapters/failures/tests.rs`. |
| `adapters/tauri.rs` | Las once órdenes de documentos: abrir por el portal, leer, recientes, rúbrica —directamente sobre `RubricStore`—, destino y abrir el PDF firmado o su carpeta, que pregunta a `SigningRoot` dónde cayó. |
| `adapters/tauri_rubric.rs` | Los mismos dos papeles que `views.rs`, solo para la rúbrica: aparte por tamaño, no porque sea otra cosa (ID-82). Pruebas en `adapters/tauri_rubric/tests.rs`. |
| `adapters/views.rs` | Destino, PDF firmado, documento abierto, soltado y reciente, con su `From` desde `domain/told.rs` y `RecentRow`. Pruebas en `adapters/views/tests.rs`. |
| `application/documents.rs` | Por dónde entra el documento y dónde cae el firmado —sobre la carpeta ya elegida—, la carpeta elegida o la de por omisión, y las dos puertas de entrada: la que recuerda y la que no (ID-286). `OpenedDocuments` es `Handles<Document>`: lo abierto en esta sesión tras su asa. Pruebas en `application/documents/tests.rs`. |
| `application/recents.rs` | La bandeja, de la memoria a la ventana: quién la lee, quién la escribe y el reparto del recuadro entre el `Spot` de cada documento y el tamaño global (ID-74, ID-75); `RecentRow` es la fila y `RecentsError` por qué no se anota; `take` pone delante el documento y solo escribe la fila si `Document` dice que queda rastro; `note_signed` exige el `CompletedCycle` de la postfirma. Pruebas en `application/recents/tests.rs`. |
| `domain/destination.rs` | `DestinationFolder`, el `FolderFact` que el disco contesta y los nombres candidatos del firmado, del preferido al último homónimo: quién los prueba es el caso de uso. Pruebas en `domain/destination/tests.rs`. |
| `domain/dropped.rs` | Qué se decide de los ficheros que llegan de fuera: soltados en la ventana —uno solo o varios, incluida una carpeta recorrida— o nombrados en la línea de órdenes (ID-67, ID-68, ID-70, ID-157, ID-306). Pruebas en `domain/dropped/tests.rs`. |
| `domain/error.rs` | Situaciones del destino y `DocumentError`: por qué un documento no se abre, no se lee o no se entrega (ADR-0009). Pruebas en `domain/error/tests.rs`. |
| `domain/document.rs` | **El documento en curso**, el único: `Document`, con su `Origin` —portal con su identificador, o ruta del anfitrión—, por dónde se lee y si de él queda rastro (`Remembrance`); y si el entorno deja ofrecer la carpeta del original. Pruebas en `domain/document/tests.rs`. |
| `domain/handles.rs` | `mint`, cómo se acuña un asa opaca (ID-61, ADR-0011), y `Handles<T>`, el único mapa de asa a lo que nombra: lo usan los documentos abiertos y los certificados listados de `identity`. Pruebas en `domain/handles/tests.rs`. |
| `domain/naming.rs` | Cómo se llama el firmado y qué pasa si el nombre existe. Pruebas en `domain/naming/tests.rs`. |
| `domain/told.rs` | Lo que el caso de uso cuenta de un documento —abierto, soltado, destino, firmado— antes de que la vista le ponga el formato del cable. Sin pruebas propias. |
| `domain/recents.rs` | La insignia, y **la bandeja**: los diez recientes, por ruta canónica, genérica sobre lo que quien firma quiera recordar del recuadro de cada uno —`Recents<Spot>` en el fichero de estado— porque el dominio de documentos no nombra el de firma (ID-74, ID-95). Lee las filas de v0.2 y descarta la que no entienda. Pruebas en `domain/recents/tests.rs`. |
| `domain/rubric.rs` | La rúbrica ya normalizada y sus situaciones (ADR-0009, ADR-0012). Pruebas en `domain/rubric/tests.rs`. |
