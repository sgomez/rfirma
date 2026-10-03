#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
platform_files="$root/scripts/platform-files.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

tauri="$work/rfirma-app/src-tauri"
mkdir -p "$tauri/src/adapters/store" "$tauri/src/domain" "$tauri/tests/support" "$tauri/target/debug"

cat >"$tauri/build.rs" <<'EOF'
fn main() {
    if cfg!(windows) {}
}
EOF
cat >"$tauri/src/lib.rs" <<'EOF'
pub mod adapters;
pub mod domain;
EOF
cat >"$tauri/src/adapters/mod.rs" <<'EOF'
#[cfg(windows)]
#[allow(dead_code)]
pub mod store;
#[cfg(target_os = "linux")] mod gtk;
#[cfg(test)]
mod tests;
pub mod shared;
EOF
cat >"$tauri/src/adapters/lock.rs" <<'EOF'
#[cfg(all(test, windows))]
#[path = "lock/windows_tests.rs"]
mod windows_tests;
#[cfg(unix)]
mod inline {
    pub fn x() {}
}
EOF
cat >"$tauri/src/adapters/store/cng.rs" <<'EOF'
pub fn sign() {}
EOF
cat >"$tauri/src/domain/name.rs" <<'EOF'
#[cfg(feature = "windows-like")]
pub fn feature_is_not_a_platform() {}
EOF
cat >"$tauri/src/domain/family.rs" <<'EOF'
#[cfg_attr(target_family = "unix", allow(unused))]
pub fn family() {}
EOF
cat >"$tauri/tests/pin_fd.rs" <<'EOF'
#[cfg(unix)]
mod support;
EOF
cat >"$tauri/target/debug/generated.rs" <<'EOF'
#[cfg(windows)]
mod generated;
EOF

expected="rfirma-app/src-tauri/build.rs
rfirma-app/src-tauri/src/adapters/gtk.rs
rfirma-app/src-tauri/src/adapters/gtk/
rfirma-app/src-tauri/src/adapters/lock.rs
rfirma-app/src-tauri/src/adapters/lock/windows_tests.rs
rfirma-app/src-tauri/src/adapters/mod.rs
rfirma-app/src-tauri/src/adapters/store.rs
rfirma-app/src-tauri/src/adapters/store/
rfirma-app/src-tauri/src/domain/family.rs
rfirma-app/src-tauri/tests/pin_fd.rs
rfirma-app/src-tauri/tests/support.rs
rfirma-app/src-tauri/tests/support/"

actual="$(cd "$work" && "$platform_files")"
if [ "$actual" != "$expected" ]; then
    echo "FALLO (arbol de prueba):" >&2
    diff <(printf '%s\n' "$expected") <(printf '%s\n' "$actual") >&2 || true
    exit 1
fi

real="$(cd "$root" && "$platform_files")"
for needle in \
    rfirma-app/src-tauri/src/identity/adapters/windows_store/ \
    rfirma-app/src-tauri/src/site/adapters/channel/acceptor.rs; do
    grep -qxF "$needle" <<<"$real" || {
        echo "FALLO (repositorio): falta $needle" >&2
        exit 1
    }
done
if grep -qxF rfirma-app/src-tauri/src/lib.rs <<<"$real"; then
    echo "FALLO (repositorio): lib.rs no sabe de plataformas" >&2
    exit 1
fi

echo "platform_files_test: correcto"
