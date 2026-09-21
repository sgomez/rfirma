# Mapa de la suite de conformidad

La suite mide un binario instalado de AutoFirma o rFirma con el `autoscript.js` de la sede como
instrumento. Es un crate aparte y fuera del CI (ADR-0013), y no es una grada ni una puerta
(ADR-0014). Su única cara es la consola web que levanta `just conformance`: no hay órdenes de
consola. Sus pruebas se corren con `cargo test` dentro de este directorio.

| Fichero | Qué es |
|---|---|
| `CONTEXT.md` | El glosario de la suite, un bounded context aparte del de la aplicación. |
| `catalogue/` | El catálogo, un TOML por conjunto con los metadatos de cada comprobación; no nombra a ningún cliente. |
| `reference/` | Las referencias, un TOML por cliente y versión con los resultados que una ficha `BUG-NN` explica. |
| `src/main.rs` | El arranque: lee el catálogo, levanta el servidor, imprime la URL con el token y la abre. |
| `src/server.rs` | El servidor HTTP local y su guarda (token, `Host`, `Origin`); sirve la página en `/`, `/informe/<nombre>` y `/comparar`, y traduce rutas a la sesión sin decidir nada. |
| `src/console.html` | La página, servida tal cual por `server.rs` en sus tres direcciones. |
| `src/console.rs` | La sesión: cliente e informe elegidos, la cola, el único hilo que corre las comprobaciones y las preguntas a la persona. |
| `src/report_view.rs` | La vista de un informe, igual lo corra la sesión o no: conjuntos en el orden del catálogo con sus recuentos; no sabe de la sesión. |
| `src/snapshot.rs` | El estado de la sesión activa que recibe la página: la vista de su informe más cliente, cola, comprobación en curso y pregunta. |
| `src/client.rs` | El cliente a prueba y qué cliente es: binario, envoltorio aislado, raíz de confianza y almacén. |
| `src/catalogue.rs` | La lectura del catálogo y la validación de su forma, que si falla no deja arrancar. |
| `src/checks.rs` | El cuerpo ejecutable: cómo se conduce cada comprobación y cómo se resuelve su resultado, sin escribir el informe. |
| `src/verdicts.rs` | Las reglas que traducen lo observado a resultado. |
| `src/outcome.rs` | El resultado de una comprobación, sus nombres en pantalla y PENDIENTE. |
| `src/validation.rs` | La validación de un informe contra una referencia: validado o sus discrepancias, que son fallos de la suite o de la referencia. |
| `src/errand.rs` | El trámite: el `driver.mjs` de `testdata/site-driver/`, el cliente invocado y lo que se extrae de cada evento. |
| `src/report.rs` | El informe en disco, `reports/conformance/<nombre>/dossier.json`: lo ve cualquiera, lo continúa solo su cliente. |
| `src/transcript.rs` | Las tramas y el registro de cada comprobación, en `transcripts/` dentro del informe. |
| `src/livelog.rs` | La línea de registro marcada por procedencia (`sede`, `cliente`, `suite`), en fichero y en vivo. |
| `src/comparison.rs` | La comparación de dos informes, comprobación a comprobación en el orden del catálogo y con su conjunto. |
| `tests/catalogue_matches_the_docs.rs` | El cruce del catálogo y la referencia con `docs/afirma/1.9.2/`: capítulos, tabla SAF y fichas A1. |
| `tests/catalogue_matches_the_harnesses.rs` | El cruce del catálogo con los arneses de `checks.rs`, leído como texto. |
