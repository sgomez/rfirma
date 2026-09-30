# Punto de entrada del proyecto: `just` es el unico orquestador (ADR-0013).
#
# Los requisitos los comprueba `just tools`; la puerta que manda es
# `just check`, que ejecuta el CI (ADR-0014).

# En Windows las recetas corren en Git Bash, no en cmd ni en PowerShell (ADR-0035).
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]

windows := if os_family() == "windows" { "true" } else { "false" }

# La raiz con barras normales: bash se come las barras invertidas de Windows.
root := replace(justfile_directory(), "\\", "/")

# GraalVM CE 25 (ADR-0004): la 21 aborta native-image; el pom compila a
# release 21 aparte. La versión exacta vive en `.graalvm-version` (ADR-0035), y
# SDKMAN la nombra de otra forma: la A.B.C.D es su `A.B.C+D.rA-graalce`.
# En Windows no hay SDKMAN: vale el JAVA_HOME.
graalvm_version := trim(read(root / ".graalvm-version"))
sdkman_graalvm := replace_regex(graalvm_version, '^(\d+)\.(\d+)\.(\d+)\.(\d+)$', '${1}.${2}.${3}+${4}.r${1}') + "-graalce"
default_graalvm := if windows == "true" { "$JAVA_HOME" } else { "$HOME/.sdkman/candidates/java/" + sdkman_graalvm }

bridge := root / "rfirma-native-bridge"
app := root / "rfirma-app"
tauri := app / "src-tauri"
conformance_suite := root / "rfirma-conformance"

# El nombre de la libreria nativa sigue la convencion de cada plataforma,
# como DLL_PREFIX y DLL_SUFFIX en Rust (ADR-0035).
native_lib_name := if windows == "true" { "rfirma_crypto.dll" } else { "librfirma_crypto.so" }

# Lo que emite native-image con -H:Name=librfirma_crypto.
native_image_output := if windows == "true" { "librfirma_crypto.dll" } else { "librfirma_crypto.so" }
native_image := if windows == "true" { "native-image.cmd" } else { "native-image" }
classpath_separator := if windows == "true" { ";" } else { ":" }

# Ruta canonica de la libreria nativa (ADR-0013).
native_lib := bridge / "target/lib/rfirma" / native_lib_name

# Version fijada: un cargo-crap con un solo mantenedor no debe poder poner en
# rojo un PR que no lo ha tocado (ADR-0014).
crap_version := "0.4.3"

# Version fijada, misma razon que crap_version (ADR-0014).
machete_version := "0.9.2"

# Version fijada, misma razon que crap_version. Igual en .github/workflows/ci.yml.
diff_cover_version := "10.6.0"

# Suelo global de lineas cubiertas en Rust (ADR-0014): la medida real en el
# momento de introducir el suelo, redondeada hacia abajo. Sube a mano, en su
# propia PR, cuando la medida real lo supere en un punto entero.
coverage_floor := "78"

# target/ compartido entre worktrees de agentes; el checkout principal se
# queda fuera porque cargo toma un cerrojo sobre el arbol mientras compila
# (ADR-0014).
worktree_target := ```
    own=$(git rev-parse --git-dir 2>/dev/null || true)
    common=$(git rev-parse --git-common-dir 2>/dev/null || true)
    if [ -n "$own" ] && [ "$own" != "$common" ] && cd "$common/.." 2>/dev/null; then
        dir="$PWD"
        if command -v cygpath >/dev/null; then dir="$(cygpath -m "$dir")"; fi
        printf '%s' "$dir/.claude/worktrees/target"
    fi
```

cargo_target := if worktree_target == "" { tauri / "target" } else { worktree_target }

# El arbol instrumentado de `cargo llvm-cov` va aparte del normal (ADR-0014).
export CARGO_TARGET_DIR := if env("CARGO_LLVM_COV", "") == "" { cargo_target } else { cargo_target / "llvm-cov-target" }

coverage_out := cargo_target / "coverage" / file_name(root)

# El arbol instrumentado se compila sin DWARF: la cobertura sale del mapa de
# LLVM, y enlazar la depuracion era la mitad de su compilacion.
no_debuginfo := "CARGO_PROFILE_DEV_DEBUG=false"

# Version fijada: sin ruff.toml, el conjunto de reglas depende de la version
# instalada. Igual en .github/workflows/ci.yml.
ruff_version := "0.16.6"

# Modulo FFI oculto de la puerta CRAP del carril rapido (ADR-0014); el carril
# lento lo mide con `just test-native`.
ffi_allow := "src/signing/adapters/ffi.rs"

