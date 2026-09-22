#!/usr/bin/env bash
#
# Monta el perfil de usar y tirar de la suite de conformidad para un cliente y
# un almacen, e imprime cuatro lineas: la clase de cliente, el envoltorio que lo
# lanza contra ese perfil, la raiz con la que sirve el canal (vacia si no
# depende del perfil) y el almacen montado.
#
# Almacenes:
#
# * rsa, ec: una NSS sin contrasena con un solo certificado de testdata/fnmt/,
#   sin SoftHSM registrado y con SOFTHSM2_CONF apuntando a un directorio de
#   tokens vacio. Ningun cliente pide PIN.
# * token: la NSS vacia con SoftHSM registrado; sus tokens piden el PIN.
#
# El certificado personal del titular no llega al perfil: AutoFirma recibe
# HOME y -Duser.home, rFirma HOME y XDG_*. rFirma nace su CA local dentro del
# perfil en un arranque en seco; la raiz de AutoFirma es la de su instalacion.
#
# El perfil se rehace entero en cada llamada.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fnmt="$here/../testdata/fnmt"
module="${RFIRMA_PKCS11_MODULE:-/usr/lib/softhsm/libsofthsm2.so}"
softhsm_conf="${SOFTHSM2_CONF:-$HOME/.config/softhsm2/softhsm2.conf}"
warmup_attempts=60
warmup_pause=0.5

subject="${1:-}"
store="${3:-rsa}"
if [ ! -x "$subject" ]; then
    echo "uso: $0 <ruta-del-binario> [autofirma|rfirma] [rsa|ec|token]" >&2
    exit 2
fi

case "${2:-$(basename "$subject")}" in
    autofirma) kind=autofirma ;;
    rfirma) kind=rfirma ;;
    *) kind=desconocido ;;
esac

case "$store" in
    rsa) p12="active-rsa.p12" ;;
    ec) p12="active-ecc.p12" ;;
    token) p12="" ;;
    *)
        echo "almacen desconocido: $store (rsa, ec o token)" >&2
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

# La contrasena del .p12 sale de la tabla de testdata/fnmt/README.md.
the_password_of() {
    awk -F'|' -v name="\`$1\`" '$2 ~ name { gsub(/[ `]/, "", $3); print $3; exit }' \
        "$fnmt/README.md"
}

nssdb="$profile/.pki/nssdb"
rm -rf "$profile"
mkdir -p "$nssdb"
certutil -d "sql:$nssdb" -N --empty-password

if [ "$store" = token ]; then
    [ -f "$module" ] || {
        echo "falta el modulo PKCS#11: $module" >&2
        echo "  sudo apt install -y softhsm2" >&2
        exit 1
    }
    modutil -dbdir "sql:$nssdb" -add softhsm2 -libfile "$module" -force >/dev/null
else
    password="$(the_password_of "$p12")"
    [ -n "$password" ] || {
        echo "no encuentro la contrasena de $p12 en $fnmt/README.md" >&2
        exit 1
    }
    pk12util -i "$fnmt/$p12" -d "sql:$nssdb" -W "$password" -K "" >/dev/null
    mkdir -p "$profile/softhsm/tokens"
    softhsm_conf="$profile/softhsm/softhsm2.conf"
    printf 'directories.tokendir = %s\nobjectstore.backend = file\n' \
        "$profile/softhsm/tokens" > "$softhsm_conf"
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

trust_root=""
if [ "$kind" = rfirma ]; then
    local_ca="$profile/.local/share/rfirma/local-ca.crt.pem"
    "$wrapper" >/dev/null 2>&1 &
    warmup=$!
    for _ in $(seq 1 "$warmup_attempts"); do
        [ -f "$local_ca" ] && break
        kill -0 "$warmup" 2>/dev/null || break
        sleep "$warmup_pause"
    done
    kill "$warmup" 2>/dev/null || true
    wait "$warmup" 2>/dev/null || true
    if [ ! -f "$local_ca" ]; then
        echo "el sujeto no ha creado su CA local en $local_ca" >&2
        echo "  arrancalo a mano con HOME=$profile para ver que le pasa" >&2
        exit 1
    fi
    trust_root="$local_ca"
fi
echo "almacen $store aislado en $profile" >&2

printf '%s\n%s\n%s\n%s\n' "$kind" "$wrapper" "$trust_root" "$store"
