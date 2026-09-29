#!/usr/bin/env bash
# El manifiesto `paquetes.json` de una entrega: escribe una fila por paquete y lo lee (ADR-0015).
#
# Uso: scripts/packages-manifest.sh write <directorio>
#      scripts/packages-manifest.sh files <directorio> [formato]
#      scripts/packages-manifest.sh signable <directorio>
set -euo pipefail

manifest_name="paquetes.json"

row_for() {
    case "$1" in
        *.flatpak) echo "flatpak|flatpak|false|Necesita el remoto de Flathub añadido: el runtime no viaja dentro." ;;
        *.deb) echo "linux|deb|false|" ;;
        *.rpm) echo "linux|rpm|true|" ;;
        *-setup.exe) echo "windows|nsis|false|Windows SmartScreen avisará de que el instalador no está firmado." ;;
        *) return 1 ;;
    esac
}

write_manifest() {
    local dir="$1" path name row platform format signable warning
    local rows=()
    for path in "$dir"/*; do
        [ -f "$path" ] || continue
        name="${path##*/}"
        case "$name" in SHA256SUMS | SHA256SUMS.asc | "$manifest_name") continue ;; esac
        if ! row="$(row_for "$name")"; then
            echo "paquete con una extension desconocida: $name" >&2
            exit 1
        fi
        IFS='|' read -r platform format signable warning <<< "$row"
        rows+=("$(jq -n --arg platform "$platform" --arg file "$name" --arg format "$format" \
            --argjson signable "$signable" --arg warning "$warning" \
            '{platform: $platform, file: $file, format: $format, signable: $signable, warning: $warning}')")
    done
    if [ "${#rows[@]}" -eq 0 ]; then
        echo "no hay ni un paquete en $dir" >&2
        exit 1
    fi
    printf '%s\n' "${rows[@]}" | jq -s '.' > "$dir/$manifest_name"
}

require_manifest() {
    if [ ! -f "$1/$manifest_name" ]; then
        echo "no existe $1/$manifest_name" >&2
        exit 2
    fi
}

command="${1-}"
dir="${2-}"
if [ -z "$command" ] || [ -z "$dir" ] || [ ! -d "$dir" ]; then
    echo "uso: scripts/packages-manifest.sh write|files|signable <directorio> [formato]" >&2
    exit 2
fi

case "$command" in
    write) write_manifest "$dir" ;;
    files)
        require_manifest "$dir"
        jq -r --arg format "${3-}" \
            '.[] | select($format == "" or .format == $format) | .file' "$dir/$manifest_name"
        ;;
    signable)
        require_manifest "$dir"
        jq -r '.[] | select(.signable) | .file' "$dir/$manifest_name"
        ;;
    *)
        echo "orden desconocida: $command" >&2
        exit 2
        ;;
esac
