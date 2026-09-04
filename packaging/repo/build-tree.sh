#!/usr/bin/env bash
# EL ARBOL SERVIDO ES DERIVADO (ID-172, ADR-0015): la fuente de verdad son las
# Releases, y esto reconstruye desde cero —en un directorio nuevo, cada vez— lo
# que `publish-tree.sh` sube al anfitrion. No se muta nada de lo que ya hay
# servido: si algo sale mal, se tira el directorio y se vuelve a empezar.
#
# POR QUE SE RECONSTRUYE ENTERO en vez de anadir la version nueva a lo que ya
# hay: esto corre pocas veces al ano y nadie va a recordar como estaba el
# volumen. Una publicacion no idempotente convierte cualquier fallo en
# arqueologia sobre un servidor.
#
# RECONSTRUIR NO OBLIGA A NADIE A REDESCARGAR (ID-173), y es lo unico que hace
# que lo anterior sea aceptable: importar el mismo bundle en un repositorio
# ostree vacio da el MISMO commit, con el mismo ContentChecksum, asi que un
# cliente ya instalado no ve nada nuevo. Tres cabos, y los tres estan atados
# mas abajo con su comentario:
#
#   1. hay que `ostree init` DELANTE, porque `build-import-bundle` no crea el
#      repositorio;
#   2. hay que importar TODOS los bundles de la serie, en orden de version: la
#      historia se trunca a lo que se importe;
#   3. hay que RE-FIRMAR SIEMPRE, porque la firma es metadato desacoplado que
#      no viaja dentro del bundle —y firmar no altera el checksum del commit,
#      que es lo que permite hacerlo en cada reconstruccion—.
#
# NADA DE LO QUE SE ESCRIBE AQUI PUEDE DEPENDER DEL MOMENTO en lo que el
# cliente descarga: los paquetes, los commits de ostree y los ficheros de alta
# son identicos en cada reconstruccion. Los indices firmados (`InRelease`,
# `repomd.xml`, el `summary` de ostree) llevan fecha y firma, asi que esos
# cambian byte a byte y no pueden no hacerlo; lo que `build-tree.test.sh`
# comprueba es lo primero, que es lo que decide si alguien redescarga.
#
# LA FORMA DEL ARBOL la fija el ADR-0015 y estas rutas viajan dentro del
# `.flatpakref` y de las ordenes de alta ya publicadas (las de la landing, en
# `index.html`):
#
#   /rfirma.asc         la clave publica (el `Signed-By` de apt y el `gpgkey`
#                       de dnf)
#   /rfirma.flatpakref  la instalacion de un clic
#   /flatpak/           el repositorio ostree
#   /apt/               con dists/stable/main/binary-amd64/
#   /rpm/               con repodata/
#
# APT CON SUITE `stable` Y NO REPOSITORIO PLANO (ID-175): el plano es mas
# barato y NO admite `Suites:`/`Components:` en un `.sources` deb822, que es el
# formato obligado para que la clave vaya en `Signed-By` sin `apt-key`
# (retirado). DNF CON URL LITERAL, sin `$basearch` ni `$releasever`: meterlas
# seria prometer arquitecturas que no se construyen.
#
# LOS `.rpm` LLEGAN AQUI YA FIRMADOS y esto lo COMPRUEBA antes de indexarlos.
# Firmar un `.rpm` lo modifica, asi que la firma va en `release.yml` —antes del
# `SHA256SUMS` y antes de la atestacion— y no aqui: si se firmara aqui, el
# `.rpm` del repositorio dnf y el de la Release dejarian de ser los mismos
# bytes y se rompe la invariante del ID-144. Aqui solo se rechaza el que venga
# sin firma, que es la unica forma de que `gpgcheck=1` no falle en la maquina
# de quien instala.
#
# Uso: build-tree.sh <serie> <arbol> <clave.asc> <huella>
#   <serie>      directorio con un subdirectorio por Release de la serie
#                vigente, tal como lo deja `download-series.sh`
#   <arbol>      directorio de salida; SE BORRA Y SE REHACE
#   <clave.asc>  la clave publica de firma de rFirma, en armadura ASCII
#   <huella>     la huella de la clave con la que se firman los tres indices.
#                La clave privada tiene que estar ya en el llavero (en el CI la
#                importa `.github/actions/import-signing-key`); aqui NO se crea
#                ninguna clave, nunca.
#
# Variables: GPG_PASSPHRASE_FILE, el fichero con la contrasena de la subclave
# (el que produce esa misma accion). Sin el, gpg pide la contrasena por
# terminal y en un runner eso es colgarse.
set -euo pipefail

