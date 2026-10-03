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

headed="$("$outline" "scripts/tests/fixtures/headed.ts")"
assert_contains "$headed" "    1  //! Un modulo con cabecera y bloque." "ts: la cabecera sale sola"
assert_contains "$headed" "    3  // Prosa del bloque que no entra en la cabecera." "ts: el bloque de debajo sigue"
assert_contains "$headed" "    7  export function headed" "ts: export tras la cabecera"

if "$outline" "$fixtures/sample.txt" >/dev/null 2>&1; then
    echo "FALLO: una extension no soportada deberia salir con 1" >&2
    exit 1
fi

single="$("$outline" "scripts/tests/fixtures/sample.rs")"
assert_contains "$single" "-- scripts/tests/fixtures/sample.rs:" "single: pie con la ruta pedida"
if [[ "$single" == *"== "* ]]; then
    echo "FALLO (single): un solo fichero no lleva cabecera" >&2
    exit 1
fi

two="$("$outline" scripts/tests/fixtures/sample.rs scripts/tests/fixtures/sample.tsx)"
assert_contains "$two" "== scripts/tests/fixtures/sample.rs ==" "two files: cabecera rs"
assert_contains "$two" "== scripts/tests/fixtures/sample.tsx ==" "two files: cabecera tsx"
assert_contains "$two" "    4  pub fn join_paths" "two files: esqueleto rs"
assert_contains "$two" "    2  export function Greeting" "two files: esqueleto tsx"

ranges="$("$outline" scripts/tests/fixtures/sample.rs:4-5,15-16)"
assert_contains "$ranges" "== scripts/tests/fixtures/sample.rs:4-5,15-16 ==" "ranges: cabecera"
assert_contains "$ranges" "    4  pub fn join_paths" "ranges: primer tramo"
assert_contains "$ranges" "   ..." "ranges: separador"
assert_contains "$ranges" "   15  fn joins_two_paths" "ranges: segundo tramo"

mixed="$("$outline" scripts/tests/fixtures/sample.rs scripts/tests/fixtures/sample.tsx:1-3)"
assert_contains "$mixed" "    9  pub struct PathPair" "mixed: esqueleto"
assert_contains "$mixed" "    2  export function Greeting" "mixed: tramo"

txt="$("$outline" scripts/tests/fixtures/sample.txt:1-1)"
assert_contains "$txt" "    1  " "ranges on non-skeleton file"

set +e
partial="$("$outline" scripts/tests/fixtures/nope.rs scripts/tests/fixtures/sample.rs scripts/tests/fixtures/sample.rs:x-y 2>/dev/null)"
code=$?
set -e
assert_contains "$partial" "    4  pub fn join_paths" "invalid among valid: los validos salen"
if [ "$code" -ne 1 ]; then
    echo "FALLO: un argumento invalido deberia dar codigo 1, dio $code" >&2
    exit 1
fi

index="$("$outline" scripts/tests/fixtures/index/)"
assert_contains "$index" "documented.rs  Un modulo con cabecera partida en dos lineas." "index: joins the first header paragraph"
assert_contains "$index" "nested/inner.rs  Un modulo anidado." "index: recurses with paths relative to the directory"
assert_contains "$index" "bare.rs  !! SIN CABECERA //!" "index: flags a module without header"
if [[ "$index" == *"tests.rs"* || "$index" == *"Prosa"* ]]; then
    echo "FALLO (index: skips test modules and prose): $index" >&2
    exit 1
fi
if [[ "$index" == *"== "* ]]; then
    echo "FALLO (index: a lone directory has no label)" >&2
    exit 1
fi
assert_contains "$index" "widget.tsx  Un componente con cabecera." "index: reads the header of a tsx"
assert_contains "$index" "plain.ts  !! SIN CABECERA //!" "index: flags a ts without header"
if [[ "$index" == *"widget.test.tsx"* ]]; then
    echo "FALLO (index: skips the tests of the window): $index" >&2
    exit 1
fi
first="$(printf '%s\n' "$index" | head -1)"
if [ "$first" != "bare.rs  !! SIN CABECERA //!" ]; then
    echo "FALLO (index: stable order), primera linea: $first" >&2
    exit 1
fi

mixed_index="$("$outline" scripts/tests/fixtures/index/ scripts/tests/fixtures/sample.rs:4-4)"
assert_contains "$mixed_index" "== scripts/tests/fixtures/index/ ==" "index mixed: label"
assert_contains "$mixed_index" "nested/inner.rs  Un modulo anidado." "index mixed: index"
assert_contains "$mixed_index" "    4  pub fn join_paths" "index mixed: range"

set +e
"$outline" scripts/tests/fixtures/index/text_only/ >/dev/null 2>&1
code=$?
set -e
if [ "$code" -ne 1 ]; then
    echo "FALLO (index without rs): deberia dar codigo 1, dio $code" >&2
    exit 1
fi

echo "outline_test: correcto"
