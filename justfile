# Punto de entrada del proyecto: `just` es el unico orquestador (ADR-0013).
#
# Los requisitos los comprueba `just tools`; la puerta que manda es
# `just check`, que ejecuta el CI (ADR-0014).

# En Windows las recetas corren en Git Bash, no en cmd ni en PowerShell (ADR-0035).
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]

# Las versiones fijadas; el fichero gana a una variable vieja del entorno (ADR-0014).
set dotenv-filename := "versions.env"
set dotenv-required := true
set dotenv-override := true

windows := if os_family() == "windows" { "true" } else { "false" }
macos := if os() == "macos" { "true" } else { "false" }

# Cargo y las herramientas de `~/.local/bin` quedan fuera del PATH de una shell no interactiva: las recetas se llaman sin prefijo.
export PATH := if windows == "true" { env("PATH", "") } else { home_directory() / ".cargo/bin" + ":" + home_directory() / ".local/bin" + ":" + env("PATH", "/usr/local/bin:/usr/bin:/bin") }

# La raiz con barras normales: bash se come las barras invertidas de Windows.
root := replace(justfile_directory(), "\\", "/")

# SDKMAN nombra la GraalVM A.B.C.D como `A.B.C+D.rA-graalce` (ADR-0004); en macOS vale el JAVA_HOME o la JDK que registre java_home.
graalvm_version := env("GRAALVM_VERSION")
sdkman_graalvm := replace_regex(graalvm_version, '^(\d+)\.(\d+)\.(\d+)\.(\d+)$', '${1}.${2}.${3}+${4}.r${1}') + "-graalce"
graalvm_major := replace_regex(graalvm_version, '\..*$', '')
default_graalvm := if windows == "true" { "$JAVA_HOME" } else if macos == "true" { "${JAVA_HOME:-$(/usr/libexec/java_home -v " + graalvm_major + " 2>/dev/null || true)}" } else { "$HOME/.sdkman/candidates/java/" + sdkman_graalvm }

bridge := root / "rfirma-native-bridge"
app := root / "rfirma-app"
tauri := app / "src-tauri"
conformance_suite := root / "rfirma-conformance"

# `prefijo:extension` de la biblioteca dinamica en cada sistema, como DLL_PREFIX y DLL_SUFFIX en Rust.
dynamic_library := if os() == "windows" { ":.dll" } else if os() == "macos" { "lib:.dylib" } else { "lib:.so" }
dll_prefix := replace_regex(dynamic_library, ':.*$', '')
dll_suffix := replace_regex(dynamic_library, '^[^:]*:', '')

native_lib_name := dll_prefix + "rfirma_crypto" + dll_suffix

# native-image conserva el `lib` de -H:Name en todos los sistemas (ADR-0035).
native_image_output := "librfirma_crypto" + dll_suffix
native_image := if windows == "true" { "native-image.cmd" } else { "native-image" }
classpath_separator := if windows == "true" { ";" } else { ":" }

# Ruta canonica de la libreria nativa (ADR-0013).
native_lib := bridge / "target/lib/rfirma" / native_lib_name

autofirma_version := env("AUTOFIRMA_VERSION")
maven := "mvn -B -Dautofirma.version=" + autofirma_version

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

# La libreria nativa de las pruebas sueltas: la del arbol si la tiene; si no,
# en un worktree, la del checkout principal.
test_native_lib_dir := if path_exists(native_lib) == "true" { parent_directory(native_lib) } else if worktree_target == "" { parent_directory(native_lib) } else { parent_directory(parent_directory(parent_directory(worktree_target))) / "rfirma-native-bridge/target/lib/rfirma" }

# El arbol instrumentado de `cargo llvm-cov` va aparte del normal (ADR-0014).
export CARGO_TARGET_DIR := if env("CARGO_LLVM_COV", "") == "" { cargo_target } else { cargo_target / "llvm-cov-target" }

# Una pasada instrumentada a la vez sobre el arbol compartido (ADR-0014).
cov_lock := if worktree_target == "" { "" } else if os() == "linux" { 'flock "' + cargo_target / "llvm-cov-target" / ".lock" + '"' } else { "" }

coverage_out := cargo_target / "coverage" / file_name(root)

