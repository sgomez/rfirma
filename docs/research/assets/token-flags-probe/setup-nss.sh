#!/bin/bash
# Perfiles NSS desechables para el sondeo #117. Solo material de pruebas de la
# FNMT (docs/research/token-pkcs11-pruebas.md); el perfil real de Firefox del
# titular no se toca.
# Uso: setup-nss.sh [directorio de trabajo]  (crea <dir>/nss/{nopass,master,emptydb})
set -u
S=${1:-/tmp/rfirma-117}/nss
rm -rf "$S"; mkdir -p "$S"; cd "$S" || exit 1
P12="$HOME/.local/share/rfirma-test-certs/Claves RSA/AC FNMT Usuarios/Nuevos/Nuevo Perfil no SMIME/ACTIVO_EIDAS_CERTIFICADO_PRUEBAS___99999999R.p12"
ls -la "$P12" || exit 1
# Los .p12 de la FNMT usan cifrado antiguo y pk12util los rechaza: se reexportan.
openssl pkcs12 -in "$P12" -passin pass:1234 -nodes -legacy 2>/dev/null \
  | openssl pkcs12 -export -passout pass:1234 -name FNMT-ACTIVO-99999999R -out modern.p12 && echo "p12 reexportado"
mkdir -p nopass master emptydb
printf '' > empty.txt; printf 'secreto' > master.txt

echo "== nopass (sin contraseña maestra, como Firefox por defecto)"
certutil -N -d sql:"$S/nopass" --empty-password && pk12util -i modern.p12 -d sql:"$S/nopass" -W 1234 -K "" && echo nopass ok
echo "== master (con contraseña maestra «secreto»)"
certutil -N -d sql:"$S/master" -f master.txt && pk12util -i modern.p12 -d sql:"$S/master" -W 1234 -k master.txt && echo master ok
echo "== emptydb (~/.pki/nssdb recién creado, contraseña vacía, sin certificados)"
certutil -N -d sql:"$S/emptydb" --empty-password && echo emptydb ok

for p in nopass master emptydb; do
  echo "== listado $p"; certutil -L -d sql:"$S/$p"
  if [ "$p" = master ]; then certutil -K -d sql:"$S/$p" -f master.txt 2>&1 | head -3; else certutil -K -d sql:"$S/$p" -f empty.txt 2>&1 | head -3; fi
done
