#!/usr/bin/env bash
# Firma de actualización (minisign) de cada instalador de Windows de una entrega, comprobada contra la pública embebida en la última etiqueta estable (ADR-0015).
#
# Uso: packaging/windows/sign-updater.sh <directorio>
# Entorno: TAURI_SIGNING_PRIVATE_KEY y TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
set -euo pipefail

dir="${1-}"
if [ -z "$dir" ] || [ ! -d "$dir" ]; then
    echo "uso: packaging/windows/sign-updater.sh <directorio>" >&2
    exit 2
fi

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/scripts/packages-manifest.sh"
config=packaging/windows/tauri.windows.json

mapfile -t installers < <("$manifest" files "$dir" nsis)
if [ "${#installers[@]}" -eq 0 ]; then
    echo "sin instalador de Windows que firmar"
    exit 0
fi

embedded_pubkey() {
    jq -r '.plugins.updater.pubkey // empty' 2> /dev/null || true
}

# No la actual: en la versión puente de una rotación ya embebe la nueva (ADR-0015).
trusted_pubkey() {
    local tag pubkey
    while read -r tag; do
        [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || continue
        pubkey="$(git -C "$root" show "$tag:$config" 2> /dev/null | embedded_pubkey)" || pubkey=""
        if [ -n "$pubkey" ]; then
            echo "$tag $pubkey"
            return
        fi
    done < <(git -C "$root" tag --list 'v*' --merged HEAD --no-contains HEAD --sort=-v:refname)
    echo "HEAD $(embedded_pubkey < "$root/$config")"
}

read -r source pubkey < <(trusted_pubkey)
if [ -z "${pubkey-}" ]; then
    echo "falta plugins.updater.pubkey en $config ($source)" >&2
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
        echo "$name.sig no verifica con la clave publica embebida en $config de $source" >&2
        exit 1
    fi
    echo "OK  $name.sig verifica con la clave publica embebida en $config de $source"
done

"$manifest" write "$dir"
