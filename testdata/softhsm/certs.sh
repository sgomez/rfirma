#!/usr/bin/env bash
#
# Instala (`install`) o quita (`uninstall`) todos los certificados de pruebas en
# SoftHSM: los tokens `rfirma-test` y `rfirma-test-ecc` de las pruebas y, si el
# kit de la FNMT esta en el equipo, el token `rfirma-kit` con los dos casos que
# las pruebas no traen: un seudonimo y un CN largo de representante. El kit no
# esta en el repositorio; se busca en $RFIRMA_TEST_CERTS o en
# ~/.local/share/rfirma-test-certs (docs/research/token-pkcs11-pruebas.md).

set -euo pipefail

module="${RFIRMA_PKCS11_MODULE:-/usr/lib/softhsm/libsofthsm2.so}"
kit="${RFIRMA_TEST_CERTS:-$HOME/.local/share/rfirma-test-certs}"
token_label="rfirma-kit"
pin="1234"
so_pin="3737"
export SOFTHSM2_CONF="${SOFTHSM2_CONF:-$HOME/.config/softhsm2/softhsm2.conf}"

rsa="$kit/Claves RSA"

# etiqueta | contrasena | .p12
selection=(
    "KIT-REPRESENTANTE|1234|$rsa/AC Representación/Certificados pruebas Representante/Nuevos perfiles No SMIME/FNMT_PER_JUR_NOSMIME.p12"
    "KIT-SEUDONIMO|1234|$rsa/AC Sector Público/Empleado Público con Seudónimo/Antiguo perfil/Activo/SP_Empleado_público_Seudonimo.p12"
)

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

delete_token() {
    if softhsm2-util --show-slots | grep -q "Label:[[:space:]]*$1[[:space:]]*$"; then
        softhsm2-util --delete-token --token "$1" >/dev/null
        echo "token $1 borrado"
    fi
}

uninstall() {
    local label
    for label in "$token_label" rfirma-test rfirma-test-ecc; do
        delete_token "$label"
    done
}

write_object() {
    pkcs11-tool --module "$module" --token-label "$token_label" --login --pin "$pin" \
        --write-object "$1" --type "$2" --id "$3" --label "$4" >/dev/null
}

install() {
    "$here/provision-token.sh"
    [ -d "$kit" ] || {
        echo "sin kit de la FNMT en $kit: no se instala $token_label"
        return 0
    }
    delete_token "$token_label"
    softhsm2-util --init-token --free --label "$token_label" \
        --so-pin "$so_pin" --pin "$pin" >/dev/null

    local workdir index=0 entry label password p12 id
    workdir="$(mktemp -d)"
    trap 'rm -rf "$workdir"' RETURN
    for entry in "${selection[@]}"; do
        IFS='|' read -r label password p12 <<<"$entry"
        index=$((index + 1))
        id="$(printf '%02x' "$index")"
        openssl pkcs12 -in "$p12" -passin "pass:$password" -nocerts -nodes -legacy \
            | openssl pkcs8 -topk8 -nocrypt -outform DER -out "$workdir/key.der"
        openssl pkcs12 -in "$p12" -passin "pass:$password" -clcerts -nokeys -legacy \
            | openssl x509 -outform DER -out "$workdir/cert.der"
        write_object "$workdir/key.der" privkey "$id" "$label"
        write_object "$workdir/cert.der" cert "$id" "$label"
        echo "importado $label (id $id)"
    done
    echo "token $token_label listo, PIN $pin"
}

case "${1:-}" in
    install) install ;;
    uninstall) uninstall ;;
    *)
        echo "uso: $0 install|uninstall" >&2
        exit 2
        ;;
esac
