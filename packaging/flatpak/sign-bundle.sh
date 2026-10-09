#!/usr/bin/env bash
# Deja un .flatpak sin firmar firmado y con origen (ID-502, ADR-0015): el remoto de rFirma, su clave y el repositorio del runtime.
#
# Importa el bundle a un repositorio temporal, firma su commit y lo vuelve a sacar. Firmar no cambia el commit, pero sí el fichero.
#
# Uso: sign-bundle.sh <bundle> <huella> <fichero-de-la-contraseña>
#   La clave privada tiene que estar ya en el llavero (en el CI la importa `.github/actions/import-signing-key`).
set -euo pipefail

origin_url="https://rfirma.sgomez.me/flatpak/"
runtime_repo="https://dl.flathub.org/repo/flathub.flatpakrepo"
branch="stable"
app_id="me.sgomez.rfirma"

if [ "$#" -ne 3 ]; then
    echo "uso: sign-bundle.sh <bundle> <huella> <fichero-de-la-contraseña>" >&2
    exit 2
fi

bundle="$(realpath "$1")"
fingerprint="$2"
passphrase_file="$3"

[ -f "$bundle" ] || { echo "el bundle '$bundle' no existe" >&2; exit 1; }
[ -f "$passphrase_file" ] || { echo "el fichero de la contraseña '$passphrase_file' no existe" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

gpg_args=(--batch --yes --local-user "$fingerprint" --pinentry-mode loopback --passphrase-file "$passphrase_file")

# `flatpak build-sign` firma por gpgme, que no sabe de `--passphrase-file`: una firma previa deja la contraseña en el agente.
prime_agent() {
    printf 'ceba el agente' | gpg "${gpg_args[@]}" --detach-sign --output /dev/null
}

ostree init --mode=archive --repo="$work/repo"
flatpak build-import-bundle "$work/repo" "$bundle"

prime_agent
flatpak build-sign --gpg-sign="$fingerprint" "$work/repo" "$app_id" "$branch"

gpg --batch --export "$fingerprint" > "$work/key.gpg"
[ -s "$work/key.gpg" ] || { echo "la clave '$fingerprint' no está en el llavero" >&2; exit 1; }

flatpak build-bundle --repo-url="$origin_url" --gpg-keys="$work/key.gpg" --runtime-repo="$runtime_repo" \
    "$work/repo" "$work/signed.flatpak" "$app_id" "$branch"
mv "$work/signed.flatpak" "$bundle"
