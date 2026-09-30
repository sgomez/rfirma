#!/usr/bin/env bash
# Instala las herramientas en la version que fija `versions.env`: las nombradas, o todas.
set -euo pipefail

install() {
    case "$1" in
        ruff) pipx install --force "ruff==${RUFF_VERSION:?}" ;;
        diff-cover) pipx install --force "diff-cover==${DIFF_COVER_VERSION:?}" ;;
        crap) cargo binstall --no-confirm --force "cargo-crap@${CRAP_VERSION:?}" ;;
        machete) cargo binstall --no-confirm --force "cargo-machete@${MACHETE_VERSION:?}" ;;
        nextest) cargo binstall --no-confirm --force "cargo-nextest@${NEXTEST_VERSION:?}" ;;
        *)
            echo "herramienta sin version fijada: $1 (ruff, diff-cover, crap, machete, nextest)" >&2
            exit 1
            ;;
    esac
}

if [ $# -eq 0 ]; then
    set -- ruff diff-cover crap machete nextest
fi
for tool in "$@"; do
    install "$tool"
done
