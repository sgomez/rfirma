# Delivery ledger

## Local calibration

- #47 triaged `oversized` (4 bundled deliverables: app tree, justfile grid, quality tooling, two-lane CI + testdata kit) but, built at opus under `--build-oversized`, converged in ONE fix cycle to CLEAN. Mechanism: the dispatcher scored breadth — four independent deliverables with no existing pattern to imitate, because `rfirma-app/` did not exist — not depth. Foundational scaffolding tickets in this repo are wide but shallow: each deliverable is boilerplate wiring with a well-specified target, and the fault lines the dispatcher produced were themselves a sufficient order of work. Signal: a sub-issue whose fault lines are all "create <thing> from scratch per <ADR>" rather than "change interacting behaviour X" — score `complex` (opus), not `oversized`.
- The review caught two things the narrow CI structurally cannot: `just native` omitting `-H:IncludeResources` for the iText fonts (runtime-only failure, "Courier not found as resource") and the slow CI lane reporting green without ever running because the PR lacked the `native` label. Both are consistent with `docs/agents/code-host.md`'s own warning that a green check here means almost nothing; worth checking whether the `native`-label requirement deserves to be louder in the agent docs.
- Los tres sub-issues (#48, #49, #50) se puntuaron `complex`/opus y el precio salió correcto: 1-2 ciclos de arreglo cada uno, ninguno escalado, ninguno se quedó sin contexto. El tier estuvo bien en los tres.
- Mecanismo estructural, no estadística: el repositorio arranca casi vacío, así que la triage de cualquier sub-issue de #46 dice «primero de su especie, no hay patrón que imitar» y eso empuja a `complex` de forma sistemática. Esa señal dejará de ser informativa en cuanto haya andamiaje: reevaluar el tier de los sub-issues posteriores (#51 en adelante) contra el código ya escrito, no contra el vacío.
- #48 (puente Java, prefirma/postfirma PAdES) necesitó dos ciclos por la MISMA clase de fallo — firma inválida en silencio con `{"ok":true}` — que volvió por puertas distintas tras el primer arreglo (primero el PDF y TimeZone.setDefault; luego TimeZone.getDefault() fuera del cerrojo y la cadena de certificados sin sellar). El sellado de sesión es propenso a esto: al arreglar una entrada sin sellar conviene enumerar TODAS las entradas del sello de una vez, no la señalada.
- #50 gastó un merge-fix por política `merge: manual`, no por dificultad del ticket: PR hermanos cortados del mismo `main`. Sin relación con el tier.

## Run log

2026-08-31 spec=#46 sub=#47 model=opus effort=medium pr=#64 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=ready-to-merge
2026-08-31 spec=#46 sub=#48 model=opus effort=medium pr=#65 verdict=CLEAN cycles=2 mergefix=0 wave=— outcome=ready-to-merge
2026-08-31 spec=#46 sub=#49 model=opus effort=medium pr=#66 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=ready-to-merge
2026-08-31 spec=#46 sub=#50 model=opus effort=medium pr=#67 verdict=CLEAN cycles=1 mergefix=1 wave=— outcome=ready-to-merge
