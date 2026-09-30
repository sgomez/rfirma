#!/usr/bin/env bash
# Prueba la firma de actualización del instalador de Windows: sin clave no firma, y la firma verifica con la pública de la configuración.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
script="$root/packaging/windows/sign-updater.sh"
manifest="$root/scripts/packages-manifest.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "FALLO ($1)" >&2
    exit 1
}

delivery() {
    mkdir "$1"
    echo flatpak > "$1/me.sgomez.rfirma.flatpak"
    [ "${2-}" = sin-instalador ] || echo instalador > "$1/rfirma_1.0.0_x64-setup.exe"
    "$manifest" write "$1"
}

echo '{}' > "$tmp/sin-clave.json"
echo '{"plugins":{"updater":{"pubkey":"cHVibGljYQ=="}}}' > "$tmp/con-clave.json"

delivery "$tmp/candidata" sin-instalador
env -u TAURI_SIGNING_PRIVATE_KEY "$script" "$tmp/candidata" "$tmp/sin-clave.json" > /dev/null \
    || fail "sin instalador no hay nada que firmar"

delivery "$tmp/sin-publica"
if TAURI_SIGNING_PRIVATE_KEY=k TAURI_SIGNING_PRIVATE_KEY_PASSWORD=p \
    "$script" "$tmp/sin-publica" "$tmp/sin-clave.json" 2> /dev/null; then
    fail "sin clave publica versionada: debia fallar"
fi
[ ! -e "$tmp/sin-publica/rfirma_1.0.0_x64-setup.exe.sig" ] || fail "sin clave publica no se firma"

delivery "$tmp/sin-privada"
if env -u TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD=p \
    "$script" "$tmp/sin-privada" "$tmp/con-clave.json" 2> /dev/null; then
    fail "sin clave privada: debia fallar"
fi
if TAURI_SIGNING_PRIVATE_KEY=k env -u TAURI_SIGNING_PRIVATE_KEY_PASSWORD \
    "$script" "$tmp/sin-privada" "$tmp/con-clave.json" 2> /dev/null; then
    fail "sin la contrasena de la clave: debia fallar"
fi

if ! command -v minisign > /dev/null 2>&1 || ! command -v npx > /dev/null 2>&1; then
    echo "AVISO sin minisign o sin npx: se salta la firma de verdad (el CI los instala)" >&2
    echo "OK  sign-updater (sin la firma de verdad)"
    exit 0
fi

cli_version="$(jq -r '.devDependencies["@tauri-apps/cli"]' "$root/rfirma-app/package.json")"
tauri() { npx --yes "@tauri-apps/cli@$cli_version" "$@" < /dev/null > /dev/null 2>&1; }
tauri signer generate --ci -p buena -w "$tmp/rfirma.key"
tauri signer generate --ci -p otra -w "$tmp/otra.key"
jq -n --arg k "$(cat "$tmp/rfirma.key.pub")" '{plugins: {updater: {pubkey: $k}}}' > "$tmp/rfirma.json"
jq -n --arg k "$(cat "$tmp/otra.key.pub")" '{plugins: {updater: {pubkey: $k}}}' > "$tmp/otra.json"

delivery "$tmp/firmada"
TAURI_SIGNING_PRIVATE_KEY="$(cat "$tmp/rfirma.key")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD=buena \
    "$script" "$tmp/firmada" "$tmp/rfirma.json" > /dev/null || fail "firma y verifica con la clave versionada"
[ -s "$tmp/firmada/rfirma_1.0.0_x64-setup.exe.sig" ] || fail "el .sig queda al lado del instalador"
[ "$(cat "$tmp/firmada/rfirma_1.0.0_x64-setup.exe")" = instalador ] || fail "firmar no toca el instalador"
[ "$("$manifest" files "$tmp/firmada" minisign)" = rfirma_1.0.0_x64-setup.exe.sig ] \
    || fail "el .sig es una fila del manifiesto"

delivery "$tmp/otra-clave"
if TAURI_SIGNING_PRIVATE_KEY="$(cat "$tmp/otra.key")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD=otra \
    "$script" "$tmp/otra-clave" "$tmp/rfirma.json" > /dev/null 2>&1; then
    fail "una firma que no verifica con la publica versionada: debia fallar"
fi

echo "OK  sign-updater"
