"""Inspecciona sin dependencias los objetos de firma de un PDF de iText/AutoFirma.

Solo lee la revision incremental (todo lo que iText escribe va en texto claro:
/Annots de las paginas, el campo de firma, el /AcroForm). No usa xref, busca
"N 0 obj" con expresiones regulares, que basta para lo que se mide aqui.
"""
import re
import sys
import zlib

data = open(sys.argv[1], "rb").read()

objs = {}
for m in re.finditer(rb"(?<![0-9])(\d+) (\d+) obj\s*(.*?)\s*endobj", data, re.S):
    num = int(m.group(1))
    objs.setdefault(num, []).append(m.group(3))  # la ultima revision manda

def last(num):
    return objs[num][-1]

def head(body, n=400):
    """Cabecera del objeto sin el stream."""
    i = body.find(b"stream")
    s = body if i < 0 else body[:i]
    return s[:n].decode("latin-1")

# Paginas: objetos /Type /Page con /Annots
print("== Paginas con /Annots ==")
for num in sorted(objs):
    body = last(num)
    if re.search(rb"/Type\s*/Page(?![s])", body) and b"/Annots" in body:
        annots = re.search(rb"/Annots\s*\[(.*?)\]", body, re.S).group(1)
        print(f"  obj {num}: /Annots [{annots.decode().strip()}]")

# Widgets de firma
print("== Widgets /Subtype /Widget con /FT /Sig ==")
for num in sorted(objs):
    body = last(num)
    if b"/Widget" in body and b"/Sig" in body:
        rect = re.search(rb"/Rect\s*\[(.*?)\]", body).group(1).decode().strip()
        p = re.search(rb"/P\s+(\d+ \d+ R)", body)
        v = re.search(rb"/V\s+(\d+ \d+ R)", body)
        t = re.search(rb"/T\s*\((.*?)\)", body)
        ap = re.search(rb"/AP\s*<<(.*?)>>", body, re.S)
        print(f"  obj {num}: /T={t.group(1).decode() if t else None} /Rect=[{rect}] /P={p.group(1).decode() if p else None} "
              f"/V={v.group(1).decode() if v else None} /AP=<<{ap.group(1).decode().strip() if ap else None}>>")
        print("     ", head(body, 300).replace("\n", " "))

# AcroForm
print("== /AcroForm ==")
for num in sorted(objs):
    body = last(num)
    if b"/Fields" in body and b"/SigFlags" in body:
        print(f"  obj {num}: {head(body, 300)}")

# Objetos de firma /Type /Sig
print("== /Type /Sig ==")
for num in sorted(objs):
    body = last(num)
    if re.search(rb"/Type\s*/Sig(?![a-zA-Z])", body):
        br = re.search(rb"/ByteRange\s*\[(.*?)\]", body).group(1).decode().split()
        sf = re.search(rb"/SubFilter\s*/([A-Za-z0-9.]+)", body).group(1).decode()
        print(f"  obj {num}: /SubFilter={sf} /ByteRange={br}")
        print("     ", re.sub(rb"/Contents\s*<[0-9a-fA-F]+>", b"/Contents <...>", body)[:300].decode("latin-1").replace("\n", " "))

# Cuantas revisiones
print("== %%EOF ==", data.count(b"%%EOF"))
