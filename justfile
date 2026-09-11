# Punto de entrada del proyecto: `just` es el unico orquestador (ADR-0013).
#
# Los requisitos los comprueba `just tools`; la puerta que manda es
# `just check`, que ejecuta el CI (ADR-0014).

# GraalVM CE 25 (ADR-0004): la 21 aborta native-image; el pom compila a
# release 21 aparte.
default_graalvm := "$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce"

bridge := justfile_directory() / "rfirma-native-bridge"
app := justfile_directory() / "rfirma-app"
tauri := app / "src-tauri"

# Ruta canonica de la libreria nativa (ADR-0013).
native_lib := bridge / "target/lib/rfirma/librfirma_crypto.so"

# Version fijada: un cargo-crap con un solo mantenedor no debe poder poner en
# rojo un PR que no lo ha tocado (ADR-0014).
crap_version := "0.4.3"

# target/ compartido entre worktrees de agentes; el checkout principal se
# queda fuera porque cargo toma un cerrojo sobre el arbol mientras compila
# (ADR-0014).
worktree_target := ```
    own=$(git rev-parse --git-dir 2>/dev/null || true)
    common=$(git rev-parse --git-common-dir 2>/dev/null || true)
    if [ -n "$own" ] && [ "$own" != "$common" ] && cd "$common/.." 2>/dev/null; then
        printf '%s' "$PWD/.claude/worktrees/target"
    fi
```

cargo_target := if worktree_target == "" { tauri / "target" } else { worktree_target }

# El arbol instrumentado de `cargo llvm-cov` va aparte del normal (ADR-0014).
export CARGO_TARGET_DIR := if env("CARGO_LLVM_COV", "") == "" { cargo_target } else { cargo_target / "llvm-cov-target" }

coverage_out := cargo_target / "coverage" / file_name(justfile_directory())

# Version fijada: sin ruff.toml, el conjunto de reglas depende de la version
# instalada. Igual en .github/workflows/ci.yml.
ruff_version := "0.16.6"

# Modulo FFI oculto de la puerta CRAP del carril rapido (ADR-0014); el carril
# lento lo mide con `just crap-ffi`.
ffi_allow := "src/signing/adapters/ffi.rs"

# Accesorio del banco de conformidad, fijado por etiqueta y sha256: la 1.9.2
# no publica autoscript.js en ningun artefacto. Pin repetido en ci.yml.
autoscript_url := "https://raw.githubusercontent.com/ctt-gob-es/clienteafirma/v1.9.2/afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js"
autoscript_sha256 := "567998128f1cd8017c304a8c187f6912a0c56b0feebb02fffa2aa33732e40439"

# Librerias -dev del WebView que necesita Tauri; lista canonica que instala
# tambien .github/workflows/ci.yml.
system_libs := "webkit2gtk-4.1:libwebkit2gtk-4.1-dev javascriptcoregtk-4.1:libjavascriptcoregtk-4.1-dev libsoup-3.0:libsoup-3.0-dev"

# Lista las recetas.
default:
    @just --list

# ---------------------------------------------------------------------------
# Contrato
# ---------------------------------------------------------------------------

# La puerta del repositorio: un carril por cadena, en paralelo en el CI.
[group('checklist')]
check: tools check-repo check-java check-ts check-rust

# Lo que no pertenece a ninguna cadena: comprobaciones rapidas que ninguna compilacion ve.
[group('ci')]
check-repo: check-version
    #!/usr/bin/env bash
    set -euo pipefail
    {{ justfile_directory() }}/packaging/flatpak/check-sources.sh
    {{ justfile_directory() }}/rfirma-app/src/design-system/check-bundle.sh
    {{ justfile_directory() }}/.github/check-workflows.sh
    {{ justfile_directory() }}/packaging/repo/build-tree.test.sh
    {{ justfile_directory() }}/packaging/repo/publish-tree.test.sh
    ruff check {{ justfile_directory() }}/packaging {{ justfile_directory() }}/scripts
    {{ justfile_directory() }}/scripts/tests/outline_test.sh

# Una sola invocacion de Maven: compila con -Xlint:all, prueba y empaqueta.
[group('ci')]
check-java: test-java

[group('ci')]
check-ts: check-po lint-ts lint-i18n build-ts test-ts check-landing

# lint-rust + crap + check-contract, sin `cargo build --release` ni `cargo test` sueltos.
[group('ci')]
check-rust: lint-rust crap check-contract

# ---------------------------------------------------------------------------
# Herramientas y dependencias
# ---------------------------------------------------------------------------

