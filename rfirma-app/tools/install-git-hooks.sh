#!/usr/bin/env bash
# Instala los hooks de lefthook, salvo desde un worktree.
#
# POR QUE EL WORKTREE SE QUEDA FUERA: git no da un .git/hooks por worktree, se
# comparte el del checkout principal. Un agente que corriera `pnpm install` en
# su worktree reescribia ahi la ruta absoluta del binario de lefthook apuntando
# a su propio node_modules; al limpiarse el worktree la ruta moria y el push de
# la persona usuaria pasaba a saludar con «Can't find lefthook in PATH» sin
# ejecutar ninguna comprobacion. Los worktrees no pierden nada: heredan el hook
# ya instalado, y lefthook resuelve su lefthook.yml desde el toplevel de cada
# uno.
set -euo pipefail

own=$(git rev-parse --path-format=absolute --git-dir 2>/dev/null || true)
common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)

if [ -z "$own" ]; then
    echo "aviso: esto no es un repositorio git; me salto la instalacion de hooks"
    exit 0
fi

if [ "$own" != "$common" ]; then
    echo "aviso: worktree; los hooks son del checkout principal y no los toco"
    exit 0
fi

exec lefthook install
