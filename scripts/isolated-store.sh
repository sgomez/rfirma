#!/usr/bin/env bash
#
# Monta el perfil de usar y tirar de la suite de conformidad para un cliente y
# un almacen, e imprime cuatro lineas: la clase de cliente, el envoltorio que lo
# lanza contra ese perfil, la raiz con la que sirve el canal (vacia si no
# depende del perfil) y el almacen montado.
#
# Almacenes:
#
# * rsa, ec, several, expired: una NSS sin contrasena con los certificados de
#   testdata/fnmt/ que le tocan; SOFTHSM2_CONF apunta a un directorio de
#   tokens propio y vacio del perfil. Ningun cliente pide PIN.
# * token: la NSS vacia con SoftHSM registrado, un token propio del perfil con
#   el RSA activo y el de curva eliptica de testdata/fnmt/, cada uno con su
#   clave; el PIN es 1234.
#
# Ningun envoltorio apunta nunca al SOFTHSM2_CONF de quien corre la suite:
# cada perfil monta el suyo, este lo use o no.
#
# El certificado personal del titular no llega al perfil: AutoFirma recibe
# HOME y -Duser.home, rFirma HOME y XDG_*. La CA local de rFirma la crea este
# script dentro del perfil, sin lanzar el cliente; la raiz de AutoFirma es la de
# su instalacion. rFirma arranca en castellano, con el tema claro, sin la
# cuenta atras de consentir y sin el asistente del primer arranque, como si la
# persona lo hubiera elegido en sus preferencias.
#
# El perfil se rehace entero en cada llamada.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fnmt="$here/../testdata/fnmt"
module="${RFIRMA_PKCS11_MODULE:-/usr/lib/softhsm/libsofthsm2.so}"

subject="${1:-}"
store="${3:-rsa}"
if [ ! -x "$subject" ]; then
    echo "uso: $0 <ruta-del-binario> [autofirma|rfirma] [rsa|ec|token|several|expired]" >&2
    exit 2
fi

case "${2:-$(basename "$subject")}" in
    autofirma) kind=autofirma ;;
    rfirma) kind=rfirma ;;
    *) kind=desconocido ;;
esac

case "$store" in
    rsa) p12s="active-rsa.p12" ;;
    ec) p12s="active-ecc.p12" ;;
    token) p12s="" ;;
    several) p12s="active-rsa.p12 active-ecc.p12 pseudonym-rsa.p12" ;;
    expired) p12s="active-ecc.p12 expired-rsa.p12" ;;
    *)
        echo "almacen desconocido: $store (rsa, ec, token, several o expired)" >&2
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

if [ "$store" = token ]; then
    for tool in softhsm2-util pkcs11-tool openssl; do
        command -v "$tool" >/dev/null || {
            echo "falta: $tool" >&2
            echo "  sudo apt install -y softhsm2 opensc openssl" >&2
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

if [ "$store" = token ]; then
    [ -f "$module" ] || {
        echo "falta el modulo PKCS#11: $module" >&2
        echo "  sudo apt install -y softhsm2" >&2
        exit 1
    }
    modutil -dbdir "sql:$nssdb" -add softhsm2 -libfile "$module" -force >/dev/null

    token_label="rfirma-conformance"
    softhsm2-util --init-token --free --label "$token_label" \
        --so-pin 3737 --pin 1234 >/dev/null

    # import_token_object <fichero .p12> <contrasena> <id> <etiqueta>
    import_token_object() {
        local p12="$1" password="$2" id="$3" label="$4"
        openssl pkcs12 -in "$fnmt/$p12" -passin "pass:$password" -clcerts -nokeys -legacy \
            | openssl x509 -outform DER -out "$profile/softhsm/cert-$id.der"
        pkcs11-tool --module "$module" --token-label "$token_label" --login --pin 1234 \
            --write-object "$profile/softhsm/cert-$id.der" --type cert --id "$id" --label "$label" \
            >/dev/null
        openssl pkcs12 -in "$fnmt/$p12" -passin "pass:$password" -nocerts -nodes -legacy \
            | openssl pkcs8 -topk8 -nocrypt -outform DER -out "$profile/softhsm/key-$id.der"
        pkcs11-tool --module "$module" --token-label "$token_label" --login --pin 1234 \
            --write-object "$profile/softhsm/key-$id.der" --type privkey --id "$id" --label "$label" \
            >/dev/null
    }

    import_token_object "active-rsa.p12" "$(the_password_of active-rsa.p12)" \
        "01" "FNMT-ACTIVO-99999999R"
    import_token_object "active-ecc.p12" "$(the_password_of active-ecc.p12)" \
        "01" "FNMT-ACTIVO-ECC-99949991H"
else
    for p12 in $p12s; do
        password="$(the_password_of "$p12")"
        [ -n "$password" ] || {
            echo "no encuentro la contrasena de $p12 en $fnmt/README.md" >&2
            exit 1
        }
        pk12util -i "$fnmt/$p12" -d "sql:$nssdb" -W "$password" -K "" >/dev/null
    done
fi

wrapper="$profile/launch-subject"
if [ "$kind" = rfirma ]; then
    cat > "$wrapper" <<WRAPPER
#!/usr/bin/env bash
export HOME="$profile"
export XDG_CONFIG_HOME="$profile/.config"
export XDG_STATE_HOME="$profile/.local/state"
export XDG_DATA_HOME="$profile/.local/share"
export XDG_CACHE_HOME="$profile/.cache"
export SOFTHSM2_CONF="$softhsm_conf"
exec "$subject" "\$@"
WRAPPER
else
    cat > "$wrapper" <<WRAPPER
#!/usr/bin/env bash
export HOME="$profile"
export JDK_JAVA_OPTIONS="-Duser.home=$profile"
export SOFTHSM2_CONF="$softhsm_conf"
exec "$subject" "\$@"
WRAPPER
fi
chmod +x "$wrapper"

# La CA local con la forma de la que genera rFirma (ADR-0005): un arranque en
# seco ya no la crea, y lanzarlo abriria su ventana principal.
the_local_ca_of_rfirma() {
    local dir="$profile/.local/share/rfirma"
    mkdir -p "$dir"
    chmod 700 "$dir"
    openssl req -x509 -new -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
        -keyout "$dir/local-ca.key.pem" -out "$dir/local-ca.crt.pem" -days 900 -sha256 \
        -subj "/CN=rFirma CA local" \
        -addext "basicConstraints=critical,CA:TRUE,pathlen:0" \
        -addext "keyUsage=critical,keyCertSign,cRLSign" \
        -addext "nameConstraints=critical,permitted;DNS:localhost,permitted;IP:127.0.0.1/255.255.255.255,permitted;IP:::1/ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff" \
        -addext "subjectKeyIdentifier=hash" 2>/dev/null
    chmod 600 "$dir/local-ca.key.pem"
    echo "$dir/local-ca.crt.pem"
}

the_configuration_of_rfirma() {
    local dir="$profile/.config/rfirma"
    mkdir -p "$dir"
    printf '{"version": 1, "consent_countdown": false, "setup_wizard_seen": true, "language": "es", "theme": "light"}\n' \
        > "$dir/config.json"
}

trust_root=""
if [ "$kind" = rfirma ]; then
    the_configuration_of_rfirma
    command -v openssl >/dev/null || {
        echo "falta: openssl" >&2
        exit 1
    }
    trust_root="$(the_local_ca_of_rfirma)"
fi
echo "almacen $store aislado en $profile" >&2

printf '%s\n%s\n%s\n%s\n' "$kind" "$wrapper" "$trust_root" "$store"
