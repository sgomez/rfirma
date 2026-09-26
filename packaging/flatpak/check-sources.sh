#!/usr/bin/env bash
# Comprueba, sin regenerarlas, que las fuentes vendorizadas del flatpak siguen a los ficheros de bloqueo (ADR-0013).
set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../.." && pwd)"
cd "$RAIZ"

fallo=0

for f in packaging/flatpak/cargo-sources.json packaging/flatpak/node-sources.json; do
    if [ ! -s "$f" ]; then
        echo "falta $f (o esta vacio)" >&2
        fallo=1
    fi
done

if [ ! -f packaging/flatpak/sources.lock ]; then
    echo "falta packaging/flatpak/sources.lock" >&2
    fallo=1
fi

if [ "$fallo" -ne 0 ]; then
    echo >&2
    echo "El flatpak se construye SIN red (ADR-0013): esos ficheros son las" >&2
    echo "dependencias vendorizadas y van versionados." >&2
    echo "Ejecuta 'just flatpak-sources'." >&2
    exit 1
fi

# --status para no imprimir una linea "OK" por fichero: aqui solo interesa el
# fallo, y el mensaje util lo damos nosotros.
if ! sha256sum --check --status packaging/flatpak/sources.lock; then
    echo "las fuentes vendorizadas del flatpak NO estan al dia" >&2
    echo >&2
    # Sin --status, para que se vea CUAL de los dos ficheros de bloqueo ha
    # cambiado: es la unica informacion que hace falta para arreglarlo.
    sha256sum --check packaging/flatpak/sources.lock >&2 || true
    echo >&2
    echo "Un fichero de bloqueo ha cambiado y cargo-sources.json /" >&2
    echo "node-sources.json siguen siendo los de antes, asi que el flatpak" >&2
    echo "se construiria con las dependencias VIEJAS." >&2
    echo >&2
    echo "Ejecuta 'just flatpak-sources' y versiona lo que cambie." >&2
    exit 1
fi

echo "fuentes del flatpak al dia"
