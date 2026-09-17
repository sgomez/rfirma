#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root/packaging/flatpak"
# Los dos generadores viven fuera de este repositorio: son de
# flatpak/flatpak-builder-tools. No se versionan aqui ni los instala
# bootstrap.sh; se traen a mano la primera vez.
if [ ! -f flatpak-cargo-generator.py ]; then
    echo "falta packaging/flatpak/flatpak-cargo-generator.py" >&2
    echo "  https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo" >&2
    exit 1
fi
command -v flatpak-node-generator >/dev/null || {
    echo "falta flatpak-node-generator" >&2
    echo "  https://github.com/flatpak/flatpak-builder-tools/tree/master/node" >&2
    exit 1
}
python3 flatpak-cargo-generator.py \
    ../../rfirma-app/src-tauri/Cargo.lock -o cargo-sources.json
flatpak-node-generator pnpm ../../rfirma-app/pnpm-lock.yaml -o node-sources.json
# El sello que lee `packaging/flatpak/check-sources.sh`: el sha256 de cada fichero de
# bloqueo TAL Y COMO estaba al generar los JSON de arriba. Se escribe en el
# formato de sha256sum para que comprobarlo sea `sha256sum -c` y no un
# analizador nuestro. Las rutas van relativas a la raiz del repositorio, que
# es desde donde comprueba el script.
cd "$root"
sha256sum rfirma-app/src-tauri/Cargo.lock rfirma-app/pnpm-lock.yaml \
    > packaging/flatpak/sources.lock
echo
echo "regeneradas. Versiona cargo-sources.json, node-sources.json y sources.lock."
