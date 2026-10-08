#!/usr/bin/env bash
# La puerta del contenido del paquete (ADR-0012, ID-144).
#
# Independiente del formato: los tres canales del ADR-0004 —flatpak, .deb y
# .rpm— tienen que cumplir la MISMA invariante, exactamente un
# librfirma_crypto.so y libawt.so en ninguna parte, y este es el UNICO sitio
# que la comprueba. Antes vivia dentro de packaging/flatpak/verifica.sh, atada
# al flatpak; se muda aqui para que los tres formatos pasen por la misma
# puerta en vez de que cada canal reinvente su propia comprobacion.
#
# Corre sobre el PAQUETE CONSTRUIDO, no sobre el arbol de compilacion
# (TD-43): el arbol de native-image sigue teniendo los auxiliares de AWT en
# target/native (ver el comentario de la receta `native` del justfile), asi
# que mirar ahi daria un falso positivo. Lo unico que cuenta es lo que va a
# llegar a quien instale.
#
# Escrita y probada HOY contra el flatpak, que es el unico canal que existe
# todavia (el #265 va antes que los paquetes a proposito). Las ramas .deb y
# .rpm quedan listas para cuando el #266 los produzca.
#
# El instalador de Windows (ADR-0035) pasa por la misma invariante con los
# nombres de Windows: exactamente un rfirma_crypto.dll y awt.dll en ninguna
# parte. Se abre con 7z, que lee los instaladores NSIS sin ejecutarlos. El
# .dmg de macOS, igual: un librfirma_crypto.dylib y libawt.dylib en ninguna parte.
#
# Uso: packaging/verifica-contenido.sh <paquete.flatpak|paquete.deb|paquete.rpm|instalador.exe|imagen.dmg|files/>
set -euo pipefail

PAQUETE="${1:?uso: packaging/verifica-contenido.sh <paquete>}"
[ -e "$PAQUETE" ] || { echo "no existe $PAQUETE" >&2; exit 1; }

LAB="$(mktemp -d)"
trap 'rm -rf "$LAB"' EXIT

if [ -d "$PAQUETE" ]; then
    # El files/ de una construccion de flatpak (ADR-0004, ADR-0013): ya son
    # los bytes finales, no hay nada que extraer.
    ln -s "$(realpath "$PAQUETE")" "$LAB/contenido"
else
    case "$PAQUETE" in
        *.flatpak)
            # Se extrae con ostree/build-import-bundle, sin instalar: es mas
            # barato y no depende de sandbox ni de permisos, y son exactamente
            # los bytes que se van a distribuir (el mismo commit que produce
            # `flatpak build-bundle`, ver el ADR-0015).
            ostree init --mode=archive --repo="$LAB/repo" >/dev/null
            flatpak build-import-bundle "$LAB/repo" "$PAQUETE" >/dev/null
            ref="$(ostree refs --repo="$LAB/repo")"
            ostree checkout --repo="$LAB/repo" -U "$ref" "$LAB/contenido" >/dev/null
            ;;
        *.deb)
            dpkg-deb -x "$PAQUETE" "$LAB/contenido"
            ;;
        *.rpm)
            # bsdtar lee el RPM entero el solo y viene en libarchive-tools, que
            # esta en cualquier Debian; rpm2cpio obliga a instalar `rpm`, que
            # NO esta en el equipo de desarrollo ni en el runner del CI. Se
            # prefiere el primero y se cae al segundo donde solo haya rpm.
            PAQUETE="$(realpath "$PAQUETE")"
            mkdir -p "$LAB/contenido"
            if command -v bsdtar >/dev/null 2>&1; then
                bsdtar -xf "$PAQUETE" -C "$LAB/contenido"
            elif command -v rpm2cpio >/dev/null 2>&1; then
                (cd "$LAB/contenido" && rpm2cpio "$PAQUETE" | cpio -idm --quiet)
            else
                echo "para mirar dentro de un .rpm hace falta bsdtar (libarchive-tools) o rpm2cpio (rpm)" >&2
                exit 1
            fi
            ;;
        *.exe | *.dmg)
            if ! command -v 7z >/dev/null 2>&1; then
                echo "para mirar dentro de un instalador NSIS o de un .dmg hace falta 7z (p7zip-full)" >&2
                exit 1
            fi
            # El enlace a /Applications del .dmg es absoluto, y 7z lo rechaza con error.
            excluye=()
            case "$PAQUETE" in *.dmg) excluye=('-x!*/Applications') ;; esac
            7z x -y -o"$LAB/contenido" "$PAQUETE" "${excluye[@]}" >/dev/null
            ;;
        *)
            echo "formato desconocido: $PAQUETE (se esperaba .flatpak, .deb, .rpm, .exe, .dmg o un directorio files/)" >&2
            exit 1
            ;;
    esac
