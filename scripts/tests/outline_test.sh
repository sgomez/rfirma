#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
fixtures="$root/scripts/tests/fixtures"
outline="$root/scripts/outline.sh"

assert_contains() {
    local haystack="$1" needle="$2" case="$3"
    if [[ "$haystack" != *"$needle"* ]]; then
        echo "FALLO ($case): se esperaba encontrar:" >&2
        echo "  $needle" >&2
        exit 1
    fi
}

rs_output="$("$outline" "scripts/tests/fixtures/sample.rs")"
assert_contains "$rs_output" "    4  pub fn join_paths" "rust: pub fn"
assert_contains "$rs_output" "    9  pub struct PathPair" "rust: struct"
assert_contains "$rs_output" "   15  fn joins_two_paths" "rust: #[test] fn"

tsx_output="$("$outline" "scripts/tests/fixtures/sample.tsx")"
assert_contains "$tsx_output" "    2  export function Greeting" "tsx: export function"
assert_contains "$tsx_output" "    6  it('renders the greeting'" "tsx: it(...)"

if "$outline" "$fixtures/sample.txt" >/dev/null 2>&1; then
    echo "FALLO: una extension no soportada deberia salir con 1" >&2
    exit 1
fi

echo "outline_test: correcto"
