#!/usr/bin/env bash
# Comprueba, sin regenerarlas, que las fuentes de cargo del flatpak siguen al Cargo.lock (ADR-0013).
set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../.." && pwd)"
cd "$RAIZ"

for f in packaging/flatpak/cargo-sources.json packaging/flatpak/sources.lock; do
    if [ ! -s "$f" ]; then
        echo "falta $f (o esta vacio)" >&2
        echo "Ejecuta 'just flatpak-sources'." >&2
        exit 1
    fi
done

if ! sha256sum --check --status packaging/flatpak/sources.lock; then
    echo "cargo-sources.json NO esta al dia con Cargo.lock: el flatpak se construiria con las dependencias VIEJAS" >&2
    sha256sum --check packaging/flatpak/sources.lock >&2 || true
    echo "Ejecuta 'just flatpak-sources' y versiona lo que cambie." >&2
    exit 1
fi

# El sello se puede escribir a mano sin regenerar: hay que mirar el contenido.
faltan="$(comm -23 \
    <(awk -F'"' '/^name = /{n=$2} /^version = /{v=$2} /^source = "registry\+/{print n "-" v}' \
        rfirma-app/src-tauri/Cargo.lock | sort -u) \
    <(grep -o '"dest": "cargo/vendor/[^"]*"' packaging/flatpak/cargo-sources.json \
        | sed 's|.*cargo/vendor/||; s|"$||' | sort -u))"
if [ -n "$faltan" ]; then
    echo "cargo-sources.json no vendoriza estos crates de Cargo.lock:" >&2
    echo "$faltan" >&2
    echo "Ejecuta 'just flatpak-sources' y versiona lo que cambie." >&2
    exit 1
fi

echo "fuentes del flatpak al dia"
