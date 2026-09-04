#!/usr/bin/env bash
# El banco de pruebas de `publish-tree.sh`, que es la unica parte de la tuberia
# de entrega que NO puede ensayarse con una etiqueta `-rc.N`: el ensayo de la
# tuberia se detiene justo antes de tocar el anfitrion (ADR-0015), asi que si
# esto no se prueba aqui no se prueba en ningun sitio.
#
# LO QUE HACE ESPECIAL A ESTE BANCO es que el destino remoto NO se simula: se
# levanta el mismo `rrsync` que vive en el `authorized_keys` del VPS, detras de
# un `ssh` de mentira que solo le pasa la orden. Todo lo que aqui pasa por la
# orden forzada pasa tambien alli, incluidas las opciones de `rsync` que rrsync
# NO admite —`--filter`, por ejemplo— y que en el VPS se manifestarian como un
# despliegue que muere a mitad.
#
# Uso: packaging/repo/publish-tree.test.sh
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
publica="$raiz/packaging/repo/publish-tree.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fallos=0

fail() {
    echo "FALLO  $*" >&2
    fallos=$((fallos + 1))
}

ok() {
    echo "OK  $*"
}

# Un arbol de mentira con la forma que fija el ADR-0015: la clave, los tres
# repositorios y un fichero dentro de cada uno para poder compararlos.
crea_arbol() {
    local destino="$1" version="$2"
    mkdir -p "$destino/flatpak" "$destino/apt/dists/stable" "$destino/rpm/repodata"
    echo "clave publica" > "$destino/rfirma.asc"
    echo "ostree $version" > "$destino/flatpak/summary"
    echo "apt $version" > "$destino/apt/dists/stable/Release"
    echo "dnf $version" > "$destino/rpm/repodata/repomd.xml"
}

# El enlace vigente, leido como lo lee quien sirve: siguiendo `actual`.
servido() {
    readlink "$1/actual" || echo "(sin enlace)"
}