# El arbol instrumentado se compila sin DWARF: la cobertura sale del mapa de
# LLVM, y enlazar la depuracion era la mitad de su compilacion.
no_debuginfo := "CARGO_PROFILE_DEV_DEBUG=false"

# Modulo FFI oculto de la puerta CRAP del carril rapido (ADR-0014); el carril
# lento lo mide con `just test-native`.
ffi_allow := "src/signing/adapters/ffi.rs"

# Adaptador que solo compila Windows: el carril rapido de Linux lo oculta y el
# carril de Windows lo mide con `just test-windows` (ADR-0014, ADR-0035).
windows_allow := "src/identity/adapters/windows_store/cng.rs"

autoscript_url := "https://raw.githubusercontent.com/ctt-gob-es/clienteafirma/v" + autofirma_version + "/afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js"

# Librerias -dev del WebView que necesita Tauri; lista canonica que instala
# tambien .github/workflows/ci.yml.
system_libs := "webkit2gtk-4.1:libwebkit2gtk-4.1-dev javascriptcoregtk-4.1:libjavascriptcoregtk-4.1-dev libsoup-3.0:libsoup-3.0-dev"

# Lista las recetas.
default:
    @just --list

# La puerta del repositorio: un carril por cadena, en paralelo en el CI.
[group('checklist')]
check: tools check-repo check-java check-ts check-landing check-rust

# Lo que no pertenece a ninguna cadena: comprobaciones rapidas que ninguna compilacion ve.
[group('ci')]
[script('bash')]
check-repo: check-version fmt-check
    set -euo pipefail
    {{ root }}/packaging/flatpak/check-sources.sh
    {{ root }}/rfirma-app/src/design-system/check-bundle.sh
    {{ root }}/.github/check-workflows.sh
    {{ root }}/scripts/check-versions.sh
    {{ root }}/packaging/repo/build-tree.test.sh
    {{ root }}/packaging/repo/publish-tree.test.sh
    {{ root }}/packaging/windows/sign-updater.test.sh
    {{ root }}/packaging/verify-packages.test.sh
    {{ root }}/packaging/check_launchers.py
    python3 -m unittest discover -s {{ root }}/packaging -p 'test_check_launchers.py'
    python3 -m unittest discover -s {{ root }}/scripts/tests -p 'test_*.py'
    ruff check {{ root }}
    {{ root }}/scripts/tests/outline_test.sh
    {{ root }}/scripts/tests/ci_lanes_test.sh
    {{ root }}/scripts/tests/platform_files_test.sh
    {{ root }}/scripts/tests/main_moved_test.sh
    {{ root }}/scripts/tests/packages_manifest_test.sh
    {{ root }}/scripts/tests/preview_comment_test.sh
    {{ root }}/scripts/tests/check_versions_test.sh
    {{ root }}/scripts/tests/check_workflows_test.sh

# Una sola invocacion de Maven: compila con -Xlint:all, prueba y empaqueta.
[group('ci')]
check-java: test-java

[group('ci')]
check-ts: check-po lint-ts lint-i18n knip build-ts test-ts test-site-driver

# lint-rust + machete + crap, sin `cargo build --release` ni `cargo test` sueltos; la instantanea del
# contrato la compara `tests/contract_discovers_adapters.rs` dentro de la pasada instrumentada.
[linux]
[group('ci')]
check-rust: lint-rust machete crap

# Sin SoftHSM no hay pasada instrumentada: el CRAP lo juzga el CI de Linux (#1526).
[windows]
[macos]
[group('ci')]
check-rust: lint-rust machete

# Comprueba las herramientas, y falla nombrando la que falte o no este en su version fijada.
[group('dev')]
tools:
    PLATFORM="{{ os() }}" DEFAULT_GRAALVM="{{ default_graalvm }}" SYSTEM_LIBS="{{ system_libs }}" \
        {{ root }}/scripts/tools.sh

# Instala las herramientas en su version fijada: todas, o las que se nombren.
[group('ci')]
[group('dev')]
install-tools *tools:
    {{ root }}/scripts/install-tools.sh {{ tools }}

# Instala las dependencias de AutoFirma en ~/.m2 si no estan (ADR-0002).
[private]
bootstrap:
    {{ root }}/scripts/bootstrap.sh

# Instala las dependencias de node de rfirma-app y de Biome.
[private]
deps:
    cd {{ root }} && pnpm install --frozen-lockfile --filter . --filter rfirma-app

