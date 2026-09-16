#!/usr/bin/env bash
#
# Monta el almacen de usar y tirar del sondeo para el sujeto que se le indique e
# imprime tres lineas: la clase de sujeto reconocida, el envoltorio que lo lanza
# contra ese almacen y la raiz de confianza con la que el sujeto va a servir el
# canal, vacia cuando no depende del perfil.
#
# Existe porque en un equipo de desarrollo los almacenes del titular contienen
# su certificado personal, que este proyecto no usa en ningun punto, y los dos
# sujetos llegan a el: AutoFirma por el almacen SHARED_NSS y rFirma por los
# perfiles NSS que busca bajo HOME. Con el perfil aislado solo alcanzan los
# tokens de pruebas de SoftHSM.
#
# Cada sujeto necesita un aislamiento distinto:
#
# * AutoFirma es Java, y la JVM no saca 'user.home' del entorno sino de la
#   entrada del usuario en el sistema: ademas de HOME hay que darle
#   -Duser.home. Su raiz de confianza vive fuera del perfil, en la instalacion
#   del sistema, asi que no la imprime.
# * rFirma resuelve sus rutas por XDG y es su propia CA: con el perfil vacio se
#   comporta como instalacion nueva y genera una CA dentro. El sondeo lo arranca
#   una vez en seco para que nazca y declara esa CA como raiz; servir con la del
#   titular exigiria su clave privada, y el perfil no toca nada suyo.
#
# El perfil se rehace entero en cada llamada.

set -euo pipefail

module="${RFIRMA_PKCS11_MODULE:-/usr/lib/softhsm/libsofthsm2.so}"
profile="${RFIRMA_PROBE_PROFILE:-${XDG_CACHE_HOME:-$HOME/.cache}/rfirma/probe-profile}"
softhsm_conf="${SOFTHSM2_CONF:-$HOME/.config/softhsm2/softhsm2.conf}"
warmup_attempts=60
warmup_pause=0.5

subject="${1:-}"
if [ ! -x "$subject" ]; then
    echo "uso: $0 <ruta-del-binario>" >&2
    exit 2
fi

case "$(basename "$subject")" in
    autofirma) kind=autofirma ;;
    rfirma) kind=rfirma ;;
    *) kind=desconocido ;;
esac

for tool in certutil modutil; do
    command -v "$tool" >/dev/null || {
        echo "falta: $tool" >&2
        echo "  sudo apt install -y libnss3-tools" >&2
        exit 1
    }
done

[ -f "$module" ] || {
    echo "falta el modulo PKCS#11: $module" >&2
    echo "  sudo apt install -y softhsm2" >&2
    exit 1
}

nssdb="$profile/.pki/nssdb"
rm -rf "$profile"
mkdir -p "$nssdb"
certutil -d "sql:$nssdb" -N --empty-password
modutil -dbdir "sql:$nssdb" -add softhsm2 -libfile "$module" -force >/dev/null

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
    echo "almacen aislado en $profile, con $module y la CA local que ha nacido dentro" >&2
else
    echo "almacen aislado en $profile, con $module y nada mas" >&2
fi

printf '%s\n%s\n%s\n' "$kind" "$wrapper" "$trust_root"
