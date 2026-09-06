# Mapa de `documents`: por dónde entra el documento y dónde cae

El contexto de **documentos**: el destino, lo que se suelta o se nombra en la
línea de órdenes, lo abierto en esta sesión, los recientes, la rúbrica y el
documento en curso (ADR-0011, ADR-0012). `adapters/recents_store.rs` es
persistencia: deriva `Serialize` y **no** cruza a la ventana, por eso ni el
contrato ni la guarda de rutas lo leen.

Rutas relativas a `src/documents/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera; lo que hoy se salta eso está en
`tests/module_directions_debt.txt` y solo mengua. Para situarte en un fichero,
`just outline <ruta>`; las pruebas de cada módulo viven en su hermano
`tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `mod.rs`, `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | | Solo `pub mod`: el reparto del contexto y el de cada capa. |
| `adapters/recents_store.rs` | 237 | Los diez recientes, por ruta canónica, con el conjunto de páginas y la posición del recuadro de cada uno (ID-74, ID-95). Lee las filas de v0.2 y descarta la que no entienda. Pruebas en `adapters/recents_store/tests.rs` (247). |
| `adapters/rubric/mod.rs` | 11 | El reparto. |
| `adapters/rubric/normalize.rs` | 177 | La normalización. Pruebas en `adapters/rubric/normalize/tests.rs` (257). |
| `adapters/rubric/store.rs` | 89 | Se copia, no se referencia (ID-33). Pruebas en `adapters/rubric/store/tests.rs` (171). |
| `adapters/tauri.rs` | 191 | Las once órdenes de documentos: abrir por el portal, leer, recientes, rúbrica, destino y abrir el PDF firmado o su carpeta. |
| `adapters/tauri_rubric.rs` | 73 | Los mismos dos papeles que `views.rs`, solo para la rúbrica: aparte por tamaño, no porque sea otra cosa (ID-82). Pruebas en `adapters/tauri_rubric/tests.rs` (50). |
| `adapters/views.rs` | 83 | Destino, PDF firmado, documento abierto, soltado y reciente. Pruebas en `adapters/views/tests.rs`. |
| `application/documents.rs` | 245 | Por dónde entra el documento y dónde cae el firmado, y las dos puertas de entrada: la que recuerda y la que no (ID-286). Pruebas en `application/documents/tests.rs` (528). |
| `application/in_hand.rs` | 94 | **El documento en curso**, que no es la fila que se guarda: quién lo tiene delante, si de él queda rastro y quién decide que la bandeja escriba (ID-286, ID-287). Pruebas en `application/in_hand/tests.rs` (85). |
| `application/opened.rs` | 104 | Los documentos abiertos en esta sesión: del identificador opaco al fichero, y si de cada concesión se guarda rastro (`Remembrance`, ID-286). Pruebas en `application/opened/tests.rs` (104). |
| `application/recents.rs` | 171 | La bandeja, del disco a la ventana: quién la lee, quién la escribe y el reparto del recuadro (ID-74, ID-75). Pruebas en `application/recents/tests.rs` (342). |
| `application/rubric.rs` | 21 | Adopta en el almacén lo que el diálogo del portal concede, y lee lo que ya había: envoltorio fino sobre `RubricStore` que solo existe por la regla de dirección (ID-79, TD-21). Pruebas en `application/rubric/tests.rs` (74). |
| `domain/destination.rs` | 101 | `DestinationFolder` y dónde cae el firmado. Pruebas en `domain/destination/tests.rs` (141). |
| `domain/dropped.rs` | 90 | Qué se decide de los ficheros que llegan de fuera: soltados en la ventana —uno solo o varios, incluida una carpeta recorrida— o nombrados en la línea de órdenes (ID-67, ID-68, ID-70, ID-157, ID-306). Pruebas en `domain/dropped/tests.rs` (242). |
| `domain/error.rs` | 65 | Situaciones del destino (ADR-0009). Pruebas en `domain/error/tests.rs` (25). |
| `domain/handles.rs` | 30 | Cómo se acuña un asa opaca (ID-61, ADR-0011). Pruebas en `domain/handles/tests.rs` (26). |
| `domain/naming.rs` | 55 | Cómo se llama el firmado y qué pasa si el nombre existe. Pruebas en `domain/naming/tests.rs` (72). |
| `domain/portal.rs` | 65 | El documento tal y como entra por el portal (ID-37). Pruebas en `domain/portal/tests.rs` (101). |
| `domain/recents.rs` | 12 | La insignia de un reciente. Cruza a la ventana: el contrato la presta de aquí. |
| `domain/rubric.rs` | 78 | La rúbrica ya normalizada y sus situaciones (ADR-0009, ADR-0012). Pruebas en `domain/rubric/tests.rs` (11). |
