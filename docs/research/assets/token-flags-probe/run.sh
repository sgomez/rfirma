#!/bin/bash
# Corre la sonda del sondeo #117 contra los cuatro almacenes, un proceso por
# almacén (C_Initialize es por proceso y módulo; ver src/main.rs).
# Uso: run.sh [directorio de trabajo con los perfiles que crea setup-nss.sh]
set -u
export PATH="$HOME/.cargo/bin:$PATH"
WORK=${1:-/tmp/rfirma-117}
NSS=$WORK/nss
HERE=$(cd "$(dirname "$0")" && pwd)
BIN=$HERE/target/debug/token-flags-probe
SOFTOKN=/usr/lib/x86_64-linux-gnu/libsoftokn3.so
SOFTHSM=/usr/lib/softhsm/libsofthsm2.so

[ -x "$BIN" ] || (cd "$HERE" && cargo build) || exit 1

# Los mismos init args que construye `Store::nss` en rfirma-app.
nss_args() { echo "configdir='sql:$1' certPrefix='' keyPrefix='' secmod='secmod.db' flags=readOnly"; }

echo "# perfil NSS SIN contraseña maestra (Firefox por defecto)"
"$BIN" "$SOFTOKN" "$(nss_args "$NSS/nopass")" -- 1234 EMPTY NONE
echo; echo "# perfil NSS CON contraseña maestra («secreto»)"
"$BIN" "$SOFTOKN" "$(nss_args "$NSS/master")" -- 1234 EMPTY NONE secreto
echo; echo "# ~/.pki/nssdb recién creado, contraseña vacía, sin certificados"
"$BIN" "$SOFTOKN" "$(nss_args "$NSS/emptydb")" -- 1234 EMPTY NONE
echo; echo "# SoftHSM, token rfirma-test"
"$BIN" "$SOFTHSM" -- 1234 EMPTY NONE