fi

echo "### contenido de $PAQUETE"

# Las busquedas van escritas enteras, una por sistema, y no con el nombre en
# una variable: BridgeContractTest comprueba que siguen aqui.
case "$PAQUETE" in
    *.exe)
        NATIVA=rfirma_crypto.dll
        AWT=awt.dll
        encontrados="$(find -L "$LAB/contenido" -iname 'rfirma_crypto.dll')"
        sobra="$(find -L "$LAB/contenido" -iname 'awt.dll')"
        ;;
    *.dmg)
        NATIVA=librfirma_crypto.dylib
        AWT=libawt.dylib
        encontrados="$(find -L "$LAB/contenido" -name 'librfirma_crypto.dylib')"
        sobra="$(find -L "$LAB/contenido" -name 'libawt.dylib')"
        ;;
    *)
        NATIVA=librfirma_crypto.so
        AWT=libawt.so
        encontrados="$(find -L "$LAB/contenido" -name 'librfirma_crypto.so')"
        sobra="$(find -L "$LAB/contenido" -name 'libawt.so')"
        ;;
esac

n="$(printf '%s\n' "$encontrados" | grep -c . || true)"
if [ "$n" -ne 1 ]; then
    echo "esperaba exactamente UN $NATIVA, encontrados: $n" >&2
    printf '%s\n' "$encontrados" >&2
    exit 1
fi
echo "OK  un solo $NATIVA ($encontrados)"

if [ -n "$sobra" ]; then
    echo "SOBRA $AWT: un JPEG con perfil ICC aborta el proceso en vez de" >&2
    echo "dar un error recuperable (ADR-0012, docs/research/exclusion-afirma-ui-utils.md)" >&2
    exit 1
fi
echo "OK  $AWT no aparece en ninguna parte"

# Las dependencias del .deb y del .rpm (ID-463): el vigilante enlaza libpcsclite,
# y OpenSC y pcscd van recomendados. Se leen del paquete con las herramientas de
# cada formato; el .rpm, sin `rpm`, por los nombres de su cabecera, que no va comprimida.
exige_relacion() {
    local relaciones="$1" nombre="$2" tipo="$3"
    if ! printf '%s\n' "$relaciones" | grep -q -- "$nombre"; then
        echo "FALTA $nombre entre las relaciones $tipo del paquete" >&2
        exit 1
    fi
    echo "OK  $tipo $nombre"
}

case "$PAQUETE" in
    *.deb)
        exige_relacion "$(dpkg-deb -f "$PAQUETE" Depends)" libpcsclite1 "obligatorias"
        recomendados="$(dpkg-deb -f "$PAQUETE" Recommends)"
        exige_relacion "$recomendados" opensc "recomendadas"
        exige_relacion "$recomendados" pcscd "recomendadas"
        ;;
    *.rpm)
        if command -v rpm >/dev/null 2>&1; then
            obligatorias="$(rpm -qp --requires "$PAQUETE")"
            recomendados="$(rpm -qp --recommends "$PAQUETE")"
        else
            obligatorias="$(grep -a -o 'libpcsclite\.so\.1[()0-9a-z]*' "$PAQUETE" || true)"
            recomendados="$(grep -a -o -e 'opensc' -e 'pcsc-lite' "$PAQUETE" || true)"
        fi
        exige_relacion "$obligatorias" libpcsclite.so.1 "obligatorias"
        exige_relacion "$recomendados" opensc "recomendadas"
        exige_relacion "$recomendados" pcsc-lite "recomendadas"
        ;;
esac