# La unica salida para probar el arbol sin una clave privada delante. Se llama
# asi de feo A PROPOSITO y `.github/check-workflows.sh` prohibe que esta cadena
# aparezca en un workflow: sin ese candado, esto seria la manera de publicar un
# repositorio sin firmar y que nadie se enterase hasta que un cliente rechaza
# el indice.
SIN_FIRMA="SIN-FIRMA-SOLO-PRUEBAS"

if [ "$#" -ne 4 ]; then
    echo "uso: build-tree.sh <serie> <arbol> <clave.asc> <huella>" >&2
    exit 2
fi

serie="${1%/}"
arbol="${2%/}"
clave="$3"
huella="$4"

[ -d "$serie" ] || { echo "la serie '$serie' no existe" >&2; exit 1; }
[ -f "$clave" ] || { echo "la clave '$clave' no existe" >&2; exit 1; }

# Las versiones de la serie, en orden de version y no de listado: el orden en
# que se importan los bundles es la historia del repositorio ostree, asi que
# tiene que ser el mismo en cada reconstruccion.
mapfile -t versiones < <(find "$serie" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -V)

if [ "${#versiones[@]}" -eq 0 ]; then
    echo "la serie '$serie' no tiene ninguna version" >&2
    exit 1
fi

# Cada version tiene que traer los tres paquetes: un arbol al que le falte uno
# es un canal que se queda a medias sin que nadie se entere hasta que alguien
# no puede instalar.
for version in "${versiones[@]}"; do
    for patron in '*.flatpak' '*.deb' '*.rpm'; do
        if [ -z "$(find "$serie/$version" -maxdepth 1 -name "$patron" -print -quit)" ]; then
            echo "a la version $version le falta un paquete $patron" >&2
            exit 1
        fi
    done
done

# Las herramientas, TODAS de golpe y por su nombre de paquete: que la
# publicacion se caiga a la mitad porque falta `createrepo_c` deja el arbol a
# medio construir y a quien lo mire buscando en el registro cual de los tres
# repositorios se quedo sin escribir.
faltan=()
for par in \
    ostree:ostree flatpak:flatpak \
    dpkg-scanpackages:dpkg-dev apt-ftparchive:apt-utils \
    createrepo_c:createrepo-c rpm:rpm gpg:gnupg gzip:gzip
do
    if ! command -v "${par%%:*}" > /dev/null 2>&1; then
        faltan+=("${par%%:*} (${par##*:})")
    fi
done
if [ "${#faltan[@]}" -ne 0 ]; then
    echo "faltan herramientas para construir el arbol: ${faltan[*]}" >&2
    echo "    sudo apt-get install -y ostree flatpak dpkg-dev apt-utils createrepo-c rpm gnupg" >&2
    exit 1
fi

# ---------------------------------------------------------------- la firma --
# Tres indices y tres firmas distintas (ADR-0015): en apt se firma SOLO el
# indice (`InRelease`), en dnf el `repomd.xml` (`repo_gpgcheck=1`) y en ostree
# los commits y el `summary`. La clave es una sola.
if [ "$huella" = "$SIN_FIRMA" ]; then
    echo "  AVISO: arbol SIN FIRMAR ($SIN_FIRMA)."
    echo "         Vale para las pruebas y no vale para publicar: apt y dnf"
    echo "         rechazan un indice sin firma en cuanto alguien lo anade."
    firmar=0
else
    firmar=1
    gpg_args=(--batch --yes --local-user "$huella")
    if [ -n "${GPG_PASSPHRASE_FILE:-}" ]; then
        gpg_args+=(--pinentry-mode loopback --passphrase-file "$GPG_PASSPHRASE_FILE")
    fi
fi

# Firma separada y en armadura: es lo que consumen `Release.gpg` de apt y
# `repomd.xml.asc` de dnf.
firma_separada() {
    [ "$firmar" -eq 1 ] || return 0
    rm -f "$2"
    gpg "${gpg_args[@]}" --armor --detach-sign --output "$2" "$1"
}

# Firma en claro: `InRelease` es el `Release` con la firma dentro, y es el que
# apt prefiere —`Release` + `Release.gpg` se sirven ademas para los clientes
# viejos, que cuestan dos lineas—.
firma_en_claro() {
    [ "$firmar" -eq 1 ] || return 0
    rm -f "$2"
    gpg "${gpg_args[@]}" --clearsign --output "$2" "$1"
}

