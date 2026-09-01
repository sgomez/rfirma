#!/usr/bin/env bash
# Comprueba que el bundle del sistema de diseno no se ha retocado a mano.
#
# POR QUE (ID-56): desde el #85 el bundle es NORMATIVO y ya no hay un origen
# que consultar. Lo que se descarga del proyecto de diseno se versiona aqui y
# esto es la unica copia. Un token cambiado a mano en `bundle/` no lo detecta
# nadie: no rompe la compilacion, no rompe una prueba de tipos, y la ficha
# `docs/design/design-system.md` solo cubre la tabla de color y el vocabulario.
#
# El sello detecta exactamente eso —una edicion en el sitio equivocado— y NO
# detecta, ni puede, que el diseno de origen haya cambiado. Esa es la propiedad
# que se busca: el bundle se cambia en el proyecto de diseno, se reexporta
# entero y se resella con `just seal-ds-bundle`.
#
# COMO: mismo patron que packaging/flatpak/check-sources.sh. `bundle.lock`
# guarda el sha256 de cada fichero del bundle en el formato de `sha256sum`,
# asi que la comprobacion ES `sha256sum -c` y no un analizador nuestro. Las
# rutas van relativas a la raiz del repositorio.
#
# Uso: rfirma-app/src/design-system/check-bundle.sh
set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../../.." && pwd)"
cd "$RAIZ"

BUNDLE="rfirma-app/src/design-system/bundle"
SELLO="rfirma-app/src/design-system/bundle.lock"

if [ ! -f "$SELLO" ]; then
    echo "falta $SELLO" >&2
    echo "Ejecuta 'just seal-ds-bundle'." >&2
    exit 1
fi

# Un fichero NUEVO dentro del bundle no lo ve `sha256sum -c`, que solo
# comprueba lo que ya esta en la lista. Se compara la lista de ficheros del
# arbol con la del sello, ordenadas igual, antes de mirar ningun hash.
# `_ds_needs_recompile` queda fuera, igual que en .gitignore: es un marcador de
# estado de design-sync-cli, no parte del sistema de diseno, y aparece y
# desaparece solo en el equipo de quien exporta.
en_el_arbol="$(find "$BUNDLE" -type f ! -name _ds_needs_recompile | LC_ALL=C sort)"
en_el_sello="$(sed -e 's/^[0-9a-f]\{64\}  //' "$SELLO" | LC_ALL=C sort)"

if [ "$en_el_arbol" != "$en_el_sello" ]; then
    echo "el bundle del sistema de diseno tiene ficheros que el sello no cubre" >&2
    echo >&2
    diff <(echo "$en_el_sello") <(echo "$en_el_arbol") >&2 || true
    echo >&2
    echo "'<' esta sellado y ya no existe; '>' existe y no esta sellado." >&2
    echo "Ejecuta 'just seal-ds-bundle' y versiona bundle.lock." >&2
    exit 1
fi

# --status para no imprimir una linea "OK" por fichero: aqui solo interesa el
# fallo, y el mensaje util lo damos nosotros.
if ! sha256sum --check --status "$SELLO"; then
    echo "el bundle del sistema de diseno NO coincide con su sello" >&2
    echo >&2
    # Sin --status, para que se vea CUAL fichero ha cambiado: es la unica
    # informacion que hace falta para arreglarlo.
    sha256sum --check "$SELLO" >&2 || true
    echo >&2
    echo "El bundle es normativo y no se edita aqui (ID-47, ID-56): se cambia" >&2
    echo "en el proyecto de sistema de diseno y se reexporta entero." >&2
    echo >&2
    echo "Si el cambio es intencionado, ejecuta 'just seal-ds-bundle' y" >&2
    echo "versiona bundle.lock." >&2
    exit 1
fi

echo "bundle del sistema de diseno sellado y sin tocar"
