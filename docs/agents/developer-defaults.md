# /developer defaults

Repo-level defaults for the `/developer` pipeline, chosen at setup. Per-run
flags override them: `--parallel` / `--sequential`, `--auto-merge` /
`--no-auto-merge`.

```
execution: parallel
merge: auto
```

- `execution` — `parallel` builds independent sub-issues concurrently, each
  as soon as its blockers are in; `sequential` delivers one sub-issue fully
  before the next starts.
- `merge` — what happens to the change that reaches `main`: the spec PR for
  a spec, the one PR for a single issue. `manual` stops at a CLEAN review:
  the PR is marked ready and the merge is left to a human. `auto` means the
  user has **pre-authorized** the code host's merge operation (`gh pr
  merge`, `glab mr merge`, …) on that PR once its review verdict is CLEAN —
  the orchestrator merges to `main` unattended, and this line is the
  standing record of that authorization.
  A local code host (see `docs/agents/code-host.md`) supports `manual` only.
- **Either way**, a spec's sub-issues are merged unattended into its
  integration branch, `agent/developer/spec-<N>`, with no checks gate and no
  review of their own: the spec PR's CI and review cover all of them. Nothing reaches `main` that way.

To change the defaults, edit the values above (or re-run
`/setup-developer-skills`).
