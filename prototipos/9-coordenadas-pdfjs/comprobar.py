#!/usr/bin/env python3
"""PROTOTIPO #9 -- comprobacion de ida y vuelta.

Lee el PDF firmado y compara el /Rect del widget de firma que dejo AutoFirma
con el recuadro que se dibujo en el visor (anotado en el propio .properties).

  ./comprobar.py salidas/a4-rot90.properties
"""
import json, os, re, sys
from pypdf import PdfReader

props = open(sys.argv[1], encoding="utf-8").read()
esp = json.loads(re.search(r"# rfirma-esperado: (.*)", props).group(1))
d = os.path.dirname(os.path.abspath(sys.argv[1]))
r = PdfReader(os.path.join(d, os.path.basename(sys.argv[1]).replace(".properties", "-firmado.pdf")))
page = r.pages[esp["pagina"] - 1]

widgets = [a.get_object() for a in page.get("/Annots", [])
           if a.get_object().get("/Subtype") == "/Widget"]
esperado = esp["widget"]
print(f'caso        {esp["caso"]}  (MediaBox {esp["mediabox"]} /Rotate {esp["rotate"]})')
print(f'dibujado    llx,lly,urx,ury = {esperado}')
if not widgets:
    print("SIN WIDGET DE FIRMA"); sys.exit(1)
malo = 0
for w in widgets:
    v = [round(float(x)) for x in w["/Rect"]]
    got = [min(v[0], v[2]), min(v[1], v[3]), max(v[0], v[2]), max(v[1], v[3])]
    dif = [g - e for g, e in zip(got, esperado)]
    ok = all(abs(x) <= 1 for x in dif)
    malo += 0 if ok else 1
    print(f'widget      llx,lly,urx,ury = {got}')
    print(f'diferencia  {dif}   {"COINCIDE" if ok else "NO COINCIDE"}')
sys.exit(1 if malo else 0)
