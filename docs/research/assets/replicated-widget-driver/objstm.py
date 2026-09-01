"""Vuelca los objetos guardados en object streams (/Type /ObjStm) de un PDF,
inflandolos con zlib. iText con setFullCompression mete ahi paginas y widgets."""
import re
import sys
import zlib

data = open(sys.argv[1], "rb").read()
want = set(int(a) for a in sys.argv[2:]) if len(sys.argv) > 2 else None

for m in re.finditer(rb"(\d+) (\d+) obj\s*(<<.*?>>)\s*stream\r?\n", data, re.S):
    d = m.group(3)
    if b"/ObjStm" not in d:
        continue
    length = int(re.search(rb"/Length\s+(\d+)", d).group(1))
    n = int(re.search(rb"/N\s+(\d+)", d).group(1))
    first = int(re.search(rb"/First\s+(\d+)", d).group(1))
    raw = data[m.end():m.end() + length]
    body = zlib.decompress(raw)
    hdr = body[:first].split()
    pairs = [(int(hdr[2 * i]), int(hdr[2 * i + 1])) for i in range(n)]
    print(f"== ObjStm obj {m.group(1).decode()}: {n} objetos ==")
    for i, (num, off) in enumerate(pairs):
        end = first + pairs[i + 1][1] if i + 1 < n else len(body)
        obj = body[first + off:end].decode("latin-1").strip()
        if want is None or num in want:
            print(f"  obj {num}: {obj}")
