#!/usr/bin/env bash
# LA FUENTE DE VERDAD SON LAS RELEASES (ID-172, ADR-0015). Esto baja la serie
# menor vigente entera —todas sus versiones, no solo la que se acaba de
# publicar— porque el arbol servido se reconstruye desde cero en cada
# publicacion: si aqui falta una version, deja de servirse.
#
# LO QUE NO ENTRA: los borradores y las CANDIDATAS. Una etiqueta `v*-rc.N`
# produce una Release marcada como prerelease y no llega a ningun repositorio,
# que es como se ensaya la tuberia entera sin publicar una version de verdad.
#
# SE VERIFICA LO DESCARGADO, VERSION A VERSION, y con las dos comprobaciones
# que hacen falta: la firma dice quien lo hizo y el `sha256sum --check` dice
# que los ficheros son esos. Sin la segunda, un asset cambiado despues de
# firmar pasaria la primera. La clave publica tiene que estar ya en el llavero
# (en el CI la importa `.github/actions/import-signing-key`).
#
# Uso: download-series.sh <etiqueta> <serie>
#   <etiqueta>  la etiqueta que se esta publicando, p. ej. v0.4.2
#   <serie>     directorio de salida; SE BORRA Y SE REHACE
# Necesita GH_TOKEN en el entorno, como cualquier uso de `gh`.
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "uso: download-series.sh <etiqueta> <serie>" >&2
    exit 2
fi

etiqueta="$1"
serie="${2%/}"

if ! [[ "$etiqueta" =~ ^(v[0-9]+\.[0-9]+)\. ]]; then
    echo "la etiqueta '$etiqueta' no tiene la forma vMAYOR.MENOR.PARCHE" >&2
    exit 1
fi
prefijo="${BASH_REMATCH[1]}."

rm -rf "$serie"
mkdir -p "$serie"

# `--exclude-drafts` no basta: una candidata tampoco es draft, y sirve para
# ensayar precisamente porque llega hasta aqui y no pasa de aqui.
mapfile -t versiones < <(
    gh release list --limit 200 --exclude-drafts \
        --json tagName,isPrerelease,isDraft \
        --jq '.[] | select(.isPrerelease | not) | select(.isDraft | not) | .tagName' \
        | grep "^${prefijo//./\\.}" | sort -V
)

if [ "${#versiones[@]}" -eq 0 ]; then
    echo "no hay ninguna Release publicada de la serie $prefijo*" >&2
    exit 1
fi

for version in "${versiones[@]}"; do
    echo "  $version"
    gh release download "$version" --dir "$serie/$version"
    (
        cd "$serie/$version"
        gpg --verify SHA256SUMS.asc SHA256SUMS
        # El PDF de la puerta manual va adjunto a la Release pero NO es un
        # paquete y no entra en el SHA256SUMS: se descarta antes de comprobar,
        # en vez de hacerle un hueco en el fichero firmado.
        rm -f manual-gate.pdf
        sha256sum --check --strict SHA256SUMS
    )
done

echo "OK  serie $prefijo* descargada y verificada: ${versiones[*]}"
