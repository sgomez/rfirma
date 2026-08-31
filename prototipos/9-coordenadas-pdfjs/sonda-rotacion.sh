#!/usr/bin/env bash
# PROTOTIPO #9 — mide la transformacion T que iText aplica al rectangulo antes
# de escribir el /Rect del widget. Pasa el MISMO rectangulo a las cuatro
# rotaciones de un PDF con MediaBox desplazada y enseña que sale.
#
# Es lo que fija la tabla de T^-1 que usa app.js. Reproducible: si cambia la
# version de afirma-lib-itext, se vuelve a correr esto antes que nada.
set -euo pipefail
cd "$(dirname "$0")"
R="100 200 300 260"
for c in offset offset-rot90 offset-rot180 offset-rot270; do
  rot=0; [[ $c == *rot* ]] && rot=${c##*rot}
  cat > salidas/sonda-$c.properties <<EOF
# sonda de T para $c
# rfirma-esperado: {"caso":"$c","pagina":1,"widget":[0,0,0,0],"mediabox":[20,30,615,872],"rotate":$rot}
signaturePage=1
signaturePositionOnPageLowerLeftX=100
signaturePositionOnPageLowerLeftY=200
signaturePositionOnPageUpperRightX=300
signaturePositionOnPageUpperRightY=260
layer2Text=SONDA $c
EOF
  ./firmar.sh salidas/sonda-$c.properties >/dev/null 2>&1
  printf "%-14s /Rotate %-3s  entrada [%s]  ->  /Rect " "$c" "$rot" "$R"
  .venv-proto/bin/python - "$c" <<'PY'
import sys
from pypdf import PdfReader
for a in PdfReader(f"salidas/sonda-{sys.argv[1]}-firmado.pdf").pages[0]["/Annots"]:
    o = a.get_object()
    if o.get("/Subtype") == "/Widget":
        print([round(float(x)) for x in o["/Rect"]])
PY
done
