#!/usr/bin/env bash
# Firma de actualización (minisign) de cada instalador de Windows de una entrega, comprobada contra la pública de firma versionada (ADR-0015).
#
# El instalador no cambia: la firma va al lado, en `<instalador>.sig`, y el
# manifiesto se reescribe para que sea una fila más. Por eso esto va DESPUES de
# `packaging/check-digests.sh` y ANTES del `SHA256SUMS`.
#
# La firma se comprueba contra la pública con la que se FIRMA (la que aceptan
# las instalaciones que ya existen), no contra la que se EMBEBE en
# `plugins.updater.pubkey`. Casi siempre son la misma; durante una rotación no:
# la versión puente embebe la nueva y la firma todavía la privada vieja
# («Rotar» en packaging/repo/README.md).
#
# Uso: packaging/windows/sign-updater.sh <directorio> <publica de firma>
# Entorno: TAURI_SIGNING_PRIVATE_KEY y TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
set -euo pipefail

dir="${1-}"
signing_pub="${2-}"
if [ -z "$dir" ] || [ ! -d "$dir" ] || [ -z "$signing_pub" ]; then
    echo "uso: packaging/windows/sign-updater.sh <directorio> <publica de firma>" >&2
    exit 2
fi

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/scripts/packages-manifest.sh"

mapfile -t installers < <("$manifest" files "$dir" nsis)
if [ "${#installers[@]}" -eq 0 ]; then
    echo "sin instalador de Windows que firmar"
    exit 0
fi

if [ ! -s "$signing_pub" ]; then
    echo "falta la clave publica de firma de actualizaciones en $signing_pub" >&2
    echo "Se crea como dice «La clave de actualizaciones de Windows» en packaging/repo/README.md." >&2
    exit 1
fi
if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ] || [ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]; then
    echo "faltan TAURI_SIGNING_PRIVATE_KEY o TAURI_SIGNING_PRIVATE_KEY_PASSWORD en el entorno" >&2
    exit 1
fi
for tool in npx minisign base64; do
    if ! command -v "$tool" > /dev/null 2>&1; then
        echo "para firmar la actualizacion hace falta $tool" >&2
        exit 1
    fi
done

cli_version="$(jq -r '.devDependencies["@tauri-apps/cli"]' "$root/rfirma-app/package.json")"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
tr -d '[:space:]' < "$signing_pub" | base64 -d > "$work/updater.pub"

for name in "${installers[@]}"; do
    installer="$dir/$name"
    rm -f "$installer.sig"
    npx --yes "@tauri-apps/cli@$cli_version" signer sign "$installer" < /dev/null
    base64 -d "$installer.sig" > "$work/updater.minisig"
    if ! minisign -V -q -p "$work/updater.pub" -m "$installer" -x "$work/updater.minisig"; then
        echo "$name.sig no verifica con la clave publica de firma $signing_pub" >&2
        exit 1
    fi
    echo "OK  $name.sig verifica con la clave publica de firma $signing_pub"
done

"$manifest" write "$dir"
