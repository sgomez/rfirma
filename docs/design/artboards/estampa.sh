#!/usr/bin/env bash
# Copia cada `_*.part` en los artboards que lo llevan: `_helmet.part` sustituye
# el <helmet> entero, y los demás, lo que haya entre `<!-- _X.part -->` y
# `<!-- /_X.part -->`. Después, `comprueba.sh`.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

for part in _*.part; do
    for f in *.dc.html; do
        if [ "$part" = _helmet.part ]; then
            PART="$part" perl -0777 -i -pe '
                BEGIN { local $/; open my $h, "<", $ENV{PART} or die; $p = <$h> }
                s{^[^\n]*<helmet>.*?</helmet>[^\n]*\n}{$p}ms' "$f"
        elif grep -qF "<!-- $part -->" "$f"; then
            PART="$part" perl -0777 -i -pe '
                BEGIN { local $/; open my $h, "<", $ENV{PART} or die; $p = <$h> }
                s{(<!-- \Q$ENV{PART}\E -->\n).*?(^[ \t]*<!-- /\Q$ENV{PART}\E -->)}{$1$p$2}gms' "$f"
        fi
    done
done
exec ./comprueba.sh
