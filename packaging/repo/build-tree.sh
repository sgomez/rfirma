#!/usr/bin/env bash
# EL ARBOL SERVIDO ES DERIVADO (ID-172, ADR-0015): la fuente de verdad son las
# Releases, y esto reconstruye desde cero —en un directorio nuevo, cada vez— lo
# que `publish-tree.sh` sube al anfitrion. No se muta nada de lo que ya hay
# servido: si algo sale mal, se tira el directorio y se vuelve a empezar.
#
# POR QUE SE RECONSTRUYE ENTERO en vez de anadir la version nueva a lo que ya
# hay: esto corre pocas veces al ano y nadie va a recordar como estaba el
# volumen. Una publicacion no idempotente convierte cualquier fallo en
# arqueologia sobre un servidor.
#
# NADA DE LO QUE SE ESCRIBE AQUI PUEDE DEPENDER DEL MOMENTO: ni fechas, ni
# nombres de ejecucion, ni orden de listado. Tirar el volumen y republicar la
# misma serie tiene que dar el mismo arbol, byte a byte, y eso es una
# comprobacion de `build-tree.test.sh`.
#
# LA FORMA DEL ARBOL la fija el ADR-0015 y estas rutas viajan dentro del
# `.flatpakref` y de las ordenes de alta ya publicadas:
#
#   /rfirma.asc   la clave publica (el `Signed-By` de apt y el `gpgkey` de dnf)
#   /flatpak/     el repositorio ostree
#   /apt/         con dists/stable/main/binary-amd64/
#   /rpm/         con repodata/
#
# HASTA DONDE LLEGA HOY: el arbol y la clave. Llenar los tres repositorios
# —importar los bundles en el ostree, escribir la suite `stable` de apt y el
# `repodata` de dnf— es el issue hermano que va encima de este mecanismo, y
# entra en este mismo fichero. Lo que ya esta decidido aqui es de donde sale el
# material (la serie descargada, verificada) y que el arbol es nuevo cada vez.
#
# Uso: build-tree.sh <serie> <arbol> <clave.asc>
#   <serie>      directorio con un subdirectorio por Release de la serie
#                vigente, tal como lo deja `download-series.sh`
#   <arbol>      directorio de salida; SE BORRA Y SE REHACE
#   <clave.asc>  la clave publica de firma de rFirma, en armadura ASCII
set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "uso: build-tree.sh <serie> <arbol> <clave.asc>" >&2
    exit 2
fi

serie="${1%/}"
arbol="${2%/}"
clave="$3"

[ -d "$serie" ] || { echo "la serie '$serie' no existe" >&2; exit 1; }
[ -f "$clave" ] || { echo "la clave '$clave' no existe" >&2; exit 1; }

# Las versiones de la serie, en orden de version y no de listado: el orden en
# que se importan los bundles es la historia del repositorio ostree, asi que
# tiene que ser el mismo en cada reconstruccion.
mapfile -t versiones < <(find "$serie" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -V)

if [ "${#versiones[@]}" -eq 0 ]; then
    echo "la serie '$serie' no tiene ninguna version" >&2
    exit 1
fi

# Cada version tiene que traer los tres paquetes: un arbol al que le falte uno
# es un canal que se queda a medias sin que nadie se entere hasta que alguien
# no puede instalar.
for version in "${versiones[@]}"; do
    for patron in '*.flatpak' '*.deb' '*.rpm'; do
        if [ -z "$(find "$serie/$version" -maxdepth 1 -name "$patron" -print -quit)" ]; then
            echo "a la version $version le falta un paquete $patron" >&2
            exit 1
        fi
    done
done

rm -rf "$arbol"
mkdir -p "$arbol/flatpak" "$arbol/apt" "$arbol/rpm"

# La clave publica no la genera nada: es la misma que firma las Releases, y
# aqui se copia tal cual para que apt y dnf la encuentren en la ruta que dicen
# las ordenes de alta publicadas.
cp "$clave" "$arbol/rfirma.asc"

echo "OK  arbol construido en $arbol con ${#versiones[@]} version(es): ${versiones[*]}"
