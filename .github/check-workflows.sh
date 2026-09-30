#!/usr/bin/env bash
# Las invariantes de los workflows que ninguna ejecucion detecta: que ninguna
# accion entra por etiqueta (ID-170), que el workflow de construccion no ve un
# secreto jamas (ID-167), y las tres que sostienen la tuberia de entrega
# —nadie hereda secretos en bloque, la Release nace en borrador (ID-168) y una
# candidata no llega a ningun repositorio—, y que la versión de GraalVM se
# escribe en un solo sitio.
#
# ------------------------------------------------------------------ ID-170 --
# NINGUNA accion de terceros entra por etiqueta.
#
# Una etiqueta (`@v4`, `@main`) es un puntero que su propietario puede mover
# cuando quiera, y moverlo es ejecutar codigo nuevo dentro de un runner que ya
# tiene el token del repositorio. Un SHA no se mueve. Lo que se compra con esto
# no es inmunidad —el SHA fijado puede ser malo desde el primer dia— sino que
# la actualizacion pase por una PR de Dependabot que alguien mira, en vez de
# ocurrir en silencio (ADR-0015).
#
# Y ES UNA PUERTA Y NO UNA CONVENCION EN UN COMENTARIO porque la convencion se
# rompe sola: quien anada un paso manana copiara el `uses: foo/bar@v1` del
# README de esa accion, que es como estan escritos todos los README del mundo.
#
# Se exige ademas el comentario con la etiqueta al lado, `# v4.4.0`: sin el,
# nadie sabe que version es ese SHA sin abrir un navegador, y Dependabot lo
# necesita para saber desde donde actualiza.
#
# Uso: .github/check-workflows.sh
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$raiz"

fallos=0