# --all rellena tambien los idiomas incompletos, con castellano; nunca en el CI.
# Fusiona el .pot con los cinco .po y regenera los catalogos.
[group('dev')]
[script('bash')]
po *args: po-import
    set -euo pipefail
    cd "{{ app }}/po"
    for f in *.po; do
        msgmerge --quiet --update --backup=none --no-fuzzy-matching "$f" messages.pot
        msgattrib --no-obsolete --output-file="$f" "$f"
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
[script('bash')]
check-po:
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

# Instala, reinstala o quita los certificados de pruebas en SoftHSM: `just certs install|reinstall|uninstall`; `install` no escribe si el kit ya está completo.
[group('dev')]
certs action:
    ./testdata/softhsm/certs.sh {{ action }}

# Descarga (a etiqueta y sha fijados) el autoscript.js del banco de conformidad.
[group('ci')]
[script('bash')]
autoscript:
    set -euo pipefail
    destino="{{ root }}/testdata/conformance/autoscript-{{ autofirma_version }}.js"
    sha="$AUTOSCRIPT_SHA256"
    if [ -f "$destino" ] && echo "$sha  $destino" | sha256sum --check --status; then
        echo "autoscript.js v{{ autofirma_version }} ya esta en testdata/conformance/"
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
    echo "autoscript.js v{{ autofirma_version }} descargado en testdata/conformance/"

# jscpd sobre los ficheros de tests en Rust y TS. Solo informa, no entra en el CI (ADR-0014).
[group('dev')]
[script('bash')]
duplication: deps
    set -euo pipefail
    cd {{ app }}
    echo "== Rust: tests/ y tests.rs =="
    pnpm exec jscpd --format rust --pattern '**/{tests/**/*.rs,tests.rs}' \
        "{{ tauri }}/src" "{{ tauri }}/tests" --reporters console
    echo
    echo "== TypeScript: *.test.ts(x) =="
    pnpm exec jscpd --pattern '**/*.{test.ts,test.tsx}' src --reporters console

# Esqueleto de ficheros .rs/.ts/.tsx (ruta) o tramos de cualquiera (ruta:A-B,C-D), en una llamada.
[group('checklist')]
outline +paths:
    @{{ root }}/scripts/outline.sh {{ paths }}

# La prueba de Rust que tocas, sobre el arbol de compilacion de las recetas: `just test-one-rust <filtro>`.
[group('checklist')]
test-one-rust *args:
    cd {{ tauri }} && RFIRMA_LIB_DIR="{{ test_native_lib_dir }}" cargo test {{ args }}

# El fichero de vitest que tocas, con el reportero callado: `just test-one-ts <fichero>`.
[group('checklist')]
test-one-ts *args:
    cd {{ app }} && pnpm exec vitest run {{ args }} --reporter=dot

# Lo que la ventana puede pedirle al backend, generado de las fuentes.
[group('dev')]
contract src=(tauri / "src"):
    cd {{ tauri }} && cargo run -q --example contract -- "{{ src }}"

# Biome sobre rfirma-app y la consola de la suite; la landing va en su carril.
[private]
lint-ts: po-import
    cd {{ root }} && pnpm exec biome ci rfirma-app rfirma-conformance/console

# Formatea las tres cadenas escribiendo.
[group('checklist')]
fmt: fmt-rust fmt-ts fmt-python

# rustfmt sobre rfirma-app/src-tauri y rfirma-conformance.
[private]
fmt-rust:
    cd {{ tauri }} && cargo fmt --all
    cd {{ conformance_suite }} && cargo fmt --all

# Formateador de biome y orden de imports sobre los tres proyectos de node; `biome format` no ordena imports.
[private]
fmt-ts:
    cd {{ root }} && pnpm exec biome check --write --linter-enabled=false .

# `ruff format` sobre todo el Python del repositorio.
[private]
fmt-python:
    ruff format {{ root }}

# La comprobacion de formato de las tres cadenas y el lint de biome, sin escribir: la llaman el pre-push y check-repo.
[group('ci')]
fmt-check: deps fmt-check-rust
    cd {{ conformance_suite }} && cargo fmt --all -- --check
    cd {{ root }} && pnpm exec biome check .
    ruff format --check {{ root }}

