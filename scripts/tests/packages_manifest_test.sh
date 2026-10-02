#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
script="$root/scripts/packages-manifest.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "FALLO ($1)" >&2
    exit 1
}

full="$tmp/full"
mkdir "$full"
for f in me.sgomez.rfirma.flatpak rfirma_1.0.0_amd64.deb rfirma-1.0.0-1.x86_64.rpm rfirma_1.0.0_x64-setup.exe rfirma_1.0.0_aarch64.dmg SHA256SUMS; do
    : > "$full/$f"
done
"$script" write "$full"
[ "$(jq length "$full/paquetes.json")" = 5 ] || fail "entrega completa: una fila por paquete"
[ "$(jq -r '.[] | select(.format == "nsis") | .platform' "$full/paquetes.json")" = windows ] \
    || fail "entrega completa: plataforma del instalador"
[ "$(jq -r '.[] | select(.format == "dmg") | .platform' "$full/paquetes.json")" = macos ] \
    || fail "entrega completa: plataforma del dmg"
jq -e 'all(.[]; has("platform") and has("file") and has("format") and has("signable") and has("warning"))' \
    "$full/paquetes.json" > /dev/null || fail "entrega completa: campos de cada fila"

[ "$("$script" signable "$full")" = "rfirma-1.0.0-1.x86_64.rpm" ] || fail "solo el rpm es firmable"
[ "$("$script" files "$full" deb)" = "rfirma_1.0.0_amd64.deb" ] || fail "filtro por formato"
[ "$("$script" files "$full" | wc -l | tr -d ' ')" = 5 ] || fail "files lista los paquetes y no el SHA256SUMS"

[ "$("$script" platforms "$full" | tr '\n' ' ')" = "flatpak linux macos windows " ] || fail "platforms: las del manifiesto, sin repetir"
[ "$("$script" platform-files "$full" linux | wc -l | tr -d ' ')" = 2 ] || fail "platform-files: deb y rpm de linux"
[ -z "$("$script" platform-files "$full" ios)" ] || fail "platform-files: una plataforma sin filas no lista nada"

notes="$("$script" notes "$full")"
grep -qF '## Instalación' <<< "$notes" || fail "notas: falta la seccion de instalacion"
grep -qF 'flatpak remote-add' <<< "$notes" || fail "notas: el aviso del flatpak"
grep -qF 'SmartScreen' <<< "$notes" || fail "notas: el aviso de windows"
! grep -qF 'rfirma_1.0.0_amd64.deb' <<< "$notes" || fail "notas: un paquete sin aviso no aparece"
jq '(.[] | select(.format == "deb") | .warning) = "Aviso nuevo del deb."' "$full/paquetes.json" > "$tmp/m.json"
mv "$tmp/m.json" "$full/paquetes.json"
grep -qF 'Aviso nuevo del deb.' <<< "$("$script" notes "$full")" || fail "notas: un cambio del manifiesto llega a las notas"

rc="$tmp/rc"
mkdir "$rc"
: > "$rc/me.sgomez.rfirma.flatpak"
: > "$rc/rfirma_1.0.0-rc.1_x64-setup.exe"
"$script" write "$rc"
[ "$(jq length "$rc/paquetes.json")" = 2 ] || fail "candidata: dos filas"
[ "$("$script" platforms "$rc" | tr '\n' ' ')" = "flatpak windows " ] || fail "candidata: sin plataforma linux"
[ -z "$("$script" files "$rc" deb)$("$script" files "$rc" rpm)" ] || fail "candidata: sin filas deb ni rpm"
"$script" write "$rc"
[ "$(jq length "$rc/paquetes.json")" = 2 ] || fail "reescribir no cuenta el manifiesto como paquete"

signed="$tmp/signed"
mkdir "$signed"
for f in me.sgomez.rfirma.flatpak rfirma_1.0.0_x64-setup.exe rfirma_1.0.0_x64-setup.exe.sig; do
    : > "$signed/$f"
done
"$script" write "$signed"
[ "$(jq -r '.[] | select(.format == "minisign") | "\(.platform) \(.file) \(.signable)"' "$signed/paquetes.json")" \
    = "windows rfirma_1.0.0_x64-setup.exe.sig false" ] || fail "firma de actualizacion: una fila de windows, no firmable"
[ "$("$script" platform-files "$signed" windows | wc -l)" = 2 ] || fail "firma de actualizacion: viaja con el instalador"
[ "$("$script" files "$signed" | wc -l)" = 3 ] || fail "files lista tambien la firma de actualizacion"
[ "$("$script" packages "$signed" | tr '\n' ' ')" = "me.sgomez.rfirma.flatpak rfirma_1.0.0_x64-setup.exe " ] \
    || fail "packages: los paquetes sin la firma de actualizacion"

unknown="$tmp/unknown"
mkdir "$unknown"
: > "$unknown/me.sgomez.rfirma.flatpak"
: > "$unknown/rfirma.msi"
if "$script" write "$unknown" 2> /dev/null; then
    fail "extension desconocida: debia fallar"
fi

echo "OK  packages-manifest"
