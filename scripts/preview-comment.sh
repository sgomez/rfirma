#!/usr/bin/env bash
# Compone el comentario de la preview de una PR a partir de los artefactos de la ejecución y del manifiesto.
#
# Uso: scripts/preview-comment.sh <paquetes.json> < artefactos.tsv
# Entrada: una línea por artefacto, «nombre<TAB>id<TAB>bytes<TAB>caducidad».
# Entorno: REPO, RUN_ID, RUN_URL, CONCLUSION, SHA.
set -euo pipefail

marker='<!-- rfirma-preview-comment -->'
manifest="${1:?falta el manifiesto paquetes.json}"
: "${REPO:?}" "${RUN_ID:?}" "${RUN_URL:?}" "${CONCLUSION:?}" "${SHA:?}"

artifacts="$(cat)"
short_sha="${SHA:0:7}"

case "$CONCLUSION" in
    success) ;;
    failure | timed_out | startup_failure)
        printf '%s\n\nLa construcción de prueba de `%s` ha fallado: [ver los registros](%s).\n' \
            "$marker" "$short_sha" "$RUN_URL"
        exit 0
        ;;
    *) exit 0 ;;
esac

previews="$(printf '%s\n' "$artifacts" | awk -F'\t' '$1 ~ /^rfirma-preview-./')"
if [ -z "$previews" ]; then
    exit 0
fi

artifact_url() { echo "https://github.com/$REPO/actions/runs/$RUN_ID/artifacts/$1"; }

printf '%s\n\n' "$marker"
printf 'Paquetes de prueba de `%s` ([ejecución](%s)).\n\n' "$short_sha" "$RUN_URL"
printf '| Plataforma | Descarga | Tamaño |\n| --- | --- | --- |\n'
while IFS=$'\t' read -r name id bytes _; do
    platform="${name#rfirma-preview-}"
    size="$(awk -v b="$bytes" 'BEGIN { m = int(b / 1048576 + 0.5); if (m < 1) m = 1; print m }')"
    printf '| %s | [%s](%s) | %s MB |\n' "$platform" "$name" "$(artifact_url "$id")" "$size"
done <<< "$previews"

expiry="$(printf '%s\n' "$previews" | cut -f4 | cut -c1-10 | sort | head -1)"
printf '\nCaducan el %s.\n' "$expiry"

platforms="$(printf '%s\n' "$previews" | cut -f1 | sed 's/^rfirma-preview-//' | jq -R . | jq -sc .)"
warnings="$(jq -r --argjson platforms "$platforms" \
    '[.[] | select(.warning != "" and (.platform as $p | $platforms | index($p)))
      | "- **\(.platform)**: \(.warning)"] | unique | .[]' "$manifest")"
if [ -n "$warnings" ]; then
    printf '\n%s\n' "$warnings"
fi

printf '\nDescargar un artefacto exige tener sesión iniciada en GitHub.\n'
manual_id="$(printf '%s\n' "$artifacts" | awk -F'\t' '$1 == "pdf-puerta-manual" { print $2; exit }')"
if [ -n "$manual_id" ]; then
    printf 'El PDF de la puerta manual está en [pdf-puerta-manual](%s).\n' "$(artifact_url "$manual_id")"
fi
printf 'Se reconstruye en cada push mientras la PR conserve la etiqueta `preview`.\n'