# Las guardas estructurales del backend (tamaño, mapas, citas de ADR, capas) sin compilar la crate: la llaman el commit y el pre-push.
[group('checklist')]
structural-guards:
    {{ root }}/scripts/structural-guards.sh

[private]
fmt-check-rust:
    cd {{ tauri }} && cargo fmt --all -- --check

# rustfmt y clippy sobre rfirma-app/src-tauri.
[private]
lint-rust: fmt-check-rust build-ts
    cd {{ tauri }} && cargo clippy --all-targets --all-features -- -D warnings

# Dependencias de Cargo.toml que no usa nadie. No compila: analiza el fuente.
[private]
machete:
    cd {{ tauri }} && cargo machete

# Compila el puente Java.
[private]
build-java: bootstrap
    cd {{ bridge }} && {{ maven }} package -DskipTests

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
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "${RFIRMA_SKIP_NATIVE:-0}" = "1" ]; then
        echo "check-native: omitida (RFIRMA_SKIP_NATIVE=1)"
        exit 0
    fi
    if [ ! -f "{{ native_lib }}" ]; then
        echo "falta la libreria nativa: {{ native_lib }}" >&2
        echo "Ejecuta 'just native' (tarda unos tres minutos y necesita GraalVM CE 25)." >&2
        exit 1
    fi

# Pruebas del puente Java (`verify`: compila, prueba y empaqueta en una JVM).
[private]
test-java: bootstrap
    cd {{ bridge }} && {{ maven }} verify

# vitest, con cobertura: el suelo de coverage.thresholds en vite.config.ts (ADR-0014).
[private]
test-ts: po-import
    cd {{ app }} && pnpm exec vitest run --coverage --reporter=dot

