#!/usr/bin/env bash
# Las guardas del backend que solo leen el árbol como texto, compiladas con rustc sueltas, sin la crate: las mismas pruebas que corre `cargo test`.
set -euo pipefail

guards=(
    files_stay_small
    modules_open_with_a_header
    adr_citations_resolve
    comments_cite_nothing_that_rots
    no_inline_test_modules
    module_directions
    single_cfg_os_site
)

PATH="$HOME/.cargo/bin:$PATH"
root=$(git rev-parse --show-toplevel)
# Un hook de git exporta GIT_DIR, y el git ls-files de cada guarda se resolvería desde ahí y no desde su directorio.
unset $(git rev-parse --local-env-vars)
tauri="$root/rfirma-app/src-tauri"
edition=$(sed -n 's/^edition = "\(.*\)"$/\1/p' "$tauri/Cargo.toml" | head -n 1)
suffix=""
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) suffix=".exe" ;; esac

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

pids=()
for guard in "${guards[@]}"; do
    CARGO_MANIFEST_DIR="$tauri" rustc --edition "$edition" --test -A warnings -C debuginfo=0 \
        -o "$out/$guard$suffix" "$tauri/tests/$guard.rs" &
    pids+=("$!")
done
failed=0
for pid in "${pids[@]}"; do
    wait "$pid" || failed=1
done
if [ "$failed" -ne 0 ]; then
    echo "✗ una guarda no compila con rustc suelto: solo puede usar std (scripts/structural-guards.sh)"
    exit 1
fi

for guard in "${guards[@]}"; do
    if ! (cd "$tauri" && "$out/$guard$suffix" -q >"$out/$guard.log" 2>&1); then
        echo "✗ $guard (cargo test --test $guard)"
        sed -n '/^---- /,/^failures:$/p' "$out/$guard.log"
        failed=1
    fi
done
exit "$failed"
