#!/usr/bin/env python3
"""Simula la fase 2 (la firma) del contrato trifasico.

Lee el XML del TriphaseData de la prefirma, toma el campo PRE (Base64 del DER
de los atributos firmados CAdES, ver docs/research/pades-triphase-contract.md),
lo firma con una clave RSA de prueba con SHA256withRSA -- equivalente a lo que
hara PKCS#11 con CKM_SHA256_RSA_PKCS sobre esos mismos bytes sin hashear, ver
docs/research/pkcs11-mecanismo-firma.md -- y deja el resultado en el campo PK1.

uso: inject-pk1.py <presign.xml> <key.pem> <salida.xml>
"""
import base64
import re
import subprocess
import sys
import tempfile

src, key, dst = sys.argv[1], sys.argv[2], sys.argv[3]
xml = open(src, encoding="utf-8").read()

m = re.search(r'<param n="PRE">([^<]*)</param>', xml)
if not m:
    sys.exit("no encuentro el campo PRE en " + src)
pre = base64.b64decode(m.group(1))

with tempfile.NamedTemporaryFile(suffix=".bin", delete=False) as f:
    f.write(pre)
    pre_path = f.name

# openssl dgst -sha256 -sign == RSA PKCS#1 v1.5 sobre SHA-256 de los bytes,
# que es exactamente lo que hace Signature.getInstance("SHA256withRSA").
pk1 = subprocess.run(
    ["openssl", "dgst", "-sha256", "-sign", key, pre_path],
    check=True, capture_output=True).stdout

pk1_b64 = base64.b64encode(pk1).decode("ascii")
print("PRE: %d bytes DER -> PK1: %d bytes de firma RSA" % (len(pre), len(pk1)))

if '<param n="PK1">' in xml:
    out = re.sub(r'<param n="PK1">[^<]*</param>',
                 '<param n="PK1">%s</param>' % pk1_b64, xml)
else:
    out = xml.replace('<param n="PRE">',
                      '<param n="PK1">%s</param>\n   <param n="PRE">' % pk1_b64, 1)

open(dst, "w", encoding="utf-8").write(out)
print("escrito " + dst)