while IFS= read -r linea; do
    fichero="${linea%%:*}"
    resto="${linea#*:}"
    numero="${resto%%:*}"
    texto="${resto#*:}"

    # Una accion local (`uses: ./.github/actions/...`) no viene de nadie de
    # fuera: es este mismo arbol, ya fijado por el commit que se comprueba.
    case "$texto" in
        *uses:*./*) continue ;;
    esac

    # `owner/repo@<40 hex>` u `owner/repo/ruta@<40 hex>`, con el comentario de
    # la etiqueta detras. La comilla opcional cubre el estilo `uses: "..."`.
    if [[ "$texto" =~ uses:[[:space:]]*\"?[A-Za-z0-9._-]+/[A-Za-z0-9._/-]+@[0-9a-f]{40}\"?[[:space:]]*\#[[:space:]]*[^[:space:]]+ ]]; then
        continue
    fi

    echo "$fichero:$numero: accion sin fijar por SHA (ID-170)" >&2
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

# ------------------------------------------------------------------ ID-167 --
# El workflow de construccion NO VE UN SECRETO JAMAS.
#
# Es la primera invariante del ADR-0015 y la que sostiene todo lo demas:
# `build.yml` es invocable, asi que si viese un secreto, reutilizarlo desde
# cualquier sitio seria un camino hacia la subclave de firma que vive en el
# entorno de release. Lo que se prohibe es NOMBRAR el contexto `secrets` —donde
# una accion pide un token se le da `github.token`, que es el mismo valor sin
# obligar al llamador a pasar nada— y declarar `secrets:` en el `workflow_call`.
BUILD=.github/workflows/build.yml
if [ -f "$BUILD" ]; then
    # Las lineas de comentario se descartan: la cabecera del propio fichero
    # explica la invariante, y para explicarla tiene que nombrarla.
    menciones="$(grep -nE 'secrets[.[]|^[[:space:]]*secrets:' "$BUILD" \
        | grep -vE '^[0-9]+:[[:space:]]*#' || true)"
    if [ -n "$menciones" ]; then
        printf '%s\n' "$menciones" >&2
        echo >&2
        echo "$BUILD no puede mencionar ningun secreto (ID-167, ADR-0015)." >&2
        echo "Para un token de la propia ejecucion, usa \${{ github.token }}." >&2
        exit 1
    fi
    if ! grep -q '^  workflow_call:' "$BUILD"; then
        echo "$BUILD tiene que seguir siendo invocable (workflow_call, ID-167)" >&2
        exit 1
    fi
    # Solo lectura, y en el nivel del workflow: declararlo por job dejaria al
    # siguiente job que alguien anada con los permisos por omision.
    if ! awk '
        /^permissions:$/ { dentro = 1; next }
        dentro && /^  contents: read$/ { ok = 1 }
        dentro && /^[^ ]/ { dentro = 0 }
        END { exit ok ? 0 : 1 }
    ' "$BUILD"; then
        echo "$BUILD tiene que declarar 'permissions: contents: read' (ID-167)" >&2
        exit 1
    fi
    echo "OK  $BUILD es invocable, de solo lectura y no menciona ningun secreto"
fi

# ------------------------------------------------- ID-167, ID-168, ID-169 --
# LAS INVARIANTES DE LOS OTROS DOS WORKFLOWS.
#
# La cabecera de build.yml deja dicho lo que ese fichero NO puede impedir por
# si mismo: que un llamador escriba `secrets: inherit` y le entregue de golpe
# todo el entorno de release, subclave GPG incluida. Eso se «vigila al revisar
# release.yml», y una vigilancia que depende de que alguien se acuerde no es
# una vigilancia. Aqui esta escrita.
#
# Las otras dos son del mismo tamano: la Release NACE EN BORRADOR (ID-168) y el
# despliegue NO OCURRE PARA UNA PRERELEASE (ADR-0015). Las dos se pierden
# borrando una sola linea, y ninguna ejecucion del CI las echaria de menos:
# solo se notarian el dia de la entrega, publicando algo que nadie ha mirado.
# Las lineas de comentario quedan fuera, igual que arriba: las cabeceras de
# build.yml y de release.yml explican la prohibicion, y para explicarla tienen
# que escribirla.
herencias="$(grep -rn 'secrets:[[:space:]]*inherit' .github/workflows \
    | grep -vE ':[0-9]+:[[:space:]]*#' || true)"
if [ -n "$herencias" ]; then
    printf '%s\n' "$herencias" >&2
    echo >&2
    echo "ningun workflow hereda secretos en bloque (ID-167, ADR-0015)." >&2
    echo "Heredarlos sobre build.yml le entrega el entorno de release entero." >&2
    echo "Pasa uno a uno los que el workflow llamado declare." >&2
    exit 1
fi
echo "OK  ningun workflow escribe 'secrets: inherit'"

# LO QUE SE PUBLICA VA FIRMADO, SIEMPRE (ID-173, ID-175). `build-tree.sh`
# tiene un modo sin firma —se llama `SIN-FIRMA-SOLO-PRUEBAS`— porque las
# claves de rFirma las crea una persona y ninguna prueba puede fabricarse
# una que valga. Ese modo construye un arbol que apt y dnf rechazan en
# cuanto alguien lo anade, y el ostree recien reimportado no trae ninguna
# firma porque la firma no viaja dentro del bundle: si llegara a CUALQUIER
# workflow, la publicacion saldria muda y el fallo lo descubriria quien
# instala. De ahi que la salida de emergencia tenga candado sobre todos los
# workflows, no solo sobre publish.yml.
sin_firmar="$(grep -rn 'SIN-FIRMA-SOLO-PRUEBAS' .github/workflows \
    | grep -vE ':[0-9]+:[[:space:]]*#' || true)"
if [ -n "$sin_firmar" ]; then
    printf '%s\n' "$sin_firmar" >&2
    echo >&2
    echo "ningun workflow puede construir el arbol sin firmar (ID-173, ID-175)." >&2
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
        echo "$RELEASE tiene que crear la Release en borrador (ID-168)." >&2
        echo "Publicarla es el gesto humano; una etiqueta no publica nada." >&2
        exit 1
    fi
    echo "OK  $RELEASE firma en el entorno de release y deja el borrador"
fi

PUBLISH=.github/workflows/publish.yml
if [ -f "$PUBLISH" ]; then
    if ! grep -q 'types: \[published\]' "$PUBLISH"; then
        echo "$PUBLISH cuelga de 'release: types: [published]', no de la etiqueta (ID-167)" >&2
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

    # EL MECANISMO DE LA PUBLICACION VIVE EN UN GUION QUE SE PRUEBA (ID-174).
    # Es la unica parte de la tuberia que no se puede ensayar con una etiqueta
    # `-rc.N` —el ensayo se detiene antes de tocar el anfitrion—, asi que un
    # `rsync` escrito a mano dentro del workflow seria una orden que nadie ha
    # ejecutado nunca hasta el dia de la entrega. Y el orden de las tres
    # ordenes (arbol, enlace, poda) es lo que hace que un despliegue a medias
    # no se vea: si se reparte entre pasos del YAML, deja de estar probado.
    if ! grep -q 'packaging/repo/publish-tree.sh' "$PUBLISH"; then
        echo "$PUBLISH tiene que publicar con packaging/repo/publish-tree.sh (ID-174)" >&2
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
        echo "$PUBLISH tiene que pasarle la huella a build-tree.sh (ID-173)." >&2
        echo "Sin ella el ostree, el InRelease de apt y el repomd de dnf se" >&2
        echo "sirven sin firma: la firma es metadato desacoplado y hay que" >&2
        echo "reponerla en CADA reconstruccion." >&2
        exit 1
    fi
    echo "OK  $PUBLISH construye el arbol firmado con la huella del repositorio"
fi

# ------------------------------------------------------------------ ID-175 --
# EL ORDEN DE `release.yml`: FIRMAR EL `.rpm` -> `SHA256SUMS` -> ATESTAR ->
# ADJUNTAR.
#
# No es de estilo: firmar un `.rpm` LO MODIFICA. Si se firmara despues de
# calcular los resumenes, el `.rpm` de la Release y el que sirve el repositorio
# dnf dejarian de ser los mismos bytes y se rompe la invariante de que los tres
# canales llevan lo mismo (ID-144, ADR-0004); si se atestara antes de firmar,
# la atestacion cubriria un fichero que ya no existe.
#
# Es un orden de pasos dentro de un fichero, o sea la clase de cosa que una
# reordenacion bienintencionada rompe sin que falle una sola ejecucion: el
# workflow sale verde igual y el estropicio se ve el dia que alguien verifica
# la atestacion de un paquete descargado.
if [ -f "$RELEASE" ]; then
    linea_de() { grep -n "$1" "$RELEASE" | grep -vE ':[0-9]+:[[:space:]]*#' | head -1 | cut -d: -f1; }
    firma_rpm="$(linea_de 'rpmsign --addsign')"
    resumenes="$(linea_de 'sha256sum -- \*')"
    atestacion="$(linea_de 'attest-build-provenance')"
    adjuntar="$(linea_de 'gh release create')"

    for paso in firma_rpm resumenes atestacion adjuntar; do
        if [ -z "${!paso}" ]; then
            echo "$RELEASE ya no tiene el paso '$paso' de la cadena de firma (ID-175)" >&2
            exit 1
        fi
    done
    if [ "$firma_rpm" -ge "$resumenes" ] || [ "$resumenes" -ge "$atestacion" ] \
        || [ "$atestacion" -ge "$adjuntar" ]; then
        echo "$RELEASE tiene los pasos en otro orden (ID-175, ADR-0015)." >&2
        echo "Firmar un .rpm lo MODIFICA, asi que el orden es obligatorio:" >&2
        echo "  firmar cada .rpm -> SHA256SUMS -> atestar -> adjuntar" >&2
        echo "  y aqui estan en las lineas $firma_rpm, $resumenes, $atestacion, $adjuntar" >&2
        exit 1
    fi
    echo "OK  $RELEASE firma cada .rpm antes de resumir, atestar y adjuntar"
fi

# ------------------------------------------------------------------ ID-174 --
# NI REGISTRO DE IMAGENES NI UN SOLO PASO DE DOCKER EN EL CI.
#
# Los repositorios no van dentro de la imagen —una imagen no es sitio para
# datos que crecen: cada publicacion produciria una capa nueva con la historia
# entera repetida—, y con los datos fuera la tuberia no toca Docker en ningun
# momento. La imagen es solo el servidor web con la landing y la construye
# Coolify desde `main`.
#
# Se vigila aqui porque la tentacion tiene nombre y viene sola: el dia que
# alguien quiera «probar la imagen en el CI» anadira un `docker build`, y
# detras de un `docker build` viene un registro, y detras de un registro
# vienen los datos dentro de la imagen. Los comentarios quedan fuera: esta
# prohibicion hay que poder explicarla nombrandola.
docker="$(grep -rniE 'docker|ghcr\.io|container-registry' .github/workflows \
    | grep -vE ':[0-9]+:[[:space:]]*#' || true)"
if [ -n "$docker" ]; then
    printf '%s\n' "$docker" >&2
    echo >&2
    echo "ningun workflow toca Docker ni un registro de imagenes (ID-174, ADR-0015)." >&2
    echo "Los tres repositorios llegan al anfitrion por rsync, fuera de la imagen." >&2
    exit 1
fi
echo "OK  ningun workflow toca Docker ni un registro de imagenes"


# ---------------------------------------------------------- caches de Rust --
# Solo `main` guarda caches de Rust: las de cada PR desbordan el cupo de 10 GB
# y GitHub desaloja primero las de `main` que menos se leen, las de Windows.
sin_save_if="$(awk '
    function cierra() {
        if (abierto && !valido) print origen
        abierto = 0
    }
    FNR == 1 { cierra() }
    /uses:[[:space:]]*Swatinem\/rust-cache@/ {
        cierra()
        abierto = 1; valido = 0; origen = FILENAME ":" FNR
        match($0, /^[[:space:]]*/); sangria = RLENGTH
        next
    }
    abierto {
        match($0, /^[[:space:]]*/)
        if ($0 !~ /^[[:space:]]*$/ && RLENGTH <= sangria) cierra()
        else if ($0 ~ /^[[:space:]]*save-if:[[:space:]]*(false|\$\{\{[[:space:]]*github\.ref[[:space:]]*==[[:space:]]*'\''refs\/heads\/main'\''[[:space:]]*\}\})[[:space:]]*$/) valido = 1
    }
    END { cierra() }
