#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
moved="$root/scripts/main-moved.sh"

run() {
    printf '{"id":%s,"head_sha":"%s","status":"%s","conclusion":"%s","display_title":"%s","created_at":"%s"}' "$@"
}

runs() {
    local IFS=,
    printf '{"workflow_runs":[%s]}' "$*"
}

expect() {
    local case="$1" expected="$2" json="$3"
    shift 3
    local actual
    actual="$(printf '%s' "$json" | "$moved" "$@" | grep '^moved=')"
    if [ "$actual" != "moved=$expected" ]; then
        echo "FALLO ($case): esperaba moved=$expected y salio $actual" >&2
        exit 1
    fi
}

now="$(run 9 cccc in_progress '' Nocturna 2026-10-03T02:00:00Z)"
yesterday="$(run 8 bbbb completed success Nocturna 2026-10-02T02:00:00Z)"
older="$(run 7 aaaa completed failure Nocturna 2026-10-01T02:00:00Z)"
cancelled="$(run 10 dddd completed cancelled Nocturna 2026-10-02T12:00:00Z)"
weekly="$(run 11 eeee completed success Semanal 2026-10-02T20:00:00Z)"

expect "primera nocturna" true "$(runs "$now")" cccc 9
expect "main no se ha movido" false "$(runs "$older" "$yesterday" "$now")" bbbb 9
expect "main se ha movido" true "$(runs "$yesterday" "$older" "$now")" cccc 9
expect "la cancelada no cuenta" false "$(runs "$yesterday" "$cancelled" "$now")" bbbb 9
expect "otro titulo no cuenta" false "$(runs "$yesterday" "$weekly" "$now")" bbbb 9 Nocturna
expect "sin titulo cuenta cualquiera" true "$(runs "$yesterday" "$weekly" "$now")" bbbb 9
expect "la API no respondio" true "" bbbb 9
expect "respuesta sin ejecuciones" true '{"message":"Not Found"}' bbbb 9

grep -q '^reason=.\+' <(runs "$yesterday" "$now" | "$moved" bbbb 9) || {
    echo "FALLO (motivo): la salida no dice por que" >&2
    exit 1
}

echo "main_moved_test: correcto"
