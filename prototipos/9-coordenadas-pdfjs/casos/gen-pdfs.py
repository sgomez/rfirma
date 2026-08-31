#!/usr/bin/env python3
"""PROTOTIPO -- genera los PDFs de prueba del ticket #9.

Cada pagina lleva una rejilla de 50 pt y etiquetas con las coordenadas de
usuario PDF, para poder leer a ojo donde ha caido el recuadro de firma.
Los casos cubren lo que rompe la conversion ingenua: rotacion, tamanos que
no son A4 y MediaBox con origen distinto de (0,0).
"""
import os, zlib

CASOS = [
    ("a4",           (0, 0, 595, 842),    0),
    ("a4-rot90",     (0, 0, 595, 842),   90),
    ("a4-rot180",    (0, 0, 595, 842),  180),
    ("a4-rot270",    (0, 0, 595, 842),  270),
    ("a5",           (0, 0, 420, 595),    0),
    ("letter",       (0, 0, 612, 792),    0),
    ("offset",       (20, 30, 615, 872),  0),
    ("offset-rot90",  (20, 30, 615, 872),  90),
    ("offset-rot180", (20, 30, 615, 872), 180),
    ("offset-rot270", (20, 30, 615, 872), 270),
]


def contenido(box):
    x0, y0, x1, y1 = box
    o = ["0.85 0.85 0.85 RG 0.5 w"]
    x = x0 - (x0 % 50) + 50
    while x < x1:
        o.append(f"{x} {y0} m {x} {y1} l S")
        x += 50
    y = y0 - (y0 % 50) + 50
    while y < y1:
        o.append(f"{x0} {y} m {x1} {y} l S")
        y += 50
    o.append(f"0 0 0 RG 1 w {x0+1} {y0+1} {x1-x0-2} {y1-y0-2} re S")
    o.append("BT /F1 9 Tf 0.2 0.2 0.2 rg")
    for x in range(x0 - (x0 % 100) + 100, x1, 100):
        for y in range(y0 - (y0 % 100) + 100, y1, 100):
            o.append(f"1 0 0 1 {x+2} {y+2} Tm ({x},{y}) Tj")
    o.append("ET")
    o.append(f"1 0 0 rg {x0} {y0} 44 12 re f")
    o.append(f"BT /F1 8 Tf 1 1 1 rg 1 0 0 1 {x0+3} {y0+3} Tm (ORIGEN) Tj ET")
    return "\n".join(o).encode()


def pdf(*paginas):
    """Un objeto Font compartido, y por cada pagina su /Page y su /Contents."""
    n = len(paginas)
    kids = " ".join("%d 0 R" % (3 + 2 * i) for i in range(n))
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        ("<< /Type /Pages /Kids [%s] /Count %d >>" % (kids, n)).encode(),
    ]
    fuente = 3 + 2 * n
    for i, (box, rot) in enumerate(paginas):
        cs = zlib.compress(contenido(box))
        objs.append(("<< /Type /Page /Parent 2 0 R /MediaBox [%d %d %d %d] /Rotate %d "
                     "/Resources << /Font << /F1 %d 0 R >> >> /Contents %d 0 R >>"
                     % (box[0], box[1], box[2], box[3], rot, fuente, 4 + 2 * i)).encode())
        objs.append(b"<< /Length %d /Filter /FlateDecode >>\nstream\n" % len(cs)
                    + cs + b"\nendstream")
    objs.append(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
    out = bytearray(b"%PDF-1.4\n")
    offs = []
    for i, o in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    for off in offs:
        out += b"%010d 00000 n \n" % off
    out += (b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n"
            % (len(objs) + 1, xref))
    return bytes(out)


d = os.path.dirname(os.path.abspath(__file__))
for nombre, box, rot in CASOS:
    open(os.path.join(d, nombre + ".pdf"), "wb").write(pdf((box, rot)))
    print(f"{nombre:14s} MediaBox={box} /Rotate={rot}")

# Multipagina: tres paginas distintas en tamano y rotacion, para comprobar que
# signaturePage apunta a la que se ve y que la conversion usa SU MediaBox.
MIXTO = [((0, 0, 595, 842), 0), ((0, 0, 420, 595), 90), ((20, 30, 615, 872), 180)]
open(os.path.join(d, "mixto.pdf"), "wb").write(pdf(*MIXTO))
print("mixto          3 paginas: A4/0, A5/90, offset/180")
