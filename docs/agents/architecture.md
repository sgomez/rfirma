# Architecture map

Where this repo's architecture is written down, and how binding it is. This
file is a **map, not the architecture**: it points at the documents, so a
worker opens the one or two that cover the paths it is about to touch instead
of the whole corpus.

```
architecture: normative
```

- `normative` — the documents describe how this repo **must** be built. A
  change that crosses a boundary they state is a review finding that blocks
  the merge, like any other correctness problem.
- `advisory` — the documents describe the **intended** direction, which the
  code has not entirely reached. A crossing is recorded as a non-blocking note
  in the review and never flips the verdict.
- `none` — no architecture documentation. Every architecture step of the
  `/developer` pipeline is skipped entirely.

`normative` is the mode this repo runs in, and `CLAUDE.md` says why: **un ADR
es la ley vigente, escrita una sola vez**. Un ADR no se supera ni se marca
`Superseded` — se reescribe en su sitio —, así que el corpus de `docs/adr/` no
describe una aspiración sino el estado obligatorio del código. Citar un ADR por
número (`ADR-0005`) es estable; citarlo por ruta o por epígrafe, no.

## Scope

Each row maps a **glob over repository paths** to the document that governs
them. A worker matches the paths its job touches against these globs, opens the
documents that match plus every `(global)` row, and reads no others.

| Scope | Document |
|---|---|
| `(global)` | `docs/adr/` — el índice con el título de cada ADR está en `docs/AGENTS.md`; ábrelo primero y baja solo a los que te toquen |
| `rfirma-app/src-tauri/**` | `docs/adr/0017-arquitectura-de-los-dos-lados.md` (contextos con capas, puertos), `rfirma-app/src-tauri/src/AGENTS.md` (mapa del backend y sus guardas) |
| `rfirma-native-bridge/**` | `docs/adr/0002-dependencias-java-desde-maven-local.md`, `docs/adr/0003-memoria-manual-en-la-frontera-ffi.md` |
| `packaging/**` | `docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md`, `docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md`, `docs/adr/0015-canal-de-distribucion-propio.md` |
| `justfile`, `.github/workflows/**` | `docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md`, `docs/adr/0014-gradas-de-prueba-y-puerta-de-calidad.md` |

Rows are matched most-specific-first; a path matching two rows gets both
documents.

**`rfirma-app/src/**` (la interfaz) no tiene fila todavía**, y eso es una
respuesta legítima, no un olvido: su arquitectura aún no está escrita. Hasta
que lo esté, un cambio de frontend se rige solo por la fila `(global)` —
`ADR-0007`, `ADR-0009`, `ADR-0018` y las fichas de `docs/design/` son las que
más le suelen tocar. Cuando exista el documento, añade la fila aquí.

## Who reads this, and when

- **`spec-surveyor`**, at the start of a run: reads the documents covering the
  spec's footprint and folds their binding constraints into the `## Spec
  baseline` comment.
- **`brief-author`**, per sub-issue: reads the documents covering that
  sub-issue's paths and writes the constraints into the `## Delivery brief`, so
  the builder is bound **before** it writes code. This is where architecture
  costs the least — a paragraph in a brief instead of a rejected change.
- **`diff-reviewer`**, per change: checks the diff against the same documents
  as a **separate, narrow axis** from the code review — does this diff cross a
  boundary these documents state?
- **`spec-surveyor`** again, at wrap-up: reads the spec's whole accumulated
  diff and reports drift the per-change reviews could not see, because from
  inside one change there is nothing to see.

## Who writes this

**Humans, and `/setup-developer-skills`.** The `/developer` pipeline never
edits an architecture document, an ADR, or this map — a run that rewrites its
own constraints changes every future run, unattended and unreviewed. When a
change legitimately needs a boundary these documents do not describe, the
answer is never to reject the change and never to quietly amend the document:
the reviewer records it as a **proposed** doc update in the review body, and
the wrap-up survey collects it on the spec issue. A person applies it.
