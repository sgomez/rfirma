#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
tauri="$root/rfirma-app/src-tauri"
native_lib="$root/rfirma-native-bridge/target/lib/rfirma/librfirma_crypto.so"
destino="$HOME/.local/share/applications"
fichero="$destino/rfirma-dev.desktop"
previo="$destino/.rfirma-dev-handler-previo"

on() {
    binario="$tauri/target/debug/rfirma"
    mkdir -p "$destino"
    # Quien atendia el esquema antes se guarda, para que `off` pueda
    # devolverselo: en un equipo con AutoFirma al lado es SU lanzador, y
    # dejarlo sin manejador seria romper lo que ya funcionaba.
    if [ ! -f "$previo" ]; then
        xdg-mime query default x-scheme-handler/afirma > "$previo" || true
    fi
    {
        echo "[Desktop Entry]"
        echo "Type=Application"
        echo "Name=rFirma (desarrollo)"
        echo "Comment=NO INSTALADO: apunta al arbol de desarrollo. just dev-handler-off lo quita."
        echo "Exec=env RFIRMA_LIB_DIR=$(dirname "$native_lib") $binario %u"
        echo "Terminal=false"
        echo "NoDisplay=true"
        echo "Categories=Utility;"
        echo "MimeType=x-scheme-handler/afirma;"
    } > "$fichero"
    command -v update-desktop-database >/dev/null && update-desktop-database "$destino" || true
    xdg-mime default rfirma-dev.desktop x-scheme-handler/afirma
    echo
    echo "manejador de afirma://: $(xdg-mime query default x-scheme-handler/afirma)"
    echo "  -> $fichero"
    if [ ! -x "$binario" ]; then
        echo
        echo "AVISO: todavia no existe $binario." >&2
        echo "Arranca 'just dev' antes de pulsar el enlace de la sede." >&2
    fi
}

off() {
    rm -f "$fichero"
    command -v update-desktop-database >/dev/null && update-desktop-database "$destino" || true
    # Se le devuelve el esquema a quien lo tenia, si lo tenia alguien.
    if [ -s "$previo" ] && [ "$(cat "$previo")" != "rfirma-dev.desktop" ]; then
        xdg-mime default "$(cat "$previo")" x-scheme-handler/afirma || true
    fi
    rm -f "$previo"
    echo "manejador de afirma://: $(xdg-mime query default x-scheme-handler/afirma || echo 'ninguno')"
}

case "${1:-}" in
    on)  on ;;
    off) off ;;
    *) echo "uso: dev-handler.sh on|off" >&2; exit 1 ;;
esac
