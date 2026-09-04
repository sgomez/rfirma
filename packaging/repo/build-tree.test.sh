#!/usr/bin/env bash
# Las comprobaciones de `build-tree.sh`. La que de verdad importa es la del
# ID-173: RECONSTRUIR NO OBLIGA A NADIE A REDESCARGAR, y eso solo es cierto
# mientras el commit de ostree salga identico al reimportar los bundles en un
# repositorio vacio. Aqui se construye el arbol DOS VECES, en dos directorios
# distintos, y se comparan los commits.
#
# La otra mitad es que las ordenes de alta que enseña la landing sigan
# describiendo lo que hay servido: `Suites: stable` no funciona sobre un
# repositorio plano y la URL literal de dnf no funciona si el arbol la escribe
# con `$basearch`. Las dos cosas se comprueban contra `index.html`, que es
# donde la gente las copia.
#
# LAS FIRMAS NO SE PRUEBAN AQUI, y no es un olvido: firmar necesita una clave
# privada, las claves de rFirma las crea una persona con
# `packaging/setup-signing-key.sh` y ninguna prueba puede fabricarse una que
# valga. Por eso el arbol se construye en el modo `SIN-FIRMA-SOLO-PRUEBAS`, que
# `.github/check-workflows.sh` prohibe que aparezca en un workflow: lo que se
# publica va firmado siempre, y esa parte se ensaya con una etiqueta `-rc.N`.
#
# HERRAMIENTAS: el arbol necesita ostree, flatpak, dpkg-dev, apt-utils,
# createrepo-c y rpm. Si falta alguna, la pata del arbol avisa y se salta —como
# la de `rrsync` en `publish-tree.test.sh`— y las comprobaciones que no
# necesitan herramientas siguen corriendo.
#
# Uso: packaging/repo/build-tree.test.sh
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
construye="$raiz/packaging/repo/build-tree.sh"
landing="$raiz/packaging/repo/index.html"
SIN_FIRMA="SIN-FIRMA-SOLO-PRUEBAS"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fallos=0
fail() { echo "FALLO  $*" >&2; fallos=$((fallos + 1)); }
ok() { echo "OK  $*"; }

# UNA CLAVE DE MENTIRA Y NINGUNA DE VERDAD: las claves de rFirma las crea una
# persona con `packaging/setup-signing-key.sh`, y aqui no hace falta ninguna
# porque el arbol se construye sin firmar. Lo que si tiene que ser cierto es la
# ARMADURA —el `.flatpakref` lleva la clave desempaquetada dentro—, y eso lo da
# `gpg --enarmor`, que empaqueta unos bytes cualesquiera sin tocar ningun
# llavero.
printf 'clave publica de mentira' | gpg --enarmor > "$tmp/rfirma.asc"

versiones=(0.4.0 0.4.2 0.4.10)

# ---------------------------------------------------------------------------
# Las herramientas
# ---------------------------------------------------------------------------
faltan=()
for herramienta in ostree flatpak dpkg-deb dpkg-scanpackages apt-ftparchive createrepo_c rpmbuild rpm; do
    command -v "$herramienta" > /dev/null 2>&1 || faltan+=("$herramienta")
done

# ---------------------------------------------------------------------------
# Las comprobaciones que no necesitan ninguna herramienta: son las que paran la
# publicacion ANTES de tocar nada, asi que corren siempre.
# ---------------------------------------------------------------------------
paquetes_de_pega() {
    local dir="$1" version="$2"
    mkdir -p "$dir"
    echo "flatpak $version" > "$dir/rfirma-$version.flatpak"
    echo "deb $version" > "$dir/rfirma_${version}_amd64.deb"
    echo "rpm $version" > "$dir/rfirma-$version.x86_64.rpm"
}

