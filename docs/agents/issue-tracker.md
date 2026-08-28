# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v`; `gh` does this automatically when run inside a clone.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>` for the diff.
- **List external PRs for triage**: `gh pr list --state open --json number,title,body,labels,author,authorAssociation,comments` then keep only `authorAssociation` of `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`).
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (`gh api` on the sub-issues endpoint). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies**, the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only, the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh issue list --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> --add-assignee @me`, the session's first write.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.


## Delivery operations (/developer pipeline)

The unattended delivery pipeline (`/developer` and its workers) drives this
tracker through the operations below. GitHub is the pipeline's **factory
default**: the delivery skills already carry these `gh` mechanics inline —
this section confirms they apply and adds the sub-issue requirement.

- **Issue ref**: the issue number (`#42` / `42`).
- **Read an issue with comments**: `gh issue view <N> --comments`.
- **Enumerate children of a parent**: the GraphQL sub-issues query (below).
- **Discover a sub-issue's blockers**: check **both** the native dependency
  summary (`gh api repos/{owner}/{repo}/issues/<N> --jq
  '.issue_dependencies_summary.blocked_by // 0'` — the count of *open*
  blockers; 0 or absent = clear) and the `## Blocked by` body section;
  either being non-clear means blocked. Extract that section whole, never a
  fixed window after the heading:

  ```bash
  gh issue view <N> --json body --jq '.body' \
    | awk '/^##[#]* *[Bb]locked by/{f=1;next} /^#/{f=0} f'
  ```
- **Check a blocker's state**: `gh issue view <N> --json state --jq .state`
  (`CLOSED` = no longer blocking).
- **Comment on an issue**: `gh issue comment <N> --body "..."`.
- **Apply a triage label**: `gh issue edit <N> --add-label "<label>"`
  (strings per `docs/agents/triage-labels.md`).
- **Close an issue**: normally never done by hand — `Closes #<N>` in the PR
  body auto-closes the issue on merge (issues and PRs live in the same
  GitHub repo). Close manually (`gh issue close <N> --comment "..."`) only
  when the code host doc says there is no auto-close.

### Parent/child issues MUST be native sub-issues

When a skill breaks a parent issue (a spec/PRD, a plan) into child issues — e.g. `/to-tickets` — each child **must be linked to the parent as a GitHub native sub-issue**, not just referenced in the body text. The `/developer` orchestrator discovers work exclusively through native sub-issue links; a child that is only mentioned in prose is invisible to it.

After creating each child issue, link it:

```bash
# CHILD_ID is the issue *database id*, not the issue number:
CHILD_ID=$(gh api repos/{owner}/{repo}/issues/<CHILD_NUMBER> --jq .id)
gh api repos/{owner}/{repo}/issues/<PARENT_NUMBER>/sub_issues \
  --method POST -F sub_issue_id=$CHILD_ID
```

Keep the `## Parent` and `## Blocked by` sections in the child's body as well — the native link gives machine discovery and the parent's progress panel; the body sections carry the dependency ordering between siblings. Wiring GitHub's native issue dependencies (blocked-by links) in addition is welcome — the pipeline reads them too — but the body sections remain required as the portable fallback.

### Every child issue MUST carry a `## Spec extract` section

A child issue is read by a builder with a **clean context**: the sub-issue is
all it gets for free. If the decisions it must honour live only in the parent
spec, every builder re-reads that whole spec — a spec with ten children pays
for its own body ten times, competing with the code exploration the builder
cannot cut.

So `/to-tickets` (or whatever splits a spec) **must** give each child a
`## Spec extract` section holding the parent's **Implementation Decisions** and
**Testing Decisions that apply to this child**, copied **verbatim** — not
summarised, not rewritten. Two or three of them is the normal size; a child
that seems to need all of them is a sign the split is wrong.

```markdown
## Spec extract

Implementation Decisions (from #<PARENT>):
- <decision, verbatim>
- <decision, verbatim>

Testing Decisions (from #<PARENT>):
- <decision, verbatim>
```

The bar is the same one that makes any agent brief work: durable and
behavioural, with verifiable criteria, and no file paths that go stale. A
child with this section is **self-sufficient** — the pipeline reads the parent
spec only as a fallback, when the section is missing.

To list a parent's sub-issues:

```bash
gh api graphql -f query='
{
  repository(owner:"{owner}", name:"{repo}") {
    issue(number: <PARENT_NUMBER>) {
      subIssues(first: 50) {
        pageInfo { hasNextPage }
        nodes { number title state labels(first: 10) { nodes { name } } }
      }
    }
  }
}' --jq '.data.repository.issue.subIssues'
```

`labels` feeds the pipeline's escalation gate (`ready-for-human` sub-issues are
skipped). `hasNextPage: true` means the parent has outgrown the pipeline —
stop and ask the user to split it rather than work from a truncated list.