# Comprueba que estan las herramientas, y falla nombrando la que falte.
[group('dev')]
tools:
    RUFF_VERSION="{{ ruff_version }}" CRAP_VERSION="{{ crap_version }}" \
        DEFAULT_GRAALVM="{{ default_graalvm }}" SYSTEM_LIBS="{{ system_libs }}" \
        {{ justfile_directory() }}/scripts/tools.sh

# Instala las dependencias de AutoFirma en ~/.m2 si no estan (ADR-0002).
[private]
bootstrap:
    {{ justfile_directory() }}/scripts/bootstrap.sh

# Instala las dependencias de node de rfirma-app.
[private]
deps:
    cd {{ app }} && pnpm install --frozen-lockfile

# --all rellena tambien los idiomas incompletos, con castellano; nunca en el CI.
# Fusiona el .pot con los cinco .po y regenera los catalogos.
[group('dev')]
po *args: deps
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ app }}/po"
    for f in *.po; do
        msgmerge --quiet --update --backup=none --no-fuzzy-matching "$f" messages.pot
    done
    cd "{{ app }}"
    node tools/po-import.mjs {{ args }}
    pnpm exec i18next-cli types -q

# Genera src/i18n/locales/*.ts desde los .po. Node puro: sin gettext.
[private]
po-import: deps
    cd {{ app }} && node tools/po-import.mjs
    cd {{ app }} && pnpm exec i18next-cli types -q

# Comprueba los cinco .po contra la plantilla.
[private]
check-po:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ app }}/po"
    command -v msgfmt >/dev/null || { echo "falta gettext: ejecuta 'just tools'" >&2; exit 1; }
    for f in *.po; do
        echo -n "$f: "
        msgfmt --check-format --statistics --output-file=/dev/null "$f"
        msgcmp --use-untranslated --use-fuzzy "$f" messages.pot
    done
    echo "los .po cuadran con messages.pot"

# i18next-cli sobre el codigo: una clave sin catalogo o un catalogo sin uso.
[private]
lint-i18n: po-import
    cd {{ app }} && pnpm exec i18next-cli extract --ci
    cd {{ app }} && pnpm exec i18next-cli status --unused

# Provisiona los tokens SoftHSM `rfirma-test` y `rfirma-test-ecc` desde testdata/fnmt/.
[group('dev')]
token:
    ./testdata/softhsm/provision-token.sh

# Descarga (a etiqueta y sha fijados) el autoscript.js del banco de conformidad.
[group('ci')]
autoscript:
    #!/usr/bin/env bash
    set -euo pipefail
    destino="{{ justfile_directory() }}/testdata/conformance/autoscript-1.9.2.js"
    sha="{{ autoscript_sha256 }}"
    if [ -f "$destino" ] && echo "$sha  $destino" | sha256sum --check --status; then
        echo "autoscript.js v1.9.2 ya esta en testdata/conformance/"
        exit 0
    fi
    mkdir -p "$(dirname "$destino")"
    curl -sSL --fail --max-time 120 -o "$destino.parcial" "{{ autoscript_url }}"
    if ! echo "$sha  $destino.parcial" | sha256sum --check --status; then
        obtenido="$(sha256sum "$destino.parcial" | cut -d' ' -f1)"
        rm -f "$destino.parcial"
        echo "autoscript.js no cuadra con el sha fijado:" >&2
        echo "  esperado $sha" >&2
        echo "  obtenido $obtenido" >&2
        exit 1
    fi
    mv "$destino.parcial" "$destino"
    echo "autoscript.js v1.9.2 descargado en testdata/conformance/"

# Genera el mapa del protocolo de AutoFirma a tag fijado y lo cruza con el de rFirma.
[group('dev')]
protocol-map *args:
    python3 {{ justfile_directory() }}/scripts/protocol-map.py {{ args }}

# ---------------------------------------------------------------------------
# Navegacion
# ---------------------------------------------------------------------------

# Esqueleto de un fichero .rs, .ts o .tsx (ruta relativa a la raiz).
[group('checklist')]
outline path:
    {{ justfile_directory() }}/scripts/outline.sh {{ path }}

# Lo que la ventana puede pedirle al backend, generado de las fuentes.
[group('dev')]
contract src=(tauri / "src"):
    cd {{ tauri }} && cargo run -q --example contract -- "{{ src }}"

# Comprueba que `just contract` sigue siendo el de la instantanea.
[private]
check-contract: build-ts
    #!/usr/bin/env bash
    set -eu
    snapshot={{ tauri }}/tests/contract.snapshot
    if ! diff -u "$snapshot" <(just contract); then
        echo "el contrato ventana-backend ha cambiado" >&2
        exit 1
    fi
    echo "check-contract: el contrato es el de la instantanea"

# Compila el puente Java con -Xlint:all; sin `clean`, que borraria la libreria nativa a mitad de `just check`.
[private]
lint-java: bootstrap
    cd {{ bridge }} && mvn -B compile

