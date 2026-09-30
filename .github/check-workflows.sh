#!/usr/bin/env bash
# Las invariantes de los workflows y de sus acciones que ninguna ejecucion detecta (ADR-0015).
#
# Uso: .github/check-workflows.sh
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$raiz"

sin_comentarios() {
    grep -vE ':[0-9]+:[[:space:]]*#'
}

# ------------------------------------------------- acciones fijadas por SHA --
fallos=0

while IFS= read -r linea; do
    fichero="${linea%%:*}"
    resto="${linea#*:}"
    numero="${resto%%:*}"
    texto="${resto#*:}"

    case "$texto" in
        *uses:*./*) continue ;;
    esac

    if [[ "$texto" =~ uses:[[:space:]]*\"?[A-Za-z0-9._-]+/[A-Za-z0-9._/-]+@[0-9a-f]{40}\"?[[:space:]]*\#[[:space:]]*[^[:space:]]+ ]]; then
        continue
    fi

    echo "$fichero:$numero: accion sin fijar por SHA (ADR-0015)" >&2
    echo "    ${texto# }" >&2
    fallos=$((fallos + 1))
done < <(grep -rn '^[[:space:]]*-\?[[:space:]]*uses:' .github/workflows .github/actions || true)

if [ "$fallos" -ne 0 ]; then
    echo >&2
    echo "$fallos accion(es) sin fijar. Cada una se escribe asi:" >&2
    echo "    uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0" >&2
    echo "El SHA se saca con: gh api repos/<owner>/<repo>/commits/<etiqueta> --jq .sha" >&2
    exit 1
fi

echo "OK  todas las acciones de .github/workflows y .github/actions estan fijadas por SHA"

# ------------------------------------------------ construccion sin secretos --
# Las acciones locales que alcanza un fichero, directa o indirectamente.
acciones_locales() {
    local pendientes=("$1") vistas=() fichero accion
    while [ "${#pendientes[@]}" -gt 0 ]; do
        fichero="${pendientes[0]}"
        pendientes=("${pendientes[@]:1}")
        while IFS= read -r accion; do
            accion="$accion/action.yml"
            [ -f "$accion" ] || continue
            case " ${vistas[*]} " in
                *" $accion "*) continue ;;
            esac
            vistas+=("$accion")
            pendientes+=("$accion")
        done < <(grep -vE '^[[:space:]]*#' "$fichero" \
            | grep -oE 'uses:[[:space:]]*\./\.github/actions/[A-Za-z0-9._-]+' \
            | sed -E 's|uses:[[:space:]]*\./||' || true)
    done
    [ "${#vistas[@]}" -eq 0 ] || printf '%s\n' "${vistas[@]}"
}

BUILD=.github/workflows/build.yml
if [ -f "$BUILD" ]; then
    mapfile -t sin_secretos < <(printf '%s\n' "$BUILD"; acciones_locales "$BUILD")
    menciones="$(grep -nHE 'secrets[.[]|^[[:space:]]*secrets:' "${sin_secretos[@]}" \
        | sin_comentarios || true)"
    if [ -n "$menciones" ]; then
        printf '%s\n' "$menciones" >&2
        echo >&2
        echo "$BUILD y las acciones que usa no pueden mencionar ningun secreto (ADR-0015)." >&2
        echo "Para un token de la propia ejecucion, usa \${{ github.token }}." >&2
        exit 1
    fi
    if ! grep -q '^  workflow_call:' "$BUILD"; then
        echo "$BUILD tiene que seguir siendo invocable (workflow_call, ADR-0015)" >&2
        exit 1
    fi
    if ! awk '
        /^permissions:$/ { dentro = 1; next }
        dentro && /^  contents: read$/ { ok = 1 }
        dentro && /^[^ ]/ { dentro = 0 }
        END { exit ok ? 0 : 1 }
    ' "$BUILD"; then
        echo "$BUILD tiene que declarar 'permissions: contents: read' (ADR-0015)" >&2
        exit 1
    fi
    echo "OK  $BUILD es invocable, de solo lectura y ni el ni sus acciones mencionan un secreto"
fi

# ------------------------------------------------- tuberia de entrega --
herencias="$(grep -rn 'secrets:[[:space:]]*inherit' .github/workflows | sin_comentarios || true)"
if [ -n "$herencias" ]; then
    printf '%s\n' "$herencias" >&2
    echo >&2
    echo "ningun workflow hereda secretos en bloque (ADR-0015)." >&2
    echo "Heredarlos sobre build.yml le entrega el entorno de release entero." >&2
    echo "Pasa uno a uno los que el workflow llamado declare." >&2
    exit 1
fi
echo "OK  ningun workflow escribe 'secrets: inherit'"

sin_firmar="$(grep -rn 'SIN-FIRMA-SOLO-PRUEBAS' .github/workflows | sin_comentarios || true)"
if [ -n "$sin_firmar" ]; then
    printf '%s\n' "$sin_firmar" >&2
    echo >&2
    echo "ningun workflow puede construir el arbol sin firmar (ADR-0015)." >&2
    echo "Ese modo es de packaging/repo/build-tree.test.sh y de nadie mas." >&2
    exit 1
fi
echo "OK  ningun workflow construye el arbol sin firmar"

RELEASE=.github/workflows/release.yml
if [ -f "$RELEASE" ]; then
    if ! grep -q '^    environment: release$' "$RELEASE"; then
        echo "$RELEASE tiene que firmar dentro de 'environment: release' (ADR-0015)" >&2
        exit 1
    fi
    if ! grep -vE '^[[:space:]]*#' "$RELEASE" | grep -q -- '--draft'; then
        echo "$RELEASE tiene que crear la Release en borrador (ADR-0015)." >&2
        echo "Publicarla es el gesto humano; una etiqueta no publica nada." >&2
        exit 1
    fi
    echo "OK  $RELEASE firma en el entorno de release y deja el borrador"
fi

minisign_fuera="$(grep -rn 'TAURI_SIGNING_PRIVATE_KEY' .github/workflows .github/actions \
    | grep -v "^$RELEASE:" | sin_comentarios || true)"
if [ -n "$minisign_fuera" ]; then
    printf '%s\n' "$minisign_fuera" >&2
    echo >&2
    echo "la clave minisign de las actualizaciones solo aparece en $RELEASE (ADR-0015)." >&2
    exit 1
fi
echo "OK  la clave minisign de las actualizaciones solo aparece en $RELEASE"

PUBLISH=.github/workflows/publish.yml
if [ -f "$PUBLISH" ]; then
    if ! grep -q 'types: \[published\]' "$PUBLISH"; then
        echo "$PUBLISH cuelga de 'release: types: [published]', no de la etiqueta (ADR-0015)" >&2
        exit 1
    fi
    sin_guarda="$(awk '
        /^jobs:/ { en_jobs = 1; next }
        en_jobs && /^[^ #]/ { en_jobs = 0 }
        en_jobs && /^  [A-Za-z0-9_-]+:[[:space:]]*$/ {
            if (job != "" && !guardado) print job
            job = $1; sub(/:$/, "", job); guardado = 0; next
        }
        en_jobs && /^    if:.*!github\.event\.release\.prerelease/ { guardado = 1 }
        END { if (job != "" && !guardado) print job }
    ' "$PUBLISH")"
    if [ -n "$sin_guarda" ]; then
        echo "$PUBLISH tiene que descartar las prereleases en el if: de cada job (ADR-0015)." >&2
        echo "Jobs sin esa condicion: $(printf %s "$sin_guarda" | tr "\n" " ")" >&2
        echo "Una etiqueta -rc.N ensaya la tuberia y no llega a ningun repositorio." >&2
        exit 1
    fi
    echo "OK  $PUBLISH solo reacciona a una Release publicada que no es candidata"

    if ! grep -q 'packaging/repo/publish-tree.sh' "$PUBLISH"; then
        echo "$PUBLISH tiene que publicar con packaging/repo/publish-tree.sh (ADR-0015)" >&2
        exit 1
    fi
    sueltos="$(grep -nE '^[[:space:]]+(-[[:space:]]+)?(run:[[:space:]]*)?rsync ' "$PUBLISH" || true)"
    if [ -n "$sueltos" ]; then
        printf '%s\n' "$sueltos" >&2
        echo >&2
        echo "el rsync de la publicacion va en packaging/repo/publish-tree.sh, no aqui." >&2
        echo "Ahi esta probado (just check-repo); en el YAML no lo prueba nadie." >&2
        exit 1
    fi
    echo "OK  $PUBLISH publica con el guion probado y no lleva rsync suelto"

    if ! grep -q 'build-tree.sh serie arbol rfirma.asc "\$FINGERPRINT"' "$PUBLISH"; then
        echo "$PUBLISH tiene que pasarle la huella a build-tree.sh (ADR-0015)." >&2
        echo "Sin ella el ostree, el InRelease de apt y el repomd de dnf se" >&2
        echo "sirven sin firma: la firma es metadato desacoplado y hay que" >&2
        echo "reponerla en CADA reconstruccion." >&2
        exit 1
    fi
    echo "OK  $PUBLISH construye el arbol firmado con la huella del repositorio"
fi

# Firmar un .rpm lo modifica: firmar, resumir, atestar y adjuntar, en ese orden (ADR-0015).
if [ -f "$RELEASE" ]; then
    linea_de() { grep -n "$1" "$RELEASE" | grep -vE ':[0-9]+:[[:space:]]*#' | head -1 | cut -d: -f1; }
    firma_rpm="$(linea_de 'rpmsign --addsign')"
    resumenes="$(linea_de 'sha256sum -- \*')"
    atestacion="$(linea_de 'attest-build-provenance')"
    adjuntar="$(linea_de 'gh release create')"

    for paso in firma_rpm resumenes atestacion adjuntar; do
        if [ -z "${!paso}" ]; then
            echo "$RELEASE ya no tiene el paso '$paso' de la cadena de firma (ADR-0015)" >&2
            exit 1
        fi
    done
    if [ "$firma_rpm" -ge "$resumenes" ] || [ "$resumenes" -ge "$atestacion" ] \
        || [ "$atestacion" -ge "$adjuntar" ]; then
        echo "$RELEASE tiene los pasos en otro orden (ADR-0015)." >&2
        echo "Firmar un .rpm lo MODIFICA, asi que el orden es obligatorio:" >&2
        echo "  firmar cada .rpm -> SHA256SUMS -> atestar -> adjuntar" >&2
        echo "  y aqui estan en las lineas $firma_rpm, $resumenes, $atestacion, $adjuntar" >&2
        exit 1
    fi
    echo "OK  $RELEASE firma cada .rpm antes de resumir, atestar y adjuntar"
fi

docker="$(grep -rniE 'docker|ghcr\.io|container-registry' .github/workflows | sin_comentarios || true)"
if [ -n "$docker" ]; then
    printf '%s\n' "$docker" >&2
    echo >&2
    echo "ningun workflow toca Docker ni un registro de imagenes (ADR-0015)." >&2
    echo "Los tres repositorios llegan al anfitrion por rsync, fuera de la imagen." >&2
    exit 1
fi
echo "OK  ningun workflow toca Docker ni un registro de imagenes"

# ---------------------------------------------------------- caches de Rust --
# Una PR lee las caches de `main` y no guarda las suyas: desbordarian el cupo (ADR-0015).
sin_save_if="$(awk '
    function cierra() {
        if (abierto && !valido) print origen
        abierto = 0
    }
    FNR == 1 { cierra() }
    /uses:[[:space:]]*Swatinem\/rust-cache@/ {
        cierra()
        abierto = 1; valido = 0; origen = FILENAME ":" FNR
        accion = (FILENAME ~ /\/actions\//)
        match($0, /^[[:space:]]*/); sangria = RLENGTH
        next
    }
    abierto {
        match($0, /^[[:space:]]*/)
        if ($0 !~ /^[[:space:]]*$/ && RLENGTH <= sangria) cierra()
        else if ($0 ~ /^[[:space:]]*save-if:[[:space:]]*(false|\$\{\{[[:space:]]*github\.ref[[:space:]]*==[[:space:]]*'\''refs\/heads\/main'\''[[:space:]]*\}\})[[:space:]]*$/) valido = 1
        else if (accion && $0 ~ /^[[:space:]]*save-if:[[:space:]]*\$\{\{[[:space:]]*inputs\.save-cache[[:space:]]*==[[:space:]]*'\''true'\''[[:space:]]*\}\}[[:space:]]*$/) valido = 1
    }
    END { cierra() }
