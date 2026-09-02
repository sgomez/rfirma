#!/usr/bin/env bash
# ¿Convive `signatureField` con `signaturePages=all` (widget replicado, #116)?
set -uo pipefail
cd "$(dirname "$0")"
J=/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce/bin/java
P12="$HOME/.local/share/rfirma-test-certs/Claves RSA/AC Sector Público/Empleado Público/SP_Empleado_publico_activo.p12"
CP="target/probe-1.jar:$(cat target/cp.txt)"

"$J" -cp "$CP" probe.Probe sign empty-fields.pdf "$P12" Firma2 signed-field-all.pdf \
    "signaturePages=all" 2>&1 | grep -v '^WARNING' | tail -1
node -e '
const P="/home/sergio/Developer/SideProjects/rfirma/rfirma-app/node_modules/pdfjs-dist/legacy/build/pdf.mjs";
(async()=>{const{getDocument}=await import(P);const fs=require("node:fs");
const doc=await getDocument({data:new Uint8Array(fs.readFileSync("signed-field-all.pdf"))}).promise;
for(let n=1;n<=doc.numPages;n++){for(const a of await (await doc.getPage(n)).getAnnotations()){
 if(a.fieldType!=="Sig")continue;
 console.log(`pagina ${n}: ${a.fieldName} id=${a.id} rect=${a.rect} flags=${a.annotationFlags}`);}}})()'
