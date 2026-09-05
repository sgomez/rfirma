# Delivery ledger

## Local calibration

- #47 triaged `oversized` (4 bundled deliverables: app tree, justfile grid, quality tooling, two-lane CI + testdata kit) but, built at opus under `--build-oversized`, converged in ONE fix cycle to CLEAN. Mechanism: the dispatcher scored breadth — four independent deliverables with no existing pattern to imitate, because `rfirma-app/` did not exist — not depth. Foundational scaffolding tickets in this repo are wide but shallow: each deliverable is boilerplate wiring with a well-specified target, and the fault lines the dispatcher produced were themselves a sufficient order of work. Signal: a sub-issue whose fault lines are all "create <thing> from scratch per <ADR>" rather than "change interacting behaviour X" — score `complex` (opus), not `oversized`.
- The review caught two things the narrow CI structurally cannot: `just native` omitting `-H:IncludeResources` for the iText fonts (runtime-only failure, "Courier not found as resource") and the slow CI lane reporting green without ever running because the PR lacked the `native` label. Both are consistent with `docs/agents/code-host.md`'s own warning that a green check here means almost nothing; worth checking whether the `native`-label requirement deserves to be louder in the agent docs.
- Los tres sub-issues (#48, #49, #50) se puntuaron `complex`/opus y el precio salió correcto: 1-2 ciclos de arreglo cada uno, ninguno escalado, ninguno se quedó sin contexto. El tier estuvo bien en los tres.
- Mecanismo estructural, no estadística: el repositorio arranca casi vacío, así que la triage de cualquier sub-issue de #46 dice «primero de su especie, no hay patrón que imitar» y eso empuja a `complex` de forma sistemática. Esa señal dejará de ser informativa en cuanto haya andamiaje: reevaluar el tier de los sub-issues posteriores (#51 en adelante) contra el código ya escrito, no contra el vacío.
- #48 (puente Java, prefirma/postfirma PAdES) necesitó dos ciclos por la MISMA clase de fallo — firma inválida en silencio con `{"ok":true}` — que volvió por puertas distintas tras el primer arreglo (primero el PDF y TimeZone.setDefault; luego TimeZone.getDefault() fuera del cerrojo y la cadena de certificados sin sellar). El sellado de sesión es propenso a esto: al arreglar una entrada sin sellar conviene enumerar TODAS las entradas del sello de una vez, no la señalada.
- #50 gastó un merge-fix por política `merge: manual`, no por dificultad del ticket: PR hermanos cortados del mismo `main`. Sin relación con el tier.
- El spec #46 completo (15 sub-issues) cerró con TODO a `complex`/opus salvo tres tickets que la triage puntuó `oversized` y se construyeron igualmente por el flag `--build-oversized`: #57 (4 superficies de UI), #59 (panel + PIN + progreso + clasificación de errores) y #62 (flatpak: manifiesto + vendorizado + garantía del .so + verificación en sandbox). Los tres salieron bien: 1, 1 y 0 ciclos de arreglo, ninguno volvió con media función, y la revisión confirmó las cuatro rebanadas en los tres casos. Señal para la calibración: en este repositorio un `oversized` de esta forma —varias superficies independientes pero cada una con su ficha de diseño ya escrita— es construible de una vez a opus. Lo que hizo que funcionara fue pasarle al constructor las líneas de fractura de la triage como ORDEN DE TRABAJO explícito, con instrucción de comitear cada rebanada al terminarla.
- El veto de no-partir funcionó en #60 y #61: los dos habrían puntuado `oversized` y la triage los bajó a `complex` al encontrar la directiva en el cuerpo. Ambos convergieron en 1 ciclo. La directiva del autor es mejor señal que la rúbrica cuando existe.
- #100 (standard/sonnet, 2 ciclos): la triage lo puntuó `standard` porque el andamiaje de `CKA_ID` de #98 ya estaba fusionado, así que leía como «un filtro más una comprobación de guion de aprovisionamiento». El coste no estuvo en la edición: el primer intento vació el listado de SoftHSM con el filtro sin sesión y rompió toda la grada C de `native_cycle.rs`, y solo la revisión lo cazó. Señal: un ticket de PKCS#11 cuyo criterio de aceptación habla de qué enseña el **listado** lo ejercita la grada C de integración, no las pruebas unitarias — puntuar `complex` sin importar cuánto andamiaje ya exista.
- #101 (standard/sonnet, 2 ciclos): sobre el papel, dos líneas de manifiesto flatpak; pero su criterio de aceptación exigía un **paso de verificación** dentro del sandbox (`packaging/flatpak/verifica.sh`), y los dos ciclos de arreglo fueron sobre que esa comprobación fuera real: primero faltaba, luego discriminaba por un mensaje de error de `touch` en inglés y salía verde en falso en un anfitrión en castellano. Señal: un ticket de empaquetado cuyo AC incluye «verificarlo dentro del sandbox» lleva un guion de shell que necesita revisión de independencia de idioma y de entorno — la edición del manifiesto no es el trabajo.
- Los tres sub-issues puntuados opus (#97, #98, #99) llegaron CLEAN a la primera revisión con cero ciclos de arreglo, así que las llamadas complex/opus salieron bien de precio. El error de tarificación de esta tanda fue de un solo sentido: standard se quedó corto dos veces, complex nunca se pasó.
- La nota de la cosecha anterior sobre «primero de su especie» se confirmó y ya expiró: siguió puntuando `complex` todo el spec incluso cuando ya había patrones que imitar (#58 tenía placement.rs de #51, #59 tenía los diálogos de #57). A partir de aquí, tier contra el código escrito, no contra el vacío.
- Coste real: los 12 sub-issues de este tramo consumieron entre 120k y 393k tokens de subagente por construcción. #60 (orquestación trifásica, indivisible por directiva) fue el techo con 393k y 228 llamadas de herramienta; nadie se quedó sin contexto.
- Patrón de fallo dominante y transversal a todo el spec: el arreglo de un fallo abre otra puerta a LA MISMA clase de fallo. #48 lo hizo dos veces con la firma inválida en silencio; #52 lo repitió (arreglar el EXIF con `into_decoder()` perdió el tope de 512 MB del búfer); #56 lo repitió (el refactor del carril nativo dejó el isolate sin desmontar y hacía `dlclose` con él vivo). Al arreglar una entrada de un invariante, enumerar TODAS las entradas de ese invariante, no solo la señalada.
- #89 (standard/sonnet, 1 ciclo): los tickets cuyo producto es una MEDICIÓN (ampliar packaging/flatpak/verifica.sh, documentar en docs/research/) tienen un modo de fallo que la rúbrica genérica de complejidad no ve: el guion puede AFIRMAR lo medido sin medirlo. Aquí el paso 6 mataba la aplicación sin comprobar antes que había arrancado, así que el ID-72 se daba por medido en falso; lo cazó la revisión, no la construcción. Señal: el criterio de aceptación es «queda medido/documentado X» y no «el código hace X». Mecanismo, no estadística: a sonnet le sale un guion que corre y sale con 0, no un guion que demuestra. Considerar subir de grada estos tickets, o exigir en el brief que cada paso verifique su precondición antes de afirmar su medición.
- #82 (triaje: oversized → complex por veto propio): el dispatcher detectó la directiva explícita de no dividir en el cuerpo del issue («van juntas porque separarlas deja el árbol rojo») y rebajó su propia puntuación a complex. Funcionó: 1 ciclo y fusionado. El veto de directiva está operando como se pretendía; no hace falta regla nueva.
- Los dos sub-issues de #108 se puntuaron complex/opus y los dos volvieron CLEAN a la primera revisión, cero ciclos de arreglo, cero merge-fixes. El mecanismo citado por la triage en ambos casos fue andamiaje previo ya fusionado (#98/#99/#100 para la fontanería del certificado, el patrón de persistencia de `visible_signature` para la escritura en memoria) — es decir, tickets que seguían un patrón existente, no diseño nuevo. Si eso basta para tarificar el próximo ticket de esta forma por debajo de opus es un juicio que necesita un segundo dato; se anota, sin escribir todavía una regla que baje el tier.
- Por qué el default es `sequential` y no `parallel`: el corte de los specs de este repositorio es horizontal por módulo, pero los PR hermanos no chocan en su módulo — chocan en `Cargo.toml`, `package.json`, `justfile` y `ci.yml`, que en un proyecto ya escrito casi nadie toca y aquí toca todo el mundo. El coste de `sequential` es tiempo de reloj; el de `parallel`, conflictos de merge en los cimientos. Con el andamiaje ya puesto, merece una reevaluación: `--parallel` en la invocación lo prueba sin tocar la configuración.
- #135 puntuó `oversized` por tamaño de fichero, pero el cuerpo del issue prohibía explícitamente dividirlo ("a propósito de una vez"), así que el veto de directiva la bajó a `complex`/opus. Se construyó entera en ~31 min con 1 ciclo de arreglo, y el único hallazgo fue un dato de tamaño de fichero obsoleto en un fichero-mapa. Señal: una señal de "sobredimensionado por reparto de tamaño de fichero" sobre un refactor mecánico de mover-y-partir está sobrevalorada en este repositorio, y `complex`/opus es el tier correcto cuando el ticket lleva una directiva explícita de no partir.
- #136 y #137 puntuaron ambos `complex`/opus y ambos volvieron CLEAN a la primera revisión con cero ciclos de arreglo, en ~19 min cada uno. Mecanismo: los dos tenían un patrón ya fusionado en el repositorio que imitar (`app/` de #135; el guarda `single_cfg_os_site.rs` para #137), y la propia triage lo nombró en sus pistas. El tier salió bien pero se quedó en lo más alto de su banda — dato a vigilar si se repite, no regla todavía.
- Los dos sub-issues estándar/sonnet de #125 (#127, #128) volvieron NEEDS_FIXES en la primera revisión; el de #128 por una puerta de calidad roja (CRAP: `choose_rubric` puntuó 42 por encima del umbral 30) más un almacén que se escribía y nunca se leía. Los dos sub-issues opus de rebanadas comparables de UI+comando (#130, #131) llegaron CLEAN a la primera revisión con cero ciclos. Señal: en este repositorio, una rebanada «un comando de Tauri + su adaptador TS + el cableado de UI» lleva reglas densas y no obvias (el umbral de CRAP, las tres guardas de `commands/mod.rs`, el ADR-0011) que el nivel sonnet no satisface de forma fiable a la primera; la triage puntuó las dos `standard`.
- El coste y la precisión de la triage salieron bien en lo demás de #125: los seis sub-issues convergieron, ninguna escalada, ningún conflicto de fusión (ejecución secuencial, cadena de dependencias estricta).
- #169 y #177 se triaron `oversized` (model=none) pero se construyeron a opus por el default `oversized: build` del repositorio, y los dos fusionaron tras un único ciclo de arreglo sin escalada. Dos de nueve, ambos primero-de-su-familia dentro del ticket: en este repositorio `oversized` dispara de más, y un ticket acotado a una sola costura debería puntuar `complex` aunque agrupe varias decisiones de ID.
- La triage de #177 informó de que sus tres bloqueantes (#172, #174, #175) «no estaban en main» cuando los tres se habían fusionado minutos antes en la misma tanda secuencial: el dispatcher leyó un checkout obsoleto. Mecanismo: la triage dedujo de esa lectura obsoleta que «no hay patrón que imitar», y eso empujó también hacia `oversized`. Un dispatcher debe hacer `fetch origin/main` antes de concluir que un patrón está ausente.
- Tanda secuencial de nueve sub-issues, cero conflictos de fusión y cero merge-fixes; seis de nueve necesitaron exactamente un ciclo de arreglo, ninguno necesitó dos. La forma «un ciclo y CLEAN» es la norma de este repositorio, no una señal de alarma.

- #201 y #202 (rebanadas de reubicación de UI entre `DocumentViewer` y `SigningPanel`) puntuaron `standard` y costaron muy por encima de su grada: ~250k y ~200k tokens, 189 y 143 llamadas de herramienta, 15-17 min cada uno — el triple de cualquier otro `standard` de la tanda. Los dos tocaban TSX + CSS + los cinco catálogos de i18n + dos ficheros de prueba. Señal: un ticket de UI que reubica un elemento entre dos componentes ya existentes y toca los cinco catálogos de locale tarifica como `complex`, no `standard`.

- La triage lee `main` en el momento de la triage, y en ejecución paralela esa foto queda obsoleta antes de que el constructor arranque. Costó dos veces en la tanda de #250: a #269 se le puntuó asumiendo que el patrón de menú de servicio de KDE de #268 no existía todavía (ya se había fusionado), y a #276 se le puntuó asumiendo que `publish.yml` no estaba en `main` (se había fusionado con #275). Las dos veces hubo que corregir a mano el prompt del constructor. Señal para el despachador: cuando los bloqueantes de un ticket ya están cerrados, hay que asumir que sus artefactos SÍ están en `main` y decirlo, en vez de informar «no hay patrón que imitar».
- #266 puntuó `standard`/sonnet por ser solo de configuración, y lo era — pero su criterio de aceptación dependía de una receta `just bundle` que el ADR-0013 daba por existente y no existía. Un ticket de solo-configuración cuyo criterio se verifica con una receta de `just` o un guion de empaquetado necesita comprobar antes si esa receta existe; cuando no existe, el ticket es `standard` como mucho y además hay que construir la receta.
- Los dos tickets sobredimensionados de esta tanda (#275, #277) se construyeron igualmente bajo `oversized: build` a opus, y los dos llegaron con sus entregables completos y coherentes, un ciclo de arreglo cada uno. Para la forma de ticket de este repositorio, `oversized` desde el despachador ha significado hasta ahora «cuatro entregables relacionados en una sola rebanada de empaquetado», que opus entrega entero — no «demasiado grande para terminar».
- #262 se retuvo deliberadamente fuera de su tanda por el orquestador por tocar el mismo fichero que el #272 en vuelo; construido después de fusionarse #272, volvió CLEAN con cero ciclos de arreglo y sin merge-fix. El solape de fichero entre miembros de una tanda merece el coste de reloj de serializarlos.
- La vía de publicación de revisiones produjo dos revisiones con cuerpo vacío (en la PR #294 y la PR #303). En la #303 el re-revisor leyó la revisión vacía como una verificación e informó `blocked`; hubo que relanzarlo con instrucción explícita de comprobar que el cuerpo no estuviera vacío. Un revisor no debe tratar una revisión sin cuerpo en HEAD como evidencia de nada.

## Run log

2026-08-31 spec=#46 sub=#47 model=opus effort=medium pr=#64 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=ready-to-merge
2026-08-31 spec=#46 sub=#48 model=opus effort=medium pr=#65 verdict=CLEAN cycles=2 mergefix=0 wave=— outcome=ready-to-merge
2026-08-31 spec=#46 sub=#49 model=opus effort=medium pr=#66 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=ready-to-merge
2026-08-31 spec=#46 sub=#50 model=opus effort=medium pr=#67 verdict=CLEAN cycles=1 mergefix=1 wave=— outcome=ready-to-merge
2026-08-31 spec=#46 sub=#51 model=opus effort=medium pr=#68 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-08-31 spec=#46 sub=#52 model=opus effort=medium pr=#69 verdict=CLEAN cycles=3 mergefix=0 wave=— outcome=merged
2026-08-31 spec=#46 sub=#53 model=opus effort=medium pr=#70 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-08-31 spec=#46 sub=#54 model=opus effort=medium pr=#71 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-08-31 spec=#46 sub=#55 model=opus effort=medium pr=#72 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-08-31 spec=#46 sub=#56 model=opus effort=medium pr=#73 verdict=CLEAN cycles=2 mergefix=0 wave=— outcome=merged
2026-08-31 spec=#46 sub=#57 model=opus effort=medium pr=#74 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged oversized=built
2026-09-01 spec=#46 sub=#58 model=opus effort=medium pr=#75 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#46 sub=#59 model=opus effort=medium pr=#76 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged oversized=built
2026-09-01 spec=#46 sub=#60 model=opus effort=medium pr=#77 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#46 sub=#61 model=opus effort=medium pr=#78 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#46 sub=#62 model=opus effort=medium pr=#79 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged oversized=built
2026-09-01 spec=#80 sub=#85 model=opus effort=medium pr=#90 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=escalated
2026-09-01 spec=#81 sub=#82 model=opus effort=medium pr=#92 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#81 sub=#83 model=opus effort=medium pr=#93 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#81 sub=#89 model=sonnet effort=medium pr=#94 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#95 sub=#97 model=opus effort=medium pr=#102 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#95 sub=#98 model=opus effort=medium pr=#103 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#95 sub=#99 model=opus effort=medium pr=#105 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#95 sub=#100 model=sonnet effort=medium pr=#106 verdict=CLEAN cycles=2 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#95 sub=#101 model=sonnet effort=medium pr=#107 verdict=CLEAN cycles=2 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#108 sub=#109 model=opus effort=medium pr=#111 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-01 spec=#108 sub=#110 model=opus effort=medium pr=#112 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#134 sub=#135 model=opus effort=medium pr=#138 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#134 sub=#136 model=opus effort=medium pr=#139 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#134 sub=#137 model=opus effort=medium pr=#140 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#125 sub=#126 model=opus effort=medium pr=#141 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#125 sub=#127 model=sonnet effort=medium pr=#142 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#125 sub=#128 model=sonnet effort=medium pr=#143 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#125 sub=#129 model=opus effort=medium pr=#144 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#125 sub=#130 model=opus effort=medium pr=#145 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#125 sub=#131 model=opus effort=medium pr=#146 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#168 sub=#169 model=opus effort=medium pr=#178 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#168 sub=#170 model=sonnet effort=medium pr=#179 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-02 spec=#168 sub=#171 model=opus effort=medium pr=#180 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#168 sub=#172 model=opus effort=medium pr=#181 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#168 sub=#173 model=opus effort=medium pr=#182 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#168 sub=#174 model=opus effort=medium pr=#183 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#168 sub=#175 model=opus effort=medium pr=#184 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#168 sub=#176 model=sonnet effort=medium pr=#186 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#168 sub=#177 model=opus effort=medium pr=#187 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#195 model=sonnet effort=medium pr=#205 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#196 model=sonnet effort=medium pr=#206 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#197 model=sonnet effort=medium pr=#209 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#198 model=sonnet effort=medium pr=#210 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#199 model=opus effort=medium pr=#211 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#200 model=sonnet effort=medium pr=#212 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#201 model=sonnet effort=medium pr=#213 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#202 model=sonnet effort=medium pr=#214 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#203 model=sonnet effort=medium pr=#215 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-03 spec=#194 sub=#204 model=sonnet effort=medium pr=#216 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-04 spec=#250 sub=#254 model=none effort=none pr=none verdict=— cycles=0 mergefix=0 wave=1 outcome=escalated
2026-09-04 spec=#250 sub=#252 model=sonnet effort=medium pr=#279 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-04 spec=#250 sub=#255 model=sonnet effort=medium pr=#280 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-04 spec=#250 sub=#253 model=opus effort=medium pr=#282 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-04 spec=#250 sub=#263 model=opus effort=medium pr=#281 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-04 spec=#250 sub=#265 model=sonnet effort=medium pr=#283 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-04 spec=#250 sub=#273 model=sonnet effort=medium pr=#284 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-04 spec=#250 sub=#256 model=sonnet effort=medium pr=#286 verdict=CLEAN cycles=1 mergefix=0 wave=2 outcome=merged
2026-09-04 spec=#250 sub=#267 model=opus effort=medium pr=#285 verdict=CLEAN cycles=1 mergefix=1 wave=1 outcome=merged
2026-09-04 spec=#250 sub=#264 model=sonnet effort=medium pr=#287 verdict=CLEAN cycles=1 mergefix=0 wave=2 outcome=merged
2026-09-04 spec=#250 sub=#257 model=opus effort=medium pr=#288 verdict=CLEAN cycles=1 mergefix=0 wave=2 outcome=merged
2026-09-04 spec=#250 sub=#270 model=opus effort=medium pr=#290 verdict=CLEAN cycles=1 mergefix=0 wave=2 outcome=merged
2026-09-04 spec=#250 sub=#258 model=sonnet effort=medium pr=#291 verdict=CLEAN cycles=1 mergefix=0 wave=3 outcome=merged
2026-09-04 spec=#250 sub=#259 model=sonnet effort=medium pr=#292 verdict=CLEAN cycles=1 mergefix=0 wave=3 outcome=merged
2026-09-04 spec=#250 sub=#260 model=sonnet effort=medium pr=#293 verdict=CLEAN cycles=0 mergefix=0 wave=4 outcome=merged
2026-09-04 spec=#250 sub=#271 model=opus effort=medium pr=#295 verdict=CLEAN cycles=0 mergefix=0 wave=5 outcome=merged
2026-09-04 spec=#250 sub=#261 model=opus effort=medium pr=#296 verdict=CLEAN cycles=0 mergefix=0 wave=5 outcome=merged
2026-09-04 spec=#250 sub=#266 model=sonnet effort=medium pr=#294 verdict=CLEAN cycles=1 mergefix=0 wave=5 outcome=merged
2026-09-04 spec=#250 sub=#268 model=opus effort=medium pr=#298 verdict=CLEAN cycles=1 mergefix=0 wave=6 outcome=merged
2026-09-04 spec=#250 sub=#272 model=sonnet effort=medium pr=#297 verdict=CLEAN cycles=1 mergefix=0 wave=6 outcome=merged
2026-09-04 spec=#250 sub=#274 model=opus effort=medium pr=#299 verdict=CLEAN cycles=1 mergefix=0 wave=6 outcome=merged
2026-09-04 spec=#250 sub=#269 model=opus effort=medium pr=#300 verdict=CLEAN cycles=1 mergefix=0 wave=7 outcome=merged
2026-09-04 spec=#250 sub=#262 model=opus effort=medium pr=#302 verdict=CLEAN cycles=0 mergefix=0 wave=7 outcome=merged
2026-09-04 spec=#250 sub=#275 model=opus effort=medium pr=#303 verdict=CLEAN cycles=1 mergefix=0 wave=8 outcome=merged
2026-09-04 spec=#250 sub=#276 model=opus effort=medium pr=#304 verdict=CLEAN cycles=1 mergefix=0 wave=8 outcome=merged
2026-09-04 spec=#250 sub=#277 model=opus effort=medium pr=#305 verdict=CLEAN cycles=1 mergefix=0 wave=8 outcome=merged
2026-09-06 spec=#340 sub=#353 model=opus effort=medium pr=#385 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=ready-to-merge
