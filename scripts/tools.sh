#!/usr/bin/env bash
set -euo pipefail

platform="${PLATFORM:?}"
ruff_version="${RUFF_VERSION:?}"
crap_version="${CRAP_VERSION:?}"
machete_version="${MACHETE_VERSION:?}"
default_graalvm="${DEFAULT_GRAALVM:?}"
system_libs="${SYSTEM_LIBS:?}"

# `orden:paquete` que la trae.
declare -A required=(
    [linux]="msgfmt:gettext msgmerge:gettext msgcmp:gettext msgattrib:gettext softhsm2-util:softhsm2 pkcs11-tool:opensc openssl:openssl certutil:libnss3-tools pk12util:libnss3-tools pkg-config:pkg-config"
    [windows]=""
)
declare -A probes=(
    [linux]="probe_system_libs"
    [windows]="probe_msvc probe_webview2 probe_native_perl"
)
# `orden:receta` que la usa.
declare -A optional=(
    [linux]="flatpak-builder:flatpak"
    [windows]="msgfmt:po msgmerge:po msgcmp:po msgattrib:po"
)

probe_system_libs() {
    command -v pkg-config >/dev/null || return 0
    local pair module
    for pair in $system_libs; do
        module="${pair%%:*}"
        pkg-config --exists "$module" || {
            echo "falta la libreria de sistema: $module"
            missing_packages="$missing_packages ${pair#*:}"
            failures=1
        }
    done
}

probe_msvc() {
    local vswhere="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
    local cl=""
    if [ -x "$vswhere" ]; then
        cl="$("$vswhere" -latest -products '*' \
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 \
            -find 'VC/Tools/MSVC/**/bin/Hostx64/x64/cl.exe' 2>/dev/null || true)"
    fi
    if [ -z "$cl" ]; then
        echo "falta: cl.exe (MSVC)"
        echo "  Instala Visual Studio Build Tools 2022 con \"Desarrollo para el escritorio con C++\""
        failures=1
    fi
}

probe_webview2() {
    local key
    for key in 'HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' \
        'HKCU\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'; do
        if MSYS2_ARG_CONV_EXCL='*' reg query "$key" /v pv 2>/dev/null | grep -q 'pv .*[1-9]'; then
            return 0
        fi
    done
    echo "falta: el runtime de WebView2"
    echo "  Instalalo desde https://developer.microsoft.com/microsoft-edge/webview2/"
    failures=1
}

probe_native_perl() {
    # El Perl de Git for Windows es de MSYS, y el Configure de OpenSSL lo rechaza (ADR-0035).
    local perl="${OPENSSL_SRC_PERL:-perl}"
    [ "$("$perl" -e 'print $^O' 2>/dev/null || true)" = "MSWin32" ] && return 0
    echo "falta: un Perl nativo de Windows para compilar OpenSSL"
    if [ -x /c/Strawberry/perl/bin/perl.exe ]; then
        echo "  Strawberry Perl esta instalado, pero este bash ve antes el de Git:"
        echo "    export OPENSSL_SRC_PERL=C:/Strawberry/perl/bin/perl.exe"
    else
        echo "  Instala Strawberry Perl (https://strawberryperl.com)"
    fi
    failures=1
}

if [ -z "${probes[$platform]+x}" ]; then
    echo "tools: no hay tabla de herramientas para $platform" >&2
    exit 1
fi

failures=0
missing_packages=""
for t in mvn git java pnpm cargo; do
    command -v "$t" >/dev/null || { echo "falta: $t"; failures=1; }
done
if ! command -v cargo >/dev/null && [ -x "$HOME/.cargo/bin/cargo" ]; then
    echo "  cargo esta en ~/.cargo/bin pero no en el PATH:"
    echo "    anade '. \"$HOME/.cargo/env\"' a tu ~/.zshrc (o ~/.bashrc)"
fi
for row in ${required[$platform]}; do
    command -v "${row%%:*}" >/dev/null || {
        echo "falta: ${row%%:*}"
        missing_packages="$missing_packages ${row#*:}"
        failures=1
    }
done
for probe in ${probes[$platform]}; do
    "$probe"
done
if [ -n "$missing_packages" ]; then
    echo
    echo "Instalalos con:"
    echo "  sudo apt install -y $(printf '%s\n' $missing_packages | sort -u | xargs)"
    echo
fi
if ! command -v ruff >/dev/null; then
    echo "falta: ruff"
    echo "  Instalalo con: pipx install ruff==$ruff_version  (o: uv tool install ruff==$ruff_version)"
    failures=1
fi
for row in ${optional[$platform]}; do
    command -v "${row%%:*}" >/dev/null ||
        echo "aviso: falta ${row%%:*} (solo hace falta para 'just ${row#*:}')"
done
graal="${GRAALVM_HOME:-$default_graalvm}"
if command -v cygpath >/dev/null; then graal="$(cygpath -u "$graal")"; fi
if [ ! -x "$graal/bin/native-image" ] && [ ! -f "$graal/bin/native-image.cmd" ]; then
    echo "aviso: falta native-image en $graal"
    echo "  (solo hace falta para 'just native'; instala GraalVM CE 25)"
fi
cargo llvm-cov --version >/dev/null 2>&1 ||
    echo "aviso: falta cargo-llvm-cov (cargo binstall cargo-llvm-cov)"
cargo nextest --version >/dev/null 2>&1 ||
    echo "aviso: falta cargo-nextest (solo hace falta para 'just test-native'; cargo binstall cargo-nextest)"
cargo crap --version >/dev/null 2>&1 ||
    echo "aviso: falta cargo-crap (cargo binstall cargo-crap@$crap_version)"
cargo machete --version >/dev/null 2>&1 ||
    echo "aviso: falta cargo-machete (cargo binstall cargo-machete@$machete_version)"
cargo mutants --version >/dev/null 2>&1 ||
    echo "aviso: falta cargo-mutants (solo hace falta para 'just mutants'; cargo binstall cargo-mutants)"
[ "$failures" = 0 ] || exit 1
echo "herramientas: correcto"