' .github/workflows/*.yml)"
if [ -n "$sin_save_if" ]; then
    printf '%s\n' "$sin_save_if" >&2
    echo >&2
    echo "cada Swatinem/rust-cache declara 'save-if: \${{ github.ref == 'refs/heads/main' }}'" >&2
    echo "o 'save-if: false': una PR lee la cache de main, no guarda la suya." >&2
    exit 1
fi
echo "OK  solo main guarda caches de Rust"

# ------------------------------------------------------ espera de Dependabot --
# `github-actions` conserva su `cooldown` de siete días (ADR-0015).
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
# El workflow Preview construye con `build.yml` sin secretos y con solo lectura (ADR-0015).
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
# El workflow que comenta solo lee: `workflow_run`, dos permisos y nada del código de la ejecución.
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
# La versión de GraalVM se escribe solo en `.graalvm-version` (ADR-0035): el resto la lee.
GRAALVM_FILE=.graalvm-version
GRAALVM_ACTION=.github/actions/setup-graalvm/action.yml
if ! grep -qE '^[0-9]+(\.[0-9]+){2,4}$' "$GRAALVM_FILE"; then
    echo "$GRAALVM_FILE tiene que contener solo la versión exacta de GraalVM CE (ADR-0035)." >&2
    exit 1
fi
instalaciones="$(grep -rn 'graalvm/setup-graalvm@' .github/workflows .github/actions justfile \
    | grep -v "^$GRAALVM_ACTION:" || true)"
fijadas="$(grep -rnEi 'graal|java-version' .github/workflows .github/actions justfile \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' | sed 's/[[:space:]]#.*$//' \
    | grep -E '[0-9]+\.[0-9]+(\.[0-9]+)+|[0-9][^[:space:]"'"'"']*-graalce' || true)"
if [ -n "$instalaciones$fijadas" ]; then
    printf '%s\n' "$instalaciones" "$fijadas" | sed '/^$/d' >&2
    echo >&2
    echo "GraalVM se instala con $GRAALVM_ACTION y su versión solo se escribe en $GRAALVM_FILE (ADR-0035)." >&2
    exit 1
fi
echo "OK  GraalVM CE $(cat "$GRAALVM_FILE"): una sola versión, en $GRAALVM_FILE"
