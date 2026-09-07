# /developer defaults

Repo-level defaults for the `/developer` pipeline, chosen at setup. Per-run
flags override them: `--parallel` / `--sequential`, `--auto-merge` /
`--no-auto-merge` and `--build-multiple`.

```
execution: sequential
merge: auto
shape: build
```

- `execution` — `parallel` builds independent sub-issues concurrently in
  waves; `sequential` delivers one sub-issue fully before the next starts.
- `merge` — `manual` stops at a CLEAN review: the PR is marked ready and the
  merge is left to a human. `auto` means the user has **pre-authorized** the
  code host's merge operation (`gh pr merge`, `glab mr merge`, …) on any PR
  whose review verdict is CLEAN — the orchestrator merges to `main`
  unattended, and this line is the standing record of that authorization.
  A local code host (see `docs/agents/code-host.md`) supports `manual` only.
- `shape` — what to do with a sub-issue whose ticket holds **several
  independent deliverables** (an "and" joining two features, a migration paired
  with the feature built on it, acceptance criteria that read as a checklist of
  separate things). `escalate` hands it to a human to re-cut and builds nothing;
  `build` builds it anyway at `opus`, taking the brief-author's fault lines as
  the builder's order of work — set it here when this repo's tickets are
  deliberately cut that way and you would rather spend the build than the round
  trip. Either way, a ticket whose body explicitly forbids splitting is always
  built.

  This is **not** a judgement about the work being long or hard. A build that
  outgrows one context window hands its branch to a fresh builder and carries
  on, and a change beyond the model that wrote it climbs the fix ladder —
  neither needs deciding in advance. `shape` is about what the finished change
  would be: one that ships two things at once cannot be reviewed as a unit,
  reverted without losing the good half, or depended on by siblings that need
  only one of them.

  (Older copies of this file call this knob `oversized`. The pipeline still
  reads that key; the meaning changed with the name.)

To change the defaults, edit the values above (or re-run
`/setup-developer-skills`).
