#!/usr/bin/env bash
# ID-154: una etiqueta de candidata (`-rc.N`) NO produce paquetes nativos.
#
# El motivo no es de politica, es del formato: el campo `Version` de un RPM no
# admite guiones, asi que `0.4.0-rc.1` no tiene representacion valida ahi. El
# ensayo de la tubula de publicacion es real —construye, firma y publica una
# Release— pero mas ESTRECHO que la entrega: la candidata produce el flatpak y
# nada mas.
#
# Esta es la unica sede de esa regla. Quien empaquete (la receta del bundler,
# el workflow de publicacion) la consulta en vez de reimplementarla:
#
#     if packaging/native-packages-allowed.sh "$VERSION"; then
#         ... construir .deb y .rpm ...
#     fi
#
# Uso: packaging/native-packages-allowed.sh <version>
# Salida: 0 = se producen .deb y .rpm; 1 = solo flatpak; 2 = uso incorrecto.
set -euo pipefail

version="${1-}"
version="${version#v}"

if [ -z "$version" ]; then
    echo "uso: packaging/native-packages-allowed.sh <version>" >&2
    exit 2
fi

# Cualquier prerelease de semver, no solo `-rc.N`: lo que rompe el RPM es el
# guion, venga de donde venga.
case "$version" in
    *-*)
        echo "$version es una candidata: solo flatpak, sin .deb ni .rpm (ID-154)"
        exit 1
        ;;
esac

echo "$version es una version de entrega: se producen .deb y .rpm"
exit 0