# an_incomplete_series_stops_the_publication
incompleta="$tmp/incompleta"
paquetes_de_pega "$incompleta/v0.4.0" "0.4.0"
rm "$incompleta/v0.4.0"/*.rpm
if "$construye" "$incompleta" "$tmp/arbol-malo" "$tmp/rfirma.asc" "$SIN_FIRMA" > /dev/null 2>&1; then
    fail "una version sin los tres paquetes no detiene la publicacion"
else
    ok "una version sin los tres paquetes detiene la publicacion"
fi

# an_empty_series_stops_the_publication
mkdir -p "$tmp/vacia"
if "$construye" "$tmp/vacia" "$tmp/arbol-vacio" "$tmp/rfirma.asc" "$SIN_FIRMA" > /dev/null 2>&1; then
    fail "una serie vacia no detiene la publicacion"
else
    ok "una serie vacia detiene la publicacion"
fi

# the_public_key_has_to_exist
if "$construye" "$incompleta" "$tmp/arbol-malo" "$tmp/no-esta.asc" "$SIN_FIRMA" > /dev/null 2>&1; then
    fail "una clave publica que no existe no detiene la publicacion"
else
    ok "una clave publica que no existe detiene la publicacion"
fi

# ---------------------------------------------------------------------------
# Las ordenes de alta de la landing: no necesitan construir nada y son la
# unica atadura entre lo que se sirve y lo que la gente copia.
# ---------------------------------------------------------------------------
# the_landing_promises_the_suite_and_the_literal_url
if grep -q '^Suites: stable' "$landing" && grep -q 'baseurl=https://rfirma.sgomez.me/rpm/' "$landing"; then
    ok "la landing sigue prometiendo la suite stable y la URL literal de dnf"
else
    fail "la landing ya no promete lo que construye build-tree.sh"
fi

if [ "${#faltan[@]}" -ne 0 ]; then
    echo
    echo "AVISO: faltan ${faltan[*]}: la pata del arbol se salta." >&2
    echo "       sudo apt-get install -y ostree flatpak dpkg-dev apt-utils createrepo-c rpm" >&2
    echo
    if [ "$fallos" -ne 0 ]; then
        echo "$fallos comprobacion(es) del arbol han fallado" >&2
        exit 1
    fi
    echo "OK  la publicacion se detiene ante una serie que no vale (arbol sin comprobar)"
    exit 0
fi

# ---------------------------------------------------------------------------
# La serie de mentira, con paquetes de verdad: un bundle de flatpak que no sea
# un bundle no se importa, y un `.rpm` que no sea un `.rpm` no se indexa.
# ---------------------------------------------------------------------------
serie="$tmp/serie"
export FLATPAK_USER_DIR="$tmp/flatpak-user"

bundle_de_prueba() {
    local version="$1" destino="$2" obra="$tmp/obra/$version"
    rm -rf "$obra"
    mkdir -p "$obra/app/files/bin" "$obra/app/export/share/applications"
    cat > "$obra/app/metadata" <<EOF
[Application]
name=me.sgomez.rfirma
runtime=org.gnome.Platform/x86_64/48
sdk=org.gnome.Sdk/x86_64/48
command=rfirma
EOF
    printf '#!/bin/sh\necho %s\n' "$version" > "$obra/app/files/bin/rfirma"
    chmod +x "$obra/app/files/bin/rfirma"
    printf '[Desktop Entry]\nName=rFirma\nExec=rfirma\nType=Application\n' \
        > "$obra/app/export/share/applications/me.sgomez.rfirma.desktop"
    ostree init --mode=archive --repo="$obra/repo" > /dev/null
    flatpak build-export "$obra/repo" "$obra/app" stable > /dev/null 2>&1
    flatpak build-bundle "$obra/repo" "$destino" me.sgomez.rfirma stable > /dev/null 2>&1
    ostree --repo="$obra/repo" rev-parse app/me.sgomez.rfirma/x86_64/stable
}

deb_de_prueba() {
    local version="$1" destino="$2" obra="$tmp/obra/deb-$version"
    rm -rf "$obra"
    mkdir -p "$obra/DEBIAN" "$obra/usr/bin"
    printf '#!/bin/sh\necho %s\n' "$version" > "$obra/usr/bin/rfirma"
    chmod +x "$obra/usr/bin/rfirma"
    cat > "$obra/DEBIAN/control" <<EOF
Package: rfirma
Version: $version
Architecture: amd64
Maintainer: rFirma <no@example.invalid>
Description: paquete de prueba
EOF
    dpkg-deb --build --root-owner-group "$obra" "$destino" > /dev/null
}

rpm_de_prueba() {
    local version="$1" destino="$2" obra="$tmp/obra/rpm-$version"
    rm -rf "$obra"
    mkdir -p "$obra"
    cat > "$obra/rfirma.spec" <<EOF
Name: rfirma
Version: $version
Release: 1
Summary: paquete de prueba
License: GPL-3.0-or-later
BuildArch: x86_64
%description
paquete de prueba
%install
mkdir -p %{buildroot}/usr/bin
printf '#!/bin/sh\n' > %{buildroot}/usr/bin/rfirma
chmod +x %{buildroot}/usr/bin/rfirma
%files
/usr/bin/rfirma
EOF
    rpmbuild --quiet --define "_topdir $obra/top" -bb "$obra/rfirma.spec" > /dev/null 2>&1
    cp "$obra/top/RPMS/x86_64/rfirma-$version-1.x86_64.rpm" "$destino"
}

declare -A commit_de
for version in "${versiones[@]}"; do
    dir="$serie/v$version"
    mkdir -p "$dir"
    commit_de[$version]="$(bundle_de_prueba "$version" "$dir/rfirma-$version.flatpak")"
    deb_de_prueba "$version" "$dir/rfirma_${version}_amd64.deb"
    rpm_de_prueba "$version" "$dir/rfirma-$version.x86_64.rpm"
done

# ---------------------------------------------------------------------------
# El arbol, dos veces y en dos directorios distintos
# ---------------------------------------------------------------------------
"$construye" "$serie" "$tmp/arbol" "$tmp/rfirma.asc" "$SIN_FIRMA" > "$tmp/salida" 2>&1 \
    || { cat "$tmp/salida" >&2; fail "la construccion falla"; }
"$construye" "$serie" "$tmp/otro-arbol" "$tmp/rfirma.asc" "$SIN_FIRMA" > /dev/null 2>&1 \
    || fail "la segunda construccion falla"

# the_tree_has_the_shape_the_addition_commands_promise
if [ -f "$tmp/arbol/rfirma.asc" ] && [ -f "$tmp/arbol/rfirma.flatpakref" ] \
    && [ -f "$tmp/arbol/flatpak/config" ] \
    && [ -f "$tmp/arbol/apt/dists/stable/main/binary-amd64/Packages" ] \
    && [ -f "$tmp/arbol/rpm/repodata/repomd.xml" ]; then
    ok "el arbol tiene la clave, el flatpakref y los tres repositorios"
else
    fail "el arbol no tiene la forma que fija el ADR-0015"
fi

# the_versions_are_ordered_by_version_and_not_by_listing
# El orden en que se importan los bundles es la historia del ostree: v0.4.10 va
# DESPUES de v0.4.2, que es justo lo que el orden alfabetico no hace.
if grep -q "v0.4.0 v0.4.2 v0.4.10" "$tmp/salida"; then
    ok "las versiones se ordenan por version, no alfabeticamente"
else
    fail "las versiones no se ordenan por version: $(cat "$tmp/salida")"
fi

# ---------------------------------------------------------------------------
# ID-173: el ostree
# ---------------------------------------------------------------------------
ref="app/me.sgomez.rfirma/x86_64/stable"

# rebuilding_the_repository_keeps_the_same_ostree_commit
# LA COMPROBACION QUE SOSTIENE TODO EL MECANISMO: si el commit cambiara al
# reconstruir, cada publicacion obligaria a todo el mundo a redescargar la
# aplicacion entera.
uno="$(ostree --repo="$tmp/arbol/flatpak" rev-parse "$ref" 2>/dev/null || echo no)"
dos="$(ostree --repo="$tmp/otro-arbol/flatpak" rev-parse "$ref" 2>/dev/null || echo tampoco)"
if [ "$uno" = "$dos" ] && [ "$uno" != "no" ]; then
    ok "reconstruir el repositorio deja el mismo commit ($uno): nadie redescarga"
else
    fail "reconstruir el repositorio cambia el commit: $uno vs $dos"
fi

# the_head_is_the_newest_version_of_the_series
if [ "$uno" = "${commit_de[0.4.10]}" ]; then
    ok "el ref apunta al commit de la version mas nueva de la serie"
else
    fail "el ref no apunta a la version mas nueva: $uno"
fi

# every_bundle_of_the_series_is_imported_and_the_history_is_not_truncated
truncada=0
for version in "${versiones[@]}"; do
    ostree --repo="$tmp/arbol/flatpak" show "${commit_de[$version]}" > /dev/null 2>&1 \
        || { truncada=1; echo "  falta el commit de $version" >&2; }
done
if [ "$truncada" -eq 0 ]; then
    ok "todos los bundles de la serie estan importados"
else
    fail "la historia se ha truncado: falta algun bundle de la serie"
fi

# the_flatpakref_points_at_the_repository_and_carries_the_key
if grep -q '^Url=https://rfirma.sgomez.me/flatpak/$' "$tmp/arbol/rfirma.flatpakref" \
    && grep -q '^Name=me.sgomez.rfirma$' "$tmp/arbol/rfirma.flatpakref" \
    && grep -q '^GPGKey=.\+' "$tmp/arbol/rfirma.flatpakref"; then
    ok "el flatpakref apunta al repositorio y lleva la clave dentro"
else
    fail "el flatpakref no sirve para instalar de un clic"
fi

# ---------------------------------------------------------------------------
# ID-175: apt con suite stable, dnf con URL literal
# ---------------------------------------------------------------------------
# apt_serves_the_stable_suite_and_not_a_flat_repository
if grep -q '^Suite: stable$' "$tmp/arbol/apt/dists/stable/Release" \
    && [ -f "$tmp/arbol/apt/dists/stable/main/binary-amd64/Packages.gz" ]; then
    ok "apt sirve la suite stable con su indice binary-amd64"
else
    fail "apt no sirve la suite stable: el .sources deb822 de la landing no valdria"
fi

# the_deb822_file_declares_signed_by
if grep -q '^Signed-By: /usr/share/keyrings/rfirma.asc$' "$tmp/arbol/apt/rfirma.sources" \
    && grep -q '^Suites: stable$' "$tmp/arbol/apt/rfirma.sources"; then
    ok "el fichero deb822 servido declara Signed-By y la suite stable"
else
    fail "el fichero deb822 servido no declara Signed-By"
fi

# every_version_of_the_series_is_installable_from_apt
faltantes=0
for version in "${versiones[@]}"; do
    grep -q "^Version: $version\$" "$tmp/arbol/apt/dists/stable/main/binary-amd64/Packages" \
        || { faltantes=1; echo "  falta $version en el indice de apt" >&2; }
done
[ "$faltantes" -eq 0 ] && ok "todas las versiones de la serie estan en el indice de apt" \
    || fail "el indice de apt no trae toda la serie"

# dnf_is_declared_with_the_literal_url_and_both_gpg_switches
repo="$tmp/arbol/rpm/rfirma.repo"
if grep -q '^baseurl=https://rfirma.sgomez.me/rpm/$' "$repo" \
    && ! grep -q '\$basearch\|\$releasever' "$repo" \
    && grep -q '^gpgcheck=1$' "$repo" && grep -q '^repo_gpgcheck=1$' "$repo" \
    && grep -q '^gpgkey=https://rfirma.sgomez.me/rfirma.asc$' "$repo"; then
    ok "dnf se declara con URL literal y con los dos interruptores de GPG a 1"
else
    fail "el .repo de dnf no es el que promete la landing"
fi

# every_version_of_the_series_is_in_the_dnf_index
# `zcat -f` porque `createrepo_c` deja el primario comprimido y con un prefijo
# de hash delante del nombre: ni la ruta ni la compresion son fijas.
primario="$(find "$tmp/arbol/rpm/repodata" -name '*primary.xml*' | head -1)"
if [ -n "$primario" ]; then
    # `grep -o`, no `grep -c`: el XML del primario no viene con un paquete por
    # linea y contar lineas daria uno.
    en_indice="$(zcat -f "$primario" | grep -o '<package type="rpm">' | wc -l)"
else
    en_indice=0
fi
servidos="$(find "$tmp/arbol/rpm" -maxdepth 1 -name '*.rpm' | wc -l)"
if [ "$en_indice" -eq "${#versiones[@]}" ] && [ "$servidos" -eq "${#versiones[@]}" ]; then
    ok "todas las versiones de la serie estan servidas por dnf"
else
    fail "el repositorio dnf no trae toda la serie: $en_indice en el indice, $servidos servidos"
fi

# ---------------------------------------------------------------------------
# Lo demas
# ---------------------------------------------------------------------------
# rebuilding_gives_the_same_packages_and_the_same_addition_files
# Los indices firmados (`InRelease`, `repomd.xml`, el `summary` de ostree)
# llevan fecha dentro y NO pueden ser identicos; lo que descarga un cliente si.
huella_estable() {
    (cd "$1" && find rfirma.asc rfirma.flatpakref apt/pool apt/rfirma.sources \
        rpm/rfirma.repo -maxdepth 4 -type f | LC_ALL=C sort \
        | while read -r f; do printf '%s %s\n' "$f" "$(sha256sum < "$f" | cut -d' ' -f1)"; done)
    (cd "$1/rpm" && find . -maxdepth 1 -name '*.rpm' | LC_ALL=C sort \
        | while read -r f; do printf '%s %s\n' "$f" "$(sha256sum < "$f" | cut -d' ' -f1)"; done)
}
if [ "$(huella_estable "$tmp/arbol")" = "$(huella_estable "$tmp/otro-arbol")" ]; then
    ok "reconstruir deja los mismos paquetes y los mismos ficheros de alta"
else
    fail "reconstruir cambia lo que descarga un cliente"
fi

# a_stale_tree_is_wiped_and_not_merged
echo "de la vez anterior" > "$tmp/arbol/sobra.txt"
"$construye" "$serie" "$tmp/arbol" "$tmp/rfirma.asc" "$SIN_FIRMA" > /dev/null 2>&1
if [ -e "$tmp/arbol/sobra.txt" ]; then
    fail "el arbol anterior se mezcla con el nuevo en vez de rehacerse"
else
    ok "el arbol se rehace entero, no se mezcla con el anterior"
fi

echo
if [ "$fallos" -ne 0 ]; then
    echo "$fallos comprobacion(es) del arbol han fallado" >&2
    exit 1
fi
echo "OK  los tres repositorios se reconstruyen enteros y nadie redescarga"
