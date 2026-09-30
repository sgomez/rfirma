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
for f in me.sgomez.rfirma.flatpak rfirma_1.0.0_amd64.deb rfirma-1.0.0-1.x86_64.rpm rfirma_1.0.0_x64-setup.exe SHA256SUMS; do
    : > "$full/$f"
done
"$script" write "$full"
[ "$(jq length "$full/paquetes.json")" = 4 ] || fail "entrega completa: una fila por paquete"
[ "$(jq -r '.[] | select(.format == "nsis") | .platform' "$full/paquetes.json")" = windows ] \
    || fail "entrega completa: plataforma del instalador"
jq -e 'all(.[]; has("platform") and has("file") and has("format") and has("signable") and has("warning"))' \
    "$full/paquetes.json" > /dev/null || fail "entrega completa: campos de cada fila"

[ "$("$script" signable "$full")" = "rfirma-1.0.0-1.x86_64.rpm" ] || fail "solo el rpm es firmable"
[ "$("$script" files "$full" deb)" = "rfirma_1.0.0_amd64.deb" ] || fail "filtro por formato"
[ "$("$script" files "$full" | wc -l)" = 4 ] || fail "files lista los paquetes y no el SHA256SUMS"

[ "$("$script" platforms "$full" | tr '\n' ' ')" = "flatpak linux windows " ] || fail "platforms: las del manifiesto, sin repetir"
[ "$("$script" platform-files "$full" linux | wc -l)" = 2 ] || fail "platform-files: deb y rpm de linux"
[ -z "$("$script" platform-files "$full" macos)" ] || fail "platform-files: una plataforma sin filas no lista nada"

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
: > "$unknown/rfirma.dmg"
if "$script" write "$unknown" 2> /dev/null; then
    fail "extension desconocida: debia fallar"
fi

digests="$root/packaging/check-digests.sh"
"$script" write "$full"
(cd "$full" && rm -f SHA256SUMS && sha256sum -- * > "$tmp/SHA256SUMS.build")
echo signature >> "$full/rfirma-1.0.0-1.x86_64.rpm"
"$digests" "$tmp/SHA256SUMS.build" "$full" > /dev/null || fail "digests: un firmable puede cambiar"
echo tampered >> "$full/rfirma_1.0.0_amd64.deb"
if "$digests" "$tmp/SHA256SUMS.build" "$full" > /dev/null 2>&1; then
    fail "digests: un paquete no firmable no puede cambiar"
fi

echo "OK  packages-manifest"