' .github/workflows/*.yml .github/actions/*/action.yml)"
if [ -n "$sin_save_if" ]; then
    printf '%s\n' "$sin_save_if" >&2
    echo >&2
    echo "cada Swatinem/rust-cache declara 'save-if: \${{ github.ref == 'refs/heads/main' }}'" >&2
    echo "o 'save-if: false', y dentro de una accion 'save-if: \${{ inputs.save-cache == 'true' }}':" >&2
    echo "una PR lee la cache de main, no guarda la suya." >&2
    exit 1
fi
echo "OK  solo main guarda caches de Rust"

# ------------------------------------------------ preparacion del runner --
SETUP_RUNNER=./.github/actions/setup-runner
sin_save_cache="$(awk -v accion="$SETUP_RUNNER" '
    function cierra() {
        if (abierto && !declarado) print origen
        abierto = 0
    }
    FNR == 1 { cierra() }
    index($0, "uses: " accion) && $0 ~ /^[[:space:]]*-?[[:space:]]*uses:/ {
        cierra()
        abierto = 1; declarado = 0; origen = FILENAME ":" FNR
        match($0, /^[[:space:]]*-?[[:space:]]*/); sangria = RLENGTH
        next
    }
    abierto {
        match($0, /^[[:space:]]*/)
        if ($0 !~ /^[[:space:]]*$/ && RLENGTH < sangria) cierra()
        else if ($0 ~ /^[[:space:]]*save-cache:[[:space:]]*[^[:space:]]/) declarado = 1
    }
    END { cierra() }
