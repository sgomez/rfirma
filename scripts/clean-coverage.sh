#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cargo_target="$1"
tauri="$2"
coverage_out="$cargo_target/coverage/$(basename "$root")"

instrumented="$cargo_target/llvm-cov-target"
echo "instrumentado, recuperable: $(du -sh "$instrumented" 2>/dev/null | cut -f1 || echo 0)"
rm -rf "$instrumented" "$coverage_out"
rm -f "$cargo_target"/*.profraw "$tauri"/*.profraw
echo "queda en $cargo_target: $(du -sh "$cargo_target" 2>/dev/null | cut -f1)"
