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
# Uso: packaging/verifica-contenido.sh <paquete.flatpak|paquete.deb|paquete.rpm>
set -euo pipefail

PAQUETE="${1:?uso: packaging/verifica-contenido.sh <paquete>}"
[ -f "$PAQUETE" ] || { echo "no existe $PAQUETE" >&2; exit 1; }

LAB="$(mktemp -d)"
trap 'rm -rf "$LAB"' EXIT

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
        mkdir -p "$LAB/contenido"
        (cd "$LAB/contenido" && rpm2cpio "$PAQUETE" | cpio -idm --quiet)
        ;;
    *)
        echo "formato desconocido: $PAQUETE (se esperaba .flatpak, .deb o .rpm)" >&2
        exit 1
        ;;
esac

echo "### contenido de $PAQUETE"

encontrados="$(find "$LAB/contenido" -name 'librfirma_crypto.so')"
n="$(printf '%s\n' "$encontrados" | grep -c . || true)"
if [ "$n" -ne 1 ]; then
    echo "esperaba exactamente UN librfirma_crypto.so, encontrados: $n" >&2
    printf '%s\n' "$encontrados" >&2
    exit 1
fi
echo "OK  un solo librfirma_crypto.so ($encontrados)"

if find "$LAB/contenido" -name 'libawt.so' | grep -q .; then
    echo "SOBRA libawt.so: un JPEG con perfil ICC aborta el proceso en vez de" >&2
    echo "dar un error recuperable (ADR-0012, docs/research/exclusion-afirma-ui-utils.md)" >&2
    exit 1
fi
echo "OK  libawt.so no aparece en ninguna parte"
