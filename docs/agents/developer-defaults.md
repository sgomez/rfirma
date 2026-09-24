# /developer defaults

Repo-level defaults for the `/developer` pipeline, chosen at setup. Per-run
flags override them: `--parallel` / `--sequential`, `--auto-merge` /
`--no-auto-merge`.

```
execution: parallel
merge: auto
```

- `execution` — `parallel` builds independent sub-issues concurrently in
  waves; `sequential` delivers one sub-issue fully before the next starts.
- `merge` — `manual` stops at a CLEAN review: the PR is marked ready and the
  merge is left to a human. `auto` means the user has **pre-authorized** the
  code host's merge operation (`gh pr merge`, `glab mr merge`, …) on any PR
  whose review verdict is CLEAN — the orchestrator merges to `main`
  unattended, and this line is the standing record of that authorization.
  A local code host (see `docs/agents/code-host.md`) supports `manual` only.

To change the defaults, edit the values above (or re-run
`/setup-developer-skills`).
