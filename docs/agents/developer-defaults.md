
# /developer defaults

Repo-level defaults for the `/developer` pipeline, chosen at setup. Per-run
flags override them: `--parallel` / `--sequential`, `--auto-merge` /
`--no-auto-merge` and `--build-oversized`.

```
execution: parallel
merge: manual
oversized: escalate
```

- `execution` — `parallel` builds independent sub-issues concurrently in
  waves; `sequential` delivers one sub-issue fully before the next starts.
- `merge` — `manual` stops at a CLEAN review: the PR is marked ready and the
  merge is left to a human. `auto` means the user has **pre-authorized** the
  code host's merge operation (`gh pr merge`, `glab mr merge`, …) on any PR
  whose review verdict is CLEAN — the orchestrator merges to `main`
  unattended, and this line is the standing record of that authorization.
  A local code host (see `docs/agents/code-host.md`) supports `manual` only.

  **Por qué este repositorio está hoy en `manual`.** El CI de
  [#11](https://github.com/sgomez/rfirma/issues/11) verifica que el puente
  Java compila y que las dependencias de AutoFirma resuelven, y **nada
  más** — `docs/agents/code-host.md` lo dice en letra grande. Con
  `merge: auto`, `/developer` fusionaría a `main` código de firma sin que
  nadie haya visto un PDF firmado. Vuelve a `auto` cuando el carril de
  pruebas de [#33](https://github.com/sgomez/rfirma/issues/33) esté vivo y
  el CI ejecute el ciclo trifásico.
- `oversized` — what to do with a sub-issue triage scores too big to fit in
  one context window. `escalate` hands it to a human to re-cut and builds
  nothing. `build` builds it anyway at `opus`, taking triage's fault lines as
  the builder's order of work — set it here when this repo's tickets are
  deliberately cut large and you would rather spend the build than the round
  trip. Either way, a ticket whose body explicitly forbids splitting is
  always built.

To change the defaults, edit the values above (or re-run
`/setup-developer-skills`).