# Adaptador que solo compila Windows: el carril rapido de Linux lo oculta y el
# carril de Windows lo mide con `just test-windows` (ADR-0014, ADR-0035).
windows_allow := "src/identity/adapters/windows_store/cng.rs"

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
    {{ root }}/packaging/flatpak/check-sources.sh
    {{ root }}/rfirma-app/src/design-system/check-bundle.sh
    {{ root }}/.github/check-workflows.sh
    {{ root }}/packaging/repo/build-tree.test.sh
    {{ root }}/packaging/repo/publish-tree.test.sh
    ruff check {{ root }}/packaging {{ root }}/scripts
    {{ root }}/scripts/tests/outline_test.sh
    {{ root }}/scripts/tests/ci_lanes_test.sh
    {{ root }}/scripts/tests/packages_manifest_test.sh
    {{ root }}/scripts/tests/preview_comment_test.sh

# Una sola invocacion de Maven: compila con -Xlint:all, prueba y empaqueta.
[group('ci')]
check-java: test-java

[group('ci')]
check-ts: check-po lint-ts lint-i18n knip build-ts test-ts test-site-driver check-landing

# lint-rust + machete + crap, sin `cargo build --release` ni `cargo test` sueltos; la instantanea del
# contrato la compara `tests/contract_discovers_adapters.rs` dentro de la pasada instrumentada.
[group('ci')]
check-rust: lint-rust machete crap

# ---------------------------------------------------------------------------
# Herramientas y dependencias
# ---------------------------------------------------------------------------

# Comprueba que estan las herramientas, y falla nombrando la que falte.
[group('dev')]
tools:
    RUFF_VERSION="{{ ruff_version }}" CRAP_VERSION="{{ crap_version }}" \
        MACHETE_VERSION="{{ machete_version }}" \
        DEFAULT_GRAALVM="{{ default_graalvm }}" SYSTEM_LIBS="{{ system_libs }}" \
        {{ root }}/scripts/tools.sh

# Instala las dependencias de AutoFirma en ~/.m2 si no estan (ADR-0002).
[private]
bootstrap:
    {{ root }}/scripts/bootstrap.sh

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

# Dependencias y exports sin uso del frontend (knip.json).
[private]
knip: po-import
    cd {{ app }} && pnpm exec knip

# Instala o quita los certificados de pruebas en SoftHSM: `just certs install|uninstall`.
[group('dev')]
certs action:
    ./testdata/softhsm/certs.sh {{ action }}

# Descarga (a etiqueta y sha fijados) el autoscript.js del banco de conformidad.
[group('ci')]
autoscript:
    #!/usr/bin/env bash
    set -euo pipefail
    destino="{{ root }}/testdata/conformance/autoscript-1.9.2.js"
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
    python3 {{ root }}/scripts/protocol-map.py {{ args }}

# jscpd sobre los ficheros de tests en Rust y TS. Solo informa, no entra en el CI (ADR-0014).
[group('dev')]
duplication: deps
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{ app }}
    echo "== Rust: tests/ y tests.rs =="
    pnpm exec jscpd --format rust --pattern '**/{tests/**/*.rs,tests.rs}' \
        "{{ tauri }}/src" "{{ tauri }}/tests" --reporters console
    echo
    echo "== TypeScript: *.test.ts(x) =="
    pnpm exec jscpd --pattern '**/*.{test.ts,test.tsx}' src --reporters console

# ---------------------------------------------------------------------------
# Navegacion
# ---------------------------------------------------------------------------

# Esqueleto de un fichero .rs, .ts o .tsx (ruta relativa a la raiz).
[group('checklist')]
outline path:
    {{ root }}/scripts/outline.sh {{ path }}

# Lo que la ventana puede pedirle al backend, generado de las fuentes.
[group('dev')]
contract src=(tauri / "src"):
    cd {{ tauri }} && cargo run -q --example contract -- "{{ src }}"

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

# rustfmt sobre rfirma-app/src-tauri y rfirma-conformance.
[private]
fmt-rust:
    cd {{ tauri }} && cargo fmt --all
    cd {{ conformance_suite }} && cargo fmt --all

# Formateador de biome sobre rfirma-app y, si esta instalada, la consola de la suite.
[private]
fmt-ts:
    cd {{ app }} && pnpm exec biome format --write .
    if [ -x {{ conformance_suite }}/console/node_modules/.bin/biome ]; then cd {{ conformance_suite }}/console && pnpm exec biome format --write .; fi

# `ruff format` sobre packaging y scripts.
[private]
fmt-python:
    ruff format {{ root }}/packaging {{ root }}/scripts

# clippy y rustfmt sobre rfirma-app/src-tauri.
[private]
lint-rust: build-ts
    cd {{ tauri }} && cargo fmt --all -- --check
    cd {{ tauri }} && cargo clippy --all-targets --all-features -- -D warnings

# Dependencias de Cargo.toml que no usa nadie. No compila: analiza el fuente.
[private]
machete:
    cd {{ tauri }} && cargo machete

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
    {{ root }}/scripts/check-native.sh {{ native_lib }}

# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------

# Pruebas del puente Java (`verify`: compila, prueba y empaqueta en una JVM).
[private]
test-java: bootstrap
    cd {{ bridge }} && mvn -B verify

# vitest, con cobertura: el suelo de coverage.thresholds en vite.config.ts (ADR-0014).
[private]
test-ts: po-import
    cd {{ app }} && pnpm exec vitest run --coverage --reporter=dot

# Las pruebas de los analizadores de firma de la sede, con el ejecutor de Node.
[private]
test-site-driver:
    cd {{ root }}/testdata/site-driver && node --test --test-force-exit --test-reporter=dot test/*.test.mjs

# cargo test, mas la compilacion de las pruebas de grada C.
[private]
test-rust: (certs "install") build-ts
    cd {{ tauri }} && cargo test --all-features
    cd {{ tauri }} && cargo test --all-features --no-run

# Las de grada C en una sola pasada instrumentada, que mide ademas el adaptador FFI (ADR-0014).
[group('ci')]
test-native: (certs "install") check-native build-ts
    mkdir -p "{{ coverage_out }}/crap-ffi"
    cd {{ tauri }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" {{ no_debuginfo }} cargo llvm-cov nextest --all-features --run-ignored only \
        --lcov --output-path "{{ coverage_out }}/crap-ffi/lcov.info"
    cd {{ tauri }} && cargo crap --path '{{ ffi_allow }}' --lcov "{{ coverage_out }}/crap-ffi/lcov.info" --threshold 30 --fail-above
    cd {{ bridge }} && mvn -B test -DexcludedGroups= -Dgroups=gradaC

# Pruebas de --lib y del canal local en una pasada instrumentada, y la puerta CRAP de `windows_allow` (ADR-0035).
[group('ci')]
test-windows: build-ts
    mkdir -p "{{ coverage_out }}/windows"
    cd {{ tauri }} && {{ no_debuginfo }} cargo llvm-cov --all-features --lib --test channel_client --test channel_operations --test service_acknowledgement --lcov --output-path "{{ coverage_out }}/windows/lcov.info"
    cd {{ tauri }} && cargo crap --path '{{ windows_allow }}' --lcov "{{ coverage_out }}/windows/lcov.info" --threshold 30 --fail-above

# ---------------------------------------------------------------------------
# CRAP: solo en Rust (ADR-0014)
# ---------------------------------------------------------------------------

# Genera el lcov de toda la suite con cargo llvm-cov y no baja del suelo (ADR-0014).
[private]
coverage: (certs "install") build-ts
    mkdir -p "{{ coverage_out }}/coverage"
    cd {{ tauri }} && {{ no_debuginfo }} cargo llvm-cov --all-features --lcov --output-path "{{ coverage_out }}/coverage/lcov.info" \
        --fail-under-lines {{ coverage_floor }}

# La puerta del carril rapido, con el modulo FFI oculto.
[private]
crap: coverage
    cd {{ tauri }} && cargo crap --lcov "{{ coverage_out }}/coverage/lcov.info" --threshold 30 --fail-above \
        --allow '{{ ffi_allow }}' --allow '{{ windows_allow }}'

# Cobertura del diff contra origin/main (ADR-0014): reutiliza el lcov.info que
# ya dejo `coverage` (dependencia de `check-rust`) en disco, sin volver a
# instrumentar la suite. Pide red (fetch de origin/main), asi que queda fuera
# de `check-rust` y la llama directamente ci.yml, despues de `just check-rust`.
[group('ci')]
diff-coverage:
    cd {{ tauri }} && diff-cover "{{ coverage_out }}/coverage/lcov.info" \
        --compare-branch=origin/main --diff-range-notation=.. --fail-under=80 \
        --exclude '**/adapters/tauri.rs' 'main.rs' '{{ ffi_allow }}'

# Borra el arbol instrumentado, los informes y los volcados; deja la compilacion normal.
[group('checklist')]
clean-coverage:
    {{ root }}/scripts/clean-coverage.sh "{{ cargo_target }}" "{{ tauri }}"

# ---------------------------------------------------------------------------
# Imagen nativa, empaquetado y desarrollo
# ---------------------------------------------------------------------------