# Biome sobre rfirma-app.
[private]
lint-ts: po-import
    cd {{ app }} && pnpm exec biome ci .

# Formatea las tres cadenas escribiendo.
[group('checklist')]
fmt: fmt-rust fmt-ts fmt-python

# rustfmt sobre rfirma-app/src-tauri.
[private]
fmt-rust:
    cd {{ tauri }} && cargo fmt --all

# Formateador de biome sobre rfirma-app.
[private]
fmt-ts:
    cd {{ app }} && pnpm exec biome format --write .

# `ruff format` sobre packaging y scripts.
[private]
fmt-python:
    ruff format {{ justfile_directory() }}/packaging {{ justfile_directory() }}/scripts

# clippy y rustfmt sobre rfirma-app/src-tauri.
[private]
lint-rust: build-ts
    cd {{ tauri }} && cargo fmt --all -- --check
    cd {{ tauri }} && cargo clippy --all-targets --all-features -- -D warnings

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

# Compila el puente Java.
[private]
build-java: bootstrap
    cd {{ bridge }} && mvn -B package -DskipTests

# tsc -b y vite build.
[group('ci')]
build-ts: po-import
    cd {{ app }} && pnpm exec tsc -b
    cd {{ app }} && pnpm exec vite build

# Compila el binario de la aplicacion.
[group('ci')]
build-rust: build-ts
    cd {{ tauri }} && cargo build --release --features custom-protocol

# Falla nombrando `just native` si la libreria nativa no esta; RFIRMA_SKIP_NATIVE=1 la salta (ADR-0013).
[private]
check-native:
    {{ justfile_directory() }}/scripts/check-native.sh {{ native_lib }}

# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------

# Pruebas del puente Java (`verify`: compila, prueba y empaqueta en una JVM).
[private]
test-java: bootstrap
    cd {{ bridge }} && mvn -B verify

# vitest.
[private]
test-ts: po-import
    cd {{ app }} && pnpm exec vitest run --reporter=dot

# cargo test, mas la compilacion de las pruebas de grada C.
[private]
test-rust: token build-ts
    cd {{ tauri }} && cargo test --all-features
    cd {{ tauri }} && cargo test --all-features --no-run

