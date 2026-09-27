#!/usr/bin/env bash
# Comprueba el formato de una cadena (`rust`, `ts` o `python`) para el pre-push de lefthook; se salta si falta su herramienta.
set -euo pipefail

case "${1:-}" in
    rust)
        export PATH="$HOME/.cargo/bin:$PATH"
        if ! command -v cargo >/dev/null; then
            echo "aviso: cargo no esta disponible; me salto el formato de Rust"
            exit 0
        fi
        cd rfirma-app/src-tauri && cargo fmt --all -- --check
        ;;
    ts)
        cd rfirma-app
        if ! command -v pnpm >/dev/null || [ ! -x node_modules/.bin/biome ]; then
            echo "aviso: biome no esta instalado (just deps); me salto el formato de TypeScript"
            exit 0
        fi
        pnpm exec biome format .
        ;;
    python)
        export PATH="$HOME/.local/bin:$PATH"
        if ! command -v ruff >/dev/null; then
            echo "aviso: ruff no esta disponible; me salto el formato de Python"
            exit 0
        fi
        ruff format --check packaging
        ;;
    *)
        echo "uso: $0 rust|ts|python" >&2
        exit 2
        ;;
esac
