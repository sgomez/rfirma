#!/bin/sh
# Guía que mide, desde dentro de un flatpak desechable, el camino OpenSC → libpcsclite → pcscd del anfitrión con un lector y un DNIe reales.
#
# Uso: sh flatpak_probe.sh [--keep]
# Empieza con el lector enchufado y el DNIe dentro. Nunca hace C_Login ni envía un PIN: lista
# lectores, ranuras y etiquetas de certificados, y redacta los números de serie del token y del lector.
# Construye me.sgomez.RfirmaDnieProbe en un directorio temporal, lo instala con --user y lo
# desinstala al acabar salvo con --keep. Dependencias: flatpak, flatpak-builder,
# org.gnome.Sdk//50 y red para bajar pcsc-lite y OpenSC.
set -eu

keep=no
[ "${1:-}" = "--keep" ] && keep=yes
work=$(mktemp -d /tmp/rfirma-flatpak-probe.XXXXXX)
app=me.sgomez.RfirmaDnieProbe

redact() {
    sed -E -e 's/(serial num *:).*/\1 <SERIE>/I' \
           -e 's/\([0-9A-Fa-f]{6,}\)/(<SERIE>)/g'
}

inside() {
    flatpak run --command=sh "$app" -c "$1" 2>&1 | redact
}

pause() {
    printf '\n>>> %s\n    Pulsa Intro para seguir. ' "$1"
    read -r _
}

cleanup() {
    if [ "$keep" = no ]; then
        flatpak uninstall --user -y "$app" >/dev/null 2>&1 || true
        rm -rf "$work"
    fi
}
trap cleanup EXIT

cat > "$work/$app.yml" <<'EOF'
app-id: me.sgomez.RfirmaDnieProbe
runtime: org.gnome.Platform
runtime-version: '50'
sdk: org.gnome.Sdk
command: opensc-tool
finish-args:
  - --socket=pcsc
modules:
  - name: pcsc-lite
    buildsystem: meson
    config-opts: [--libdir=lib, -Dlibsystemd=false, -Dlibudev=false, -Dlibusb=false, -Dpolkit=false,
                  -Dipcdir=/run/pcscd, -Dusbdropdir=/app/lib/pcsc/drivers]
    sources:
      - type: archive
        url: https://pcsclite.apdu.fr/files/pcsc-lite-2.5.1.tar.xz
        sha256: bfcfe38a20afc49849c6bf55325e38f449fc4b26d3923fdc32b969ae41a8741b
  - name: opensc
    buildsystem: autotools
    config-opts: [--disable-static, --disable-notify, --disable-doc, --disable-strict,
                  --disable-tests, --enable-pcsc, --enable-sm, --enable-openssl, --enable-zlib,
                  --disable-readline, --disable-openpace, --with-pcsc-provider=libpcsclite.so.1,
                  --enable-p11_system_config_modules=/app/share/p11-kit/modules]
    make-install-args: [completiondir=/app/share/bash-completion/completions]
    sources:
      - type: archive
        url: https://github.com/OpenSC/OpenSC/releases/download/0.27.1/opensc-0.27.1.tar.gz
        sha256: 976f4a23eaf3397a1a2c3a7aac80bf971a8c3d829c9a79f06145bfaeeae5eca7
EOF

echo "== Construyendo $app en $work (unos minutos la primera vez)"
flatpak-builder --user --install --force-clean --state-dir="$work/state" \
    "$work/build" "$work/$app.yml" > "$work/build.log" 2>&1 \
    || { echo "La construcción falló: $work/build.log"; keep=yes; exit 1; }

since=$(date '+%Y-%m-%d %H:%M:%S')

echo "== 1. El anfitrión"
echo "pcscd: $(pcscd --version 2>&1 | head -1)"
systemctl is-active pcscd.socket || true

echo "== 2. El socket dentro del sandbox"
inside 'ls -la /run/pcscd/; echo "PCSCLITE_CSOCK_NAME=$PCSCLITE_CSOCK_NAME"'

echo "== 3. Protocolo y lectores (access_pcsc)"
flatpak run --env=PCSCLITE_DEBUG=0 --command=sh "$app" -c 'opensc-tool -l' 2>&1 \
    | grep -E 'protocol version|backward|readers|^[0-9]+ ' | redact

echo "== 4. SCardConnect a la tarjeta (access_card); el ATR no se imprime"
inside 'opensc-tool -a >/dev/null 2>&1 && echo "ATR leído" || echo "SCardConnect falló"'
inside 'opensc-tool -n'

echo "== 5. PKCS#11: ranuras y token, sin login"
inside 'pkcs11-tool --module /app/lib/pkcs11/opensc-pkcs11.so -L' \
    | grep -E 'Slot|token label|token flags|serial num|No slots|Available'

echo "== 6. Etiquetas de los certificados, sin login (lista blanca: Cert*)"
inside 'pkcs11-tool --module /app/lib/pkcs11/opensc-pkcs11.so -O --type cert' \
    | grep -E '^ *label: *Cert' || echo "ningún certificado Cert*"

echo "== 7. Lo que dice el pcscd del anfitrión desde el paso 1"
journalctl -u pcscd --since "$since" --no-pager -o cat 2>/dev/null \
    | grep -E 'mismatch|protocol is|backward|NOT authorized|Rejected' || echo "sin avisos"

echo "== 8. En caliente, con un único sandbox abierto 40 s"
echo "   Durante la cuenta: saca el DNIe, desenchufa el lector, enchúfalo y mete el DNIe."
pause "La cuenta empieza al pulsar Intro"
inside 'last=""; i=0; while [ $i -lt 40 ]; do
    now=$(opensc-tool -l 2>/dev/null | awk "/^[0-9]+ /{print \$2}" | tr "\n" " ")
    [ -z "$now" ] && now="sin lector"
    [ "$now" != "$last" ] && echo "t=${i}s tarjeta por lector: $now"
    last=$now; i=$((i+1)); sleep 1
done'

echo "== Hecho. Copia la salida tal cual: ya va redactada."
