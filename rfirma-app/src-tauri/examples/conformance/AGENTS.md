# Mapa de la suite de conformidad

La suite mide un binario instalado de AutoFirma o rFirma con el cliente publicado como instrumento,
y no es una grada ni una puerta (ADR-0014). Su única cara es la consola web que levanta
`just conformance`: no hay órdenes de consola.

| Fichero | Qué es |
|---|---|
| `CONTEXT.md` | El glosario de la suite, un bounded context aparte del de la aplicación. |
| `main.rs` | El arranque: lee el catálogo, levanta el servidor, imprime la URL con el token y la abre. |
| `server.rs` | El servidor HTTP local y su guarda (token, `Host`, `Origin`); sirve la página en `/`, `/informe/<nombre>` y `/comparar`, y traduce rutas a la sesión sin decidir nada. |
| `console.html` | La página, servida tal cual por `server.rs` en sus tres direcciones: sin paso de compilación y fuera del bundle de la app. |
| `console.rs` | La sesión: cliente e informe elegidos, la cola, el único hilo que corre las comprobaciones y las preguntas a la persona. |
| `report_view.rs` | La vista de un informe, igual lo corra la sesión o no: conjuntos en el orden del catálogo con sus recuentos; no sabe de la sesión. |
| `snapshot.rs` | El estado de la sesión activa que recibe la página: la vista de su informe más cliente, cola, comprobación en curso y pregunta. |
| `subject.rs` | La resolución del sujeto a partir de su perfil: binario, envoltorio aislado, raíz de confianza y almacén. |
| `catalogue.rs` | La lectura de `catalogue/`, un TOML por conjunto con los metadatos de cada exigencia; no nombra a ningún cliente. |
| `checks.rs` | El cuerpo ejecutable: cómo se conduce cada entrada y cómo se resuelve su veredicto, sin escribir el informe. |
| `verdicts.rs` | Las reglas que traducen lo observado a veredicto. |
| `baseline.rs` | El perfil del cliente, que solo decide cómo se lanza, y los nombres de los resultados; no juzga nada. |
| `validation.rs` | La validación de una tanda contra una referencia: validada o sus discrepancias, que son fallos de la suite o de la referencia. |
| `reference/` | Las referencias, un TOML por cliente y versión con los resultados que una ficha `BUG-NN` explica. |
| `errand.rs` | El trámite: el conductor de Node, el sujeto invocado y lo que se extrae de cada evento. |
| `dossier.rs` | El expediente de un informe, `reports/conformance/<nombre>/dossier.json`: lo ve cualquiera, lo continúa solo su cliente. |
| `transcript.rs` | Las tramas y el registro de cada comprobación, en `transcripts/` dentro del informe. |
| `livelog.rs` | La línea de registro marcada por procedencia (`sede`, `cliente`, `suite`), en fichero y en vivo. |
| `comparison.rs` | La comparación de dos informes, comprobación a comprobación en el orden del catálogo y con su conjunto. |
