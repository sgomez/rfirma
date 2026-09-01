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

## El guardián del worktree

Entubar la salida de `gh` a `head` (u otro consumidor que corte pronto)
devuelve **vacío con código 0** dentro del guardián del worktree: la salida
se pierde en vez de fallar de forma visible, así que no lo interpretes como
«no hay datos». Lo mismo le pasa a `gh issue view N --comments` **sin
entubar nada**. Redirige siempre a un fichero y lee el fichero
(`gh issue view N --json body,comments > /tmp/x.json`, luego léelo entero o
con un script). Lo mismo aplica a una consulta GraphQL larga pasada inline:
escríbela en un fichero y pásala con `gh api graphql -F query=@fichero`.

El guardián también rechaza, con un error explícito y no con salida vacía,
dos formas de comando que aparecen constantemente al automatizar `gh`:
un `cmd_a && cmd_b` encadenado («demasiado complejo para verificar que se
queda dentro del worktree» — sepáralos en llamadas independientes) y un
`python3 - <<'PY' ... PY` con varias rutas dentro (escribe el script en el
directorio de scratchpad y ejecútalo por su ruta, en vez de pasarlo inline).

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

The fast lane still does **not** verify that a signature is valid or that a
PDF opens: **the slow lane now does**, since
[#48](https://github.com/sgomez/rfirma/issues/48) — `just test-native` signs a
PDF end to end through the Java bridge and `pdfsig` validates it, which is the
automatic oracle [ADR-0014](../adr/0014-gradas-de-prueba-y-puerta-de-calidad.md)
decided. That Java-side run does not touch PKCS#11: phase 2 is the JCE with the
FNMT test key, so what it proves is the **contract** — a PKCS#1 over the DER
bytes of the presign — not the card path.

Since [#61](https://github.com/sgomez/rfirma/issues/61) the same lane also runs
**the real card path**: `rfirma-app/src-tauri/tests/native_cycle.rs` drives the
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

**The fast lane does not build the native library, deliberately**, so it sets
`RFIRMA_SKIP_NATIVE=1` to skip the guard `just build` performs by
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
measured **~9 min cold, 3-4 min warm** (2 m 59 s on one warm run, 4 m 11 s on
another). Pretending it is still 48 s would be a lie, so: it is not. **3-4 min
is the number to hold**, because that is what a second PR on the same branch
actually costs.

Almost all of it is Rust, and almost all of *that* is compiling the Tauri
dependency tree **four times** — `cargo clippy --all-targets`, `cargo build
--release`, `cargo test`, and the `cargo llvm-cov` instrumented build each get
their own metadata hash, so none of them reuses another's artifacts. The
five-to-six-minute gap between cold and warm is what the caching buys: `~/.m2`,
the pnpm store, `Swatinem/rust-cache`, and prebuilt binaries instead of
`cargo install`. **If it creeps past what an agent
will wait for, the thing to cut is the coverage build, not the caching** — the
CRAP gate is the one piece of `just check` that pays for a whole extra
compile.

`native-image` fits comfortably on a standard runner — that question is
settled — but the Java bridge will barely be touched once written, so
rebuilding the image on every PR would cost several times the fast lane to
learn nothing new. **If your PR touches the bridge, add the `native` label.**
This is not a formality: on #73 the fast lane was green and the review
verdict was CLEAN, and only the slow lane — triggered by adding the label —
caught a real bug at the FFI boundary, twice, before the fix stuck. Without
the label that PR would have merged broken and nothing short of the next
tagged release or the weekly cron would have caught it.

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
CI runs exactly `just check`, so a local pass and a CI pass mean the same thing
— with the one documented exception of `RFIRMA_SKIP_NATIVE` above: locally you
need `just native` once, and then `just check` covers strictly more than CI's
fast lane does.

**`just check` deletes the native library it needs.** Its `lint-java` recipe
runs `mvn -B clean compile`, and `clean` takes
`rfirma-native-bridge/target/` with it — including
`target/lib/rfirma/librfirma_crypto.so`, which is where `just native`
installs it. So `just check` followed by `just test-native` or `just
crap-full` fails saying the native library is missing, pointing at a file
that existed when the recipe started. It is not that `just native` was
skipped; `just check` just erased it mid-run. Order that works locally:
`RFIRMA_SKIP_NATIVE=1 just check` (the CI exception, on purpose) and then
`just native` once more before the slow-lane targets — never `just native`
first, `just check` second. If the PR does not touch Java, skip the
three-minute rebuild entirely: point `RFIRMA_LIB_DIR` at an
already-built `librfirma_crypto.so` (or copy it into the worktree's
`target/lib/rfirma`) and the tier C tests run against that.

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