# Las pruebas de los analizadores de firma de la sede, con el ejecutor de Node.
[private]
test-site-driver:
    cd {{ root }}/testdata/site-driver && node --test --test-force-exit --test-reporter=dot test/*.test.mjs

# Las de grada C en una sola pasada instrumentada, que mide ademas el adaptador FFI (ADR-0014).
[group('ci')]
test-native: (certs "install") check-native build-ts llvm-cov-tag
    mkdir -p "{{ coverage_out }}/crap-ffi"
    cd {{ tauri }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" {{ no_debuginfo }} {{ cov_lock }} cargo llvm-cov nextest --all-features --run-ignored only --test-threads 8 \
        --lcov --output-path "{{ coverage_out }}/crap-ffi/lcov.info"
    cd {{ tauri }} && cargo crap --path '{{ ffi_allow }}' --lcov "{{ coverage_out }}/crap-ffi/lcov.info" --threshold 30 --fail-above
    cd {{ bridge }} && {{ maven }} test -DexcludedGroups= -Dgroups=gradaC

# Pruebas de --lib y del canal local en una pasada instrumentada, y la puerta CRAP de `windows_allow` (ADR-0035).
[group('ci')]
test-windows: build-ts
    mkdir -p "{{ coverage_out }}/windows"
    cd {{ tauri }} && {{ no_debuginfo }} cargo llvm-cov --all-features --lib --test channel_client --test channel_operations --test service_acknowledgement --lcov --output-path "{{ coverage_out }}/windows/lcov.info"
    cd {{ tauri }} && cargo crap --path '{{ windows_allow }}' --lcov "{{ coverage_out }}/windows/lcov.info" --threshold 30 --fail-above

# Pruebas de --lib y del canal local en macOS, sin grada B ni C (ADR-0040).
[group('ci')]
test-macos: build-ts
    cd {{ tauri }} && cargo test --all-features --lib --test channel_client --test channel_operations --test service_acknowledgement

# ---------------------------------------------------------------------------
# CRAP: solo en Rust (ADR-0014)
# ---------------------------------------------------------------------------

# Sin la marca de caché, `cargo llvm-cov` no limpia su árbol y mezcla los volcados de otros worktrees (ADR-0014).
[private]
llvm-cov-tag:
    mkdir -p "{{ cargo_target }}/llvm-cov-target"
    test -f "{{ cargo_target }}/llvm-cov-target/CACHEDIR.TAG" || printf 'Signature: 8a477f597d28d172789f06886806bc55\n' > "{{ cargo_target }}/llvm-cov-target/CACHEDIR.TAG"

# Genera el lcov de toda la suite con cargo llvm-cov y no baja del suelo (ADR-0014), sobre una copia privada del almacén de SoftHSM.
[private]
coverage: (certs "install") build-ts llvm-cov-tag
    mkdir -p "{{ coverage_out }}/coverage"
    cd {{ tauri }} && {{ no_debuginfo }} {{ cov_lock }} {{ root }}/scripts/token-per-test.sh cargo llvm-cov --all-features --lcov --output-path "{{ coverage_out }}/coverage/lcov.info" \
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
    rm -rf "{{ cargo_target }}/llvm-cov-target" "{{ coverage_out }}"
    rm -f "{{ cargo_target }}"/*.profraw "{{ tauri }}"/*.profraw

# Construye la libreria nativa compartida con GraalVM CE 25 (ADR-0013).
[group('ci')]
native: build-java
    #!/usr/bin/env bash
    set -euo pipefail
    graal="${GRAALVM_HOME:-{{ default_graalvm }}}"
    if command -v cygpath >/dev/null; then graal="$(cygpath -u "$graal")"; fi
    [ -n "$graal" ] || { echo "no encuentro GraalVM CE 25: define GRAALVM_HOME" >&2; exit 1; }
    build_dir="{{ bridge }}/target/native"
    dest="$(dirname "{{ native_lib }}")"
    mkdir -p "$build_dir" && cd "$build_dir"
    "$graal/bin/{{ native_image }}" --shared \
        -cp "{{ bridge }}/target/rfirma-native-bridge-0.1.0.jar{{ classpath_separator }}$(cat {{ bridge }}/target/cp.txt)"
    rm -rf "$dest"
    mkdir -p "$dest"
    install -m644 "$build_dir/{{ native_image_output }}" "$dest/{{ native_lib_name }}"
    if [ "{{ macos }}" = true ]; then
        install_name_tool -id "@rpath/{{ native_lib_name }}" "$dest/{{ native_lib_name }}"
        codesign --force --sign - "$dest/{{ native_lib_name }}"
    fi
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

# Verifica un directorio de paquetes: el contenido; con --against, contra los resúmenes de la construcción; con --signed, la firma (ADR-0015).
[group('ci')]
verify-packages dir *options:
    {{ root }}/packaging/verify-packages.sh {{ dir }} {{ options }}

# Comprueba el suelo de glibc de la libreria nativa (docs/research/glibc-libreria-nativa.md).
[group('ci')]
check-glibc lib=native_lib:
    {{ root }}/scripts/check-glibc.sh {{ lib }}

# Construye el flatpak, uno de los tres canales junto al .deb y el .rpm (ADR-0015).
[group('ci')]
[script('bash')]
flatpak: check-native build-ts
    set -euo pipefail
    cd "{{ root }}/packaging/flatpak"
    flatpak-builder --force-clean --user --install --repo=repo \
        build-dir me.sgomez.rfirma.yml
    flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
        repo me.sgomez.rfirma.flatpak me.sgomez.rfirma stable
    echo
    echo "bundle: $PWD/me.sgomez.rfirma.flatpak ($(du -h me.sgomez.rfirma.flatpak | cut -f1))"
    echo "  flatpak install --user me.sgomez.rfirma.flatpak"

# Prueba de humo manual del flatpak: ventana, portal, almacenes NSS y ciclo trifasico contra la libreria del bundle.
[group('release')]
flatpak-smoke: check-native build-ts
    {{ root }}/packaging/flatpak/verifica.sh

# Construye el .deb y el .rpm con el bundler de Tauri (ADR-0004); quick="true" salta el candado de version.
[linux]
[group('ci')]
[script('bash')]
bundle quick="false": check-native build-ts
    set -euo pipefail
    cd "{{ root }}"
    if [ "{{ quick }}" != "true" ]; then
        version="$(scripts/app_version.py check)"
        if [[ "$version" == *-* ]]; then
            echo "bundle: una candidata no produce .deb ni .rpm: $version" >&2
            exit 1
        fi
    fi
    (cd "{{ app }}" && pnpm exec tauri build)
    salida="$CARGO_TARGET_DIR/release/bundle"
    for formato in deb rpm; do
        paquete="$(find "$salida/$formato" -maxdepth 1 -type f -name "*.$formato" | sort | tail -1)"
        if [ -z "$paquete" ]; then
            echo "el bundler no produjo ningun .$formato en $salida/$formato" >&2
            exit 1
        fi
        packaging/verifica-contenido.sh "$paquete"
        echo "$formato: $paquete ($(du -h "$paquete" | cut -f1))"
    done

# Construye el instalador NSIS con el bundler de Tauri y el runtime de Visual C++ al lado (ADR-0035).
[windows]
[group('ci')]
bundle: check-native build-ts
    #!/usr/bin/env bash
    set -euo pipefail
    # La ruta que citan los recursos de tauri.windows.json, no el CARGO_TARGET_DIR.
    runtime="{{ tauri }}/target/windows-runtime"
    vswhere="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
    vs="$(cygpath -u "$("$vswhere" -latest -products '*' -property installationPath | tr -d '\r')")"
    crt="$(ls -d "$vs"/VC/Redist/MSVC/1*/x64/Microsoft.VC*.CRT 2>/dev/null | sort -V | tail -1)"
    if [ -z "$crt" ]; then
        echo "no encuentro el runtime de Visual C++ redistribuible en $vs/VC/Redist" >&2
        exit 1
    fi
    rm -rf "$runtime"
    mkdir -p "$runtime"
    cp "$crt/vcruntime140.dll" "$crt/vcruntime140_1.dll" "$runtime/"
    (cd "{{ app }}" && pnpm exec tauri build --bundles nsis --config "{{ root }}/packaging/windows/tauri.windows.json")
    salida="$CARGO_TARGET_DIR/release/bundle/nsis"
    instalador="$(find "$salida" -maxdepth 1 -type f -name '*-setup.exe' | sort | tail -1)"
    if [ -z "$instalador" ]; then
        echo "el bundler no produjo ningun instalador en $salida" >&2
        exit 1
    fi
    echo "nsis: $instalador ($(du -h "$instalador" | cut -f1))"

