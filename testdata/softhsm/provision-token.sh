#!/usr/bin/env bash
#
# Provisiona el token SoftHSM `rfirma-test` que necesitan las pruebas de
# **grada B** (ADR-0014). Es idempotente: se puede ejecutar tantas veces como
# haga falta y solo escribe lo que falte, asi que `just check` lo llama siempre.
#
# El material sale de testdata/fnmt/, que es publico por diseno: lo publica la
# FNMT con la contrasena incluida. El certificado personal del titular no
# interviene aqui ni en ningun otro punto del proyecto.
#
# Se importan CINCO certificados y TRES claves privadas:
#
#   id 01  FNMT-ACTIVO-99999999R     clave + certificado  (camino feliz)
#   id 02  FNMT-CADUCADO-99999999R   solo certificado     (caduco en 2020)
#   id 03  FNMT-REVOCADO-99999999R   solo certificado     (revocado en 2024)
#   id 04  FNMT-GEMELO-99999999R     clave + certificado  (par de claves activo)
#   id 05  FNMT-GEMELO-99999999R     clave + certificado  (par de claves caducado)
#
# El caducado y el revocado entran SIN clave a proposito: existen para que el
# listado tenga que clasificarlos, no para firmar con ellos.
#
# Los dos GEMELOS comparten CKA_LABEL y tienen CKA_ID y par de claves distintos:
# reproducen lo medido en un perfil de Firefox de verdad, donde dos claves
# privadas comparten etiqueta. Buscar la clave por etiqueta devolveria una de
# las dos arbitrariamente y se firmaria con la clave equivocada sin que nadie se
# entere; emparejar por CKA_ID es lo que lo impide (#98, ID-06).
#
# `softhsm2-util --import` no admite PKCS#12 (falla con «Could not read the
# PKCS#8 file»), asi que el .p12 se parte con OpenSSL y los objetos se escriben
# con pkcs11-tool. Los .p12 de la FNMT usan cifrado antiguo: OpenSSL 3 exige
# `-legacy`.

set -euo pipefail

module="${RFIRMA_PKCS11_MODULE:-/usr/lib/softhsm/libsofthsm2.so}"
token_label="rfirma-test"
pin="1234"
so_pin="3737"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
kit="$here/../fnmt"

for tool in softhsm2-util pkcs11-tool openssl; do
    command -v "$tool" >/dev/null || {
        echo "falta: $tool" >&2
        echo "  sudo apt install -y softhsm2 opensc openssl" >&2
        exit 1
    }
done

[ -f "$module" ] || {
    echo "falta el modulo PKCS#11: $module" >&2
    echo "  sudo apt install -y softhsm2" >&2
    exit 1
}

# SoftHSM busca su configuracion en $SOFTHSM2_CONF, luego en
# ~/.config/softhsm2/softhsm2.conf y solo despues en /etc. El paquete de Debian
# deja el almacen en /var/lib/softhsm/tokens, que no es escribible sin sudo, asi
# que se escribe una configuracion de usuario cuando no hay ninguna.
conf="${SOFTHSM2_CONF:-$HOME/.config/softhsm2/softhsm2.conf}"
if [ ! -f "$conf" ]; then
    tokendir="$HOME/.local/share/softhsm/tokens"
    mkdir -p "$(dirname "$conf")" "$tokendir"
    cat > "$conf" <<CONF
directories.tokendir = $tokendir
objectstore.backend = file
log.level = ERROR
CONF
    echo "escrita la configuracion de SoftHSM en $conf"
fi
export SOFTHSM2_CONF="$conf"

if ! softhsm2-util --show-slots | grep -q "$token_label"; then
    softhsm2-util --init-token --free --label "$token_label" \
        --so-pin "$so_pin" --pin "$pin" >/dev/null
    echo "token $token_label inicializado"
fi

# Se listan por tipo para no tener que adivinar a que objeto pertenece cada
# linea «label:» de la salida de pkcs11-tool.
list_objects() {
    pkcs11-tool --module "$module" --token-label "$token_label" \
        --login --pin "$pin" --list-objects --type "$1" 2>/dev/null || true
}
certificates="$(list_objects cert)"
private_keys="$(list_objects privkey)"

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

# La idempotencia mira el CKA_ID y no la etiqueta: los dos gemelos comparten
# etiqueta, asi que buscarlos por ella daria por importado el segundo en cuanto
# estuviera el primero.
has_id() {
    printf '%s\n' "$1" | grep -q "ID:[[:space:]]*$2[[:space:]]*$"
}

# import_certificate <fichero .p12> <contrasena> <id> <etiqueta>
import_certificate() {
    local p12="$1" password="$2" id="$3" label="$4"
    has_id "$certificates" "$id" && return 0
    openssl pkcs12 -in "$p12" -passin "pass:$password" -clcerts -nokeys -legacy \
        | openssl x509 -outform DER -out "$workdir/cert.der"
    pkcs11-tool --module "$module" --token-label "$token_label" --login --pin "$pin" \
        --write-object "$workdir/cert.der" --type cert --id "$id" --label "$label" \
        >/dev/null
    echo "importado el certificado $label (id $id)"
}

# import_private_key <fichero .p12> <contrasena> <id> <etiqueta>
import_private_key() {
    local p12="$1" password="$2" id="$3" label="$4"
    has_id "$private_keys" "$id" && return 0
    openssl pkcs12 -in "$p12" -passin "pass:$password" -nocerts -nodes -legacy \
        | openssl pkcs8 -topk8 -nocrypt -outform DER -out "$workdir/key.der"
    pkcs11-tool --module "$module" --token-label "$token_label" --login --pin "$pin" \
        --write-object "$workdir/key.der" --type privkey --id "$id" --label "$label" \
        >/dev/null
    echo "importada la clave privada $label (id $id)"
}

import_private_key "$kit/active-rsa.p12"  "1234"         "01" "FNMT-ACTIVO-99999999R"
import_certificate "$kit/active-rsa.p12"  "1234"         "01" "FNMT-ACTIVO-99999999R"
import_certificate "$kit/expired-rsa.p12" "G5cp,fYC9gje" "02" "FNMT-CADUCADO-99999999R"
import_certificate "$kit/revoked-rsa.p12" "1234"         "03" "FNMT-REVOCADO-99999999R"

# Los gemelos: misma etiqueta, distinto CKA_ID, distinto par de claves.
import_private_key "$kit/active-rsa.p12"  "1234"         "04" "FNMT-GEMELO-99999999R"
import_certificate "$kit/active-rsa.p12"  "1234"         "04" "FNMT-GEMELO-99999999R"
import_private_key "$kit/expired-rsa.p12" "G5cp,fYC9gje" "05" "FNMT-GEMELO-99999999R"
import_certificate "$kit/expired-rsa.p12" "G5cp,fYC9gje" "05" "FNMT-GEMELO-99999999R"

echo "token $token_label listo en $module"
