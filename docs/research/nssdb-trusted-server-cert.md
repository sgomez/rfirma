# Un certificado de servidor autofirmado confiado en el `nssdb`: sí, pero con el bit `P` **y** el `C`

Sondeo del [#326](https://github.com/sgomez/rfirma/issues/326), que bloquea al
[#311](https://github.com/sgomez/rfirma/issues/311).

**Pregunta:** ¿respetan Firefox y Chrome un certificado de servidor autofirmado registrado
directamente como certificado de confianza en el almacén NSS, sin CA por encima?

**Respuesta corta: los dos lo respetan, y la opción (C) del #311 existe** — pero *no* con el
`certutil -t "P,,"` que el enunciado daba por supuesto. Ese bit basta en Chrome y **no basta en
Firefox**. El de Firefox, `C,,`, no sólo no basta en Chrome: lo rompe con el error **no
salteable**. La única cadena de bits que sirve en los dos es **`PC,,`**, y con ella los dos
navegadores cargan la página sin aviso.

| Bits (`certutil -t`) | NSS (`vfychain`) | Firefox 155 | Chrome 152 |
| --- | --- | --- | --- |
| `P,,` | *Chain is good!* | **rechaza** (`MOZILLA_PKIX_ERROR_SELF_SIGNED_CERT`) | **carga** |
| `C,,` | *Chain is bad!* | **carga** | **rechaza** (`ERR_CERT_INVALID`, sin salida manual) |
| `CT,C,C` | — | **carga** | **rechaza** (`ERR_CERT_INVALID`, sin salida manual) |
| **`PC,,`** | *Chain is good!* | **carga** | **carga** |
| `PCT,C,C` | — | — | **rechaza** (`ERR_CERT_INVALID`) |
| sin registrar | *Chain is bad!* | rechaza (salteable) | rechaza (salteable) |

Medido en este equipo con Firefox 155.0 y Google Chrome 152.0.7977.64 sobre servidores TLS de
verdad. El montaje es el que dejó el [#310](https://github.com/sgomez/rfirma/issues/310) en
[`nss-nameconstraints-ip.md`](nss-nameconstraints-ip.md); el apéndice de abajo recoge sólo lo
que cambia.

---

## 1. El experimento

Tres certificados **autofirmados de servidor**, los tres con la forma exacta que fijó el #310 —
`CN=localhost`, SAN con **las dos** entradas — y los tres emitidos y firmados por sí mismos, sin
autoridad por encima:

```ini
[v3_leaf]
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectKeyIdentifier=hash
subjectAltName=DNS:localhost,IP:127.0.0.1
```

| Pieza | Papel |
| --- | --- |
| `self` | el que se registra en el almacén |
| `self2` | **segundo** certificado, misma forma y **mismo sujeto**, para el solape de renovación |
| `rogue` | tercero idéntico que **nunca** se registra — control negativo |

Los tres tienen sujeto y emisor `CN=localhost`, así que en el `nssdb` conviven tres registros
homónimos: es exactamente la situación de una renovación con solape, no una simplificación.

A ellos se suma el **control positivo** que reproduce el diseño (A) del #311: una CA local con su
certificado de servidor firmado por ella. Cuatro servidores `openssl s_server` en cuatro puertos,
y el navegador entra siempre por `https://localhost:<puerto>/`, que casa con la SAN en los cuatro
casos. Que el control positivo cargue y el `rogue` no es lo que descarta que un resultado sea un
defecto del montaje.

---

## 2. Chrome: sí, y es el que fija la cadena de bits

Chrome es el que decidía el sondeo, porque lee el `nssdb` pero valida con su propio verificador.

`HOME` desechable, el certificado en las dos rutas (`~/.pki/nssdb` y `~/.local/share/pki/nssdb`,
que Chrome 146 añadió), Chrome headless:

| Servidor | `P,,` | `C,,` | `PC,,` |
| --- | --- | --- | --- |
| `self` (registrado) | **carga** | `ERR_CERT_INVALID`, **sin** `proceed-link` | **carga** |
| `self2` (registrado) | **carga** | `ERR_CERT_AUTHORITY_INVALID`, con `proceed-link` | **carga** |
| `rogue` (no registrado) | `ERR_CERT_AUTHORITY_INVALID`, con `proceed-link` | ídem | ídem |

Lo relevante no es sólo que `P,,` funcione: es que **`C,,` falla del peor modo posible**. El
error que sale es `ERR_CERT_INVALID`, y ése es el fatal —el mismo que el #310 midió para
`nameConstraints`—, así que la página **no ofrece enlace para continuar** y la persona se queda
sin salida manual. Poner el bit «equivocado» no degrada a un aviso salteable: cierra la puerta.

El motivo está en `net/cert/internal/trust_store_nss.cc`, que traduce los bits de NSS a la
confianza de BoringSSL:

```cpp
bool is_trusted_ca = (trust_flags & CERTDB_TRUSTED_CA) == CERTDB_TRUSTED_CA;
constexpr unsigned int kTrustedPeerBits = CERTDB_TERMINAL_RECORD | CERTDB_TRUSTED;
bool is_trusted_leaf = (trust_flags & kTrustedPeerBits) == kTrustedPeerBits;

if (is_trusted_ca && is_trusted_leaf) { return ...ForTrustAnchorOrLeaf()...; }
else if (is_trusted_ca)               { return ...ForTrustAnchor()...; }
else if (is_trusted_leaf)             { return ...ForTrustedLeaf(); }
return bssl::CertificateTrust::ForUnspecified();
```

`P` pone `CERTDB_TRUSTED | CERTDB_TERMINAL_RECORD`, o sea **`ForTrustedLeaf`**: Chrome acepta el
certificado *como hoja*, sin pedirle que emita nada, y por eso carga. `C` pone
`CERTDB_TRUSTED_CA`, o sea **`ForTrustAnchor`**: Chrome lo trata como raíz y entonces le exige ser
una CA — pero lleva `CA:FALSE` y no tiene `keyCertSign`, así que la construcción de la cadena
falla y cae en el mismo `CERT_STATUS_INVALID` de siempre. `PC` pone los dos y da
**`ForTrustAnchorOrLeaf`**, que es la que sirve.

Que `PCT,C,C` vuelva a fallar es el mismo mecanismo con otra ropa: no hay nada que ganar añadiendo
bits, y sí que perder.

---

## 3. Firefox: sí, pero exige `C`, que es justo el que Chrome rechaza

Perfil nuevo por caso, el certificado metido con `certutil` en su `cert9.db`, Firefox headless
conducido por Marionette:

| Bits | `self` | `self2` | `rogue` |
| --- | --- | --- | --- |
| `P,,` | **no carga** | no carga | no carga |
| `C,,` | **carga** | **carga** | no carga |
| `CT,C,C` | **carga** | — | — |
| `PC,,` | **carga** | **carga** | no carga |
| CA local `CT,C,C` (control) | — | — | su hoja **carga** |
| perfil vacío | no carga | no carga | no carga |

`mozilla::pkix` **no tiene la noción de hoja de confianza** que tiene Chrome. En
`security/certverifier/NSSCertDBTrustDomain.cpp`, `GetCertTrust` sólo asciende a raíz de confianza
por un camino:

```cpp
if (flags & CERTDB_TRUSTED_CA) {
  if (policy.IsAnyPolicy()) {
    trustLevel = TrustLevel::TrustAnchor;
```

y todo lo demás que no sea un registro terminal sin su bit acaba en `TrustLevel::InheritsTrust`.
Con `P,,` el certificado hereda confianza de un emisor que no existe, así que Firefox lo ve como
lo que es —un autofirmado desconocido— y da `MOZILLA_PKIX_ERROR_SELF_SIGNED_CERT`. Con `C,,` sí
es raíz, y a una raíz Firefox no le pide `CA:TRUE` para servirse a sí misma: carga.

**Firefox es aquí más permisivo que la propia biblioteca NSS**, que es al revés de lo que uno
esperaría. Ver el apartado 5.

---

## 4. Cómo falla cuando falla, y el solape

**Cuando la confianza no está** —perfil recién creado, caducado, alguien lo borró— los dos
navegadores fallan **con salida manual**, al contrario que el caso de `nameConstraints` del #310:

| | Código | ¿Se puede continuar a mano? |
| --- | --- | --- |
| Chrome 152 | `ERR_CERT_AUTHORITY_INVALID` | **sí**, `proceed-link` en el DOM |
| Firefox 155 | `MOZILLA_PKIX_ERROR_SELF_SIGNED_CERT` | **sí**, *«Proceed to localhost:NNNNN (Risky)»* |

En Firefox 155 la página de error es la nueva tarjeta de *felt privacy*: un elemento
`<net-error-card>` con **shadow DOM**, así que el código de error y el botón de continuar **no
aparecen en el HTML** que devuelve `GetPageSource`. Al abrirse sólo se ven *«Advanced»* y *«Go
back (Recommended)»*; el botón `exception-button` y el código sólo existen **después** de pulsar
*Advanced*. Quien mida esto sin atravesar el shadow DOM concluirá, equivocándose, que no hay
salida manual.

El único caso sin salida manual que apareció en todo el sondeo es el ya dicho: Chrome con `C,,`
sobre un certificado que no es CA, `ERR_CERT_INVALID`.

**El solape funciona.** Dos certificados de servidor de confianza, misma forma y **mismo sujeto
`CN=localhost`**, conviviendo en el mismo almacén con `PC,,`, y los dos servidores cargan sin
aviso — en Firefox y en Chrome, y en cualquier orden de inserción. No se estorban. La renovación
con solape que decidió el #311 no encuentra aquí ningún obstáculo nuevo.

---

## 5. `vfychain` tampoco es oráculo aquí

El #310 avisó de que el número que imprime `vfychain` no sirve para diagnosticar. Este sondeo
añade algo peor: **el veredicto binario tampoco vale**, porque la biblioteca NSS clásica y
`mozilla::pkix` no coinciden.

| Bits | `vfychain -u 1` | Firefox |
| --- | --- | --- |
| `P,,` | *Chain is good!* | rechaza |
| `C,,` | *Chain is bad!* | carga |

Y el número sigue mintiendo: el rechazo del certificado no registrado sale como

```
ERROR -8156: Issuer certificate is invalid.
```

−8156 es `SEC_ERROR_CA_CERT_INVALID` (`SEC_ERROR_BASE + 36`): un error **sobre una CA**, en un
montaje donde no hay ninguna CA. La línea se repite además una
vez por cada homónimo de confianza que haya en el almacén: con dos, sale dos veces; con tres,
tres. Es el constructor de cadenas probando cada certificado de mismo sujeto como posible emisor,
no tres fallos distintos.

**Para esta pregunta hay que medir en los binarios de los navegadores. `vfychain` responde por una
tercera implementación que no manda en ninguno de los dos.**

---

## 6. Qué se lleva el #311

1. **La opción (C) existe.** Un solo certificado de servidor autofirmado, registrado él mismo,
   sirve en Firefox y en Chrome. La elección (A) contra (C) sigue viva y es la del #311.
2. **La cadena de bits es `PC,,`, no `P,,`.** Si el ADR-0005 acaba recogiendo (C), esto es un
   dato de implementación que hay que escribir, porque cada navegador acepta un bit distinto y
   **ninguno de los dos por separado sirve**.
3. **Poner sólo `C` es peor que no poner nada** en Chrome: el error pasa de salteable a fatal.
4. **La comprobación de que la confianza quedó puesta no puede ser `vfychain`.** Un `PC,,` y un
   `P,,` le dan el mismo *Chain is good!*, y sólo uno de los dos funciona en Firefox. Lo que se
   verifica es la cadena de bits que devuelve `certutil -L`.
5. **El solape no añade problemas**: dos de estos certificados en el mismo almacén conviven.
6. **La renovación sigue siendo visible**, como ya suponía el #311: cada una escribe en el
   `nssdb` y arrastra las dos restricciones ya medidas (Chrome no relee en caliente, Firefox
   envenena su caché tras un fallo). Este sondeo no las cambia.

Este documento **no toca el ADR-0005 ni `CONTEXT.md`**: aporta el dato, la decisión es del #311.

---

## Fuentes

- `net/cert/internal/trust_store_nss.cc` (Chromium) — traducción de bits de NSS a
  `bssl::CertificateTrust`: `ForTrustedLeaf`, `ForTrustAnchor`, `ForTrustAnchorOrLeaf`.
  <https://chromium.googlesource.com/chromium/src/+/main/net/cert/internal/trust_store_nss.cc>
- `security/certverifier/NSSCertDBTrustDomain.cpp` (mozilla-central) — `GetCertTrust`: sólo
  `CERTDB_TRUSTED_CA` asciende a `TrustLevel::TrustAnchor`.
  <https://searchfox.org/mozilla-central/source/security/certverifier/NSSCertDBTrustDomain.cpp>
- [`nss-nameconstraints-ip.md`](nss-nameconstraints-ip.md) (#310) — el banco de pruebas, la forma
  exacta del certificado y el aviso sobre `vfychain`.
- Medición propia: Firefox 155.0 y Google Chrome 152.0.7977.64 sobre Linux, `certutil`/`vfychain`
  de `libnss3-tools`, servidores `openssl s_server`.

---

## Apéndice: lo que cambia respecto al banco del #310

```sh
# --- un solo certificado, autofirmado, de servidor (sin CA por encima) ---
cat > leaf.cnf <<'C'
[req]
distinguished_name=dn
x509_extensions=v3_leaf
prompt=no
[dn]
CN=localhost
[v3_leaf]
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectKeyIdentifier=hash
subjectAltName=DNS:localhost,IP:127.0.0.1
C
for n in self self2 rogue; do
  openssl req -x509 -newkey rsa:2048 -nodes -keyout $n.key -out $n.pem \
    -days 365 -config leaf.cnf -sha256
done

# --- registrarlo: la cadena que sirve en los dos navegadores ---
certutil -A -d sql:<almacen> -n selfsrv -t "PC,," -i self.pem
certutil -L -d sql:<almacen>          # verificar los bits; el codigo de salida no vale

# --- Chrome ---
HOME="$H" google-chrome --headless=new --user-data-dir="$H/p" --no-sandbox \
  --virtual-time-budget=8000 --dump-dom https://localhost:63127/ \
  | grep -c 'Ciphers supported'       # >0 -> la pagina del s_server llego

# --- Firefox: dos trampas del montaje ---
# 1) si ya hay un Firefox de la persona corriendo, la invocacion se le engancha
#    y mide su perfil, no el nuestro: hacen falta --no-remote y MOZ_NO_REMOTE=1.
# 2) MOZ_MARIONETTE_PORT no se respeto; el puerto se fija en el perfil:
echo 'user_pref("marionette.port", 3010);' >> <perfil>/prefs.js
# y hay que dar unos segundos de calentamiento antes de la primera navegacion:
# la primera tras arrancar falla de forma intermitente y falsea el resultado.

# --- Firefox: el codigo de error vive en el shadow DOM, tras pulsar Advanced ---
# WebDriver:ExecuteScript, dentro de la pagina:
#   const r = document.querySelector('net-error-card').shadowRoot;
#   r.getElementById('advanced-button').click();
#   // ...esperar, y entonces:
#   r.textContent.match(/(SEC_ERROR_[A-Z_]+|MOZILLA_PKIX_ERROR_[A-Z_]+)/);
#   !!r.querySelector('#exception-button');   // el boton de continuar
```