# Construye la .app y el .dmg de Apple Silicon con la .dylib en Contents/Frameworks (ADR-0040).
[macos]
[group('ci')]
[script('bash')]
bundle: check-native build-ts
    set -euo pipefail
    target="aarch64-apple-darwin"
    lib="{{ bridge }}/target/lib/rfirma/{{ native_lib_name }}"
    if ! lipo -archs "$lib" | grep -qw arm64; then
        echo "$lib no es arm64 ($(lipo -archs "$lib")): compila 'just native' en un Mac con Apple Silicon" >&2
        exit 1
    fi
    (cd "{{ app }}" && pnpm exec tauri build --target "$target" --bundles app,dmg --config "{{ root }}/packaging/macos/tauri.macos.json")
    salida="$CARGO_TARGET_DIR/$target/release/bundle"
    if [ ! -f "$salida/macos/rfirma.app/Contents/Frameworks/{{ native_lib_name }}" ]; then
        echo "la .app no lleva {{ native_lib_name }} en Contents/Frameworks" >&2
        exit 1
    fi
    codesign --verify --deep --strict "$salida/macos/rfirma.app"
    dmg="$(find "$salida/dmg" -maxdepth 1 -type f -name '*.dmg' | sort | tail -1)"
    if [ -z "$dmg" ]; then
        echo "el bundler no produjo ningun .dmg en $salida/dmg" >&2
        exit 1
    fi
    echo "dmg: $dmg ($(du -h "$dmg" | cut -f1))"

# Regenera cargo-sources.json y el sello de Cargo.lock.
[group('release')]
flatpak-sources:
    {{ root }}/scripts/flatpak-sources.sh

# Mutation testing incremental, a mano antes de publicar una version: no bloquea (ADR-0014).
[group('release')]
[script('bash')]
mutants:
    set -euo pipefail
    cd {{ tauri }}
    tag="$(git describe --tags --abbrev=0 --match 'v*')"
    diff="$(mktemp)"
    trap 'rm -f "$diff"' EXIT
    git diff --relative "$tag" -- . > "$diff"
    cargo mutants --in-place --in-diff "$diff" \
        --exclude 'adapters/tauri.rs' --exclude 'main.rs' --exclude '{{ ffi_allow }}'

