#!/usr/bin/env python3
"""Compara dos rasterizados de la misma pagina y describe la region que cambia.

El #14 dejo escrito que `pdftotext` da un falso negativo con la rubrica: la
apariencia vive en el appearance stream del widget de firma, no en el
contenido de la pagina. Hay que rasterizar y mirar los pixeles.

Uso: diff-pagina.py <base.png> <firmado.png> [salida-diff.png]
"""
import sys
from PIL import Image, ImageChops

base = Image.open(sys.argv[1]).convert("RGB")
firm = Image.open(sys.argv[2]).convert("RGB")
if base.size != firm.size:
    print(f"TAMANOS DISTINTOS: {base.size} vs {firm.size}")
    sys.exit(1)

dif = ImageChops.difference(base, firm)
bbox = dif.getbbox()
distintos = sum(1 for p in dif.convert("L").getdata() if p > 8)
print(f"pixeles distintos: {distintos}   bbox: {bbox}   pagina: {base.size}")
if bbox:
    salida = sys.argv[3] if len(sys.argv) > 3 else "diff-rubrica.png"
    firm.crop(bbox).resize(
        ((bbox[2] - bbox[0]) * 3, (bbox[3] - bbox[1]) * 3), Image.NEAREST
    ).save(salida)
    print(f"recorte de la region distinta -> {salida}")
