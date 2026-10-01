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

breaks() {
    local case="$1" expected="$2" file="$3" expr="$4" dir
    dir="$(tree "$case")"
    sed -i -E "$expr" "$dir/$file"
    fails_naming "$case" "$dir" "$expected"
}

breaks unpinned-action "sin fijar por SHA" .github/workflows/ci.yml \
    '0,/uses: actions\/checkout@[0-9a-f]{40}/s//uses: actions\/checkout@v4/'

dir="$(tree secret-to-setup-runner)"
sed -i -E '0,/^      - uses: .\/.github\/actions\/setup-runner$/s||&\n        with:\n          save-cache: ${{ secrets.X }}|' \
    "$dir/.github/workflows/ci.yml"
fails_naming "secreto pasado a la accion de preparacion por un llamador" "$dir" ".github/workflows/ci.yml:"

dir="$(tree secret-to-build)"
sed -i -E 's|^(    uses: ./.github/workflows/build.yml)$|\1\n    secrets:\n      X: ${{ secrets.X }}|' \
    "$dir/.github/workflows/preview.yml"
fails_naming "secreto pasado a build.yml por un llamador" "$dir" ".github/workflows/preview.yml:"

dir="$(tree secret-before-uses)"
sed -i -E '0,/^      - uses: .\/.github\/actions\/setup-runner$/s||      - name: Prepara\n        env:\n          X: ${{ secrets.X }}\n        uses: ./.github/actions/setup-runner|' \
    "$dir/.github/workflows/ci.yml"
fails_naming "secreto en el paso antes del uses" "$dir" ".github/workflows/ci.yml:"

dir="$(tree secret-in-job-env)"
sed -i -E "/^  java:$/,/^    steps:$/s|^    steps:$|    env:\n      X: \${{ secrets.X }}\n    steps:|" "$dir/.github/workflows/ci.yml"
fails_naming "secreto en el env del job que llama a la accion" "$dir" ".github/workflows/ci.yml:"

dir="$(tree secret-after-uses-comment)"
sed -i -E '0,/^      - uses: .\/.github\/actions\/setup-runner$/s||      - uses: ./.github/actions/setup-runner # prepara\n        env:\n          X: ${{ secrets.X }}|' \
    "$dir/.github/workflows/ci.yml"
fails_naming "secreto tras un uses con comentario" "$dir" ".github/workflows/ci.yml:"

dir="$(tree secret-in-preview-job)"
sed -i -E '0,/^    steps:$/s||    env:\n      X: ${{ secrets.X }}\n    steps:|' "$dir/.github/workflows/preview.yml"
fails_naming "secreto en un job de Preview que no es build" "$dir" ".github/workflows/preview.yml:"

breaks missing-local-action "accion local que no existe" .github/workflows/ci.yml \
    '0,/uses: \.\/\.github\/actions\/setup-runner/s//uses: .\/.github\/actions\/setup-just/'

breaks just-without-setup-runner "llaman a just sin preparar el runner" .github/workflows/release.yml \
    's|packaging/verify-packages.sh paquetes|just verify-packages paquetes|'

breaks publish-without-build-tree "tiene que bajar la serie" .github/workflows/publish.yml \
    '/packaging\/repo\/build-tree.sh/d'

dir="$(tree secrets-inherit)"
append_step "$dir/.github/workflows/cache-cleanup.yml" '    secrets: inherit'
fails_naming "herencia de secretos en bloque" "$dir" "ningun workflow hereda secretos"

breaks unsigned-tree "ningun workflow puede construir el arbol sin firmar" .github/workflows/publish.yml \
    's|^(.*packaging/repo/build-tree.sh) .*$|\1 SIN-FIRMA-SOLO-PRUEBAS|'

breaks release-without-environment "firmar dentro de 'environment: release'" .github/workflows/release.yml \
    '/^    environment: release$/d'

breaks release-without-draft "crear la Release en borrador" .github/workflows/release.yml \
    '/^[^#]*--draft/d'

dir="$(tree draft-named-in-comment)"
sed -i -E '/^[^#]*--draft/d' "$dir/.github/workflows/release.yml"
append_step "$dir/.github/workflows/release.yml" '# --draft'
fails_naming "borrador nombrado solo en un comentario" "$dir" "crear la Release en borrador"

breaks publish-on-tag "cuelga de 'release: types: [published]'" .github/workflows/publish.yml \
    's|types: \[published\]|types: [created]|'

dir="$(tree published-named-in-comment)"
sed -i -E 's|^( *)types: \[published\]|\1# types: [published]|' "$dir/.github/workflows/publish.yml"
fails_naming "published nombrado solo en un comentario" "$dir" "cuelga de 'release: types: [published]'"

breaks publish-job-keeps-candidates "descartar las prereleases en el if: de cada job" .github/workflows/publish.yml \
    "s|^(    if:).*\$|\\1 \${{ github.event_name == 'workflow_dispatch' }}|"

breaks preview-checkout-of-the-run "solo hace checkout de la rama por defecto" .github/workflows/preview-comment.yml \
    's|^( *ref:).*default_branch.*$|\1 ${{ github.event.workflow_run.head_sha }}|'

breaks minisign-key-outside-release "solo aparece en .github/workflows/release.yml" .github/workflows/ci.yml \
    '0,/^    steps:$/s||    env:\n      TAURI_SIGNING_PRIVATE_KEY: x\n    steps:|'

breaks docker-in-workflow "ningun workflow toca Docker" .github/workflows/ci.yml \
    '0,/^    steps:$/s||    container: ghcr.io/x/y\n    steps:|'

breaks dependabot-without-cooldown "tiene que declarar 'cooldown'" .github/dependabot.yml \
    '/^    cooldown:/,+1d'

dir="$(tree rpm-signed-after-hashing)"
sed -i -E 's|rpmsign --addsign|true|' "$dir/.github/workflows/release.yml"
append_step "$dir/.github/workflows/release.yml" '          rpmsign --addsign "${rpms[@]}"'
fails_naming "rpm firmado despues de resumir" "$dir" "tiene los pasos en otro orden"

breaks tree-before-download "bajar la serie" .github/workflows/publish.yml \
    's|packaging/repo/download-series.sh|true|'

breaks second-graalvm-install "GraalVM se instala solo con" .github/workflows/ci.yml \
    '0,/^    steps:$/s||    steps:\n      - uses: graalvm/setup-graalvm@0000000000000000000000000000000000000000 # v1|'

breaks rust-cache-outside-setup-runner "la cache de Rust solo la abre" .github/workflows/ci.yml \
    '0,/^    steps:$/s||    steps:\n      - uses: Swatinem/rust-cache@0000000000000000000000000000000000000000 # v2|'

echo "OK  check-workflows.sh: cada invariante tiene un caso que la rompe y el arbol limpio la cumple"
