# Code host CI: GitHub

Annex to [`code-host.md`](./code-host.md). **Open it only when you are about
to wait for, read or classify a change's CI** — publishing a change, checking
it out, reviewing the diff and merging need nothing from here.

Three operations read the same `CI` workflow, for different readers.

- **Wait for the checks and gate the merge** (the orchestrator, before
  merging):

  ```bash
  gh pr checks <PR> --watch --fail-fast   # exits non-zero if any check fails
  ```

  A non-zero exit is **not** a merge conflict: it is a red build, and the
  answer is another fix cycle, never a merge-fix job.

- **Read the checks already recorded for the head sha** (the reviewer, before
  deciding whether to run the suite locally):

  ```bash
  gh pr checks <PR> --json name,state,link --jq \
    '[.[] | select(.state != "SUCCESS" and .state != "SKIPPED")]'
  ```

  Empty output with at least one check present = green. Any entry is a
  failing or still-running check; its `link` is the job URL to quote. Green
  here does **not** license skipping the local run in this repo — read "What
  green actually means" below before deciding.

- **Classify a red — did the failing job actually execute?** (any reader,
  before spending a fix cycle on it): take `<run-id>` from the failing
  check's `link` (`…/actions/runs/<run-id>/job/<job-id>`), then

  ```bash
  gh run view <run-id> --json conclusion,jobs --jq '{run: .conclusion,
    failed: [.jobs[] | select(.conclusion != "success" and .conclusion != "skipped")
    | {name, steps: (.steps | length)}]}'
  ```

  A failed job with `steps > 0` ran against the change: **code-red** — a
  fix cycle, and the failing job's URL is what a fixer needs (the raw log
  stays out of the orchestrator's context). Every failed job at `steps: 0`,
  a run conclusion of `startup_failure`, or a job no runner ever picked up:
  **infra-red** — the job never started (runner offline, Actions minutes
  exhausted) and the red says nothing about the code.

### What green actually means

**Narrow.** The fast lane runs `just check` across all three toolchains —
split into one job per chain (`Cadena Java`, `Cadena TypeScript`, `Cadena
Rust`), which together are exactly `just check` minus `just tools`. It
verifies:

- the Java bridge **compiles** under GraalVM CE 25 with `-Xlint:all`;
- AutoFirma's dependencies **resolve and build** on a clean runner
  (`bootstrap.sh` against the immutable upstream tag `v1.9.1`);
- **Biome** passes on `rfirma-app/` (`biome ci`), and `tsc -b` typechecks
  before `vite build`;
- **clippy with `-D warnings`** and `cargo fmt --check` pass on
  `rfirma-app/src-tauri/`, over `--all-targets --all-features`. It does **not**
  verify that `cargo build --release` links: that moved to the slow lane, next
  to the packaging that consumes it;
- the **tier A** tests run: `vitest` on the frontend, `cargo test` on Rust —
  which today is the FNMT kit guard (fingerprints of the three `.p12`, and
  `active-rsa.p12` still being in date) — and `mvn test` on the Java bridge:
  the CAdES signed attributes of the presign parsed as ASN.1 DER, the session
  stamp of [ADR-0016](../adr/0016-sello-de-sesion-una-sola-invariante.md), and
  the bridge's own contract (the three `autofirma_*` entry points,
  `afirma-ui-utils` absent from the classpath, the versioned `native-image`
  metadata);
- the **tier C** tests still **compile** — `cargo test --no-run` on Rust, and
  `mvn test` compiles the `@Tag("gradaC")` classes it does not run — so a test
  that stops building against the FFI cannot be skipped in silence;
- the **CRAP gate**: `cargo crap --threshold 30 --fail-above`, at a version
  pinned in the `justfile`, with `--allow` over the FFI module path;
- on the slow lane only: that `native-image --shared` still **produces the
  shared library**, that the tier C tests **pass** (`--include-ignored` on
  Rust, `-DexcludedGroups=` on Maven), and the same CRAP measurement
  **without** the FFI exclusion.

The fast lane does **not** verify that a signature is valid or that a PDF
opens; **the slow lane does**: `just test-native` signs a PDF end to end
through the Java bridge and `pdfsig` validates it, which is the automatic
oracle [ADR-0014](../adr/0014-gradas-de-prueba-y-puerta-de-calidad.md)
decided. That Java-side run does not touch PKCS#11: phase 2 is the JCE with the
FNMT test key, so what it proves is the **contract** — a PKCS#1 over the DER
bytes of the presign — not the card path.

The same lane also runs **the real card path**:
`rfirma-app/src-tauri/tests/native_cycle.rs` drives the
whole triphase cycle with phase 2 on the **SoftHSM token**, over the four
visible-signature cases (no text and no rubric, text only, rubric only, both)
plus a cosignature, with `pdfsig` on every PDF it produces. The rubric is
checked by **rasterising** the page with `pdftoppm` and counting dark pixels
inside the box — `pdftotext` cannot see an image and gives a false negative
(TD-03). The three session-seal invariants of
[ADR-0016](../adr/0016-sello-de-sesion-una-sola-invariante.md) — signing
instant, time zone, effective `extraParams` — are covered as **expected
failures**: each alters exactly that field of the real seal and asserts the
postsign aborts, because the bug it guards against is a postsign that completes
without error and leaves a `Digest Mismatch` PDF nobody can explain.

Nothing anywhere verifies that a certificate **chains**: without the FNMT CA in
the store `pdfsig` will always report the signature as not verified against a
trust root, so the tests deliberately assert cryptographic validity and nothing
more.

### The manual gate, and how to reach it

