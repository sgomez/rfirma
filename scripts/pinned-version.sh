#!/usr/bin/env bash
# Imprime el valor de una clave de `versions.env`, para los arranques que no pasan por `just`.
set -euo pipefail

key="${1:?uso: pinned-version.sh <CLAVE>}"
file="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/versions.env"
value="$(sed -n "s/^$key=//p" "$file" | tr -d '\r')"
if [ -z "$value" ]; then
    echo "versions.env no fija $key" >&2
    exit 1
fi
printf '%s\n' "$value"