' .github/workflows/*.yml)"
if [ -n "$sin_save_cache" ]; then
    printf '%s\n' "$sin_save_cache" >&2
    echo >&2
    echo "cada uso de $SETUP_RUNNER declara 'save-cache'." >&2
    exit 1
fi
invocables="$(grep -lE '^  workflow_call:' .github/workflows/*.yml || true)"
por_ref="$(echo "$invocables" | xargs -r grep -nHE '^[[:space:]]*save-cache:.*github\.(ref|ref_name|head_ref|base_ref)' \
    | sin_comentarios || true)"
if [ -n "$por_ref" ]; then
    printf '%s\n' "$por_ref" >&2
    echo >&2
    echo "'save-cache' no se deriva de la ref: bajo workflow_call es la del llamador." >&2
    exit 1
fi
echo "OK  cada uso de $SETUP_RUNNER declara 'save-cache' y ninguno lo deriva de la ref"

# ------------------------------------------------------ espera de Dependabot --
if ! awk '
    /^  - package-ecosystem:/ { dentro = ($0 ~ /github-actions/) }
    dentro && /^    cooldown:/ { ok = 1 }
    END { exit ok ? 0 : 1 }
' .github/dependabot.yml; then
    echo ".github/dependabot.yml tiene que declarar 'cooldown' para github-actions (ADR-0015)." >&2
    echo "Sin la espera, Dependabot propone una version el mismo dia que se publica." >&2
    exit 1