The official validator (VALIDe) stays a **manual release gate** — network, web,
no stable API (TD-04) — run by a person once per `v*` tag. A green check does
**not** demonstrate what the milestone's done-criterion promises.

The PDF that goes to it is `manual-gate.pdf`: the maximal case, a box with
**both** text and rubric, produced by
`full_cycle::a_signature_with_text_and_rubric_is_the_pdf_of_the_manual_gate`.
It lands in the test's `CARGO_TARGET_TMPDIR`
(`rfirma-app/src-tauri/target/tmp/manual-gate.pdf` today), the test prints its
absolute path, and the slow lane uploads it as the workflow artifact
**`pdf-puerta-manual`**. So closing the gate is: run the slow lane (tag, manual
dispatch, or a PR labelled `native`), download that artifact, upload it to
VALIDe. If the maximal case validates, the other three are subsets of it.

**The fast lane does not build the native library, deliberately.** It no longer
needs `RFIRMA_SKIP_NATIVE=1` for that — `check-rust` does not go through `just
build`, so there is no guard to skip. The guard itself still stands where
[ADR-0013](../adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md)
put it: locally `just build` and `just dev` **fail naming `just native`**
rather than chaining a three-minute `native-image` run onto every compile, and
`RFIRMA_SKIP_NATIVE=1` is how you say you know what you are doing. Do not copy
that variable into a local shell profile.

**So the reviewer still installs and runs everything itself** — a green check
is not a substitute for it.

### Two lanes, split by speed

This is first of all an **agent's** feedback loop, so what runs every time has
to be fast.

| Lane | Job | When |
| --- | --- | --- |
| fast | `Cadena Java`, `Cadena TypeScript`, `Cadena Rust` (parallel) | every PR, every push to `main` |
| slow | `Imagen nativa` (`native-image` itself is 1 m 22 s) | tags `v*`, manual dispatch, weekly cron, or a PR labelled `native` |
| cron | `Caducidad del kit FNMT` | weekly cron and manual dispatch only |

The fast lane costs **~2 min warm**, and that number is the **Rust** job: the
other two finish inside it and are free in wall-clock terms. Java and
TypeScript do not wait for Rust and Rust does not wait for them, so what a
second PR on the same branch costs is the slowest single chain, not the sum.

Almost all of the Rust job is compiling the Tauri dependency tree, and each
distinct flag set gets its own metadata hash and reuses nothing from the
others. So the count of those trees *is* the cost, and the fast lane is down to
**two** — `cargo clippy --all-targets --all-features` and the `cargo llvm-cov`
instrumented build. It used to be four: `cargo build --release` moved to the
slow lane (nothing in the fast lane ran that binary), and the bare `cargo test`
went away because `cargo llvm-cov` **runs the suite itself** and propagates its
exit code, so keeping both meant running every test twice in two trees.

**If it creeps back up, the thing to cut is a compile, not the caching** — and
do not reach for the coverage build, which is now the only thing running the
Rust tests at all. What the caching buys (`~/.m2`, the pnpm store,
`Swatinem/rust-cache`, prebuilt binaries instead of `cargo install`) is the
gap between a cold run and that warm number.

**One tradeoff was taken on purpose:** in the fast lane the Rust tests only
ever run *instrumented*, under `llvm-cov`. The uninstrumented run still
happens, in the slow lane's `just test-native`, on every push to `main` and
every weekly cron — so a failure that only shows up without instrumentation is
caught at merge, not at PR.

`native-image` fits comfortably on a standard runner, but the Java bridge is
barely touched once written, so rebuilding the image on every PR would cost
several times the fast lane to learn nothing new. **If your PR touches the
bridge or the signing path, add the `native` label.** This is not a
formality: without the label the slow lane **reports green having never run**,
so a bug at the FFI boundary merges unseen and nothing short of the next
tagged release or the weekly cron catches it.

The weekly cron does triple duty: it keeps the `~/.m2` cache from expiring
(GitHub evicts after 7 days unused, and refilling it means compiling all of
AutoFirma), it is the safety net for the slow lane, and it is the **watchman
for the FNMT test kit** — 90 days before `testdata/fnmt/active-rsa.p12`
expires on **2028-10-30** it opens an issue. That warning lives in the cron and
not in the fast lane on purpose: warning there would break every open PR at
once, on an arbitrary day in 2028, with auto-merge on. The *hard* half of the
guard is in the fast lane, in `rfirma-app/src-tauri/tests/fnmt_kit.rs`, and it
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
The fast lane's three jobs are `just check-java`, `just check-repo check-ts`
and `just check-rust` — together exactly what `just check` runs, minus the
`tools` probe that each runner's setup actions already guarantee. So a local
pass and a CI pass mean the same thing.

`just check` **no longer needs the native library at all**: `check-rust`
dropped the `build` chain, so `RFIRMA_SKIP_NATIVE` is not needed to run it and
CI no longer sets it. The variable still exists for `just build` and `just
dev`, which do check for the library (ADR-0013).

It also no longer **deletes** it. `lint-java` used to run `mvn -B clean
compile`, and that `clean` took `rfirma-native-bridge/target/` with it —
including `target/lib/rfirma/librfirma_crypto.so`, where `just native` installs
it — so `just check` followed by `just test-native` failed pointing at a file
that existed when the recipe started. The `clean` is gone (`-Xlint:all` is not
`-Werror`, so nothing was gating on the full recompile), and `check-java` runs
a single `mvn -B verify`. Any order works now.

If the PR does not touch Java, you can still skip the three-minute rebuild
entirely: point `RFIRMA_LIB_DIR` at an already-built `librfirma_crypto.so` (or
copy it into the worktree's `target/lib/rfirma`) and the tier C tests run
against that.

