# Mapa de la suite de conformidad

La suite mide un binario instalado de AutoFirma o rFirma con el `autoscript.js` de la sede como
instrumento. Es un crate aparte y fuera del CI (ADR-0013), y no es una grada ni una puerta
(ADR-0014). Su única cara es la consola web que levanta `just conformance`: no hay órdenes de
consola. Sus pruebas se corren con `cargo test` dentro de este directorio; las de la consola, con
`pnpm exec vitest run` y `pnpm exec biome ci` dentro de `console/`.

| Fichero | Qué es |
|---|---|
| `CONTEXT.md` | El glosario de la suite, un bounded context aparte del de la aplicación. |
| `docs/adr/` | Las decisiones que solo afectan a la suite, numeradas con las de `docs/adr/` de la raíz. |
| `catalogue/` | El catálogo: sus conjuntos en `sets.toml`, con su orden y sus capítulos, y un TOML por conjunto con los metadatos de cada comprobación; no nombra a ningún cliente. |
| `catalogue/matrix.toml` | La matriz de happy paths: sus ejes, sus planos y el estado de cada celda, cubierta por el id de una comprobación, no aplicable o hueco. |
| `bugs/autofirma-1.9.2.toml` | El registro de bugs conocidos: cada ficha `BUG-NN` del anexo A1 con su título y su estado en `master`; qué comprobación incumple cada uno lo declara el catálogo. |
| `reference/` | Las referencias, un TOML por cliente y versión con los resultados que una ficha `BUG-NN` explica. |
| `src/lib.rs` | La raíz del crate: sus módulos y la pasada que mide (`Probe`); lo que usan el binario y las pruebas de `tests/`. |
| `src/main.rs` | El arranque: lee el catálogo, levanta el servidor, imprime la URL con el token y la abre. |
| `src/server.rs` | El servidor HTTP local y su guarda (token, `Host`, `Origin`); sirve la consola compilada de `console/dist/` en `/`, `/informe/<nombre>` y `/comparar`, y traduce rutas a la sesión sin decidir nada. |
| `src/console.rs` | La sesión: cliente e informe elegidos, la cola en tramos, el único hilo que corre las comprobaciones y el testigo de verdad, que avisa a la persona y la espera por la página. |
| `src/witness.rs` | El testigo, el seam entre quien corre las comprobaciones y la persona, que le cuenta lo que tiene que hacer y espera a que dé paso; con su falso para las pruebas. |
| `src/report_view.rs` | La vista de un informe, igual lo corra la sesión o no: conjuntos en el orden del catálogo con sus recuentos; no sabe de la sesión. |
| `src/snapshot.rs` | El estado de la sesión activa que recibe la página: la vista de su informe más cliente, cola, comprobación en curso y la llamada a la persona. |
| `src/client.rs` | El cliente a prueba y qué cliente es: binario y un perfil aislado por almacén (`rsa`, `ec`, `token`, `token_apart`, `several`, `expired`), cada uno con su envoltorio y su raíz de confianza. |
| `src/catalogue.rs` | La comprobación como tipo —exigencia, cómo se provoca, qué hace la persona y una sola expectativa— y la lectura del catálogo con lo que el tipo no puede decir: ids repetidos, dos comprobaciones que miden lo mismo y lo que no casa con el manifiesto de la sede. |
| `src/manifest.rs` | El vocabulario de la sede que publica `driver.mjs --manifest`: modos y guiones con su sede, su familia y sus condiciones. |
| `src/matrix.rs` | La lectura de la matriz de happy paths y lo que no casa con el catálogo: celdas sin estado, repetidas o fuera de todo plano, y comprobaciones que no miden la celda que cubren. |
| `src/checks.rs` | El cuerpo ejecutable: cómo se conduce un grupo o se juzga con un trámite ya observado, la parada entre tramos, la guarda de las comprobaciones sin persona y los saludos por familia, sin escribir el informe. |
| `src/harness.rs` | El registro de arneses que el catálogo liga por nombre: lo que una comprobación monta alrededor del trámite —puertos ocupados, ficheros preparados—; no juzga. |
| `src/judge.rs` | El juez: lo observado frente a la expectativa declarada, a un resultado; con el vocabulario cerrado de expectativas, y sin lanzar trámites ni preguntar a nadie. |
| `src/known_bug.rs` | La lectura del registro de bugs conocidos, con el que el catálogo resuelve el `bug` de cada comprobación. |
| `src/outcome.rs` | El resultado de una comprobación, sus nombres en pantalla y PENDIENTE. |
| `src/validation.rs` | La validación de un informe contra una referencia: validado o sus discrepancias, que son fallos de la suite o de la referencia. |
| `src/errand.rs` | El trámite: su clave, lo observado que se guarda, el seam `ErrandRunner` con su adaptador de Node y su falso de tramas grabadas, y lo que se extrae de cada evento. |
| `src/report.rs` | El informe en disco, `reports/conformance/<nombre>/dossier.json`, con el estado de cada comprobación y los trámites observados por clave: lo ve cualquiera, lo continúa solo su cliente. |
| `src/transcript.rs` | Las tramas y el registro de cada comprobación, en `transcripts/` dentro del informe. |
| `src/livelog.rs` | La línea de registro marcada por procedencia (`sede`, `cliente`, `suite`), en fichero y en vivo. |
| `src/comparison.rs` | La comparación de dos informes, comprobación a comprobación en el orden del catálogo y con su conjunto. |
| `../testdata/site-driver/driver.mjs` | La sede bajo Node: corre un guion en un modo, o publica el manifiesto con `--manifest`; la comparte con los `conformance_*.rs` de la aplicación. |
| `../testdata/site-driver/manifest.mjs` | El manifiesto: los modos y todos los guiones, cada uno con su sede, su familia, sus modos, sus condiciones y si es solo del banco. |
| `../testdata/site-driver/lib/` | Lo común a los guiones: eventos y condiciones, modos, parches del `autoscript.js`, navegador mínimo, documentos de referencia, los analizadores de firma (CMS, XML, ZIP, PDF y PKCS#1) con los que se miden las condiciones, su canonicalización XML y la verificación con la clave del certificado devuelto (ADR-0031). |
| `../testdata/site-driver/test/` | Las pruebas de los analizadores de firma, que se corren con `node --test test/*.test.mjs`, y sus muestras hechas con OpenSSL y `zip`. |
| `../testdata/site-driver/certificates/` | Los certificados, sin su clave, del kit de la FNMT que montan los almacenes `several`, `expired` y `token_apart`: con ellos los guiones reconocen qué certificado volvió. |
| `../testdata/site-driver/scripts/` | Los guiones, un módulo por familia: certificado, firma, petición, ficheros, lote, servidor intermedio servido por HTTP, canal WebSocket y socket a mano. |
| `tests/catalogue_matches_the_docs.rs` | El cruce del catálogo, leído con el cargador del crate, y la referencia con `docs/afirma/1.9.2/`: capítulos, tabla SAF, fichas A1 y el registro de bugs, y el bug de cada comprobación con la causa de la referencia. |
| `tests/transcripts/` | Tramas grabadas de trámites de verdad, las que reproduce el ejecutor falso. |
| `.cargo/config.toml` | Dónde deja ts-rs los tipos del contrato: `console/src/contract/`. |
| `console/` | La consola web, un proyecto pnpm propio con React y Vite, fuera de `rfirma-app` y del CI; `just conformance-console` la compila en `console/dist/`. |
| `console/src/contract/` | Los tipos del JSON y del SSE, generados por ts-rs desde Rust y commiteados; no se editan a mano. |
| `console/src/main.tsx` | El arranque en el navegador: el token de la URL, el tema y el router. |
| `console/src/App.tsx` | Las tres vistas con sus URL: `/`, `/informe/<nombre>` y `/comparar`. |
| `console/src/suite/suite.ts` | El contrato HTTP tipado sobre un `Wire`, el seam cuyos adaptadores son el navegador y el servidor falso de las pruebas. |
| `console/src/suite/live.tsx` | El estado vivo que comparten las vistas: el último estado de la sesión, las líneas del registro y los avisos de error. |
| `console/src/shell/Shell.tsx` | El marco común: barra superior, conexión, ayuda de atajos y avisos. |
| `console/src/session/` | La sesión activa en `/`: los pasos cliente, informe y ejecutar, y la barra de la tanda en curso con su llamada a la persona. |
| `console/src/report/` | Un informe, con o sin controles de ejecución: conjuntos plegables, filtro por resultado, la ficha de cada comprobación, la validación y las tramas. |
| `console/src/log/LogDock.tsx` | El registro al pie, con filtro por procedencia, pausa y altura ajustable. |
| `console/src/compare/ComparePage.tsx` | La comparación de dos informes: solo lo que difiere, por conjunto. |
| `console/src/ui/` | Piezas sin dominio: iconos de resultado, tema, atajos de teclado, diálogo, reloj y notificaciones del escritorio. |
| `console/src/words.ts` | Los nombres en pantalla de resultados y clientes, y el formato de fechas y duraciones. |
| `console/src/styles.css` | La hoja de estilos única, con el tema claro y el oscuro en variables CSS. |
| `console/src/test/` | El servidor falso que cumple el contrato, los datos de prueba y el montaje de la consola para vitest. |
