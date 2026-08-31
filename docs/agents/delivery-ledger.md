# Delivery ledger

## Local calibration

- #47 triaged `oversized` (4 bundled deliverables: app tree, justfile grid, quality tooling, two-lane CI + testdata kit) but, built at opus under `--build-oversized`, converged in ONE fix cycle to CLEAN. Mechanism: the dispatcher scored breadth — four independent deliverables with no existing pattern to imitate, because `rfirma-app/` did not exist — not depth. Foundational scaffolding tickets in this repo are wide but shallow: each deliverable is boilerplate wiring with a well-specified target, and the fault lines the dispatcher produced were themselves a sufficient order of work. Signal: a sub-issue whose fault lines are all "create <thing> from scratch per <ADR>" rather than "change interacting behaviour X" — score `complex` (opus), not `oversized`.
- The review caught two things the narrow CI structurally cannot: `just native` omitting `-H:IncludeResources` for the iText fonts (runtime-only failure, "Courier not found as resource") and the slow CI lane reporting green without ever running because the PR lacked the `native` label. Both are consistent with `docs/agents/code-host.md`'s own warning that a green check here means almost nothing; worth checking whether the `native`-label requirement deserves to be louder in the agent docs.

## Run log

2026-08-31 spec=#46 sub=#47 model=opus effort=medium pr=#64 verdict=CLEAN cycles=1 mergefix=0 wave=— outcome=ready-to-merge
