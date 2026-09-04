#!/usr/bin/env bash
# LO QUE SE FIRMA TIENE QUE SER LO QUE SE CONSTRUYO (ADR-0015).
#
# `build.yml` calcula el `SHA256SUMS` de los paquetes y lo saca ademas como
# salida del workflow. `release.yml` descarga esos mismos paquetes por un
# artefacto, los firma y los adjunta a la Release. Entre las dos cosas hay una
# descarga y un artefacto, y esta puerta es lo unico que dice que por ahi no se
# ha colado nada distinto.
#
# EL `.rpm` CAMBIA A PROPOSITO, Y ES EL UNICO QUE PUEDE. Firmar un `.rpm` lo
# MODIFICA —la firma va dentro de la cabecera—, asi que su resumen no puede
# coincidir y compararlo seria imposible de pasar. Lo que si se exige de el es
# que siga estando: el conjunto de NOMBRES tiene que ser identico en los dos
# lados. Un paquete que aparece de la nada, o uno que desaparece entre la
# construccion y la firma, es exactamente lo que esto detecta.
#
# Es un script del repositorio y no un paso `run:` por la tercera invariante
# del ADR-0015: una puerta que no puedes reproducir en tu equipo es una puerta
# que un dia se salta con `continue-on-error`. Aqui se reproduce asi:
#
#     packaging/check-digests.sh SHA256SUMS.construccion paquetes/
#
# Uso: packaging/check-digests.sh <SHA256SUMS de referencia> <directorio>
# Salida: 0 = coinciden; 1 = no coinciden; 2 = uso incorrecto.
set -euo pipefail

referencia="${1-}"
directorio="${2-}"

if [ -z "$referencia" ] || [ -z "$directorio" ]; then
    echo "uso: packaging/check-digests.sh <SHA256SUMS de referencia> <directorio>" >&2
    exit 2
fi
if [ ! -f "$referencia" ]; then
    echo "no existe el SHA256SUMS de referencia: $referencia" >&2
    exit 2
fi
if [ ! -d "$directorio" ]; then
    echo "no existe el directorio de paquetes: $directorio" >&2
    exit 2
fi

# El `SHA256SUMS` de referencia lo escribio `sha256sum` sobre el directorio de
# paquetes, asi que sus nombres son planos y sin ruta. El propio `SHA256SUMS`
# —y su firma, si ya existe— quedan fuera de la comparacion: son productos de
# esta fase, no paquetes.
declare -A esperado=()
while read -r suma nombre; do
    [ -n "${suma:-}" ] || continue
    nombre="${nombre#\*}"
    nombre="${nombre##*/}"
    case "$nombre" in
        SHA256SUMS | SHA256SUMS.asc) continue ;;
    esac
    esperado["$nombre"]="$suma"
done < "$referencia"

if [ "${#esperado[@]}" -eq 0 ]; then
    echo "el SHA256SUMS de referencia no nombra ni un paquete: $referencia" >&2
    exit 2
fi

fallos=0
declare -A visto=()

for ruta in "$directorio"/*; do
    [ -f "$ruta" ] || continue
    nombre="${ruta##*/}"
    case "$nombre" in
        SHA256SUMS | SHA256SUMS.asc) continue ;;
    esac
    visto["$nombre"]=1

    if [ -z "${esperado[$nombre]:-}" ]; then
        echo "$nombre no estaba en la construccion" >&2
        fallos=$((fallos + 1))
        continue
    fi

    # El unico cambio permitido entre construir y firmar.
    case "$nombre" in
        *.rpm)
            echo "OK  $nombre (firmado: su resumen cambia a proposito)"
            continue
            ;;
    esac

    suma="$(sha256sum "$ruta" | cut -d' ' -f1)"
    if [ "$suma" != "${esperado[$nombre]}" ]; then
        echo "$nombre no son los bytes que salieron de la construccion" >&2
        echo "    construido: ${esperado[$nombre]}" >&2
        echo "    aqui:       $suma" >&2
        fallos=$((fallos + 1))
        continue
    fi
    echo "OK  $nombre"
done

for nombre in "${!esperado[@]}"; do
    if [ -z "${visto[$nombre]:-}" ]; then
        echo "$nombre se construyo y aqui no esta" >&2
        fallos=$((fallos + 1))
    fi
done

if [ "$fallos" -ne 0 ]; then
    echo >&2
    echo "$fallos diferencia(s) entre lo construido y lo que se va a firmar." >&2
    echo "Lo unico que puede cambiar entre las dos fases es la firma de un .rpm." >&2
    exit 1
fi

echo "OK  lo que se va a firmar es lo que se construyo"