# EL AGENTE, CEBADO A MANO, y no es un apano: `flatpak build-sign` firma por
# gpgme y gpgme no sabe de `--passphrase-file`, asi que sin contrasena en la
# cache del agente se queda esperando un pinentry que en un runner no hay
# nadie para contestar. Una firma cualquiera hecha antes con `loopback` deja la
# contrasena en esa cache, y las dos ordenes de flatpak van detras.
ceba_el_agente() {
    [ "$firmar" -eq 1 ] || return 0
    [ -n "${GPG_PASSPHRASE_FILE:-}" ] || return 0
    printf 'ceba el agente' | gpg "${gpg_args[@]}" --detach-sign --output /dev/null
}

rm -rf "$arbol"
mkdir -p "$arbol/flatpak" "$arbol/apt" "$arbol/rpm"

# La clave publica no la genera nada: es la misma que firma las Releases, y
# aqui se copia tal cual para que apt y dnf la encuentren en la ruta que dicen
# las ordenes de alta publicadas.
cp "$clave" "$arbol/rfirma.asc"

# ------------------------------------------------------------- 1. el ostree --
# `ostree init` DELANTE (cabo 1 del ID-173): `flatpak build-import-bundle` no
# crea el repositorio y falla si no existe. `--mode=archive` porque esto lo
# sirve un servidor web con ficheros sueltos, no un cliente local.
echo "  ostree: inicializando el repositorio"
ostree init --mode=archive --repo="$arbol/flatpak"

