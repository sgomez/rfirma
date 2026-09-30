#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root/packaging/flatpak"
# El generador vive fuera del repositorio (flatpak/flatpak-builder-tools) y se trae a mano.
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
# `uv run --script` y no `python3`: el generador declara aiohttp y tomlkit en su
# cabecera, y el Python del sistema no los trae.
uv run --quiet --script flatpak-cargo-generator.py \
    ../../rfirma-app/src-tauri/Cargo.lock -o cargo-sources.json
cd "$root"
sha256sum rfirma-app/src-tauri/Cargo.lock > packaging/flatpak/sources.lock
echo
echo "regeneradas. Versiona cargo-sources.json y sources.lock."
