#!/usr/bin/env bash
# Regenera las fixtures del banco de pruebas en target/fixtures/, que los
# tickets #2, #12, #13 y #14 daban por existentes pero nunca dejaron escritas
# en ningun guion. Todo lo que produce es determinista salvo el par de claves
# (cert.pem/key.pem), que se genera una sola vez y se reutiliza si ya esta.
#
# Uso: make-fixtures.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIX="$ROOT/target/fixtures"
mkdir -p "$FIX"

# --- PDF de partida: 22 paginas, sin firmas previas ------------------------
python3 - "$FIX/test.pdf" <<'PY'
import sys, zlib
paginas = 22
def obj(n, cuerpo): return f"{n} 0 obj\n{cuerpo}\nendobj\n".encode("latin-1")
partes, offsets = [b"%PDF-1.4\n"], {}
def add(n, cuerpo):
    offsets[n] = sum(len(p) for p in partes)
    partes.append(obj(n, cuerpo))
kids = " ".join(f"{3+2*i} 0 R" for i in range(paginas))
add(1, "<< /Type /Catalog /Pages 2 0 R >>")
add(2, f"<< /Type /Pages /Kids [{kids}] /Count {paginas} >>")
for i in range(paginas):
    p, c = 3+2*i, 4+2*i
    add(p, f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] "
           f"/Resources << /Font << /F1 {3+2*paginas} 0 R >> >> /Contents {c} 0 R >>")
    txt = f"BT /F1 24 Tf 72 750 Td (Pagina {i+1} de prueba rfirma) Tj ET"
    add(c, f"<< /Length {len(txt)} >>\nstream\n{txt}\nendstream")
add(3+2*paginas, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
xref_pos = sum(len(p) for p in partes)
n = 4+2*paginas
xref = [f"xref\n0 {n}\n", "0000000000 65535 f \n"]
for i in range(1, n):
    xref.append(f"{offsets.get(i,0):010d} 00000 n \n")
partes.append("".join(xref).encode("latin-1"))
partes.append(f"trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n".encode("latin-1"))
open(sys.argv[1], "wb").write(b"".join(partes))
PY
base64 -w0 < "$FIX/test.pdf" > "$FIX/test.pdf.b64"

# --- Certificado autofirmado RSA 2048 --------------------------------------
if [ ! -f "$FIX/key.pem" ]; then
    openssl req -x509 -newkey rsa:2048 -keyout "$FIX/key.pem" -out "$FIX/cert.pem" \
        -days 3650 -nodes -subj "/CN=Prueba rfirma/O=rfirma/C=ES" 2>/dev/null
fi
openssl x509 -in "$FIX/cert.pem" -outform DER | base64 -w0 > "$FIX/cert.b64"

# --- Rubricas ---------------------------------------------------------------
python3 - "$FIX" <<'PY'
import sys, io
from PIL import Image, ImageCms
fix = sys.argv[1]

# Degradado de 40x40 con canal alfa (el de #14).
img = Image.new("RGBA", (40, 40))
for y in range(40):
    for x in range(40):
        img.putpixel((x, y), (255 - x*6, y*6, 128, 255))
img.save(f"{fix}/rubrica.png")
img.convert("RGB").save(f"{fix}/rubrica.jpg", "JPEG", quality=90)

# El mismo JPEG pero CON perfil ICC sRGB incrustado (segmento APP2).
perfil = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
img.convert("RGB").save(f"{fix}/rubrica-icc.jpg", "JPEG", quality=90, icc_profile=perfil)

# PNG con un cuadrante totalmente transparente, para comprobar de que color
# queda el aplanado (el ADR-0012 afirma que blanco).
alfa = Image.new("RGBA", (40, 40), (255, 0, 0, 255))
for y in range(20):
    for x in range(20):
        alfa.putpixel((x, y), (0, 255, 0, 0))
alfa.save(f"{fix}/alfa.png")
PY

# --- extraParams -------------------------------------------------------------
comun=$'signaturePage=1\nsignaturePositionOnPageLowerLeftX=100\nsignaturePositionOnPageLowerLeftY=100\nsignaturePositionOnPageUpperRightX=300\nsignaturePositionOnPageUpperRightY=180\n'
printf '%s' "$comun"                                     > "$FIX/visible-texto.properties"
printf 'layer2Text=Firmado por Prueba rfirma\n'         >> "$FIX/visible-texto.properties"

: > "$FIX/sin-rubrica.properties"

# JPEG producido por el crate `image` de Rust (issue #36, pregunta 4). Si no
# hay cargo, se salta y las demas fixtures siguen sirviendo.
RUBRICAS="imagen:rubrica.png jpeg:rubrica.jpg icc:rubrica-icc.jpg"
if command -v cargo >/dev/null; then
    (cd "$ROOT/rfirma-native-bridge/testbench/rubrica-rs" \
        && cargo run --release --quiet -- "$FIX/rubrica.png" "$FIX/rubrica-rust.jpg")
    RUBRICAS="$RUBRICAS rust:rubrica-rust.jpg"
fi

for par in $RUBRICAS; do
    etq="${par%%:*}"; fic="${par##*:}"
    printf '%s' "$comun"                                  > "$FIX/visible-$etq.properties"
    printf 'layer2Text=Firmado por Prueba rfirma\n'      >> "$FIX/visible-$etq.properties"
    printf 'signatureRubricImage=%s\n' "$(base64 -w0 < "$FIX/$fic")" \
                                                         >> "$FIX/visible-$etq.properties"
done

ls -la "$FIX"
