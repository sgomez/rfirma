#!/usr/bin/env bash
# Ejecuta el ciclo trifasico COMPLETO con rubrica de imagen sobre varias glibc
# y compara el PDF resultante, bit a bit, con el de referencia del anfitrion.
#
# Uso: run-cross-glibc.sh [dir-imagen]
#
# La prefirma y la postfirma corren DENTRO del entorno bajo prueba; la firma
# PK1 y la validacion (pdfsig/pdftoppm) se hacen fuera, en el anfitrion, para
# no exigir python3 ni poppler dentro de cada contenedor.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
SRC="$BRIDGE/target/${1:-ce25-awt}"
LAB="$BRIDGE/target/cross-glibc"
LIBS="librfirma_crypto.so libawt.so libawt_headless.so libjavajpeg.so libjava.so libjvm.so"

rm -rf "$LAB"; mkdir -p "$LAB"
gcc -O2 -o "$LAB/loader" "$BRIDGE/testbench/loader.c" -ldl

prepara() {                      # $1 = etiqueta
    d="$LAB/$1"; mkdir -p "$d"
    for l in $LIBS; do cp "$SRC/$l" "$d/"; done
    cp "$LAB/loader" "$d/"
    cp "$FIX/test.pdf.b64" "$FIX/cert.b64" "$FIX/visible-imagen.properties" "$d/"
}

# $1 = etiqueta, $2 = fase (presign|postsign), resto = prefijo de ejecucion
corre() {
    tag=$1; fase=$2; shift 2
    if [ "$fase" = presign ]; then
        args="presign test.pdf.b64 cert.b64 visible-imagen.properties"
    else
        args="postsign test.pdf.b64 cert.b64 signed.xml visible-imagen.properties"
    fi
    "$@" ./loader ./librfirma_crypto.so $args
}


prueba_docker() {                # $1 = etiqueta, $2 = imagen docker
    tag=$1; img=$2
    echo "### $tag"
    prepara "$tag"
    v=$(docker run --rm "$img" ldd --version 2>&1 | head -1)
    echo "glibc: $v"
    docker run --rm -u "$(id -u):$(id -g)" -v "$LAB/$tag:/w" -w /w "$img" \
        env -i PATH=/usr/bin:/bin HOME=/tmp \
        ./loader ./librfirma_crypto.so presign test.pdf.b64 cert.b64 visible-imagen.properties 2>&1 | tail -2
    [ -f "$LAB/$tag/presign.xml" ] || { echo "PREFIRMA FALLO"; return 1; }
    python3 "$BRIDGE/testbench/inject-pk1.py" "$LAB/$tag/presign.xml" \
        "$FIX/key.pem" "$LAB/$tag/signed.xml" >/dev/null
    docker run --rm -u "$(id -u):$(id -g)" -v "$LAB/$tag:/w" -w /w "$img" \
        env -i PATH=/usr/bin:/bin HOME=/tmp \
        ./loader ./librfirma_crypto.so postsign test.pdf.b64 cert.b64 signed.xml visible-imagen.properties 2>&1 | tail -2
    valida "$tag"
}

prueba_flatpak() {               # $1 = rama del runtime
    tag="gnome-$1"
    echo "### $tag"
    prepara "$tag"
    v=$(flatpak run --devel --command=sh "org.gnome.Platform//$1" -c 'ldd --version | head -1' 2>&1)
    echo "glibc: $v"
    flatpak run --devel --filesystem="$LAB/$tag" --command=sh "org.gnome.Platform//$1" \
        -c "cd '$LAB/$tag' && ./loader ./librfirma_crypto.so presign test.pdf.b64 cert.b64 visible-imagen.properties" 2>&1 | tail -2
    [ -f "$LAB/$tag/presign.xml" ] || { echo "PREFIRMA FALLO"; return 1; }
    python3 "$BRIDGE/testbench/inject-pk1.py" "$LAB/$tag/presign.xml" \
        "$FIX/key.pem" "$LAB/$tag/signed.xml" >/dev/null
    flatpak run --devel --filesystem="$LAB/$tag" --command=sh "org.gnome.Platform//$1" \
        -c "cd '$LAB/$tag' && ./loader ./librfirma_crypto.so postsign test.pdf.b64 cert.b64 signed.xml visible-imagen.properties" 2>&1 | tail -2
    valida "$tag"
}

