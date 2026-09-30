#!/usr/bin/env bash
# Comprueba las herramientas del entorno, y falla nombrando la que falte o no este en su version fijada.
set -euo pipefail

ruff_version="${RUFF_VERSION:?}"
default_graalvm="${DEFAULT_GRAALVM:?}"
system_libs="${SYSTEM_LIBS:?}"

check_pinned() {
    local name="$1" expected="$2" found
    shift 2
    found="$("$@" 2>/dev/null | grep -oE '[0-9]+(\.[0-9]+)+' | head -n1 || true)"
    if [ -n "$found" ] && [ "$found" != "$expected" ]; then
        echo "version distinta: $name $found, la fijada es $expected (just install-tools)"
        failures=1
    fi
}

# softhsm y NSS no entran: la grada B no corre en Windows (ADR-0035).
check_windows() {
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
    local webview2="" key
    for key in 'HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' \
        'HKCU\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'; do
        if MSYS2_ARG_CONV_EXCL='*' reg query "$key" /v pv 2>/dev/null | grep -q 'pv .*[1-9]'; then
            webview2=1
        fi
    done
    if [ -z "$webview2" ]; then
        echo "falta: el runtime de WebView2"
        echo "  Instalalo desde https://developer.microsoft.com/microsoft-edge/webview2/"
        failures=1
    fi
    # El Perl de Git es de MSYS y el Configure de OpenSSL lo rechaza (ADR-0035).
    local perl="${OPENSSL_SRC_PERL:-perl}"
    if [ "$("$perl" -e 'print $^O' 2>/dev/null || true)" != "MSWin32" ]; then
        echo "falta: un Perl nativo de Windows para compilar OpenSSL"
        if [ -x /c/Strawberry/perl/bin/perl.exe ]; then
            echo "  Strawberry Perl esta instalado, pero este bash ve antes el de Git:"
            echo "    export OPENSSL_SRC_PERL=C:/Strawberry/perl/bin/perl.exe"
        else
            echo "  Instala Strawberry Perl (https://strawberryperl.com)"
        fi
        failures=1
    fi
    local t
    for t in msgfmt msgmerge msgcmp msgattrib; do
        command -v "$t" >/dev/null ||
            echo "aviso: falta $t (solo hace falta para 'just po' y 'check-po')"
    done
}

failures=0
for t in mvn git java pnpm cargo; do
    command -v "$t" >/dev/null || { echo "falta: $t"; failures=1; }
done
if ! command -v cargo >/dev/null && [ -x "$HOME/.cargo/bin/cargo" ]; then
    echo "  cargo esta en ~/.cargo/bin pero no en el PATH:"
    echo "    anade '. \"$HOME/.cargo/env\"' a tu ~/.zshrc (o ~/.bashrc)"
fi
on_windows=""
case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*)
    on_windows=1
    check_windows
    ;;
*)
    gettext_apt=""
    for t in msgfmt msgmerge msgcmp msgattrib; do
        command -v "$t" >/dev/null || { echo "falta: $t"; gettext_apt="gettext"; failures=1; }
    done
    if [ -n "$gettext_apt" ]; then
        echo
        echo "Instalalo con:"
        echo "  sudo apt install -y $gettext_apt"
        echo
    fi
    softhsm_apt=""
    command -v softhsm2-util >/dev/null || { echo "falta: softhsm2-util"; softhsm_apt="$softhsm_apt softhsm2"; failures=1; }
    command -v pkcs11-tool  >/dev/null || { echo "falta: pkcs11-tool";  softhsm_apt="$softhsm_apt opensc";   failures=1; }
    command -v openssl      >/dev/null || { echo "falta: openssl";      softhsm_apt="$softhsm_apt openssl";  failures=1; }
    command -v certutil     >/dev/null || { echo "falta: certutil";     softhsm_apt="$softhsm_apt libnss3-tools"; failures=1; }
    command -v pk12util     >/dev/null || { echo "falta: pk12util";     softhsm_apt="$softhsm_apt libnss3-tools"; failures=1; }
    if [ -n "$softhsm_apt" ]; then
        echo
        echo "Instalalos con:"
        echo "  sudo apt install -y$softhsm_apt"
        echo "y monta el token con: just certs install"
        echo
    fi
    # pkg-config, y no el gestor de paquetes: falta el -dev aunque este el de runtime.
    if command -v pkg-config >/dev/null; then
        missing_apt=""
        for pair in $system_libs; do
            module="${pair%%:*}"
            package="${pair#*:}"
            pkg-config --exists "$module" || {
                echo "falta la libreria de sistema: $module"
                missing_apt="$missing_apt $package"
                failures=1
            }
        done
        if [ -n "$missing_apt" ]; then
            echo
            echo "Instalalas con:"
            echo "  sudo apt install -y$missing_apt"
            echo
        fi
    else
        echo "falta: pkg-config"
        failures=1
    fi
    ;;
esac
if ! command -v ruff >/dev/null; then
    echo "falta: ruff"
    echo "  Instalalo con: just install-tools ruff"
    failures=1
fi
check_pinned ruff "$ruff_version" ruff --version
check_pinned just "${JUST_VERSION:?}" just --version
check_pinned diff-cover "${DIFF_COVER_VERSION:?}" diff-cover --version
check_pinned cargo-crap "${CRAP_VERSION:?}" cargo crap --version
check_pinned cargo-machete "${MACHETE_VERSION:?}" cargo machete --version
check_pinned cargo-nextest "${NEXTEST_VERSION:?}" cargo nextest --version
graal="${GRAALVM_HOME:-$default_graalvm}"
if command -v cygpath >/dev/null; then graal="$(cygpath -u "$graal")"; fi
if [ ! -x "$graal/bin/native-image" ] && [ ! -f "$graal/bin/native-image.cmd" ]; then
    echo "aviso: falta native-image en $graal"
    echo "  (solo hace falta para 'just native'; instala GraalVM CE 25)"
fi
[ -n "$on_windows" ] || command -v flatpak-builder >/dev/null || \
    echo "aviso: falta flatpak-builder (solo hace falta para 'just flatpak')"
cargo llvm-cov --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-llvm-cov (cargo binstall cargo-llvm-cov)"
cargo nextest --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-nextest (solo hace falta para 'just test-native'; just install-tools nextest)"
cargo crap --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-crap (just install-tools crap)"
cargo machete --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-machete (just install-tools machete)"
cargo mutants --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-mutants (solo hace falta para 'just mutants'; cargo binstall cargo-mutants)"
[ "$failures" = 0 ] || exit 1
echo "herramientas: correcto"
