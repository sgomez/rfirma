#!/usr/bin/env bash
set -euo pipefail

suelo="2.34"
lib="$1"
if [ ! -f "$lib" ]; then
    echo "no existe $lib; ejecuta 'just native'" >&2
    exit 1
fi
maximo="$(objdump -T "$lib" | grep -oE 'GLIBC_[0-9]+\.[0-9]+(\.[0-9]+)?' \
    | sed 's/^GLIBC_//' | sort -V | tail -1 || true)"
if [ -z "$maximo" ]; then
    echo "objdump no encontro ningun simbolo GLIBC_* en $lib" >&2
    exit 1
fi
echo "GLIBC_* maximo en $lib: $maximo (suelo prometido: $suelo)"
mayor="$(printf '%s\n%s\n' "$suelo" "$maximo" | sort -V | tail -1)"
if [ "$mayor" != "$suelo" ]; then
    echo "SUBE el suelo de glibc: $maximo > $suelo" >&2
    echo "revisa docs/research/glibc-libreria-nativa.md; si el suelo ha" >&2
    echo "subido de verdad, sube el pin de esta receta a la vez" >&2
    exit 1
fi
echo "OK  dentro del suelo prometido"
