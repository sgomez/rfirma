# Delivery ledger

## Local calibration

Solo mecanismos vivos: lo que se repite, por qué, y qué hacer distinto. Lo que
ya es política escrita vive en `developer-defaults.md`; lo que es mecánica del
CI, en `code-host-ci.md`; ninguno de los dos se repite aquí.

- **Arreglar una entrada de un invariante abre otra a la misma clase de
  fallo.** Confirmado tres veces: #48 (firma inválida en silencio con
  `{"ok":true}`, dos veces por puertas distintas del sello de sesión), #52
  (arreglar el EXIF con `into_decoder()` perdió el tope de 512 MB del búfer) y
  #56 (el refactor del carril nativo dejó el isolate sin desmontar y hacía
  `dlclose` con él vivo). Al arreglar una entrada, enumerar TODAS las entradas
  de ese invariante, no solo la señalada.
- **Un ticket cuyo producto es una MEDICIÓN sale con el guion que afirma lo
  medido sin medirlo.** #89: el paso de `verifica.sh` mataba la aplicación sin
  comprobar antes que hubiera arrancado, así que daba la medición por buena.
  #499: se dio por buena una aserción sin comprobar que el carril lento
  ejecutara de verdad las pruebas `#[ignore]`. Las dos veces lo cazó la
  revisión, no la construcción, y la segunda pese al aviso de la primera. Al
  construir un ticket de medición, comprobar que cada paso verifica su
  precondición antes de afirmar su medición.
