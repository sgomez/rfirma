#!/usr/bin/env bash
#
# Monta un perfil NSS de usar y tirar con SoftHSM como unico modulo PKCS#11 y
# escribe dentro un envoltorio que lanza contra el el binario que se le indique.
# Imprime por la salida estandar la ruta del envoltorio, que es lo que `just
# probe` le pasa al sondeo como sujeto en lugar del binario a secas.
#
# Existe porque AutoFirma en Linux usa el almacen SHARED_NSS: lee la base de
# datos NSS del usuario y carga de su 'pkcs11.txt' los modulos externos. En un
# equipo de desarrollo esa base contiene el certificado personal del titular,
# que este proyecto no usa en ningun punto; con el perfil aislado el sondeo solo
# alcanza los tokens de pruebas, por construccion y no por disciplina.
#
# No basta con sustituir HOME: la JVM no saca 'user.home' del entorno sino de la
# entrada del usuario en el sistema, asi que con HOME cambiado y nada mas
# AutoFirma sigue leyendo la base real. El envoltorio fija las dos cosas.
#
# El perfil se rehace entero en cada llamada y no se toca la base del titular.

set -euo pipefail

module="${RFIRMA_PKCS11_MODULE:-/usr/lib/softhsm/libsofthsm2.so}"
profile="${RFIRMA_PROBE_PROFILE:-${XDG_CACHE_HOME:-$HOME/.cache}/rfirma/probe-profile}"
softhsm_conf="${SOFTHSM2_CONF:-$HOME/.config/softhsm2/softhsm2.conf}"

subject="${1:-}"
if [ ! -x "$subject" ]; then
    echo "uso: $0 <ruta-del-binario>" >&2
    exit 2
fi

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
cat > "$wrapper" <<WRAPPER
#!/usr/bin/env bash
export HOME="$profile"
export JDK_JAVA_OPTIONS="-Duser.home=$profile"
export SOFTHSM2_CONF="$softhsm_conf"
exec "$subject" "\$@"
WRAPPER
chmod +x "$wrapper"

echo "almacen aislado en $profile, con $module y nada mas" >&2
printf '%s\n' "$wrapper"