prueba_anfitrion() {
    tag=anfitrion
    echo "### $tag (referencia)"
    prepara "$tag"
    echo "glibc: $(ldd --version | head -1)"
    (cd "$LAB/$tag" && env -i PATH=/usr/bin:/bin HOME=/tmp \
        ./loader ./librfirma_crypto.so presign test.pdf.b64 cert.b64 visible-imagen.properties) 2>&1 | tail -2
    python3 "$BRIDGE/testbench/inject-pk1.py" "$LAB/$tag/presign.xml" \
        "$FIX/key.pem" "$LAB/$tag/signed.xml" >/dev/null
    (cd "$LAB/$tag" && env -i PATH=/usr/bin:/bin HOME=/tmp \
        ./loader ./librfirma_crypto.so postsign test.pdf.b64 cert.b64 signed.xml visible-imagen.properties) 2>&1 | tail -2
    valida "$tag"
}

valida() {                       # $1 = etiqueta
    p="$LAB/$1/postsign.pdf"
    [ -f "$p" ] || { echo "POSTFIRMA FALLO (no hay PDF)"; echo; return 1; }
    echo -n "pdfsig: "; pdfsig "$p" 2>&1 | grep -oE "Signature is (Valid|Invalid)[^.]*" | head -1
    # rubrica visible: rasterizar la pagina 1 y comprobar que cambia respecto al PDF sin firmar
    pdftoppm -png -r 50 -f 1 -l 1 "$p" "$LAB/$1/pag1" 2>/dev/null
    echo "rasterizado: $(ls -la "$LAB/$1"/pag1*.png 2>/dev/null | awk '{print $5" bytes"}')"
    echo "sha256: $(sha256sum "$p" | cut -c1-16)  tam: $(stat -c%s "$p")"
    echo
}

prueba_anfitrion
prueba_docker ubuntu-24.04 ubuntu:24.04
prueba_flatpak 49
prueba_flatpak 50

echo "=== equivalencia bit a bit contra el anfitrion ==="
ref="$LAB/anfitrion/postsign.pdf"
for d in "$LAB"/*/; do
    t=$(basename "$d")
    [ "$t" = anfitrion ] && continue
    [ -f "$d/postsign.pdf" ] || { printf '%-16s %s\n' "$t" "SIN PDF"; continue; }
    if cmp -s "$ref" "$d/postsign.pdf"; then printf '%-16s %s\n' "$t" IDENTICO
    else printf '%-16s %s\n' "$t" DIFIERE; fi
done

# --- Segunda prueba: la MISMA sesion trifasica postfirmada en cada entorno ---
# Arriba cada entorno hace su propia prefirma, asi que el TIME difiere y los
# PDF no pueden salir iguales. Aqui se reutiliza el signed.xml del anfitrion:
# mismos extraParams y mismo TIME, que es la condicion que #13 y #14 fijaron.
echo
echo "=== misma sesion trifasica, postfirmada en cada entorno ==="
SES="$LAB/anfitrion/signed.xml"
mismo() {                        # $1 = etiqueta destino
    d="$LAB/mismo-$1"; rm -rf "$d"; mkdir -p "$d"
    for l in $LIBS; do cp "$SRC/$l" "$d/"; done
    cp "$LAB/loader" "$d/"
    cp "$FIX/test.pdf.b64" "$FIX/cert.b64" "$FIX/visible-imagen.properties" "$d/"
    cp "$SES" "$d/signed.xml"
}
POST="postsign test.pdf.b64 cert.b64 signed.xml visible-imagen.properties"

mismo anfitrion
(cd "$LAB/mismo-anfitrion" && env -i PATH=/usr/bin:/bin HOME=/tmp \
    ./loader ./librfirma_crypto.so $POST) >/dev/null 2>&1

mismo ubuntu-24.04
docker run --rm -u "$(id -u):$(id -g)" -v "$LAB/mismo-ubuntu-24.04:/w" -w /w ubuntu:24.04 \
    env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so $POST >/dev/null 2>&1

for b in 49 50; do
    mismo "gnome-$b"
    flatpak run --devel --filesystem="$LAB/mismo-gnome-$b" --command=sh \
        "org.gnome.Platform//$b" -c "cd '$LAB/mismo-gnome-$b' && ./loader ./librfirma_crypto.so $POST" >/dev/null 2>&1
done

ref2="$LAB/mismo-anfitrion/postsign.pdf"
for d in "$LAB"/mismo-*/; do
    t=$(basename "$d"); t=${t#mismo-}
    if [ ! -f "$d/postsign.pdf" ]; then printf '%-16s %s\n' "$t" "SIN PDF"; continue; fi
    v=$(pdfsig "$d/postsign.pdf" 2>&1 | grep -oE "Signature is (Valid|Invalid)" | head -1)
    if cmp -s "$ref2" "$d/postsign.pdf"; then r=IDENTICO; else r=DIFIERE; fi
    printf '%-16s %-10s %s\n' "$t" "$r" "$v"
done