- **El re-revisor se ancla en su propio comentario.** Confirmado tres veces
  (#519 sobre la PR de #502, #522 sobre la de #505, #566 sobre la de #533): se
  bloquea con `reason=no new commits since the last review` aunque haya commits
  nuevos, porque las respuestas del arreglador a los hilos quedan registradas
  como envíos `COMMENTED` en el sha de HEAD y la consulta de «última revisión
  no-PENDING» acaba viendo su propia ancla. Se sortea pasando el sha de la
  revisión genuina de forma explícita en el prompt de re-revisión.
- **Una revisión con cuerpo vacío no es evidencia de nada.** Ocurrió en las PR
  #294 y #303; en la #303 el re-revisor la leyó como verificación e informó
  `blocked`. Comprobar que el cuerpo no esté vacío antes de darla por hecha.
- **Dónde chocan los PR hermanos en paralelo.** El corte de los specs es
  horizontal por módulo, y el temor era el choque en `Cargo.toml`,
  `package.json`, `justfile` y `ci.yml`. Reevaluado el 2026-09-08 con
  `--parallel` sobre diez sub-issues del #468: un solo conflicto y no en los
  cimientos, sino en `operation.rs`/`cycle.rs`, la capa de sitio/firma en Rust
  que casi todo ticket de firma toca. Cuando dos tickets de la misma tanda
  anuncian tocar esa capa, serializarlos: el solape de fichero merece el coste
  de reloj (#262 se retuvo así y volvió CLEAN sin merge-fix).
- **En paralelo, la foto de `main` que lee un trabajador queda obsoleta antes
  de que arranque el siguiente.** En la tanda de #250 costó dos correcciones a
  mano del prompt (#269 y #276, ambos dando por ausente un patrón que ya estaba
  fusionado). Hacer `fetch origin/main` antes de concluir que no hay patrón que
  imitar, y cuando los bloqueantes de un ticket están cerrados, asumir que sus
  artefactos SÍ están en `main`.

## Run log

Ventana rodante de evidencia, no archivo: las filas se podan una vez su señal
está recogida en `## Local calibration`. Los ficheros de tanda completos siguen
en `.scratch/archive/`.

2026-09-07 spec=#467 sub=#479 pr=#487 verdict=CLEAN cycles=1 rebriefs=0 continuations=0 arch=clear mergefix=0 tokens=828258 tools=452 wave=— outcome=merged
2026-09-07 spec=#467 sub=#480 pr=#489 verdict=CLEAN cycles=1 rebriefs=0 continuations=0 arch=clear mergefix=0 tokens=533559 tools=275 wave=— outcome=merged
2026-09-07 spec=#467 sub=#481 pr=#490 verdict=CLEAN cycles=1 rebriefs=0 continuations=0 arch=clear mergefix=0 tokens=835154 tools=444 wave=— outcome=merged
2026-09-07 spec=#467 sub=#491 pr=#507 verdict=CLEAN cycles=0 rebriefs=0 continuations=0 arch=clear mergefix=0 tokens=277542 tools=122 wave=— outcome=merged
2026-09-07 spec=#467 sub=#492 pr=#508 verdict=CLEAN cycles=0 rebriefs=0 continuations=0 arch=clear mergefix=0 tokens=358936 tools=152 wave=— outcome=merged
2026-09-07 spec=#467 sub=#493 pr=#509 verdict=CLEAN cycles=1 rebriefs=0 continuations=0 arch=clear mergefix=0 tokens=807507 tools=380 wave=— outcome=merged
2026-09-08 spec=#467 sub=#494 pr=#511 verdict=CLEAN cycles=1 rebriefs=0 continuations=0 arch=clear mergefix=0 tokens=714290 tools=335 wave=— outcome=merged
2026-09-08 spec=#467 sub=#495 model=opus effort=medium pr=#512 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#496 model=sonnet effort=medium pr=#513 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#497 model=sonnet effort=medium pr=#514 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#499 model=sonnet effort=medium pr=#515 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#500 model=opus effort=medium pr=#516 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#498 model=opus effort=medium pr=#517 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#501 model=sonnet effort=medium pr=#518 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#502 model=opus effort=medium pr=#519 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#503 model=opus effort=medium pr=#520 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#504 model=opus effort=medium pr=#521 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#467 sub=#505 model=sonnet effort=medium pr=#522 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#468 sub=#524 model=opus effort=medium pr=#554 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#468 sub=#526 model=sonnet effort=medium pr=#555 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#468 sub=#527 model=opus effort=medium pr=#556 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#468 sub=#525 model=opus effort=medium pr=#557 verdict=CLEAN cycles=0 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#468 sub=#528 model=opus effort=medium pr=#558 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=merged
2026-09-08 spec=#468 sub=#529 model=opus effort=medium pr=#559 verdict=CLEAN cycles=0 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#530 model=opus effort=medium pr=#561 verdict=CLEAN cycles=0 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#534 model=sonnet effort=medium pr=#560 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#543 model=opus effort=medium pr=#563 verdict=CLEAN cycles=0 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#532 model=opus effort=medium pr=#562 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#531 model=sonnet effort=medium pr=#564 verdict=CLEAN cycles=0 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#550 model=sonnet effort=medium pr=#565 verdict=CLEAN cycles=0 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#535 model=opus effort=medium pr=#568 verdict=CLEAN cycles=1 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#546 model=opus effort=medium pr=#569 verdict=CLEAN cycles=0 mergefix=0 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#533 model=opus effort=medium pr=#566 verdict=CLEAN cycles=2 mergefix=1 wave=1 outcome=merged
2026-09-08 spec=#468 sub=#536 model=opus effort=medium pr=#570 verdict=CLEAN cycles=0 mergefix=0 wave=2 outcome=merged
2026-09-08 spec=#468 sub=#544 model=opus effort=medium pr=#571 verdict=CLEAN cycles=0 mergefix=0 wave=2 outcome=merged
2026-09-08 spec=#468 sub=#539 model=sonnet effort=medium pr=#572 verdict=CLEAN cycles=1 mergefix=0 wave=2 outcome=merged
2026-09-08 spec=#468 sub=#547 model=opus effort=medium pr=#573 verdict=CLEAN cycles=0 mergefix=0 wave=3 outcome=merged
2026-09-08 spec=#468 sub=#537 model=opus effort=medium pr=#574 verdict=CLEAN cycles=0 mergefix=0 wave=3 outcome=merged