# Las de grada C, que el carril lento ejecuta con --ignored.
[group('ci')]
test-native: token check-native build-ts
    cd {{ tauri }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" cargo test --all-features -- --ignored
    cd {{ bridge }} && mvn -B test -DexcludedGroups= -Dgroups=gradaC

# ---------------------------------------------------------------------------
# CRAP: solo en Rust (ADR-0014)
# ---------------------------------------------------------------------------

# Genera el lcov de toda la suite con cargo llvm-cov.
[private]
coverage: token build-ts
    mkdir -p "{{ coverage_out }}/coverage"
    cd {{ tauri }} && cargo llvm-cov --all-features --lcov --output-path "{{ coverage_out }}/coverage/lcov.info"

# La puerta del carril rapido, con el modulo FFI oculto.
[private]
crap: coverage
    cd {{ tauri }} && cargo crap --lcov "{{ coverage_out }}/coverage/lcov.info" --threshold 30 --fail-above \
        --allow '{{ ffi_allow }}'

# Corre unicamente el ciclo nativo (grada C) y mide el adaptador FFI.
[group('ci')]
crap-ffi: token check-native build-ts
    mkdir -p "{{ coverage_out }}/crap-ffi"
    cd {{ tauri }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" cargo llvm-cov --test native_cycle --all-features --lcov --output-path "{{ coverage_out }}/crap-ffi/lcov.info" \
        -- --ignored
    cd {{ tauri }} && cargo crap --path '{{ ffi_allow }}' --lcov "{{ coverage_out }}/crap-ffi/lcov.info" --threshold 30 --fail-above

# Borra el arbol instrumentado, los informes y los volcados; deja la compilacion normal.
[group('checklist')]
clean-coverage:
    {{ justfile_directory() }}/scripts/clean-coverage.sh {{ cargo_target }}

# ---------------------------------------------------------------------------
# Imagen nativa, empaquetado y desarrollo
# ---------------------------------------------------------------------------

# Construye la libreria nativa compartida con GraalVM CE 25 (ADR-0013).
[group('ci')]
native: build-java
    #!/usr/bin/env bash
    set -euo pipefail
    graal="${GRAALVM_HOME:-{{ default_graalvm }}}"
    build_dir="{{ bridge }}/target/native"
    dest="$(dirname "{{ native_lib }}")"
    mkdir -p "$build_dir" && cd "$build_dir"
    "$graal/bin/native-image" --shared \
        -cp "{{ bridge }}/target/rfirma-native-bridge-0.1.0.jar:$(cat {{ bridge }}/target/cp.txt)"
    rm -rf "$dest"
    mkdir -p "$dest"
    install -m644 "$build_dir/librfirma_crypto.so" "$dest/librfirma_crypto.so"
    sobran="$(ls -1 "$dest" | grep -v '^librfirma_crypto\.so$' || true)"
    if [ -n "$sobran" ]; then
        echo "sobra algo en $dest:" >&2
        echo "$sobran" >&2
        exit 1
    fi
    ls -la "$dest"

# Comprueba el suelo de glibc de la libreria nativa (docs/research/glibc-libreria-nativa.md).
[group('ci')]
check-glibc lib=native_lib:
    {{ justfile_directory() }}/scripts/check-glibc.sh {{ lib }}

# Construye el flatpak, el unico canal soportado (ADR-0015).
[group('ci')]
flatpak: check-native build-ts
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}/packaging/flatpak"
    flatpak-builder --force-clean --user --install --repo=repo \
        build-dir me.sgomez.rfirma.yml
    flatpak build-bundle repo me.sgomez.rfirma.flatpak me.sgomez.rfirma stable
    echo
    echo "bundle: $PWD/me.sgomez.rfirma.flatpak ($(du -h me.sgomez.rfirma.flatpak | cut -f1))"
    echo "  flatpak install --user me.sgomez.rfirma.flatpak"

# Construye el .deb y el .rpm con el bundler de Tauri (ADR-0004); quick="true" salta el candado de version.
[group('ci')]
bundle quick="false": check-native build-ts
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}"
    if [ "{{ quick }}" != "true" ]; then
        version="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' rfirma-app/src-tauri/tauri.conf.json)"
        if ! packaging/native-packages-allowed.sh "$version"; then
            echo "bundle: no hay nada que construir para $version" >&2
            exit 1
        fi
    fi
    (cd "{{ app }}" && pnpm exec tauri build)
    salida="rfirma-app/src-tauri/target/release/bundle"
    for formato in deb rpm; do
        paquete="$(find "$salida/$formato" -maxdepth 1 -type f -name "*.$formato" | sort | tail -1)"
        if [ -z "$paquete" ]; then
            echo "el bundler no produjo ningun .$formato en $salida/$formato" >&2
            exit 1
        fi
        packaging/verifica-contenido.sh "$paquete"
        echo "$formato: $PWD/$paquete ($(du -h "$paquete" | cut -f1))"
    done

# Regenera cargo-sources.json y node-sources.json.
[group('release')]
flatpak-sources:
    {{ justfile_directory() }}/scripts/flatpak-sources.sh

# Instala, prueba y construye la landing de rfirma.sgomez.me.
[private]
check-landing:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{ justfile_directory() }}/packaging/repo/site
    pnpm install --frozen-lockfile --reporter=silent
    pnpm exec vitest run --reporter=dot
    pnpm exec astro build

# Comprueba el candado de la version y el nombre del producto.
[group('ci')]
check-version:
    {{ justfile_directory() }}/packaging/check-version.py

# Resella el bundle del sistema de diseno.
[group('release')]
seal-ds-bundle:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}"
    find rfirma-app/src/design-system/bundle -type f ! -name _ds_needs_recompile \
        | LC_ALL=C sort \
        | xargs sha256sum \
        > rfirma-app/src/design-system/bundle.lock
    echo
    echo "resellado. Versiona rfirma-app/src/design-system/bundle.lock."

# Abre la ventana con recarga en caliente; los argumentos van a la aplicacion.
[group('dev')]
dev *args: check-native po-import
    cd {{ app }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" pnpm exec tauri dev -- -- {{ args }}

# Registra (`on`) o quita (`off`) el manejador de desarrollo de afirma://.
[group('dev')]
dev-handler mode="on":
    {{ justfile_directory() }}/scripts/dev-handler.sh {{ mode }}

# Borra lo construido y los volcados de cobertura sueltos en el arbol de fuentes.
[group('dev')]
clean:
    #!/usr/bin/env bash
    set -eu
    cd "{{ bridge }}" && mvn -B clean
    if [ -z "{{ worktree_target }}" ]; then
        cd "{{ tauri }}" && cargo clean
    else
        rm -rf "{{ coverage_out }}"
        echo "worktree: el arbol compartido {{ cargo_target }} se queda"
    fi
    rm -f "{{ tauri }}"/*.profraw
    rm -rf "{{ app }}/dist"

# Reune los fragmentos de changelog.d/ en la seccion de <version> de CHANGELOG.md.
[group('release')]
changelog-release version:
    {{ justfile_directory() }}/scripts/changelog-release.sh {{ version }}
