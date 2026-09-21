#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root/packaging/flatpak"
# Los dos generadores viven fuera de este repositorio: son de
# flatpak/flatpak-builder-tools. No se versionan aqui ni los instala
# bootstrap.sh; se traen a mano la primera vez.
tools="https://github.com/flatpak/flatpak-builder-tools"
command -v uv >/dev/null || {
    echo "falta uv: https://docs.astral.sh/uv/" >&2
    exit 1
}
if [ ! -f flatpak-cargo-generator.py ]; then
    echo "falta packaging/flatpak/flatpak-cargo-generator.py. Traelo con:" >&2
    echo "  curl -fsSL -o packaging/flatpak/flatpak-cargo-generator.py \\" >&2
    echo "    https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py" >&2
    exit 1
fi
command -v flatpak-node-generator >/dev/null || {
    echo "falta flatpak-node-generator. Instalalo con:" >&2
    echo "  uv tool install \"git+$tools.git#subdirectory=node\"" >&2
    exit 1
}
# `uv run --script` y no `python3`: el generador declara aiohttp y tomlkit en su
# cabecera, y el Python del sistema no los trae.
uv run --quiet --script flatpak-cargo-generator.py \
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
