#!/usr/bin/env bash
# Verifica un directorio de paquetes: su contenido, contra los resúmenes de la construcción o firmado (ADR-0015).
#
# Uso: packaging/verify-packages.sh <directorio>
#      packaging/verify-packages.sh <directorio> --against <SHA256SUMS de la construcción>
#      packaging/verify-packages.sh <directorio> --signed
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/scripts/packages-manifest.sh"
manual_gate="manual-gate.pdf"

usage() {
    echo "uso: packaging/verify-packages.sh <directorio> [--against <SHA256SUMS> | --signed]" >&2
    exit 2
}

names_in_sums() {
    local sum name
    while read -r sum name; do
        [ -n "${sum:-}" ] || continue
        name="${name#\*}"
        echo "${name##*/}"
    done < "$1"
}

check_content() {
    local dir="$1" packages package
    packages="$("$manifest" packages "$dir")"
    if [ -z "$packages" ]; then
        echo "el manifiesto de $dir no nombra ni un paquete" >&2
        exit 1
    fi
    while read -r package; do
        "$root/packaging/verifica-contenido.sh" "$dir/$package"
    done <<< "$packages"
    echo "OK  el contenido de todos los paquetes de $dir"
}

check_against() {
    local dir="$1" reference="$2" signables name path sum failures=0
    if [ ! -f "$reference" ]; then
        echo "no existe el SHA256SUMS de la construcción: $reference" >&2
        exit 2
    fi
    signables="$("$manifest" signable "$dir")"

    declare -A expected=() signable=() seen=()
    while read -r sum name; do
        [ -n "${sum:-}" ] || continue
        name="${name#\*}"
        name="${name##*/}"
        case "$name" in SHA256SUMS | SHA256SUMS.asc) continue ;; esac
        expected["$name"]="$sum"
    done < "$reference"
    if [ "${#expected[@]}" -eq 0 ]; then
        echo "el SHA256SUMS de la construcción no nombra ni un paquete: $reference" >&2
        exit 2
    fi
    while read -r name; do
        [ -n "$name" ] && signable["$name"]=1
    done <<< "$signables"

    for path in "$dir"/*; do
        [ -f "$path" ] || continue
        name="${path##*/}"
        case "$name" in SHA256SUMS | SHA256SUMS.asc) continue ;; esac
        seen["$name"]=1
        if [ -z "${expected[$name]:-}" ]; then
            echo "$name no estaba en la construcción" >&2
            failures=$((failures + 1))
        elif [ -n "${signable[$name]:-}" ]; then
            echo "OK  $name (firmable: su resumen cambia a propósito)"
        elif sum="$(sha256sum "$path" | cut -d' ' -f1)" && [ "$sum" != "${expected[$name]}" ]; then
            echo "$name no son los bytes que salieron de la construcción" >&2
            failures=$((failures + 1))
        else
            echo "OK  $name"
        fi
    done
    for name in "${!expected[@]}"; do
        if [ -z "${seen[$name]:-}" ]; then
            echo "$name se construyó y aquí no está" >&2
            failures=$((failures + 1))
        fi
    done

    if [ "$failures" -ne 0 ]; then
        echo "$failures diferencia(s) con la construcción: solo puede cambiar lo que el manifiesto marca como firmable." >&2
        exit 1
    fi
    echo "OK  $dir es lo que se construyó"
}

check_signed() {
    local dir="$1" name failures=0
    if ! (cd "$dir" && gpg --verify SHA256SUMS.asc SHA256SUMS); then
        echo "la firma del SHA256SUMS de $dir no es válida" >&2
        exit 1
    fi
    if ! (cd "$dir" && sha256sum --check --strict --quiet SHA256SUMS); then
        echo "los ficheros de $dir no son los que resume su SHA256SUMS" >&2
        exit 1
    fi

    declare -A listed=()
    while read -r name; do
        listed["$name"]=1
    done < <(names_in_sums "$dir/SHA256SUMS")
    for path in "$dir"/*; do
        name="${path##*/}"
        case "$name" in SHA256SUMS | SHA256SUMS.asc | "$manual_gate") continue ;; esac
        if [ -z "${listed[$name]:-}" ]; then
            echo "$name está en la Release y no en su SHA256SUMS" >&2
            failures=$((failures + 1))
        fi
    done
    if [ -f "$dir/paquetes.json" ]; then
        while read -r name; do
            if [ -z "${listed[$name]:-}" ]; then
                echo "$name está en el manifiesto y no en el SHA256SUMS" >&2
                failures=$((failures + 1))
            fi
        done < <("$manifest" files "$dir")
    fi

    if [ "$failures" -ne 0 ]; then
        echo "$failures fichero(s) de $dir fuera del SHA256SUMS firmado" >&2
        exit 1
    fi
    echo "OK  $dir firmado y sin nada fuera del SHA256SUMS"
}

dir="${1-}"
[ -n "$dir" ] || usage
if [ ! -d "$dir" ]; then
    echo "no existe el directorio de paquetes: $dir" >&2
    exit 2
fi
dir="${dir%/}"

case "${2-}" in
    "") [ "$#" -eq 1 ] || usage; check_content "$dir" ;;
    --against) [ "$#" -eq 3 ] || usage; check_against "$dir" "$3" ;;
    --signed) [ "$#" -eq 2 ] || usage; check_signed "$dir" ;;
    *) usage ;;
esac