# Lint, pruebas y construccion de la landing de rfirma.sgomez.me.
[group('ci')]
[script('bash')]
check-landing:
    set -euo pipefail
    cd {{ root }}
    pnpm install --frozen-lockfile --reporter=silent --filter . --filter rfirma-landing
    pnpm exec biome ci packaging/repo/site
    cd packaging/repo/site
    pnpm exec vitest run --reporter=dot
    pnpm exec astro build

# Comprueba que la version de la aplicacion y el nombre del producto cuadran en todos sus sitios.
[group('ci')]
check-version:
    {{ root }}/scripts/app_version.py check

# Resella el bundle del sistema de diseno.
[group('release')]
[script('bash')]
seal-ds-bundle:
    set -euo pipefail
    cd "{{ root }}"
    find rfirma-app/src/design-system/bundle -type f ! -name _ds_needs_recompile \
        | LC_ALL=C sort \
        | xargs sha256sum \
        > rfirma-app/src/design-system/bundle.lock
    echo
    echo "resellado. Versiona rfirma-app/src/design-system/bundle.lock."

# Arranca Storybook con las historias de la interfaz, en los cinco idiomas y los dos temas.
[group('dev')]
storybook: po-import
    cd {{ app }} && pnpm exec storybook dev --no-open -p 6006

# Corre axe en navegador, con contraste, sobre todas las historias en claro y en oscuro; bajo demanda, no entra en el CI.
[group('dev')]
storybook-a11y: po-import
    cd {{ app }} && node tools/storybook-a11y.mjs

# Abre la ventana con recarga en caliente; los argumentos van a la aplicacion.
[group('dev')]
dev *args: check-native po-import
    cd {{ app }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" pnpm exec tauri dev -- -- {{ args }}

# Registra (`on`) o quita (`off`) el manejador de desarrollo de afirma://.
[group('dev')]
dev-handler mode="on":
    RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" {{ root }}/scripts/dev-handler.sh {{ mode }}

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
    cd {{ root }} && pnpm install --frozen-lockfile --filter rfirma-conformance-console
    cd {{ conformance_suite }}/console && pnpm build

# Borra lo construido y los volcados de cobertura sueltos en el arbol de fuentes.
[group('dev')]
[script('bash')]
clean:
    set -eu
    cd "{{ bridge }}" && {{ maven }} clean
    if [ -z "{{ worktree_target }}" ]; then
        cd "{{ tauri }}" && cargo clean
    else
        rm -rf "{{ coverage_out }}"
        echo "worktree: el arbol compartido {{ cargo_target }} se queda"
    fi
    rm -f "{{ tauri }}"/*.profraw
    rm -rf "{{ app }}/dist" "{{ conformance_suite }}/console/dist"

# Sube <version> en Cargo.toml, Cargo.lock, el metainfo, el CHANGELOG y el sello de fuentes.
[group('release')]
[private]
bump-version version:
    {{ root }}/scripts/app_version.py bump {{ version }}

# Publica <version> desde main: changelog, bump, commit, etiqueta y push atomico.
[group('release')]
[script('bash')]
release version:
    set -euo pipefail
    cd "{{ root }}"
    version="{{ version }}"
    tag="v$version"
    fail() { echo "$1" >&2; exit 1; }
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "La versión tiene que ser X.Y.Z: $version"
    [ "$(git branch --show-current)" = main ] || fail "Se publica desde main."
    [ -z "$(git status --porcelain)" ] || fail "El árbol tiene cambios sin comitear."
    git fetch --quiet origin main
    [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || fail "main no coincide con origin/main."
    if git rev-parse --quiet --verify "refs/tags/$tag" >/dev/null; then fail "La etiqueta $tag ya existe en local."; fi
    if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null; then fail "La etiqueta $tag ya existe en origin."; fi
    scripts/app_version.py bump "$version"
    git add -A
    git commit --quiet -m "chore: version $version"
    git tag "$tag"
    echo
    git show --stat --format='%h %s' HEAD
    echo
    read -r -p "¿Subir main y $tag a origin? [s/N] " answer
    if [ "$answer" = s ] || [ "$answer" = S ]; then
        git push --atomic origin main "$tag"
    else
        echo "Sin subir. Para publicar: git push --atomic origin main $tag"
        echo "Para deshacer: git tag -d $tag && git reset --hard origin/main"
    fi
