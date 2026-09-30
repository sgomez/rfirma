#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

tree() {
    local dir="$work/$1"
    mkdir -p "$dir"
    cp -r "$root/.github" "$dir/"
    cp "$root/justfile" "$dir/"
    printf '%s\n' "$dir"
}

passes() {
    local case="$1" dir="$2"
    if ! "$dir/.github/check-workflows.sh" >"$work/out" 2>&1; then
        echo "FALLO ($case): la guarda rechaza un arbol coherente" >&2
        cat "$work/out" >&2
        exit 1
    fi
}

fails_naming() {
    local case="$1" dir="$2" expected="$3"
    if "$dir/.github/check-workflows.sh" >"$work/out" 2>&1; then
        echo "FALLO ($case): la guarda deja pasar el arbol" >&2
        exit 1
    fi
    if ! grep -qF -- "$expected" "$work/out"; then
        echo "FALLO ($case): esperaba '$expected' en" >&2
        cat "$work/out" >&2
        exit 1
    fi
}

append_step() {
    printf '%s\n' "$2" >>"$1"
}

passes "arbol del repositorio" "$(tree clean)"

dir="$(tree secret-in-setup-runner)"
append_step "$dir/.github/actions/setup-runner/action.yml" \
    '    - shell: bash
      run: echo "${{ secrets.GITHUB_TOKEN }}"'
fails_naming "secreto en la accion de preparacion" "$dir" ".github/actions/setup-runner/action.yml:"

dir="$(tree secret-in-nested-action)"
append_step "$dir/.github/actions/load-versions/action.yml" \
    '    - shell: bash
      run: echo "${{ secrets.GITHUB_TOKEN }}"'
fails_naming "secreto en una accion que alcanza la de preparacion" "$dir" ".github/actions/load-versions/action.yml:"

dir="$(tree commented-secret-in-setup-runner)"
append_step "$dir/.github/actions/setup-runner/action.yml" '    # secrets.GITHUB_TOKEN no entra aqui'
passes "secreto nombrado en un comentario de la accion" "$dir"

dir="$(tree secret-in-unused-action)"
mkdir -p "$dir/.github/actions/unused"
printf 'runs:\n  using: composite\n  steps:\n    - shell: bash\n      run: echo "${{ secrets.X }}"\n' \
    >"$dir/.github/actions/unused/action.yml"
passes "secreto en una accion que build.yml no usa" "$dir"

dir="$(tree secret-in-build)"
sed -i 's|^          save-cache: false$|          save-cache: ${{ secrets.X }}|' "$dir/.github/workflows/build.yml"
fails_naming "secreto en build.yml" "$dir" ".github/workflows/build.yml:"

dir="$(tree missing-save-cache)"
awk '!/^          save-cache: false$/ || done++' "$root/.github/workflows/build.yml" \
    >"$dir/.github/workflows/build.yml"
fails_naming "uso de la accion sin save-cache" "$dir" "declara 'save-cache'"

dir="$(tree save-cache-from-ref)"
sed -i "0,/^          save-cache: false$/s||          save-cache: \${{ github.ref == 'refs/heads/main' }}|" \
    "$dir/.github/workflows/build.yml"
fails_naming "save-cache derivado de la ref" "$dir" "no se deriva de la ref"

dir="$(tree rust-cache-saves-always)"
sed -i "s|save-if: \${{ inputs.save-cache == 'true' }}|save-if: true|" \
    "$dir/.github/actions/setup-runner/action.yml"
fails_naming "rust-cache de la accion que guarda siempre" "$dir" ".github/actions/setup-runner/action.yml:"

echo "OK  check-workflows.sh: secretos y caches, tambien a traves de la accion de preparacion"
