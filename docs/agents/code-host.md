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
  (`git fetch origin main`, then `git checkout -b <branch> origin/main` as
  a separate command) — never `git checkout main`. When the job names
  another base — `/developer` builds a spec's sub-issues on its integration
  branch, `agent/developer/spec-<N>` — that branch replaces `main` here and in the
  change's target.
- **Issue auto-close**: yes — `Closes #<n>` in the PR body closes issue
  `#<n>` when the PR merges **into `main`** (a PR into an integration branch
  closes nothing — the spec PR closes the spec and its sub-issues). This
  repo's issues live in this repo's GitHub Issues (see
  `docs/agents/issue-tracker.md`), so auto-close applies.
- **PR title is the changelog line**: `just release` writes `CHANGELOG.md`
  from the titles of `feat`, `fix` and `perf` PRs merged into `main`. After
  the conventional prefix, reuse the title of the issue the PR closes; with
  no issue, write it in Spanish as the person using rFirma would notice the
  change, not as the code does it.
- **Merge policy support**: both `merge: auto` and `merge: manual`.
- **Publishing commits**: `git push origin <branch>` (from a local
  fix branch — `fix/pr-<PR>`, or `agent/developer/fix-pr-<PR>` under /developer:
  `git push origin HEAD:<pr-branch>`).
- **Bodies passed as files** (`gh … -F body=@<file>`, `--body-file`): create
  the file with `mktemp`, never a fixed path like `/tmp/review_body.txt`.
  Concurrent workers share `/tmp`; a fixed name let one overwrite another's
  review body, and the wrong review was posted on a PR.
- **CI**: GitHub Actions, workflow `CI` (`.github/workflows/ci.yml`), on
  every pull request. How to wait for the checks, read the ones recorded for
  a head sha, tell a code-red from an infra-red, and what green does and does
  not verify all live in the annex [`code-host-ci.md`](./code-host-ci.md) —
  **open it only when you are about to do one of those things**, which for
  most jobs is never.

## Read the last reviewed revision

The reviewer, to settle its scope (`review-pr` step 2) — GitHub records the
sha each review was submitted against:

```bash
gh api "repos/{owner}/{repo}/pulls/<PR>/reviews" \
  --jq 'map(select(.state != "PENDING" and (.body // "") != "")) | last | .commit_id // empty'
```

The body filter skips thread replies: GitHub records each one as an
empty-bodied review pinned to the head at reply time, which after a fix push
is the fix commit itself. Empty = never reviewed (full scope). Otherwise the sha anchors the
incremental diff, once `git merge-base --is-ancestor <sha> HEAD` confirms the
branch was not rewritten under it.

## Is the change mergeable?

Read before waiting on anything (the orchestrator, at the top of its checks
gate):

```bash
gh pr view <PR> --json mergeStateStatus --jq .mergeStateStatus
```

`DIRTY` = conflicts with the base: GitHub runs **no checks** against it, so
waiting for one can only time out. It is a conflict, never a red and never an
un-startable CI — take the merge-fix path. `BEHIND` = mergeable but stale
(`gh pr update-branch <PR>`). `CLEAN`/`UNSTABLE`/`BLOCKED` = the checks are
the question — that is the point where the CI annex gets opened, and not
before. The review verdict and the checks are **both** gates, and neither
substitutes for the other: see "What green actually means" in the annex.
