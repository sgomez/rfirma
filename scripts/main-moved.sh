#!/usr/bin/env bash
# Lee por stdin las ejecuciones de un workflow (la API de runs) y dice si `main` se ha movido desde la anterior.
#
# Uso: gh api .../runs | main-moved.sh SHA RUN_ID [TITULO]
# Con TITULO solo cuentan las ejecuciones con ese display_title. Sin respuesta legible, se ha movido.
set -euo pipefail

sha="$1"
run_id="$2"
title="${3:-}"

previous="$(jq -r --argjson id "$run_id" --arg title "$title" '
    [(.workflow_runs // [])[]
        | select(.id != $id and .status == "completed" and .conclusion != "cancelled")
        | select($title == "" or .display_title == $title)]
    | sort_by(.created_at) | last | .head_sha // ""
' 2>/dev/null || true)"

if [ -z "$previous" ]; then
    echo "moved=true"
    echo "reason=no hay una ejecucion anterior con la que comparar"
elif [ "$previous" = "$sha" ]; then
    echo "moved=false"
    echo "reason=main sigue en ${sha:0:7}, el mismo commit que la ejecucion anterior"
else
    echo "moved=true"
    echo "reason=main ha pasado de ${previous:0:7} a ${sha:0:7}"
fi
