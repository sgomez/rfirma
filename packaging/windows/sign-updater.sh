#!/usr/bin/env bash
# Firma de actualización (minisign) de cada instalador de Windows de una entrega, comprobada contra la pública versionada (ADR-0015).
#
# El instalador no cambia: la firma va al lado, en `<instalador>.sig`, y el
# manifiesto se reescribe para que sea una fila más. Por eso esto va DESPUES de
# `packaging/check-digests.sh` y ANTES del `SHA256SUMS`.
#
# Uso: packaging/windows/sign-updater.sh <directorio> <configuracion de Windows>
# Entorno: TAURI_SIGNING_PRIVATE_KEY y TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
set -euo pipefail

dir="${1-}"
config="${2-}"
if [ -z "$dir" ] || [ ! -d "$dir" ] || [ -z "$config" ] || [ ! -f "$config" ]; then
    echo "uso: packaging/windows/sign-updater.sh <directorio> <configuracion de Windows>" >&2
    exit 2
fi

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/scripts/packages-manifest.sh"

mapfile -t installers < <("$manifest" files "$dir" nsis)
if [ "${#installers[@]}" -eq 0 ]; then
    echo "sin instalador de Windows que firmar"
    exit 0
fi

pubkey="$(jq -r '.plugins.updater.pubkey // empty' "$config")"
if [ -z "$pubkey" ]; then
    echo "$config no tiene la clave publica de actualizaciones en plugins.updater.pubkey" >&2
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
printf '%s' "$pubkey" | base64 -d > "$work/updater.pub"

for name in "${installers[@]}"; do
    installer="$dir/$name"
    rm -f "$installer.sig"
    npx --yes "@tauri-apps/cli@$cli_version" signer sign "$installer" < /dev/null
    base64 -d "$installer.sig" > "$work/updater.minisig"
    if ! minisign -V -q -p "$work/updater.pub" -m "$installer" -x "$work/updater.minisig"; then
        echo "$name.sig no verifica con la clave publica de $config" >&2
        exit 1
    fi
    echo "OK  $name.sig verifica con la clave publica de $config"
done

"$manifest" write "$dir"
