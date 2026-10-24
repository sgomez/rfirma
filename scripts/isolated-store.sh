#!/usr/bin/env bash
#
# Monta el almacen de un perfil de usar y tirar de la suite de conformidad, igual
# para cualquier cliente, e imprime tres lineas: el perfil, el SOFTHSM2_CONF que
# ha montado en el y el almacen. Las preferencias, el entorno de lanzamiento y la
# raiz de confianza de cada cliente los pone su receta, en
# rfirma-conformance/src/recipe.rs.
#
# Almacenes:
#
# * rsa, ec, several, expired, rsa_without_sha1: una NSS sin contrasena con los certificados de
#   testdata/fnmt/ que le tocan; SOFTHSM2_CONF apunta a un directorio de
#   tokens propio y vacio del perfil. Ningun cliente pide PIN.
# * token: la NSS vacia con SoftHSM registrado, un token propio del perfil con
#   el RSA activo y el de curva eliptica de testdata/fnmt/, cada uno con su
#   clave; el PIN es 1234.
# * token_apart: el mismo token, sin registrar en la NSS, y en la NSS solo el
#   de seudonimo, que no esta en el token: distingue el almacen que nombra la
#   sede por su biblioteca del almacen del sistema.
# * ed25519: la NSS vacia y un token propio sin registrar con un solo
#   certificado, el Ed25519 de testdata/site-driver/, con su clave; el PIN es
#   1234.
# * dnie: la NSS vacia con el OpenSC del sistema registrado; del DNIe no se
#   copia nada: la tarjeta esta en el lector y la persona teclea el PIN.
# * rsa_without_sha1: la NSS de rsa; lo que cambia es la preferencia que le
#   pone la receta de rFirma.
#
# Cada perfil monta su SOFTHSM2_CONF, lo use o no, para que el cliente nunca
# vea el de quien corre la suite.
#
# El perfil se rehace entero en cada llamada.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fnmt="$here/../testdata/fnmt"
site_driver="$here/../testdata/site-driver"
source "$here/../testdata/softhsm/platform.sh"

store="${1:-rsa}"

case "$store" in
    rsa | rsa_without_sha1) p12s="active-rsa.p12" ;;
    ec) p12s="active-ecc.p12" ;;
    token) p12s="" ;;
    token_apart) p12s="pseudonym-rsa.p12" ;;
    ed25519) p12s="" ;;
    several) p12s="active-rsa.p12 active-ecc.p12 pseudonym-rsa.p12" ;;
    expired) p12s="active-ecc.p12 expired-rsa.p12" ;;
    dnie) p12s="" ;;
    *)
        echo "almacen desconocido: $store (rsa, ec, token, token_apart, ed25519, several, expired, dnie o rsa_without_sha1)" >&2
        exit 2
        ;;
esac

profile="${RFIRMA_PROBE_PROFILE:-${XDG_CACHE_HOME:-$HOME/.cache}/rfirma/probe-profile-$store}"

for tool in certutil modutil pk12util; do
    command -v "$tool" >/dev/null || {
        echo "falta: $tool" >&2
        echo "  sudo apt install -y libnss3-tools" >&2
        exit 1
    }
done

with_token=false
case "$store" in token | token_apart | ed25519) with_token=true ;; esac

the_opensc_module() {
    local candidate
    for candidate in "${RFIRMA_OPENSC_MODULE:-}" \
        /usr/lib/x86_64-linux-gnu/opensc-pkcs11.so \
        /usr/lib/aarch64-linux-gnu/opensc-pkcs11.so \
        /usr/lib64/pkcs11/opensc-pkcs11.so \
        /usr/lib/opensc-pkcs11.so; do
        if [ -n "$candidate" ] && [ -f "$candidate" ]; then
            echo "$candidate"
            return 0
        fi
    done
    return 1
}

