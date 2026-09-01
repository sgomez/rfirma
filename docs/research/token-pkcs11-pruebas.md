# Entorno PKCS#11 de pruebas

Registro del entorno provisionado en el ticket
[Provisionar un token PKCS#11 de pruebas](https://github.com/sgomez/rfirma/issues/5).
Todo el material de clave que aparece aquí es **público por diseño**: lo publica
la propia FNMT con la contraseña incluida. **El certificado personal del titular
no interviene en ningún punto del proyecto.**

## Coordenadas del entorno

| Qué | Valor |
| --- | --- |
| Kit FNMT descomprimido | `~/.local/share/rfirma-test-certs` (155 ficheros, fuera del repositorio) |
| Origen del kit | `https://www.sede.fnmt.gob.es/documents/10445900/10649507/Certificados_pruebas_todas_CAs.rar` |
| Módulo PKCS#11 | `/usr/lib/softhsm/libsofthsm2.so` (paquete `softhsm2` 2.6, Ubuntu) |
| Configuración SoftHSM | `~/.config/softhsm2/softhsm2.conf` (ruta por defecto: **no hace falta `SOFTHSM2_CONF`**) |
| Almacén de tokens | `~/.local/share/softhsm/tokens` |
| Etiqueta del token | `rfirma-test` |
| PIN de usuario | `1234` |
| PIN de SO | `3737` |
| `CKA_LABEL` de clave y certificado | `FNMT-ACTIVO-99999999R` (ambos con `CKA_ID = 01`) |
| URI PKCS#11 de la clave | `pkcs11:token=rfirma-test;id=%01;object=FNMT-ACTIVO-99999999R;type=private` |

El número de slot **no es estable** (SoftHSM lo reasigna al inicializar el
token): direcciona siempre por etiqueta de token o por URI, nunca por índice.

Herramientas instaladas: `softhsm2`, `opensc` (`pkcs11-tool`), `gnutls-bin`
(`p11tool`).

## Cómo se reprodujo

`softhsm2-util --import` **no admite PKCS#12**, solo PKCS#8 (falla con
«Could not read the PKCS#8 file»). El `.p12` se parte con OpenSSL y los dos
objetos se escriben con `pkcs11-tool`. Los `.p12` de la FNMT usan cifrado
antiguo, así que OpenSSL 3 exige `-legacy`.

```bash
sudo apt-get install -y softhsm2 opensc gnutls-bin
mkdir -p ~/.local/share/softhsm/tokens ~/.config/softhsm2
cat > ~/.config/softhsm2/softhsm2.conf <<'CONF'
directories.tokendir = /home/sergio/.local/share/softhsm/tokens
objectstore.backend = file
log.level = ERROR
CONF

softhsm2-util --init-token --free --label rfirma-test --so-pin 3737 --pin 1234

P12=~/.local/share/rfirma-test-certs/"Claves RSA/AC FNMT Usuarios/Nuevos/Nuevo Perfil no SMIME/ACTIVO_EIDAS_CERTIFICADO_PRUEBAS___99999999R.p12"
openssl pkcs12 -in "$P12" -passin pass:1234 -nocerts -nodes -legacy \
  | openssl pkcs8 -topk8 -nocrypt -outform DER -out key.der
openssl pkcs12 -in "$P12" -passin pass:1234 -clcerts -nokeys -legacy \
  | openssl x509 -outform DER -out cert.der

M=/usr/lib/softhsm/libsofthsm2.so
pkcs11-tool --module $M --token-label rfirma-test --login --pin 1234 \
  --write-object key.der  --type privkey --id 01 --label FNMT-ACTIVO-99999999R
pkcs11-tool --module $M --token-label rfirma-test --login --pin 1234 \
  --write-object cert.der --type cert    --id 01 --label FNMT-ACTIVO-99999999R
```

## La orden que demuestra que se puede firmar

Con el mecanismo que fijó
[Qué firma exactamente PKCS#11 en Rust](https://github.com/sgomez/rfirma/issues/8),
`CKM_SHA256_RSA_PKCS`, que recibe los bytes **sin hashear**:

```bash
echo -n "datos de prueba rfirma" > data.bin
pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so --token-label rfirma-test \
  --login --pin 1234 --sign --mechanism SHA256-RSA-PKCS \
  --input-file data.bin --output-file sig.bin
# → 256 bytes (RSA 2048)

pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so --token-label rfirma-test \
  --read-object --type cert --id 01 --output-file cert.der
openssl x509 -inform DER -in cert.der -pubkey -noout > pub.pem
openssl dgst -sha256 -verify pub.pem -signature sig.bin data.bin
# → Verified OK
```

El token ofrece `RSA-PKCS`, `SHA256-RSA-PKCS` y `SHA256-RSA-PKCS-PSS`, así que
sirve también para contrastar el mecanismo descartado.

## Certificados del kit útiles como casos de prueba

Casos que de otro modo no podríamos fabricar, porque exigen una CA real.

| Certificado | Papel | Comprobado |
| --- | --- | --- |
| `Claves RSA/AC FNMT Usuarios/Nuevos/Nuevo Perfil no SMIME/ACTIVO_EIDAS_CERTIFICADO_PRUEBAS___99999999R.p12` (`1234`) | **Camino feliz.** RSA 2048, vigente hasta 2028-10-30, OCSP `good`. Es el que está en el token. | Cadena OK contra la raíz de Ubuntu; firma verificada |
| `.../REVOCADO_EIDAS_CERTIFICADO_PRUEBAS___99999999R.p12` (`1234`) | **Revocado de verdad**, no caducado: la firma se construye pero la validación debe rechazarla. | OCSP responde `revoked`, motivo `superseded`, desde 2024-10-30 |
| `.../Antiguo perfil SMIME/REVOCADO_EIDAS_CERTIFICADO_PRUEBAS_SMIME___99999999R.p12` (`1234`) | Perfil SMIME antiguo, también revocado. | — |
| `Caducados/PF_CADUCADO_EIDAS.p12` (`G5cp,fYC9gje`) | **Caducado** (2020-11-08): el rechazo debe ocurrir antes de pedir el PIN. | Abre con esa contraseña |
| `Caducados/PF_REVOCADO_EIDAS.p12` (`15MCJ8iy.3ps`) | Caducado **y** revocado. | Abre con esa contraseña |
| `Claves ECC/…AC Usuarios G2/Ciudadano/ESPECIMEN_UNO_…___99949991H.p12` (`1234`) | **Curva elíptica P-256.** Material para el problema `r‖s` crudo frente al `SEQUENCE` DER que espera CAdES. Vigente hasta 2029-09-04. | Cadena OK contra `AC_RAIZ_FNMT_RCM_G2` |
| `Claves ECC/…AC Usuarios G2/Ciudadano/ESPECIMENDIEZ_…___99949990V_revocado.p12` (`1234`) | ECC **revocado**. | — |
| `Claves RSA/AC FNMT Usuarios/Nuevos/Kit … Organismo Supervisor/*.p12` | Diez titulares distintos, para casos con nombres raros (apóstrofes, `Ñ`, `Ü`) en la rúbrica visible. | — |

El `Caducados/password.txt` del kit nombra un `PF_ACTIVO_EIDAS.p12` que no
existe: la contraseña que anuncia para él es la de `PF_CADUCADO_EIDAS.p12`.

## Trampa de la raíz G2

La rama ECC cuelga de `AC RAIZ FNMT-RCM G2`
(`02:07:86:53:9F:02:B6:23:CF:DF:32:5B:2E:4E:45:E0:D9:E3:F1:B8:E5:EC:96:00:EB:F0:AC:8F:BF:03:FB:E2`),
que **no está en el almacén de Ubuntu**. Solo están `AC_RAIZ_FNMT-RCM.pem`
(la que usa la rama RSA, huella idéntica a la del kit) y
`AC_RAIZ_FNMT-RCM_SERVIDORES_SEGUROS.pem`. Para validar cadenas ECC hay que
añadir la G2 a mano, y ese paso pertenece al caso de prueba, no al producto.

## Ampliación: el token lo monta un script (#49)

Las órdenes de arriba se ejecutaron a mano una vez. Desde el
[issue #49](https://github.com/sgomez/rfirma/issues/49) las hace
`testdata/softhsm/provision-token.sh`, que es idempotente y al que llama
`just token` —y, a través de `test-rust`, el propio `just check`—. Parte de
`testdata/fnmt/`, que es el subconjunto del kit versionado en el repositorio, no
de `~/.local/share/rfirma-test-certs`.

El token pasa a tener **cinco certificados y cinco claves**:

| `CKA_ID` | `CKA_LABEL` | qué tiene |
| --- | --- | --- |
| `01` | `FNMT-ACTIVO-99999999R` | clave privada + certificado |
| `02` | `FNMT-CADUCADO-99999999R` | clave privada + certificado (caducó en 2020) |
| `03` | `FNMT-REVOCADO-99999999R` | clave privada + certificado (revocado en 2024) |
| `04` | `FNMT-GEMELO-99999999R` | clave privada + certificado (el par activo) |
| `05` | `FNMT-GEMELO-99999999R` | clave privada + certificado (el par caducado) |

El caducado y el revocado **entraron al principio sin clave**, a propósito:
existían para que el listado tuviera que clasificarlos antes de pedir el PIN,
no para firmar con ellos. Desde el #100 tienen la suya —la tabla de arriba ya
lo refleja—; el porqué está en la ampliación de más abajo.

## Ampliación: los gemelos (#98)

Los dos `FNMT-GEMELO-99999999R` comparten `CKA_LABEL` y no comparten ni
`CKA_ID` ni par de claves. Reproducen lo que hay en un perfil de Firefox de
verdad, donde `certutil -K` enseña **dos claves privadas con la misma
etiqueta**: buscar la clave por etiqueta devuelve una de las dos
arbitrariamente y se firma con una clave que no es la del certificado elegido.
La firma sale y verifica —contra la otra clave pública—, así que el fallo no se
nota. Por eso `pkcs11::sign` empareja por `CKA_ID`, que es lo que hace el propio
NSS, y por eso la prueba de grada B no se conforma con que cada firma verifique:
comprueba además que **no** verifica contra el gemelo.

Como los dos comparten etiqueta, la idempotencia del script mira el `CKA_ID` y
no la etiqueta: buscarlos por ella daría el segundo por importado en cuanto
estuviera el primero.

## Ampliación: la clave del caducado y del revocado (#100)

Desde el [issue #100](https://github.com/sgomez/rfirma/issues/100) el listado
solo ofrece los certificados **firmables**: los que tienen una clave privada
emparejada por `CKA_ID` en el mismo token. Con eso, «solo el certificado»
dejó de ser un banco de pruebas útil y pasó a ser un artefacto que se comía a
sus propios sujetos: sin clave, el caducado y el revocado desaparecían del
listado y las pruebas de grada B que distinguen un certificado caducado de un
fallo del token, o uno en vigor de uno revocado, se quedaban sin nada que
clasificar.

Así que `provision-token.sh` importa también la clave privada del `02` y la
del `03` —dos líneas, y la idempotencia sigue mirando el `CKA_ID`, así que una
segunda pasada no reimporta nada—. Lo que da estado a un certificado es su
propio contenido (`notAfter`, la revocación), no que le falte la clave: el
banco de pruebas no pierde nada al dárselas, y gana que el filtro de firmables
no lo vacíe.

El único certificado **sin** clave privada del proyecto vive ahora en el perfil
NSS desechable de `tests/nss_store.rs`, que es donde hace falta para comprobar
qué pasa al pedirle una firma a algo que no puede firmar.

## Fuera

El certificado FNMT personal del titular. No se importa, no se exporta, no se
usa y no aparece en ningún fixture.
