# Mapa de `signing`: la firma local

El contexto de la **firma local**: las reglas puras, el ciclo trifásico, la
sesión, la frontera nativa —puente FFI e hilo del aislado— y la memoria entre
sesiones (ADR-0010). La clave privada no cruza al puente (ADR-0001). Rutas
relativas a `src/signing/`; para situarte en un fichero, `just outline <ruta>`.

## Dónde vive qué

Cada módulo dice qué es en su cabecera `//!`, y `just outline
rfirma-app/src-tauri/src/signing/` las junta en un índice; acótalo a un
subdirectorio (`signing/domain/`) si ya sabes la capa.
