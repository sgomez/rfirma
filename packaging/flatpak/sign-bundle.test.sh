#!/usr/bin/env bash
# Prueba la firma del .flatpak (ID-502): con una clave desechable el bundle se instala sin error de firma, con el origen de rFirma y el mismo commit.
#
# Sin flatpak, ostree o gpg avisa y se salta, como `packaging/repo/build-tree.test.sh`.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
signer="$root/packaging/flatpak/sign-bundle.sh"

for tool in ostree flatpak gpg; do
    if ! command -v "$tool" > /dev/null 2>&1; then
        echo "AVISO: falta $tool: la prueba de la firma del flatpak se salta." >&2
        exit 0
    fi
done

tmp="$(mktemp -d)"
trap 'gpgconf --kill gpg-agent > /dev/null 2>&1 || true; rm -rf "$tmp"' EXIT
export FLATPAK_USER_DIR="$tmp/flatpak-user"
export GNUPGHOME="$tmp/gnupg"
mkdir -m 700 "$GNUPGHOME"

fail() {
    echo "FALLO ($1)" >&2
    exit 1
}

printf 'desechable' > "$tmp/passphrase"
gpg --batch --pinentry-mode loopback --passphrase-file "$tmp/passphrase" \
    --quick-gen-key "rfirma prueba <prueba@example.invalid>" ed25519 sign never > /dev/null 2>&1
fingerprint="$(gpg --batch --list-keys --with-colons | awk -F: '/^fpr/ {print $10; exit}')"

build_unsigned_bundle() {
    local work="$tmp/work" bundle="$1"
    mkdir -p "$work/app/files/bin" "$work/app/export/share/applications"
    cat > "$work/app/metadata" <<META
[Application]
name=me.sgomez.rfirma
runtime=org.gnome.Platform/x86_64/48
sdk=org.gnome.Sdk/x86_64/48
command=rfirma
META
    printf '#!/bin/sh\necho hola\n' > "$work/app/files/bin/rfirma"
    chmod +x "$work/app/files/bin/rfirma"
    printf '[Desktop Entry]\nName=rFirma\nExec=rfirma\nType=Application\n' \
        > "$work/app/export/share/applications/me.sgomez.rfirma.desktop"
    ostree init --mode=archive --repo="$work/repo" > /dev/null
    flatpak build-export "$work/repo" "$work/app" stable > /dev/null 2>&1
    flatpak build-bundle "$work/repo" "$bundle" me.sgomez.rfirma stable > /dev/null 2>&1
    ostree --repo="$work/repo" rev-parse app/me.sgomez.rfirma/x86_64/stable
}

bundle="$tmp/me.sgomez.rfirma.flatpak"
input_commit="$(build_unsigned_bundle "$bundle")"

"$signer" "$bundle" "$fingerprint" "$tmp/passphrase" > "$tmp/sign.log" 2>&1 \
    || { cat "$tmp/sign.log" >&2; fail "el script no firma el bundle"; }

flatpak install --user --noninteractive --no-deps "$bundle" > "$tmp/install.log" 2>&1 \
    || { cat "$tmp/install.log" >&2; fail "el bundle firmado no se instala"; }

origin="$(flatpak list --user --app --columns=application,origin,branch | awk '$1 == "me.sgomez.rfirma"')"
origin_name="$(awk '{print $2}' <<< "$origin")"
origin_url="$(flatpak remotes --user --columns=name,url | awk -v n="$origin_name" '$1 == n {print $2}')"
[ "$origin_url" = "https://rfirma.sgomez.me/flatpak/" ] || fail "el origen es '$origin_url'"
[ "$(awk '{print $3}' <<< "$origin")" = "stable" ] || fail "la rama no es stable: $origin"

installed_commit="$(flatpak info --user --show-commit me.sgomez.rfirma)"
[ "$installed_commit" = "$input_commit" ] || fail "el commit cambia al firmar: $input_commit != $installed_commit"

echo "OK  el flatpak firmado se instala con el origen de rFirma y conserva su commit"