# TODOS los bundles y EN ORDEN DE VERSION (cabo 2 del ID-173): la historia del
# repositorio es exactamente lo que se importe aqui, asi que saltarse una
# version es borrarla del canal. `--no-update-summary` porque el summary se
# escribe UNA vez al final, ya con todo dentro y firmado.
for version in "${versiones[@]}"; do
    for bundle in "$serie/$version"/*.flatpak; do
        echo "  ostree: importando $version ($(basename "$bundle"))"
        flatpak build-import-bundle --no-update-summary "$arbol/flatpak" "$bundle"
    done
done

# RE-FIRMAR SIEMPRE (cabo 3 del ID-173). La firma no viaja dentro del bundle:
# es metadato desacoplado del commit, asi que un repositorio recien reimportado
# tiene los commits buenos y ninguna firma. Firmar no altera el checksum del
# commit —esa es toda la razon de que reconstruir no obligue a redescargar—,
# de modo que aqui no hay nada que decidir: se firma en cada reconstruccion.
ceba_el_agente
if [ "$firmar" -eq 1 ]; then
    echo "  ostree: firmando los commits y el summary"
    flatpak build-sign --gpg-sign="$huella" "$arbol/flatpak"
    flatpak build-update-repo --gpg-sign="$huella" "$arbol/flatpak"
else
    flatpak build-update-repo "$arbol/flatpak"
fi

# EL `.flatpakref` es la instalacion de un clic de la landing, y lleva la clave
# DENTRO —en binario y en base64, que es como lo quiere flatpak—: quien lo abre
# no tiene que anadir ninguna clave a mano. `RuntimeRepo` es lo que hace que
# flatpak sepa de donde sacar el runtime de GNOME, que no viaja en el bundle.
clave_binaria="$(gpg --dearmor < "$clave" | base64 -w0)"
if [ -z "$clave_binaria" ]; then
    echo "la clave '$clave' no esta en armadura ASCII: sin ella el" >&2
    echo ".flatpakref no lleva clave y la instalacion de un clic no verifica" >&2
    echo "nada. Se exporta con: gpg --armor --export <huella>" >&2
    exit 1
fi
{
    echo "[Flatpak Ref]"
    echo "Title=rFirma"
    echo "Name=me.sgomez.rfirma"
    echo "Branch=stable"
    echo "Url=https://rfirma.sgomez.me/flatpak/"
    echo "IsRuntime=false"
    echo "RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo"
    echo "GPGKey=$clave_binaria"
} > "$arbol/rfirma.flatpakref"

# ---------------------------------------------------------------- 2. el apt --
# `pool/` con TODAS las versiones de la serie y un solo `dists/stable`: un
# repositorio por distribucion no tiene sentido aqui porque el `.deb` vale
# igual en Debian y en Ubuntu (dependencias debiles, ADR-0013).
echo "  apt: montando pool y dists/stable"
mkdir -p "$arbol/apt/pool/main/r/rfirma" "$arbol/apt/dists/stable/main/binary-amd64"
for version in "${versiones[@]}"; do
    cp "$serie/$version"/*.deb "$arbol/apt/pool/main/r/rfirma/"
done

release_tmp="$(mktemp)"
trap 'rm -f "$release_tmp"' EXIT
(
    cd "$arbol/apt"
    # `--multiversion` para que las versiones viejas de la serie sigan en el
    # indice: un cliente que fija una version tiene que poder instalarla.
    # `dpkg-scanpackages` ordena su salida, asi que el `Packages` es el mismo
    # en cada reconstruccion.
    dpkg-scanpackages --multiversion pool/main > dists/stable/main/binary-amd64/Packages 2> /dev/null
    gzip -9 -n -c dists/stable/main/binary-amd64/Packages \
        > dists/stable/main/binary-amd64/Packages.gz

    # `Suites: stable` en el `.sources` de la landing solo funciona si aqui hay
    # un `dists/stable` de verdad: es toda la razon de no montar un repositorio
    # plano.
    #
    # La salida va a un fichero de fuera y se mueve despues: `apt-ftparchive
    # release` recorre el directorio que se le da, asi que escribir el `Release`
    # dentro de el mientras lo recorre lo haria aparecer en su propia lista de
    # resumenes.
    apt-ftparchive \
        -o APT::FTPArchive::Release::Origin=rFirma \
        -o APT::FTPArchive::Release::Label=rFirma \
        -o APT::FTPArchive::Release::Suite=stable \
        -o APT::FTPArchive::Release::Codename=stable \
        -o APT::FTPArchive::Release::Architectures=amd64 \
        -o APT::FTPArchive::Release::Components=main \
        -o APT::FTPArchive::Release::Description="rFirma para Debian, Ubuntu y derivadas" \
        release dists/stable > "$release_tmp"
    mv "$release_tmp" dists/stable/Release
    chmod 644 dists/stable/Release
)

firma_en_claro "$arbol/apt/dists/stable/Release" "$arbol/apt/dists/stable/InRelease"
firma_separada "$arbol/apt/dists/stable/Release" "$arbol/apt/dists/stable/Release.gpg"

# El `.sources` servido es EL MISMO que enseña la landing, y se sirve para que
# el alta sea una descarga en vez de un copiar y pegar. `Signed-By` con una
# ruta de fichero es lo que sustituye a `apt-key`, retirado.
cat > "$arbol/apt/rfirma.sources" <<'EOF'
Types: deb
URIs: https://rfirma.sgomez.me/apt/
Suites: stable
Components: main
Signed-By: /usr/share/keyrings/rfirma.asc
EOF

# ---------------------------------------------------------------- 3. el dnf --
for version in "${versiones[@]}"; do
    for paquete in "$serie/$version"/*.rpm; do
        # `%{SIGPGP}`/`%{SIGGPG}` son las dos cabeceras donde puede estar la
        # firma segun el algoritmo; sin ninguna de las dos, el paquete NO esta
        # firmado y `gpgcheck=1` lo rechazaria en la maquina de quien instala.
        # Se mira aqui —y no solo en `release.yml`— porque este es el ultimo
        # sitio donde el fichero se toca antes de servirse. En el modo sin
        # firma no se mira: ahi no hay ninguna clave delante y el arbol entero
        # es inservible para publicar, que es justo lo que dice su nombre.
        if [ "$firmar" -eq 1 ]; then
            sig="$(rpm -qp --nosignature --qf '%{SIGPGP}%{SIGGPG}' "$paquete" 2> /dev/null || true)"
            if [ -z "$sig" ] || [ "$sig" = "(none)(none)" ]; then
                echo "el paquete $(basename "$paquete") no lleva firma dentro" >&2
                echo "se firma en release.yml, ANTES del SHA256SUMS y de la" >&2
                echo "atestacion: firmarlo despues cambia sus bytes (ID-144)." >&2
                exit 1
            fi
        fi
        cp "$paquete" "$arbol/rpm/"
    done
done

echo "  dnf: generando repodata"
# `--no-database` porque los sqlite del indice solo los usa yum viejo y son la
# mitad del peso; dnf lee el XML.
createrepo_c --quiet --no-database "$arbol/rpm"
firma_separada "$arbol/rpm/repodata/repomd.xml" "$arbol/rpm/repodata/repomd.xml.asc"

# El `.repo` servido, igual que el `.sources` de apt y con la MISMA URL literal
# que la landing. Los dos interruptores a 1 y no uno: `repo_gpgcheck` verifica
# el `repomd.xml.asc` de arriba y `gpgcheck` la firma de dentro de cada `.rpm`.
cat > "$arbol/rpm/rfirma.repo" <<'EOF'
[rfirma]
name=rfirma
baseurl=https://rfirma.sgomez.me/rpm/
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=https://rfirma.sgomez.me/rfirma.asc
EOF

echo "OK  arbol construido en $arbol con ${#versiones[@]} version(es): ${versiones[*]}"
