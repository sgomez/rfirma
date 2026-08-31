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

**Still narrow, but no longer only Java.** As of
[issue #47](https://github.com/sgomez/rfirma/issues/47) the fast lane runs
`just check` across all three toolchains. It verifies:

- the Java bridge **compiles** under GraalVM CE 25 with `-Xlint:all`;
- AutoFirma's dependencies **resolve and build** on a clean runner
  (`bootstrap.sh` against the immutable upstream tag `v1.9.1`);
- **Biome** passes on `rfirma-app/` (`biome ci`), and `tsc -b` typechecks
  before `vite build`;
- **clippy with `-D warnings`** and `cargo fmt --check` pass on
  `rfirma-app/src-tauri/`, and `cargo build --release` links the app;
- the **tier A** tests run: `vitest` on the frontend, `cargo test` on Rust —
  which today is the FNMT kit guard (fingerprints of the three `.p12`, and
  `activo-rsa.p12` still being in date);
- the **tier C** tests still **compile** (`cargo test --no-run`), so a test
  that stops building against the FFI cannot be skipped in silence;
- the **CRAP gate**: `cargo crap --threshold 30 --fail-above`, at a version
  pinned in the `justfile`, with `--allow` over the FFI module path;
- on the slow lane only: that `native-image --shared` still **produces the
  shared library**, that the tier C tests **pass** (`--include-ignored`), and
  the same CRAP measurement **without** the FFI exclusion.

It still does **not** verify that a signature is valid, that a PDF opens, or
that a certificate chains: that is tier C and tier D work, and it lands with
the sub-issues that write the signing code. `pdfsig` as the automatic validity
oracle and the official validator as a manual release gate are decided
([ADR-0014](../adr/0014-gradas-de-prueba-y-puerta-de-calidad.md)) but not yet
built.

**The fast lane does not build the native library, deliberately**, so it sets
`RFIRMA_SIN_NATIVA=1` to skip the guard `just build` performs by
[ADR-0013](../adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md).
Locally, without that variable, `just build` and `just dev` **fail naming
`just native`** rather than chaining a three-minute `native-image` run onto
every compile. Do not copy that variable into a local shell profile: it is the
CI's exception, not a default.

**So the reviewer still installs and runs everything itself** — a green check
is not a substitute. What is green now is *the toolchains and the scaffolding*,
not the product.

### Two lanes, split by speed

This is first of all an **agent's** feedback loop, so what runs every time has
to be fast.

| Lane | Job | When |
| --- | --- | --- |
| fast | `Compila y resuelve dependencias` | every PR, every push to `main` |
| slow | `Imagen nativa` (`native-image` itself is 1 m 22 s) | tags `v*`, manual dispatch, weekly cron, or a PR labelled `native` |
| cron | `Caducidad del kit FNMT` | weekly cron and manual dispatch only |

The fast lane was **~48 s** when it was Java alone (measured under #11). #47
added the Node and Rust toolchains, their system dependencies
(`libwebkit2gtk-4.1-dev` and friends), and two `cargo binstall`ed binaries, and
measured **~9 min cold, ~3 min warm**. Pretending it is still 48 s would be a
lie, so: it is not. **~3 min is the number to hold**, because that is what a
second PR on the same branch actually costs.

Almost all of it is Rust, and almost all of *that* is compiling the Tauri
dependency tree **twice** — once for `cargo build --release`, once
instrumented for `cargo llvm-cov`. The six-minute gap between cold and warm is
what the caching buys: `~/.m2`, the pnpm store, `Swatinem/rust-cache`, and
prebuilt binaries instead of `cargo install`. **If it creeps past what an agent
will wait for, the thing to cut is the coverage build, not the caching** — the
CRAP gate is the one piece of `just check` that pays for a whole second
compile.

`native-image` fits comfortably on a standard runner — that question is
settled — but the Java bridge will barely be touched once written, so
rebuilding the image on every PR would cost several times the fast lane to
learn nothing new. **If your PR touches the bridge, add the `native` label.**

The weekly cron does triple duty: it keeps the `~/.m2` cache from expiring
(GitHub evicts after 7 days unused, and refilling it means compiling all of
AutoFirma), it is the safety net for the slow lane, and it is the **watchman
for the FNMT test kit** — 90 days before `testdata/fnmt/activo-rsa.p12`
expires on **2028-10-30** it opens an issue. That warning lives in the cron and
not in the fast lane on purpose: warning there would break every open PR at
once, on an arbitrary day in 2028, with auto-merge on. The *hard* half of the
guard is in the fast lane, in `rfirma-app/src-tauri/tests/kit_fnmt.rs`, and it
only fires once the certificate has actually expired.

### Running the same thing locally

One entry point, `just`:

```bash
apt-get install -y just maven libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
                   libayatana-appindicator3-dev libsoup-3.0-dev libxdo-dev
cargo binstall cargo-llvm-cov cargo-crap
just check
```

`just tools` names whatever is still missing, and `just --list` shows the rest.
CI runs exactly `just check`, so a local pass and a CI pass mean the same thing
— with the one documented exception of `RFIRMA_SIN_NATIVA` above: locally you
need `just native` once, and then `just check` covers strictly more than CI's
fast lane does.

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
