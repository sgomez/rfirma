#!/usr/bin/env bash
# Comprueba que todos los artboards llevan EL MISMO <helmet> que `_helmet.part`,
# y que los que pintan la ventana principal de fondo llevan, entre sus
# marcadores, exactamente los demas `_*.part`. Los estampa `estampa.sh`.
#
# Antes esto solo comparaba los artboards entre si, y eso deja pasar el fallo
# que de verdad ocurre: redactar un artboard nuevo copiando el <helmet> de un
# `get_file` del proyecto de Claude Design, cuya copia se ha quedado atras. Si
# se importaran todos de ahi, trece ficheros de acuerdo entre si darian verde
# con el sistema de diseno equivocado entero. Paso el 02/09/2026 con los dos
# tokens de sombra, y lo salvo la casualidad de que solo eran tres ficheros.
#
# La direccion es siempre repo -> proyecto: el proyecto no puede ejecutar nada.
# `_helmet.part` es la copia buena, y sale del bundle versionado (ID-47), que
# es lo unico normativo.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

if [ ! -f _helmet.part ]; then
    echo "ERROR: falta _helmet.part, que es de donde sale el <helmet> de todos" >&2
    exit 1
fi
patron=$(sha256sum _helmet.part | cut -d' ' -f1)

malos=()
for f in *.dc.html; do
    suyo=$(sed -n '/<helmet>/,/<\/helmet>/p' "$f" | sha256sum | cut -d' ' -f1)
    [ "$suyo" = "$patron" ] || malos+=("$f")
done

if [ ${#malos[@]} -ne 0 ]; then
    echo "ERROR: estos artboards no llevan el <helmet> de _helmet.part:" >&2
    printf '  %s\n' "${malos[@]}" >&2
    exit 1
fi

con_fondo_de_main=(Main EstadoPin EstadoPinIncorrecto EstadoAcercaDe EstadoFirmarDeTodosModos EstadoPaginasSinFirmaVisible)
# La barra de la ventana principal la llevan tambien las vistas sin pestanas.
solo_cabecera=(PreferenciasPantalla PanelEstado RetirarCertificado PrimerArranque)

for part in _*.part; do
    [ "$part" = _helmet.part ] && continue
    patron=$(sha256sum "$part" | cut -d' ' -f1)
    malos=()
    for f in *.dc.html; do
        abre=$(grep -cF "<!-- $part -->" "$f" || true)
        obligado=no
        for a in "${con_fondo_de_main[@]}"; do [ "$f" = "$a.dc.html" ] && obligado=si; done
        if [ "$part" = _cabecera.part ]; then
            for a in "${solo_cabecera[@]}"; do [ "$f" = "$a.dc.html" ] && obligado=si; done
        fi
        [ "$abre" = 0 ] && [ "$obligado" = no ] && continue
        cierra=$(grep -cF "<!-- /$part -->" "$f" || true)
        suyo=$(awk -v a="<!-- $part -->" -v c="<!-- /$part -->" \
            'index($0, c) { dentro = 0 } dentro { print } index($0, a) { dentro = 1 }' "$f" | sha256sum | cut -d' ' -f1)
        { [ "$abre" = 1 ] && [ "$cierra" = 1 ] && [ "$suyo" = "$patron" ]; } || malos+=("$f")
    done
    if [ ${#malos[@]} -ne 0 ]; then
        echo "ERROR: estos artboards no llevan $part tal cual (./estampa.sh lo copia):" >&2
        printf '  %s\n' "${malos[@]}" >&2
        exit 1
    fi
done
echo "OK: $(ls -1 *.dc.html | wc -l) artboards con el <helmet> de _helmet.part; ${#con_fondo_de_main[@]} con el fondo de Main; $(( ${#con_fondo_de_main[@]} + ${#solo_cabecera[@]} )) con _cabecera.part"