# Construye la libreria nativa compartida con GraalVM CE 25 (ADR-0013).
[group('ci')]
native: build-java
    #!/usr/bin/env bash
    set -euo pipefail
    graal="${GRAALVM_HOME:-{{ default_graalvm }}}"
    if command -v cygpath >/dev/null; then graal="$(cygpath -u "$graal")"; fi
    build_dir="{{ bridge }}/target/native"
    dest="$(dirname "{{ native_lib }}")"
    mkdir -p "$build_dir" && cd "$build_dir"
    "$graal/bin/{{ native_image }}" --shared \
        -cp "{{ bridge }}/target/rfirma-native-bridge-0.1.0.jar{{ classpath_separator }}$(cat {{ bridge }}/target/cp.txt)"
    rm -rf "$dest"
    mkdir -p "$dest"
    install -m644 "$build_dir/{{ native_image_output }}" "$dest/{{ native_lib_name }}"
    sobran="$(ls -1 "$dest" | grep -vxF '{{ native_lib_name }}' || true)"
    if [ -n "$sobran" ]; then
        echo "sobra algo en $dest:" >&2
        echo "$sobran" >&2
        exit 1
    fi
    ls -la "$dest"

# Escribe el paquetes.json de un directorio de paquetes (ADR-0015).
[group('ci')]
packages-manifest dir:
    {{ root }}/scripts/packages-manifest.sh write {{ dir }}

# Comprueba el suelo de glibc de la libreria nativa (docs/research/glibc-libreria-nativa.md).
[group('ci')]
check-glibc lib=native_lib:
    {{ root }}/scripts/check-glibc.sh {{ lib }}

# Construye el flatpak, uno de los tres canales junto al .deb y el .rpm (ADR-0015).
[group('ci')]
flatpak: check-native build-ts
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ root }}/packaging/flatpak"
    flatpak-builder --force-clean --user --install --repo=repo \
        build-dir me.sgomez.rfirma.yml
    flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
        repo me.sgomez.rfirma.flatpak me.sgomez.rfirma stable
    echo
    echo "bundle: $PWD/me.sgomez.rfirma.flatpak ($(du -h me.sgomez.rfirma.flatpak | cut -f1))"
    echo "  flatpak install --user me.sgomez.rfirma.flatpak"

# Construye el .deb y el .rpm con el bundler de Tauri (ADR-0004); quick="true" salta el candado de version.
[group('ci')]
bundle quick="false": check-native build-ts
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ root }}"
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

# Construye el instalador NSIS de Windows con el bundler de Tauri (ADR-0035).
[group('ci')]
bundle-windows: check-native build-ts
    {{ root }}/scripts/bundle-windows.sh {{ root }}

# Regenera cargo-sources.json y node-sources.json.
[group('release')]
flatpak-sources:
    {{ root }}/scripts/flatpak-sources.sh

# Mutation testing incremental, a mano antes de publicar una version: no bloquea (ADR-0014).
[group('release')]
mutants:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{ tauri }}
    tag="$(git describe --tags --abbrev=0 --match 'v*')"
    diff="$(mktemp)"
    trap 'rm -f "$diff"' EXIT
    git diff --relative "$tag" -- . > "$diff"
    cargo mutants --in-place --in-diff "$diff" \
        --exclude 'adapters/tauri.rs' --exclude 'main.rs' --exclude '{{ ffi_allow }}'

# Instala, prueba y construye la landing de rfirma.sgomez.me.
[private]
check-landing:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{ root }}/packaging/repo/site
    pnpm install --frozen-lockfile --reporter=silent
    pnpm exec vitest run --reporter=dot
    pnpm exec astro build

# Comprueba el candado de la version y el nombre del producto.
[group('ci')]
check-version:
    {{ root }}/packaging/check-version.py

# Resella el bundle del sistema de diseno.
[group('release')]
seal-ds-bundle:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ root }}"
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
    {{ root }}/scripts/dev-handler.sh {{ mode }}

# Levanta la consola web de la suite de conformidad de rfirma-conformance/, imprime su URL con el
# token y la abre en el navegador. Cliente, informe y comprobaciones se eligen en la pagina; cada
# informe vive en reports/conformance/<nombre>/, con sus transcripciones dentro.
[group('dev')]
conformance: autoscript conformance-console
    cd {{ conformance_suite }} && cargo run -q

# Compila la consola web de la suite en rfirma-conformance/console/dist, que el servidor lee al
# arrancar: regenera antes los tipos del contrato que ts-rs deriva de Rust.
[group('dev')]
conformance-console:
    cd {{ conformance_suite }} && cargo test -q export_bindings > /dev/null
    cd {{ conformance_suite }}/console && pnpm install --frozen-lockfile && pnpm build

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
    rm -rf "{{ app }}/dist" "{{ conformance_suite }}/console/dist"

# Escribe en CHANGELOG.md la seccion de <version> desde los titulos de PR.
[group('release')]
[private]
changelog-release version:
    {{ root }}/scripts/changelog-release.sh {{ version }}

# Sube la version en los sitios del candado de check-version.py (ID-150).
[group('release')]
[private]
bump-version version:
    {{ root }}/scripts/bump-version.sh {{ version }}

# Publica <version> desde main: changelog, bump, commit, etiqueta y push atomico.
[group('release')]
release version:
    {{ root }}/scripts/release.sh {{ version }}
