#!/usr/bin/env bash
set -euo pipefail

ruff_version="${RUFF_VERSION:?}"
crap_version="${CRAP_VERSION:?}"
machete_version="${MACHETE_VERSION:?}"
default_graalvm="${DEFAULT_GRAALVM:?}"
system_libs="${SYSTEM_LIBS:?}"

# Windows (ADR-0035): el WebView es WebView2, OpenSSL se compila con el
# Perl de Strawberry y todo se enlaza con MSVC. softhsm y NSS no entran: la
# grada B no corre en Windows.
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
    # El Perl de Git for Windows es de MSYS y el Configure de OpenSSL lo
    # rechaza; openssl-src usa OPENSSL_SRC_PERL o, si no, el primer perl.
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
# Un cargo instalado pero fuera del PATH es el falso negativo mas caro de
# esta receta: "falta: cargo" manda a reinstalar rustup a quien solo tiene
# que cargar el env. rustup lo deja en ~/.cargo/env, que ~/.profile carga
# y zsh NO lee en shells interactivas.
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
    # gettext es DEPENDENCIA REQUERIDA desde v0.3: las cadenas viven en
    # rfirma-app/po/ y msgmerge es la bisagra entre la plantilla y los cinco .po.
    # El importador NO lo necesita —es Node puro— asi que un clon limpio compila
    # sin esto; lo necesita quien DESARROLLA y lo necesita el CI.
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
    # El token de la grada B (ADR-0014). No es opcional: sus pruebas corren en el
    # carril rapido, asi que sin estas tres ordenes `test-rust` falla.
    softhsm_apt=""
    command -v softhsm2-util >/dev/null || { echo "falta: softhsm2-util"; softhsm_apt="$softhsm_apt softhsm2"; failures=1; }
    command -v pkcs11-tool  >/dev/null || { echo "falta: pkcs11-tool";  softhsm_apt="$softhsm_apt opensc";   failures=1; }
    command -v openssl      >/dev/null || { echo "falta: openssl";      softhsm_apt="$softhsm_apt openssl";  failures=1; }
    # El almacen NSS es la otra mitad de la grada B: certutil y pk12util montan
    # el perfil desechable de cada prueba, y libsoftokn3.so es el modulo que lo
    # abre. El perfil real de Firefox de nadie interviene.
    command -v certutil     >/dev/null || { echo "falta: certutil";     softhsm_apt="$softhsm_apt libnss3-tools"; failures=1; }
    command -v pk12util     >/dev/null || { echo "falta: pk12util";     softhsm_apt="$softhsm_apt libnss3-tools"; failures=1; }
    if [ -n "$softhsm_apt" ]; then
        echo
        echo "Instalalos con:"
        echo "  sudo apt install -y$softhsm_apt"
        echo "y monta el token con: just certs install"
        echo
    fi
    # Las librerias de sistema del WebView. pkg-config es quien decide, porque es
    # quien consulta el build script que falla: el paquete de runtime puede estar
    # instalado y faltar solo el -dev, que es el que trae el .pc.
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
# ruff es la puerta del unico Python del repositorio y va dentro de
# `check-repo`, asi que sin el la cadena de TypeScript falla entera. No esta
# en apt: se instala desde PyPI. La version va clavada, igual que
# crap_version mas abajo: sin ruff.toml ni pyproject.toml en el repositorio,
# el conjunto de reglas por defecto es el que traiga la version instalada, y
# una version distinta a la del CI puede poner esta puerta en rojo sin que
# nadie haya tocado una linea de Python.
if ! command -v ruff >/dev/null; then
    echo "falta: ruff"
    echo "  Instalalo con: pipx install ruff==$ruff_version  (o: uv tool install ruff==$ruff_version)"
    failures=1
fi
# Opcionales: no rompen `check`, pero si la receta que los usa.
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
    echo "aviso: falta cargo-nextest (solo hace falta para 'just test-native'; cargo binstall cargo-nextest)"
cargo crap --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-crap (cargo binstall cargo-crap@$crap_version)"
cargo machete --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-machete (cargo binstall cargo-machete@$machete_version)"
cargo mutants --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-mutants (solo hace falta para 'just mutants'; cargo binstall cargo-mutants)"
[ "$failures" = 0 ] || exit 1
echo "herramientas: correcto"
