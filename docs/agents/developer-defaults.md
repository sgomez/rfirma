
# /developer defaults

Repo-level defaults for the `/developer` pipeline, chosen at setup. Per-run
flags override them: `--parallel` / `--sequential`, `--auto-merge` /
`--no-auto-merge` and `--build-oversized`.

```
execution: sequential
merge: manual
oversized: escalate
```

> **Por qué `sequential` y no `parallel`.** El corte del spec
> [#46](https://github.com/sgomez/rfirma/issues/46) es **horizontal por módulo**, que es
> justo la forma que invita a construir en paralelo, y el
> [#10](https://github.com/sgomez/rfirma/issues/10) llegó a escribir `parallel` en su
> apartado *Decidido ya*. Se cambia por un hecho que ninguna de esas decisiones tuvo
> delante: **el repositorio arranca de cero**. Ocho sub-issues simultáneos en la oleada
> más ancha no se pisan en su módulo, se pisan en el `Cargo.toml`, el `package.json`, el
> `justfile` y el `ci.yml` — ficheros compartidos que en un proyecto ya escrito casi nadie
> toca y que aquí toca todo el mundo. El coste de `sequential` es tiempo de reloj; el de
> `parallel` sería resolver conflictos de merge en los cimientos. El corte horizontal **no
> cambia**: sigue siendo la unidad de trabajo, solo que se entregan de uno en uno, en el
> orden que dictan las aristas `blocked_by` nativas del #46.
>
> Reevaluarlo cuando el andamiaje esté puesto y estable tiene sentido: `--parallel` en la
> invocación lo activa sin tocar este fichero.

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
