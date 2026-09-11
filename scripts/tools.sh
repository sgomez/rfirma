#!/usr/bin/env bash
set -euo pipefail

ruff_version="${RUFF_VERSION:?}"
crap_version="${CRAP_VERSION:?}"
default_graalvm="${DEFAULT_GRAALVM:?}"
system_libs="${SYSTEM_LIBS:?}"

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
    echo "y monta el token con: just token"
    echo
fi
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
# Opcionales: no rompen `check`, pero si la receta que los usa.
graal="${GRAALVM_HOME:-$default_graalvm}"
if [ ! -x "$graal/bin/native-image" ]; then
    echo "aviso: falta native-image en $graal"
    echo "  (solo hace falta para 'just native'; instala GraalVM CE 25)"
fi
command -v flatpak-builder >/dev/null || \
    echo "aviso: falta flatpak-builder (solo hace falta para 'just flatpak')"
cargo llvm-cov --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-llvm-cov (cargo binstall cargo-llvm-cov)"
cargo crap --version >/dev/null 2>&1 || \
    echo "aviso: falta cargo-crap (cargo binstall cargo-crap@$crap_version)"
[ "$failures" = 0 ] || exit 1
echo "herramientas: correcto"
