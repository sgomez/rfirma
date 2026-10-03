# Mapa de `documents`: por dónde entra el documento y dónde cae

El contexto de **documentos**: destino, soltados, abiertos, recientes, rúbrica y
documento en curso (ADR-0011, ADR-0012). La memoria entre sesiones no vive aquí:
la sirve `signing/adapters/memory.rs` por el puerto `DocumentsMemory`. Rutas
relativas a `src/documents/`; para situarte en un fichero, `just outline <ruta>`.

## Dónde vive qué

Cada módulo dice qué es en su cabecera `//!`, y `just outline
rfirma-app/src-tauri/src/documents/` las junta en un índice; acótalo a un
subdirectorio (`documents/domain/`) si ya sabes la capa.
