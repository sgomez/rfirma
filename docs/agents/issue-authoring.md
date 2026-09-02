# Issue authoring: GitHub

How child issues must be **created** so the `/developer` pipeline can find and
work them. Annex to [`issue-tracker.md`](./issue-tracker.md), read by whatever
splits a spec into children (`/to-tickets` and the like). **Nothing in the
delivery pipeline reads this file** — by the time an issue is triaged,
implemented or reviewed, the rules below have already been applied or not.

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

### A child that touches both sides MUST be split, and the halves ordered

A child that will touch **both** `rfirma-app/src-tauri/` and `rfirma-app/src/`
is two children: a backend one and a frontend one. Split it, and give the
frontend half a `## Blocked by #<backend child>` section (plus the native
dependency link, which the pipeline also reads).

The reason is arithmetic, not tidiness. A builder's cost grows with the
**square** of its session length: everything it reads stays in context and is
re-sent on every later request, so one session of 150 requests costs about
twice two sessions of 75. Sub-issue #126 touched 23 files across both sides and
ran to 148 requests; the same work as two children would have cost roughly half.

The ordering is what makes the split pay. Run in parallel, the frontend half
has to read `src-tauri/src/commands/` to discover the signatures of commands
that do not exist yet, and both halves explore the same ground. Run after the
backend half lands, it asks `just contract` and gets the whole surface —
commands and crossing types, with the field names TypeScript sees — generated
from the sources it needs.

Two children that each stay on one side of the boundary are also the ones whose
`## Spec extract` is naturally two or three decisions long, which is the size
the section above calls normal.

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