# ---------------------------------------------------------------------------
# La bateria entera contra un destino: o un directorio local, o `anfitrion:`
# con la orden forzada de rrsync detras.
bateria() {
    local etiqueta_destino="$1" destino="$2" raiz_remota="$3"

    echo
    echo "== $etiqueta_destino =="

    # a_fresh_tree_enters_service_through_the_symlink
    crea_arbol "$tmp/v1" "0.4.0"
    "$publica" "$tmp/v1" "v0.4.0" "$destino" > "$tmp/salida" 2>&1 \
        || { cat "$tmp/salida" >&2; fail "$etiqueta_destino: la primera publicacion falla"; return; }
    if [ "$(servido "$raiz_remota")" != "arboles/v0.4.0" ]; then
        fail "$etiqueta_destino: 'actual' no apunta al arbol nuevo"
    elif ! diff -r "$tmp/v1" "$raiz_remota/actual/" > /dev/null; then
        fail "$etiqueta_destino: lo servido no es el arbol que se publico"
    else
        ok "$etiqueta_destino: el arbol entra en servicio por el enlace"
    fi

    # a_half_written_tree_is_never_served
    # Los bytes del arbol siguiente llegan a su propio directorio: mientras el
    # enlace no se intercambie, nadie los ve. Se imita la primera pata del
    # despliegue —solo la subida— y se mira el enlace.
    mkdir -p "$raiz_remota/arboles/v0.4.1"
    echo "a medias" > "$raiz_remota/arboles/v0.4.1/rfirma.asc"
    if [ "$(servido "$raiz_remota")" != "arboles/v0.4.0" ]; then
        fail "$etiqueta_destino: un arbol a medias ha cambiado lo servido"
    else
        ok "$etiqueta_destino: un despliegue a medias no llega a verse"
    fi
    rm -rf "$raiz_remota/arboles/v0.4.1"

    # republishing_after_wiping_the_volume_restores_the_same_service
    local antes
    antes="$(cd "$raiz_remota/actual" && find . -type f | LC_ALL=C sort | xargs sha256sum)"
    rm -rf "${raiz_remota:?}/"*
    "$publica" "$tmp/v1" "v0.4.0" "$destino" > "$tmp/salida" 2>&1 \
        || { cat "$tmp/salida" >&2; fail "$etiqueta_destino: republicar tras tirar el volumen falla"; return; }
    if [ "$(cd "$raiz_remota/actual" && find . -type f | LC_ALL=C sort | xargs sha256sum)" != "$antes" ]; then
        fail "$etiqueta_destino: tirar el volumen y republicar no deja el servicio identico"
    else
        ok "$etiqueta_destino: tirar el volumen y republicar deja el servicio identico"
    fi

    # the_previous_tree_survives_and_older_ones_are_pruned
    crea_arbol "$tmp/v2" "0.4.1"
    crea_arbol "$tmp/v3" "0.4.2"
    "$publica" "$tmp/v2" "v0.4.1" "$destino" > "$tmp/salida" 2>&1 \
        || { cat "$tmp/salida" >&2; fail "$etiqueta_destino: la segunda publicacion falla"; return; }
    "$publica" "$tmp/v3" "v0.4.2" "$destino" > "$tmp/salida" 2>&1 \
        || { cat "$tmp/salida" >&2; fail "$etiqueta_destino: la tercera publicacion falla"; return; }
    local arboles
    arboles="$(cd "$raiz_remota/arboles" && ls | LC_ALL=C sort | tr '\n' ' ')"
    if [ "$arboles" != "v0.4.1 v0.4.2 " ]; then
        fail "$etiqueta_destino: la retencion deja '$arboles' en vez del vigente y el anterior"
    else
        ok "$etiqueta_destino: en el anfitrion quedan el arbol vigente y el anterior"
    fi

    # the_previous_tree_stays_whole_so_the_rollback_is_a_symlink
    if ! diff -r "$tmp/v2" "$raiz_remota/arboles/v0.4.1" > /dev/null; then
        fail "$etiqueta_destino: el arbol anterior no esta entero, la vuelta atras no seria un enlace"
    else
        ok "$etiqueta_destino: el arbol anterior queda entero para la vuelta atras"
    fi

    # a_stale_file_of_the_previous_tree_never_survives_into_the_new_one
    # El mismo nombre de arbol dos veces (una republicacion) no puede dejar
    # dentro un fichero que ya no esta en el origen: el arbol es derivado.
    echo "basura" > "$raiz_remota/arboles/v0.4.2/sobra.txt"
    "$publica" "$tmp/v3" "v0.4.2" "$destino" > "$tmp/salida" 2>&1 \
        || { cat "$tmp/salida" >&2; fail "$etiqueta_destino: la republicacion falla"; return; }
    if [ -e "$raiz_remota/arboles/v0.4.2/sobra.txt" ]; then
        fail "$etiqueta_destino: un fichero que ya no esta en el origen sobrevive en el anfitrion"
    else
        ok "$etiqueta_destino: el arbol servido es exactamente el que se construyo"
    fi

    rm -rf "$tmp/v1" "$tmp/v2" "$tmp/v3"
}

# ---------------------------------------------------------------------------
# 1) Destino local: el mismo camino, sin ssh por en medio.
local_remoto="$tmp/local"
mkdir -p "$local_remoto"
bateria "destino local" "$local_remoto" "$local_remoto"

# 2) Destino remoto con la ORDEN FORZADA de verdad.
rrsync="$(command -v rrsync || true)"
[ -n "$rrsync" ] || [ ! -x /usr/bin/rrsync ] || rrsync=/usr/bin/rrsync
if [ -n "$rrsync" ]; then
    forzado_remoto="$tmp/forzado"
    mkdir -p "$forzado_remoto"
    cat > "$tmp/ssh-de-mentira" <<EOF
#!/usr/bin/env bash
# Lo que el VPS hace con la clave del CI: ignorar el nombre del anfitrion y
# entregarle la orden a rrsync, confinado en el directorio que ya sirve.
shift
SSH_ORIGINAL_COMMAND="\$*" exec "$rrsync" "$forzado_remoto"
EOF
    chmod +x "$tmp/ssh-de-mentira"
    RSYNC_RSH="$tmp/ssh-de-mentira" bateria "orden forzada (rrsync)" "anfitrion:" "$forzado_remoto"
else
    echo
    echo "AVISO  rrsync no esta instalado: la pata de la orden forzada no se ha probado." >&2
    echo "       Instala el paquete rsync completo para probarla (packaging/repo/README.md)." >&2
fi

echo
if [ "$fallos" -ne 0 ]; then
    echo "$fallos comprobacion(es) de la publicacion han fallado" >&2
    exit 1
fi
echo "OK  la publicacion sube el arbol, intercambia el enlace y retiene dos"
