import sys
paginas = 3
def obj(n, cuerpo): return f"{n} 0 obj\n{cuerpo}\nendobj\n".encode("latin-1")
partes, offsets = [b"%PDF-1.4\n"], {}
def add(n, cuerpo):
    offsets[n] = sum(len(p) for p in partes); partes.append(obj(n, cuerpo))
kids = " ".join(f"{3+2*i} 0 R" for i in range(paginas))
add(1, "<< /Type /Catalog /Pages 2 0 R >>")
add(2, f"<< /Type /Pages /Kids [{kids}] /Count {paginas} >>")
for i in range(paginas):
    p, c = 3+2*i, 4+2*i
    add(p, f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {3+2*paginas} 0 R >> >> /Contents {c} 0 R >>")
    txt = f"BT /F1 24 Tf 72 750 Td (Pagina {i+1} de 3, sondeo rfirma 116) Tj ET"
    add(c, f"<< /Length {len(txt)} >>\nstream\n{txt}\nendstream")
add(3+2*paginas, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
xref_pos = sum(len(p) for p in partes); n = 4+2*paginas
xref = [f"xref\n0 {n}\n", "0000000000 65535 f \n"] + [f"{offsets.get(i,0):010d} 00000 n \n" for i in range(1, n)]
partes.append("".join(xref).encode("latin-1"))
partes.append(f"trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n".encode("latin-1"))
open(sys.argv[1], "wb").write(b"".join(partes))
