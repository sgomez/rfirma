#!/usr/bin/env bash
# El manifiesto `paquetes.json` de una entrega: escribe una fila por paquete y lo lee (ADR-0015).
#
# Uso: scripts/packages-manifest.sh write <directorio>
#      scripts/packages-manifest.sh files <directorio> [formato]
#      scripts/packages-manifest.sh packages <directorio>
#      scripts/packages-manifest.sh signable <directorio>
#      scripts/packages-manifest.sh platforms <directorio>
#      scripts/packages-manifest.sh platform-files <directorio> <plataforma>
#      scripts/packages-manifest.sh notes <directorio>
set -euo pipefail

manifest_name="paquetes.json"

row_for() {
    case "$1" in
        *.flatpak) echo "flatpak|flatpak|false|Necesita el remoto de Flathub añadido (\`flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo\`): el runtime no viaja dentro." ;;
        *.deb) echo "linux|deb|false|" ;;
        *.rpm) echo "linux|rpm|true|" ;;
        *-setup.exe) echo "windows|nsis|false|Windows SmartScreen avisará de que el instalador no está firmado: pulsa «Más información» y «Ejecutar de todas formas»." ;;
        *-setup.exe.sig) echo "windows|minisign|false|" ;;
        *.dmg) echo "macos|dmg|false|Gatekeeper bloqueará la aplicación, que no está notarizada: pulsa «Abrir igualmente» en Ajustes del Sistema › Privacidad y seguridad." ;;
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

write_notes() {
    cat <<'NOTES'
## Antes de publicar

Publicar esta Release es lo que dispara la distribución a los tres repositorios de
`rfirma.sgomez.me`. Antes de hacerlo:

1. Descarga `manual-gate.pdf` y llévalo a un validador oficial (VALIDe). La puerta manual no la
   cierra ninguna prueba automática.
2. Comprueba la firma de los paquetes:

   ```
   gpg --verify SHA256SUMS.asc SHA256SUMS
   sha256sum --check SHA256SUMS
   ```

La clave que firma es la de rFirma, publicada en <https://rfirma.sgomez.me/rfirma.asc>. Su huella y
su política de revocación están en `SECURITY.md`.

## Instalación

NOTES
    jq -r '.[] | select(.warning != "") | "- `\(.file)`: \(.warning)"' "$1/$manifest_name"
}

command="${1-}"
dir="${2-}"
if [ -z "$command" ] || [ -z "$dir" ] || [ ! -d "$dir" ]; then
    echo "uso: scripts/packages-manifest.sh write|files|packages|signable|platforms|platform-files|notes <directorio> [formato|plataforma]" >&2
    exit 2
fi

case "$command" in
    write) write_manifest "$dir" ;;
    files)
        require_manifest "$dir"
        jq -r --arg format "${3-}" \
            '.[] | select($format == "" or .format == $format) | .file' "$dir/$manifest_name"
        ;;
    packages)
        require_manifest "$dir"
        jq -r '.[] | select(.format != "minisign") | .file' "$dir/$manifest_name"
        ;;
    signable)
        require_manifest "$dir"
        jq -r '.[] | select(.signable) | .file' "$dir/$manifest_name"
        ;;
    platforms)
        require_manifest "$dir"
        jq -r '[.[].platform] | unique | .[]' "$dir/$manifest_name"
        ;;
    platform-files)
        require_manifest "$dir"
        jq -r --arg platform "${3:?falta la plataforma}" \
            '.[] | select(.platform == $platform) | .file' "$dir/$manifest_name"
        ;;
    notes)
        require_manifest "$dir"
        write_notes "$dir"
        ;;
    *)
        echo "orden desconocida: $command" >&2
        exit 2
        ;;
esac
