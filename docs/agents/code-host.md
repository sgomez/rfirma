# Code host: GitHub

Changes for this repo are delivered as **GitHub pull requests**, using the
`gh` CLI (it infers OWNER/REPO from `git remote -v`).

GitHub is the **factory default** of the delivery skills (`implement-issue`,
`review-pr`, `fix-pr`, `/developer`): every code-host operation they name —
publish a change, check out a change in a worktree, post a review, mark
ready, reply to threads, merge — already carries its `gh` mechanics inline
in the skill. **No overrides: follow the skills' inline commands as
written.**

Repo-specific facts:

- **Change ref**: the PR number.
- **Base branch**: `main`. Start work from `origin/main`
  (`git fetch origin main && git checkout -b <branch> origin/main`) —
  never `git checkout main`.
- **Issue auto-close**: yes — `Closes #<n>` in the PR body closes issue
  `#<n>` when the PR merges. This repo's issues live in this repo's GitHub
  Issues (see `docs/agents/issue-tracker.md`), so auto-close applies.
- **Merge policy support**: both `merge: auto` and `merge: manual`.
- **Publishing commits**: `git push origin <branch>` (from a local
  `fix/pr-<PR>` branch: `git push origin HEAD:<pr-branch>`).
- **CI**: GitHub Actions, workflow `CI` (`.github/workflows/ci.yml`). See
  the section below for what it does and does not verify.

## CI

The orchestrator waits on the `CI` workflow before merging. Read the checks
with:

```bash
gh pr checks <PR> --watch
gh run view <RUN_ID> --log-failed
```

A red check blocks the merge; take the fix path rather than merging past it.

### What green actually means

**Narrow, and deliberately so.** As of
[issue #11](https://github.com/sgomez/rfirma/issues/11), CI verifies:

- the Java bridge **compiles** under GraalVM CE 25 with `-Xlint:all`;
- AutoFirma's dependencies **resolve and build** on a clean runner
  (`bootstrap.sh` against the immutable upstream tag `v1.9.1`);
- on `main`, on demand, and on PRs touching `rfirma-native-bridge/`,
  `bootstrap.sh` or `justfile`, that `native-image --shared` still
  **produces the shared library**.

It does **not** verify that anything works. This repo has **no production
code yet** — `NativeBridge.java` is the measurement bridge from issues #2 and
#13, and `just test` runs an empty suite. The Rust and TypeScript lanes, the
signing tests and the CRAP thresholds were deliberately left out of #11
because they would test code that does not exist; they arrive with the lanes
themselves.

**So the reviewer still installs and runs everything itself** — a green check
is not a substitute. That stays true until this section says the suite covers
production code.

### Running the same thing locally

One entry point, `just` (`apt-get install -y just maven`):

```bash
just check
```

`just --list` shows the rest. CI runs exactly `just check`, so a local pass
and a CI pass mean the same thing.

## Is the change mergeable?

Read before merging (the orchestrator, at the top of its checks gate):

```bash
gh pr view <PR> --json mergeStateStatus --jq .mergeStateStatus
```

`DIRTY` = conflicts with the base — take the merge-fix path. `BEHIND` =
mergeable but stale (`gh pr update-branch <PR>`). `CLEAN` = no conflict and
checks passing. `UNSTABLE` = no conflict but a check is failing — read it
before deciding. The review verdict and the checks are **both** gates, and
neither substitutes for the other: see "What green actually means" above.
