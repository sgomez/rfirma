#!/usr/bin/env bash
set -euo pipefail

lib="$1"
if [ "${RFIRMA_SKIP_NATIVE:-0}" = "1" ]; then
    echo "check-native: omitida (RFIRMA_SKIP_NATIVE=1)"
    exit 0
fi
if [ ! -f "$lib" ]; then
    echo "falta la libreria nativa:" >&2
    echo "  $lib" >&2
    echo >&2
    echo "Ejecuta 'just native' (tarda unos tres minutos y necesita" >&2
    echo "GraalVM CE 25). No se construye sola a proposito: ver ADR-0013." >&2
    exit 1
fi