fi
echo "OK  dependabot.yml espera antes de proponer una accion nueva"

# ------------------------------------------------------------------ Preview --
PREVIEW=.github/workflows/preview.yml
if [ ! -f "$PREVIEW" ]; then
    echo "falta $PREVIEW (ADR-0015)." >&2
    exit 1
fi
secretos="$(grep -nE 'secrets[.[]|^[[:space:]]*secrets:' "$PREVIEW" \
    | grep -vE '^[0-9]+:[[:space:]]*#' || true)"
if [ -n "$secretos" ]; then
    printf '%s\n' "$secretos" >&2
    echo "$PREVIEW no pasa ni menciona ningun secreto (ADR-0015)." >&2
    exit 1
fi
permisos_de_mas="$(awk '
    /^[[:space:]]*permissions:[[:space:]]*[^[:space:]#]/ && $0 !~ /permissions:[[:space:]]*\{\}/ { print FNR ": " $0; next }
    /^[[:space:]]*permissions:[[:space:]]*$/ {
        match($0, /^[[:space:]]*/); base = RLENGTH; dentro = 1; next
    }
    dentro {
        if ($0 ~ /^[[:space:]]*(#.*)?$/) next
        match($0, /^[[:space:]]*/)
        if (RLENGTH <= base) { dentro = 0; next }
        if ($0 !~ /^[[:space:]]*contents:[[:space:]]*read[[:space:]]*$/) print FNR ": " $0
    }
' "$PREVIEW")"
if [ -n "$permisos_de_mas" ]; then
    printf '%s\n' "$permisos_de_mas" >&2
    echo "$PREVIEW declara solo 'contents: read' (ADR-0015)." >&2
    exit 1
fi
echo "OK  $PREVIEW no pasa secretos y declara solo contents: read"

# ----------------------------------------------------------- Preview Comment --
COMMENT=.github/workflows/preview-comment.yml
if [ ! -f "$COMMENT" ]; then
    echo "falta $COMMENT (ADR-0015)." >&2
    exit 1
fi
disparadores="$(awk '
    /^on:[[:space:]]*$/ { dentro = 1; next }
    dentro && /^[^[:space:]#]/ { dentro = 0 }
    dentro && /^[[:space:]]*#/ { next }
    dentro && /^  [^[:space:]]/ { sub(/:.*/, ""); gsub(/[[:space:]]/, ""); print; next }
    dentro && /^    (workflows|types):/ { gsub(/[[:space:]]/, ""); print }
' "$COMMENT" | tr '\n' ' ')"
if [ "$disparadores" != "workflow_run workflows:[Preview] types:[completed] " ]; then
    echo "$COMMENT se dispara solo por workflow_run de Preview al completarse (ADR-0015)." >&2
    echo "encontrado: $disparadores" >&2
    exit 1
fi
permisos="$(awk '
    /^[[:space:]]*permissions:/ && !/^permissions:[[:space:]]*$/ { print "fuera-de-lugar:" FNR; next }
    /^permissions:[[:space:]]*$/ { dentro = 1; next }
    dentro && /^[^[:space:]#]/ { dentro = 0 }
    dentro && /^[[:space:]]*(#.*)?$/ { next }
    dentro { gsub(/[[:space:]]/, ""); print }
' "$COMMENT" | sort | tr '\n' ' ')"
if [ "$permisos" != "actions:read pull-requests:write " ]; then
    echo "$COMMENT declara exactamente 'actions: read' y 'pull-requests: write', a nivel de workflow (ADR-0015)." >&2
    echo "encontrado: $permisos" >&2
    exit 1
fi
codigo_de_la_ejecucion="$(awk '
    /uses:[[:space:]]*actions\/checkout@/ { dentro = 1; next }
    dentro && /^      - / { dentro = 0 }
    dentro && /^[[:space:]]*(ref|repository):/ && $0 !~ /^[[:space:]]*ref:[[:space:]]*\$\{\{[[:space:]]*github\.event\.repository\.default_branch[[:space:]]*\}\}[[:space:]]*$/ { print FNR ": " $0 }
' "$COMMENT")"
if [ -n "$codigo_de_la_ejecucion" ]; then
    printf '%s\n' "$codigo_de_la_ejecucion" >&2
    echo "$COMMENT solo hace checkout de la rama por defecto, nunca del código de la ejecución (ADR-0015)." >&2
    exit 1
fi
echo "OK  $COMMENT: workflow_run, actions: read + pull-requests: write y sin código de la ejecución"

# ---------------------------------------------------------------- GraalVM --
GRAALVM_ACTION=.github/actions/setup-runner/action.yml
instalaciones="$(grep -rn 'graalvm/setup-graalvm@' .github/workflows .github/actions justfile \
    | grep -v "^$GRAALVM_ACTION:" || true)"
if [ -n "$instalaciones" ]; then
    printf '%s\n' "$instalaciones" >&2
    echo "GraalVM se instala solo con $GRAALVM_ACTION (ADR-0035)." >&2
    exit 1
fi
echo "OK  GraalVM: una sola instalacion, la de $GRAALVM_ACTION"
