#!/usr/bin/env bash
#
# Provisiona un perfil NSS DESECHABLE para las pruebas de **grada B** del
# almacen de Mozilla (ADR-0014, TD-02). Hermano de
# testdata/softhsm/provision-token.sh, y con el mismo material publico de la
# FNMT que hay en testdata/fnmt/.
#
#   bash testdata/nss/provision-profile.sh <directorio>
#
# El perfil se crea EN EL DIRECTORIO QUE SE LE PASA y en ningun otro sitio: las
# pruebas le pasan un directorio temporal. El perfil real de Firefox de nadie se
# toca jamas, ni se lee.
#
# Es idempotente: si el directorio ya tiene el perfil montado no reescribe nada.
#
# Lo que queda dentro, y por que:
#
#   EIDAS_CERTIFICADO_PRUEBAS___99999999R  clave + certificado  EN VIGOR
#   EIDAS_CERTIFICADO_PRUEBAS___99999999R  clave + certificado  CADUCO en 2020
#   AC-DE-PRUEBAS-FNMT                     SOLO certificado     una CA suelta
#
# Los dos primeros comparten CKA_LABEL —la FNMT le pone el mismo friendlyName a
# los tres .p12 del kit y NSS lo usa como apodo— y no comparten CKA_ID. Eso no
# es un accidente que haya que arreglar: es EXACTAMENTE lo que hay en un perfil
# de Firefox de verdad, donde dos claves privadas llevan la misma etiqueta, y es
# el motivo de que la clave se busque por CKA_ID (#98, ID-06). Aqui sale gratis.
#
# La CA suelta entra a proposito: es lo que un perfil de Firefox de verdad tiene
# a cientos, y es lo que el filtro de #100 tendra que descartar.
#
# La contrasena maestra del perfil es la CADENA VACIA, que es el caso corriente
# de un Firefox recien instalado y el que hace interesante a este ticket: para
# C_Login la cadena vacia NO es lo mismo que «sin PIN».

set -euo pipefail

profile="${1:-}"
[ -n "$profile" ] || {
    echo "uso: $0 <directorio-del-perfil>" >&2
    exit 2
}

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
kit="$here/../fnmt"

for tool in certutil pk12util openssl; do
    command -v "$tool" >/dev/null || {
        echo "falta: $tool" >&2
        echo "  sudo apt install -y libnss3-tools openssl" >&2
        exit 1
    }
done

mkdir -p "$profile"
db="sql:$profile"

# `certutil -N --empty-password` deja la contrasena maestra en la cadena vacia.
# No es un descuido: es exactamente el perfil que hay que poder firmar sin
# atascar a nadie en un dialogo imposible.
if [ ! -f "$profile/cert9.db" ]; then
    certutil -N -d "$db" --empty-password
fi

# El fichero de contrasena que quieren pk12util y certutil: vacio de verdad, no
# ausente.
empty_password="$profile/.empty-password"
: > "$empty_password"

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

# Cuantas claves privadas hay ya dentro. La idempotencia se mide por AHI y no
# por el apodo: los dos certificados de persona comparten apodo, asi que buscar
# por el daria por importado el segundo en cuanto estuviera el primero.
private_keys() {
    certutil -K -d "$db" -f "$empty_password" 2>/dev/null | grep -c '^<' || true
}

# import_pkcs12 <fichero .p12> <contrasena> <cuantas claves debe haber despues>
import_pkcs12() {
    local p12="$1" password="$2" expected="$3"
    [ "$(private_keys)" -ge "$expected" ] && return 0
    pk12util -i "$p12" -d "$db" -W "$password" -k "$empty_password" >/dev/null
    echo "importado $(basename "$p12")"
}

# import_ca <fichero .p12 de donde sacar la cadena> <contrasena> <apodo>
#
# Una CA suelta: certificado y NADA de clave privada. Los .p12 de la FNMT usan
# cifrado antiguo, asi que OpenSSL 3 exige `-legacy`.
import_ca() {
    local p12="$1" password="$2" nickname="$3"
    certutil -L -d "$db" -n "$nickname" >/dev/null 2>&1 && return 0

    openssl pkcs12 -in "$p12" -passin "pass:$password" -cacerts -nokeys -legacy \
        -out "$workdir/ca.pem" 2>/dev/null
    # Con el primero de la cadena basta: lo que hace falta es que haya algo sin
    # clave privada dentro del almacen.
    openssl x509 -in "$workdir/ca.pem" -outform DER -out "$workdir/ca.der"
    certutil -A -d "$db" -n "$nickname" -t ",," -i "$workdir/ca.der" -f "$empty_password"
    echo "importada la CA $nickname"
}

import_pkcs12 "$kit/active-rsa.p12"  "1234"         1
import_pkcs12 "$kit/expired-rsa.p12" "G5cp,fYC9gje" 2
import_ca     "$kit/active-rsa.p12"  "1234"         "AC-DE-PRUEBAS-FNMT"

echo "perfil NSS listo en $profile"
