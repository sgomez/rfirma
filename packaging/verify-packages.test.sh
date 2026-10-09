#!/usr/bin/env bash
# Las pruebas de `verify-packages.sh`, sobre directorios temporales y sin red.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
verify="$root/packaging/verify-packages.sh"
manifest="$root/scripts/packages-manifest.sh"
tmp="$(mktemp -d)"
export GNUPGHOME="$tmp/gnupg"
mkdir -m 700 "$GNUPGHOME"
trap 'gpgconf --kill all > /dev/null 2>&1 || true; rm -rf "$tmp"' EXIT

failures=0
fail() { echo "FALLO  $*" >&2; failures=$((failures + 1)); }
ok() { echo "OK  $*"; }

passes() {
    local name="$1"
    shift
    if "$verify" "$@" > "$tmp/out" 2>&1; then ok "$name"; else fail "$name: $(cat "$tmp/out")"; fi
}

fails_with() {
    local name="$1" message="$2"
    shift 2
    if "$verify" "$@" > "$tmp/out" 2>&1; then
        fail "$name: tenía que fallar"
    elif ! grep -qF -- "$message" "$tmp/out"; then
        fail "$name: esperaba «$message» y dijo: $(cat "$tmp/out")"
    else
        ok "$name"
    fi
}

delivery() {
    local dir="$tmp/$1"
    mkdir "$dir"
    echo flatpak > "$dir/me.sgomez.rfirma.flatpak"
    echo deb > "$dir/rfirma_1.0.0_amd64.deb"
    echo rpm > "$dir/rfirma-1.0.0-1.x86_64.rpm"
    echo exe > "$dir/rfirma_1.0.0_x64-setup.exe"
    "$manifest" write "$dir"
    echo "$dir"
}

# --- Contra los resúmenes de la construcción --------------------------------
built="$(delivery built)"
(cd "$built" && sha256sum -- * > "$tmp/SHA256SUMS.build")

echo signature >> "$built/rfirma-1.0.0-1.x86_64.rpm"
echo signature >> "$built/me.sgomez.rfirma.flatpak"
passes "entrega: un firmable puede cambiar" "$built" --against "$tmp/SHA256SUMS.build"

extra="$(delivery extra)"
echo intruso > "$extra/subido-a-mano.exe"
fails_with "entrega: un fichero de más" "subido-a-mano.exe no estaba en la construcción" \
    "$extra" --against "$tmp/SHA256SUMS.build"

missing="$(delivery missing)"
rm "$missing/rfirma-1.0.0-1.x86_64.rpm"
fails_with "entrega: un firmable que falta" "rfirma-1.0.0-1.x86_64.rpm se construyó y aquí no está" \
    "$missing" --against "$tmp/SHA256SUMS.build"

tampered="$(delivery tampered)"
echo tampered >> "$tampered/rfirma_1.0.0_amd64.deb"
fails_with "entrega: un resumen que no casa" "rfirma_1.0.0_amd64.deb no son los bytes que salieron de la construcción" \
    "$tampered" --against "$tmp/SHA256SUMS.build"

# --- Firmada, como la descarga de la serie ----------------------------------
gpg --batch --quiet --passphrase '' --quick-gen-key 'rfirma pruebas <pruebas@example.invalid>' ed25519 sign never 2> /dev/null

release() {
    local dir
    dir="$(delivery "$1")"
    echo "sig" > "$dir/rfirma_1.0.0_x64-setup.exe.sig"
    (cd "$dir" && sha256sum -- * > SHA256SUMS)
    gpg --batch --quiet --yes --armor --detach-sign --output "$dir/SHA256SUMS.asc" "$dir/SHA256SUMS"
    echo pdf > "$dir/manual-gate.pdf"
    echo "$dir"
}

passes "release: firmada y con el PDF de la puerta manual" "$(release good)" --signed

manual="$(release manual)"
echo intruso > "$manual/subido-a-mano.exe"
fails_with "release: un asset de más" "subido-a-mano.exe está en la Release y no en su SHA256SUMS" "$manual" --signed

unlisted="$(release unlisted)"
grep -v 'x64-setup.exe$' "$unlisted/SHA256SUMS" > "$tmp/sums" && mv "$tmp/sums" "$unlisted/SHA256SUMS"
rm "$unlisted/rfirma_1.0.0_x64-setup.exe"
gpg --batch --quiet --yes --armor --detach-sign --output "$unlisted/SHA256SUMS.asc" "$unlisted/SHA256SUMS"
fails_with "release: un firmable del manifiesto que falta" "rfirma_1.0.0_x64-setup.exe está en el manifiesto y no en el SHA256SUMS" \
    "$unlisted" --signed

changed="$(release changed)"
echo tampered >> "$changed/rfirma_1.0.0_amd64.deb"
fails_with "release: un resumen que no casa" "no son los que resume su SHA256SUMS" "$changed" --signed

forged="$(release forged)"
echo "0000  otro" >> "$forged/SHA256SUMS"
fails_with "release: una firma inválida" "la firma del SHA256SUMS" "$forged" --signed

# --- La puerta del contenido ------------------------------------------------
fails_with "contenido: sin manifiesto" "no existe" "$tmp/gnupg"
fails_with "argumentos: modo desconocido" "uso:" "$built" --otro

if command -v dpkg-deb > /dev/null 2>&1; then
    deb() {
        local out="$1" library="$2" relations="${3-Depends: libpcsclite1
Recommends: opensc, pcscd
}" staging
        staging="$(mktemp -d "$tmp/deb.XXXX")"
        mkdir -p "$staging/DEBIAN" "$staging/usr/lib/rfirma"
        printf 'Package: rfirma\nVersion: 1.0.0\nArchitecture: amd64\nMaintainer: pruebas\nDescription: pruebas\n%s' \
            "$relations" > "$staging/DEBIAN/control"
        : > "$staging/usr/lib/rfirma/librfirma_crypto.so"
        [ -z "$library" ] || : > "$staging/usr/lib/rfirma/$library"
        dpkg-deb --build --root-owner-group "$staging" "$out" > /dev/null
    }
    clean="$tmp/clean"
    mkdir "$clean"
    deb "$clean/rfirma_1.0.0_amd64.deb" ""
    "$manifest" write "$clean"
    passes "contenido: un .deb con una sola biblioteca nativa" "$clean"

    awt="$tmp/awt"
    mkdir "$awt"
    deb "$awt/rfirma_1.0.0_amd64.deb" libawt.so
    "$manifest" write "$awt"
    fails_with "contenido: un .deb con libawt.so" "SOBRA libawt.so" "$awt"

    bare="$tmp/bare"
    mkdir "$bare"
    deb "$bare/rfirma_1.0.0_amd64.deb" "" ""
    "$manifest" write "$bare"
    fails_with "contenido: un .deb sin depender de libpcsclite" "libpcsclite" "$bare"

    unadvised="$tmp/unadvised"
    mkdir "$unadvised"
    deb "$unadvised/rfirma_1.0.0_amd64.deb" "" "Depends: libpcsclite1
"
    "$manifest" write "$unadvised"
    fails_with "contenido: un .deb que no recomienda opensc ni pcscd" "opensc" "$unadvised"
else
    echo "AVISO  sin dpkg-deb: se salta la puerta del contenido sobre un .deb"
fi

if [ "$failures" -ne 0 ]; then
    echo "$failures prueba(s) de verify-packages fallaron" >&2
    exit 1
fi
echo "OK  verify-packages"