if [ "$store" = dnie ]; then
    opensc_module="$(the_opensc_module)" || {
        echo "falta el modulo PKCS#11 de OpenSC (RFIRMA_OPENSC_MODULE lo fija a mano)" >&2
        echo "  sudo apt install -y opensc-pkcs11" >&2
        exit 1
    }
fi

if $with_token; then
    for tool in softhsm2-util pkcs11-tool openssl; do
        command -v "$tool" >/dev/null || {
            echo "falta: $tool" >&2
            echo "  $install_tools" >&2
            exit 1
        }
    done
fi

# La contrasena del .p12 sale de la tabla de testdata/fnmt/README.md.
the_password_of() {
    awk -F'|' -v name="\`$1\`" '$2 ~ name { gsub(/[ `]/, "", $3); print $3; exit }' \
        "$fnmt/README.md"
}

nssdb="$profile/.pki/nssdb"
rm -rf "$profile"
mkdir -p "$nssdb"
certutil -d "sql:$nssdb" -N --empty-password

mkdir -p "$profile/softhsm/tokens"
softhsm_conf="$profile/softhsm/softhsm2.conf"
printf 'directories.tokendir = %s\nobjectstore.backend = file\n' \
    "$profile/softhsm/tokens" > "$softhsm_conf"
export SOFTHSM2_CONF="$softhsm_conf"

if $with_token; then
    [ -f "$module" ] || {
        echo "falta el modulo PKCS#11: $module" >&2
        echo "  $install_softhsm" >&2
        exit 1
    }
    if [ "$store" = token ]; then
        modutil -dbdir "sql:$nssdb" -add softhsm2 -libfile "$module" -force >/dev/null
    fi

    token_label="rfirma-conformance"
    softhsm2-util --init-token --free --label "$token_label" \
        --so-pin 3737 --pin 1234 >/dev/null

    # import_token_object <ruta del .p12> <contrasena> <id> <etiqueta>
    import_token_object() {
        local p12="$1" password="$2" id="$3" label="$4"
        openssl pkcs12 -in "$p12" -passin "pass:$password" -clcerts -nokeys -legacy \
            | openssl x509 -outform DER -out "$profile/softhsm/cert-$id.der"
        pkcs11-tool --module "$module" --token-label "$token_label" --login --pin 1234 \
            --write-object "$profile/softhsm/cert-$id.der" --type cert --id "$id" --label "$label" \
            >/dev/null
        openssl pkcs12 -in "$p12" -passin "pass:$password" -nocerts -nodes -legacy \
            | openssl pkcs8 -topk8 -nocrypt -outform DER -out "$profile/softhsm/key-$id.der"
        pkcs11-tool --module "$module" --token-label "$token_label" --login --pin 1234 \
            --write-object "$profile/softhsm/key-$id.der" --type privkey --id "$id" --label "$label" \
            >/dev/null
    }

    if [ "$store" = ed25519 ]; then
        import_token_object "$site_driver/ed25519.p12" 1234 "03" "ED25519"
    else
        import_token_object "$fnmt/active-rsa.p12" "$(the_password_of active-rsa.p12)" \
            "01" "FNMT-ACTIVO-99999999R"
        import_token_object "$fnmt/active-ecc.p12" "$(the_password_of active-ecc.p12)" \
            "02" "FNMT-ACTIVO-ECC-99949991H"
    fi
fi

if [ "$store" = dnie ]; then
    modutil -dbdir "sql:$nssdb" -add opensc -libfile "$opensc_module" -force >/dev/null
fi

for p12 in $p12s; do
    password="$(the_password_of "$p12")"
    [ -n "$password" ] || {
        echo "no encuentro la contrasena de $p12 en $fnmt/README.md" >&2
        exit 1
    }
    pk12util -i "$fnmt/$p12" -d "sql:$nssdb" -W "$password" -K "" >/dev/null
done

echo "almacen $store aislado en $profile" >&2

printf '%s\n%s\n%s\n' "$profile" "$softhsm_conf" "$store"
