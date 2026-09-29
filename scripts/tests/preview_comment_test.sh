#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
script="$root/scripts/preview-comment.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "FALLO ($1)" >&2
    exit 1
}

manifest="$tmp/paquetes.json"
cat > "$manifest" << 'JSON'
[
  {"platform": "windows", "file": "a-setup.exe", "format": "nsis", "signable": false, "warning": "Aviso de prueba de windows."},
  {"platform": "linux", "file": "a.deb", "format": "deb", "signable": false, "warning": ""},
  {"platform": "flatpak", "file": "a.flatpak", "format": "flatpak", "signable": false, "warning": "Aviso de prueba de flatpak."},
  {"platform": "solaris", "file": "a.pkg", "format": "pkg", "signable": false, "warning": "Aviso de una plataforma ausente."}
]
JSON

tab=$'\t'
four="rfirma-preview-windows${tab}11${tab}52428800${tab}2026-10-13T10:00:00Z
rfirma-preview-linux${tab}12${tab}31457280${tab}2026-10-13T10:00:00Z
rfirma-preview-flatpak${tab}13${tab}10485760${tab}2026-10-13T10:00:00Z
rfirma-preview-macos${tab}14${tab}1048576${tab}2026-10-13T10:00:00Z
paquetes${tab}15${tab}100${tab}2026-10-13T10:00:00Z
pdf-puerta-manual${tab}16${tab}2048${tab}2026-10-13T10:00:00Z"

compose() {
    local conclusion="$1" input="$2"
    printf '%s\n' "$input" | REPO=o/r RUN_ID=99 RUN_URL=https://example.test/run/99 \
        CONCLUSION="$conclusion" SHA=0123456789abcdef "$script" "$manifest"
}

marker="<!-- rfirma-preview-comment -->"

ok="$(compose success "$four")"
[ "$(head -1 <<< "$ok")" = "$marker" ] || fail "el marcador va en la primera línea"
[ "$(grep -c '^| [a-z]* | \[rfirma-preview' <<< "$ok")" = 4 ] || fail "cuatro artefactos, cuatro filas"
grep -q '^| windows | \[rfirma-preview-windows\](https://github.com/o/r/actions/runs/99/artifacts/11) | 50 MB |$' <<< "$ok" \
    || fail "fila con enlace y tamaño redondeado"
grep -q 'Caducan el 2026-10-13' <<< "$ok" || fail "fecha de caducidad"
grep -q '`0123456`' <<< "$ok" || fail "sha corto"
grep -q 'https://example.test/run/99' <<< "$ok" || fail "enlace a la ejecución"
grep -q 'sesión' <<< "$ok" || fail "nota de la sesión de GitHub"
grep -q 'artifacts/16' <<< "$ok" || fail "enlace al PDF de la puerta manual"
grep -q 'cada push' <<< "$ok" || fail "nota de reconstrucción"
grep -q 'Aviso de prueba de windows.' <<< "$ok" || fail "aviso de una fila del manifiesto"
grep -q 'Aviso de prueba de flatpak.' <<< "$ok" || fail "aviso de flatpak"
! grep -q 'solaris\|plataforma ausente' <<< "$ok" || fail "plataforma sin artefacto: ni fila ni aviso"

without_windows="$(compose success "$(grep -v 'preview-windows' <<< "$four")")"
[ "$(head -1 <<< "$without_windows")" = "$marker" ] || fail "el marcador va primero sin windows"
! grep -q 'windows' <<< "$without_windows" || fail "sin artefacto de windows no hay fila ni aviso"

failed="$(compose failure "$four")"
[ "$(head -1 <<< "$failed")" = "$marker" ] || fail "el marcador va primero al fallar"
grep -q 'fallado' <<< "$failed" || fail "frase de fallo"
grep -q 'https://example.test/run/99' <<< "$failed" || fail "enlace a los registros"
! grep -q '^|' <<< "$failed" || fail "un fallo no lleva tabla"

[ -z "$(compose success "paquetes${tab}15${tab}100${tab}2026-10-13T10:00:00Z")" ] || fail "éxito sin artefactos de preview: salida vacía"
[ -z "$(compose success "")" ] || fail "éxito sin ningún artefacto: salida vacía"
[ -z "$(compose cancelled "$four")" ] || fail "cancelada: salida vacía"
