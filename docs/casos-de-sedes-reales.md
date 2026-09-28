# Casos de sedes reales

Cuando una sede en producción destapa un fallo, la incidencia se convierte en
un caso reproducible: una URL `afirma://` capturada, redactada y guardada como
prueba. No hace falta saber de protocolo para seguir este procedimiento.

## 1. Capturar la URL

Dos fuentes, cualquiera de las dos vale:

- **La pestaña de red del navegador**, filtrada por WS: el trámite abre un
  canal con rFirma o AutoFirma nada más pulsar el enlace de la sede, y la
  invocación completa queda ahí.
- **La traza de una compilación de desarrollo** (`just dev`): cada arranque y
  cada operación que llega por el canal ya abierto se escriben en `stderr`
  (`site/adapters/trace.rs`). Los valores largos —`dat`, `properties`— salen
  abreviados a su tamaño, así que esta fuente sirve para la forma de la URL,
  no para el contenido completo de lo que lleva dentro.

## 2. Redactar

La URL capturada **no se guarda tal cual**. Antes de que entre en el
repositorio:

- Se quita `idsession`: no lo necesita ninguna prueba de lectura de la
  petición.
- Se quita cualquier dato personal que aparezca en claro o en el `dat`.
- El `dat` se reduce a lo que el caso necesita: si el fallo no depende del
  contenido del documento o del lote, un valor mínimo que conserve la misma
  forma (mismos campos, mismo formato) basta.

## 3. Guardar el caso

- **Si el caso es solo de lectura de la petición** —la URL se analiza sin más,
  no hace falta que nada responda por la red—, se añade como prueba en la
  grada A: un `#[test]` en el `tests/` del módulo de
  `rfirma-app/src-tauri/src/site/domain/protocol/operation/` que le
  corresponde por verbo (`batch.rs` para `op=batch`, `sign.rs` para `op=sign`,
  etc.), que comprueba que la URL redactada se lee **sin rechazo**.
- **Si el caso destapó además un fallo de envío** —algo que solo se ve al
  hablar de verdad con un servidor, remoto o simulado—, deja siempre dos
  cosas, no solo la prueba de lectura: un guion de la sede en
  `testdata/site-driver/scripts/` y su comprobación de conformidad en
  `rfirma-conformance/catalogue/` (ver `rfirma-conformance/AGENTS.md`).

El primer caso, un login con lote remoto JSON —CAdES, `SHA256withRSA`, una
firma con `mode=explicit` y `precalculatedHashAlgorithm`, almacén
`MOZ_UNI`—, está en
`rfirma-app/src-tauri/src/site/domain/protocol/operation/tests/batch.rs`.
