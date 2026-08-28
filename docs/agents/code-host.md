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
- **CI**: none. This repo has no CI on pull requests, so the pipeline skips
  every CI operation: the orchestrator does not wait on checks before
  merging, and the reviewer always installs and runs the test suite itself
  rather than trusting a green report. If GitHub Actions is added later,
  change this bullet and restore the CI section from the
  `/setup-developer-skills` template (`code-host-github.md`).

## Is the change mergeable?

Read before merging (the orchestrator, at the top of its checks gate):

```bash
gh pr view <PR> --json mergeStateStatus --jq .mergeStateStatus
```

`DIRTY` = conflicts with the base — take the merge-fix path. `BEHIND` =
mergeable but stale (`gh pr update-branch <PR>`). `CLEAN`/`UNSTABLE`/
`BLOCKED` = no conflict; with `CI: none` there are no checks to weigh, so
the review verdict is the only gate.
