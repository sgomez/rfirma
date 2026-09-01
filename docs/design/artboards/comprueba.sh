#!/usr/bin/env bash
# Comprueba que los doce artboards comparten el mismo <helmet> byte a byte.
# Si uno diverge, la transcripcion estaria copiando dos sistemas de diseno
# distintos sin saberlo.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
huellas=$(for f in *.dc.html; do
    sed -n '/<helmet>/,/<\/helmet>/p' "$f" | sha256sum | cut -d' ' -f1
done | sort -u | wc -l)
if [ "$huellas" -ne 1 ]; then
    echo "ERROR: los artboards NO comparten el mismo <helmet> ($huellas variantes)" >&2
    exit 1
fi
echo "OK: $(ls -1 *.dc.html | wc -l) artboards con un <helmet> identico"
