#!/usr/bin/env bash
# COMO LLEGAN LOS BYTES AL ANFITRION Y COMO ENTRAN EN SERVICIO (ID-172, ID-174,
# ADR-0015). Este fichero es el mecanismo entero; lo que se sube —la forma de
# los tres repositorios— lo construye `build-tree.sh` y no se mira aqui.
#
# TRES ORDENES DE `rsync` Y EL ORDEN IMPORTA:
#
#   1. el arbol nuevo entero, a un directorio SUYO (`arboles/<etiqueta>`);
#   2. el enlace `actual`, que es lo unico que sirve el servidor web;
#   3. la poda, que deja el vigente y el anterior.
#
# Mientras (1) no termina, nadie ve nada: el enlace sigue apuntando al arbol de
# antes, asi que un despliegue a medias es invisible y se reintenta sin mas. La
# poda va DESPUES del intercambio para que un fallo nunca borre lo que se esta
# sirviendo, y deja el anterior porque la vuelta atras es reapuntar el enlace,
# no volver a publicar.
#
# EL TRANSPORTE ES `rsync` SOBRE SSH CON ORDEN FORZADA: la clave del CI esta
# atada en el `authorized_keys` del VPS a `command="rrsync /srv/rfirma-repo"`,
# sin pty ni reenvio de puertos. Eso NO es decoracion para este fichero: rrsync
# lleva su propia lista de opciones admitidas y las que no estan en ella matan
# la orden. Por eso aqui no hay `--filter` (rrsync no lo admite) y la poda se
# hace con `-d --delete --force`, que si. Cambiar una opcion de este fichero
# sin pasar `publish-tree.test.sh` es cambiarla a ciegas.
#
# El arbol servido es DERIVADO: no se muta nada en el servidor, se reconstruye
# entero y se intercambia. Tirar `/srv/rfirma-repo` y volver a publicar tiene
# que dejar exactamente el mismo servicio, asi que aqui no se escribe ninguna
# marca de fecha ni de ejecucion.
#
# Uso: publish-tree.sh <arbol> <etiqueta> <destino>
#   <arbol>     directorio local ya construido, tal cual se va a servir
#   <etiqueta>  nombre del arbol en el anfitrion (la etiqueta de la Release)
#   <destino>   'usuario@anfitrion:' (con la orden forzada detras) o un
#               directorio local, para ensayar sin tocar el VPS
#
# Variables: RSYNC_RSH, la de siempre de rsync, para las opciones de ssh.
set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "uso: publish-tree.sh <arbol> <etiqueta> <destino>" >&2
    exit 2
fi

arbol="${1%/}"
etiqueta="$2"
destino="${3%/}"

if [ ! -d "$arbol" ]; then
    echo "el arbol '$arbol' no existe" >&2
    exit 1
fi

# Un arbol vacio publicado por descuido dejaria el dominio sirviendo nada: no
# hay ninguna razon legitima para publicar cero ficheros.
if [ -z "$(find "$arbol" -type f -print -quit)" ]; then
    echo "el arbol '$arbol' no tiene ni un fichero: no se publica" >&2
    exit 1
fi

# La etiqueta es un nombre de directorio en el anfitrion y ademas viaja dentro
# del enlace simbolico: cualquier cosa rara ahi es una travesia de rutas.
if ! [[ "$etiqueta" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "la etiqueta '$etiqueta' no es un nombre de directorio admisible" >&2
    exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Lo que se esta sirviendo AHORA. Se lee por rsync y no por ssh a proposito: la
# orden forzada no da consola, asi que esta es la unica forma de preguntarselo
# al anfitrion. Que no haya enlace es lo normal la primera vez.
anterior=""
if rsync --list-only --links "$destino/actual" > "$tmp/enlace-vigente" 2>/dev/null; then
    anterior="$(sed -n 's/.* -> //p' "$tmp/enlace-vigente" | head -1)"
    anterior="${anterior#arboles/}"
fi

if [ -n "$anterior" ]; then
    echo "  sirviendo ahora: $anterior"
else
    echo "  sirviendo ahora: nada (primera publicacion)"
fi

# --------------------------------------------------------------- 1. el arbol
# `--delete` porque el arbol es derivado: si se republica la misma etiqueta, lo
# que ya no esta en el origen tampoco puede quedarse en el anfitrion.
# `--mkpath` crea `arboles/` la primera vez, que es el unico momento en que el
# destino no existe todavia.
echo "  subiendo el arbol a arboles/$etiqueta"
rsync --archive --delete --mkpath --human-readable \
    "$arbol/" "$destino/arboles/$etiqueta/"

# -------------------------------------------------------------- 2. el enlace
# EL GESTO ATOMICO. Se envia un enlace simbolico llamado `actual`; rsync lo
# escribe con un nombre temporal y lo renombra encima del que hubiera
# (`--delay-updates` fuerza ese camino), asi que quien pida un fichero durante
# el intercambio recibe el arbol de antes o el de ahora, nunca medio arbol.
echo "  intercambiando el enlace: actual -> arboles/$etiqueta"
mkdir "$tmp/enlace"
ln -s "arboles/$etiqueta" "$tmp/enlace/actual"
rsync --archive --links --delay-updates "$tmp/enlace/" "$destino/"

# El enlace es TODO el servicio: si no ha quedado donde se cree, el despliegue
# no ha ocurrido y no se poda nada.
rsync --list-only --links "$destino/actual" > "$tmp/enlace-nuevo"
if ! grep -q -- "-> arboles/$etiqueta\$" "$tmp/enlace-nuevo"; then
    echo "el enlace 'actual' no ha quedado apuntando a arboles/$etiqueta" >&2
    cat "$tmp/enlace-nuevo" >&2
    exit 1
fi

# ----------------------------------------------------------------- 3. la poda
# Se queda el vigente y el anterior, y ni uno mas: el anterior existe para que
# la vuelta atras sea reapuntar el enlace, y los de antes no le sirven a nadie
# —las Releases, que son la fuente de verdad, no se borran nunca—.
#
# La lista de lo que se queda se envia como directorios VACIOS y sin recursion
# (`-d`): rsync borra del anfitrion lo que no este en esa lista y, al no bajar
# dentro de los que si estan, no toca su contenido. `--force` es lo que le
# permite borrar un arbol viejo, que nunca esta vacio.
mkdir "$tmp/retencion"
mkdir "$tmp/retencion/$etiqueta"
if [ -n "$anterior" ] && [ "$anterior" != "$etiqueta" ]; then
    mkdir -p "$tmp/retencion/$anterior"
    echo "  podando: se quedan $etiqueta y $anterior"
else
    echo "  podando: se queda $etiqueta"
fi
rsync --dirs --delete --force "$tmp/retencion/" "$destino/arboles/"

echo "OK  publicado $etiqueta"
