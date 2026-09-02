"""PDF de 3 paginas A4 con tres campos de firma vacios escritos a mano.

  - "Firma1"          pagina 1, /Rect [72 600 300 700]   vacio Y CON /AP
  - "FirmaInvisible"  pagina 2, /Rect [0 0 0 0]          caso degenerado
  - "Firma2"          pagina 3, /Rect [200 100 450 180]  vacio y sin /AP

Ningun campo tiene /V: los tres estan vacios segun ISO 32000-1, 12.7.4.5.
"Firma1" lleva ademas una apariencia normal (/AP /N), que es lo que ponen
Acrobat y LibreOffice al crear un campo de firma en blanco: sirve para
demostrar que "tiene apariencia" no equivale a "esta firmado".
"""
import sys

paginas = 3
campos = {  # pagina (1-based) -> (nombre, rect)
    1: ("Firma1", "[72 600 300 700]"),
    2: ("FirmaInvisible", "[0 0 0 0]"),
    3: ("Firma2", "[200 100 450 180]"),
}

partes, offsets = [b"%PDF-1.6\n"], {}


def add(n, cuerpo):
    offsets[n] = sum(len(p) for p in partes)
    partes.append(f"{n} 0 obj\n{cuerpo}\nendobj\n".encode("latin-1"))


FONT = 3 + 2 * paginas
FIELD0 = FONT + 1  # un objeto campo/widget fusionado por pagina
AP = FIELD0 + paginas  # apariencia normal del primer campo

kids = " ".join(f"{3 + 2 * i} 0 R" for i in range(paginas))
fields = " ".join(f"{FIELD0 + i} 0 R" for i in range(paginas))
add(1, f"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{fields}] /SigFlags 3 >> >>")
add(2, f"<< /Type /Pages /Kids [{kids}] /Count {paginas} >>")
for i in range(paginas):
    p, c = 3 + 2 * i, 4 + 2 * i
    # segundo argumento opcional: /Rotate de la ultima pagina
    rot = f"/Rotate {sys.argv[2]} " if len(sys.argv) > 2 and i == paginas - 1 else ""
    add(p, f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] {rot}"
           f"/Resources << /Font << /F1 {FONT} 0 R >> >> /Contents {c} 0 R "
           f"/Annots [{FIELD0 + i} 0 R] >>")
    txt = f"BT /F1 18 Tf 72 780 Td (Pagina {i + 1} de {paginas} - sondeo rfirma 149) Tj ET"
    add(c, f"<< /Length {len(txt)} >>\nstream\n{txt}\nendstream")
add(FONT, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
for i in range(paginas):
    nombre, rect = campos[i + 1]
    ap = f"/AP << /N {AP} 0 R >> " if nombre == "Firma1" else ""
    # campo de formulario y widget fusionados en un solo objeto (ISO 32000-1, 12.5.6.19)
    add(FIELD0 + i, f"<< /Type /Annot /Subtype /Widget /FT /Sig /T ({nombre}) "
                    f"/Rect {rect} /F 4 {ap}/P {3 + 2 * i} 0 R >>")
ap_stream = "0.5 w 0 0 0 RG 1 1 226 98 re S"
add(AP, f"<< /Type /XObject /Subtype /Form /BBox [0 0 228 100] "
        f"/Resources << >> /Length {len(ap_stream)} >>\nstream\n{ap_stream}\nendstream")

xref_pos = sum(len(p) for p in partes)
n = AP + 1
xref = [f"xref\n0 {n}\n", "0000000000 65535 f \n"] + [
    f"{offsets.get(i, 0):010d} 00000 n \n" for i in range(1, n)]
partes.append("".join(xref).encode("latin-1"))
partes.append(f"trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n".encode("latin-1"))
open(sys.argv[1], "wb").write(b"".join(partes))
print("escrito", sys.argv[1])
