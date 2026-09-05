# `nameConstraints` sobre `iPAddress`: quién lo impone y cómo se comprueba

Sondeo del [#310](https://github.com/sgomez/rfirma/issues/310), hijo del mapa
[#308](https://github.com/sgomez/rfirma/issues/308). Resuelve la incógnita que deja abierta la
promesa del [ADR-0005](../adr/0005-servidor-local-https-y-ca-en-los-almacenes-nss.md): «la CA va
restringida: `keyUsage` reducido a `keyCertSign`+`cRLSign`, y `nameConstraints` limitando la
emisión a `localhost` y `127.0.0.1`». La mitad `localhost` es un `dNSName` y no tiene misterio;
la mitad `127.0.0.1` es un `iPAddress`, y de que **eso** se imponga de verdad depende que el
residuo de la CA local se pueda acotar **por forma** en vez de sólo **por fecha**.

**Respuesta corta: sí, se impone, y en los tres motores que importan.** NSS, `mozilla::pkix`
(Firefox) y el verificador integrado de Chrome comparan `iPAddress` con máscara según el
RFC 5280 §4.2.1.10, no sólo `dNSName`. Se impone además **estando la restricción en la raíz
importada por la persona usuaria**, que es exactamente el caso de rfirma. Y cuando se viola, el
fallo es **visible y no salteable**: ni Firefox ni Chrome ofrecen el botón de «continuar de todos
modos» que sí ofrecen para una CA desconocida.

Todo lo de abajo está **medido en este equipo**, no deducido: OpenSSL 3.5.5, NSS 3.120 con
`certutil`/`vfychain`, Firefox 155.0 y Google Chrome 152.0.7977.64, contra tres servidores TLS
reales en `127.0.0.1`.

Lo que **no** se ha medido y se dice como lo que es: nada del comportamiento en Windows ni en
macOS, ni el de Chrome con `CAPlatformIntegrationEnabled: false`, ni el de un almacén NSS con
contraseña maestra puesta.

---

## 1. El experimento, y por qué está construido así

El error de un experimento ingenuo es servir un certificado cuyo `iPAddress` está fuera de
ámbito y comprobar que el navegador lo rechaza: lo rechazaría igual **por no casar el nombre del
host**, y no se sabría cuál de las dos comprobaciones ha actuado. Así que las tres hojas de
prueba se diferencian **en una sola entrada de la SAN**, y el navegador entra siempre por
`https://localhost:<puerto>/`, que casa en las tres:

| Hoja | Sujeto | SAN | Qué se sale del ámbito |
| --- | --- | --- | --- |
| `ok` | `CN=localhost` | `DNS:localhost`, `IP:127.0.0.1` | nada (control positivo) |
| `outip` | `CN=localhost` | `DNS:localhost`, `IP:127.0.0.1`, **`IP:192.0.2.1`** | sólo un `iPAddress` |
| `outdns` | `CN=localhost` | `DNS:localhost`, `IP:127.0.0.1`, **`DNS:example.com`** | sólo un `dNSName` |

Si `ok` pasa y `outip` no, la restricción sobre IP se está imponiendo, y no puede ser otra cosa.
`outdns` es la referencia: el caso que el enunciado del ticket daba por descontado.

Hay además un **control negativo** imprescindible: una segunda CA idéntica **sin**
`nameConstraints`, que firma las mismas peticiones. Si esas hojas pasan, el rechazo de las
primeras es de la extensión y no de un defecto del montaje.

La CA se fabrica con OpenSSL. La sintaxis de IP en `nameConstraints` **exige dirección y máscara
en notación puntuada**, no CIDR — así lo dice `x509v3_config(5)`:

```ini
[v3_ca]
basicConstraints=critical,CA:TRUE,pathlen:0
keyUsage=critical,keyCertSign,cRLSign
nameConstraints=critical,permitted;DNS:localhost,permitted;IP:127.0.0.1/255.255.255.255
```

que `openssl x509 -text` devuelve como:

```
X509v3 Name Constraints: critical
    Permitted:
      DNS:localhost
      IP:127.0.0.1/255.255.255.255
```

---

## 2. NSS: sí, y con la restricción en la raíz importada

### Lo medido

La CA entra en un almacén `sql:` recién creado con confianza `CT,C,C` —o sea, es el **ancla de
confianza**, no un intermedio— y se valida con `vfychain -u 1` (uso «servidor SSL»), en las tres
variantes de motor que ofrece la herramienta: el validador clásico, `-p`
(`CERT_VerifyCertificate`) y `-pp` (`CERT_PKIXVerifyCert`, o sea libpkix).

| Hoja | `vfychain` | `vfychain -p` | `vfychain -pp` |
| --- | --- | --- | --- |
| `ok` | `Chain is good!` | `Chain is good!` | `Chain is good!` |
| `outip` | `Chain is bad!` | `Chain is bad!` | `Chain is bad!` |
| `outdns` | `Chain is bad!` | `Chain is bad!` | `Chain is bad!` |
| `IP:192.0.2.1` a secas | `Chain is bad!` | `Chain is bad!` | `Chain is bad!` |

Y el control negativo, las mismas peticiones firmadas por la CA sin la extensión:

```
openssl evilip   evilip-free.pem: OK
vfychain evilip  Chain is good!
openssl mixedip  mixedip-free.pem: OK
vfychain mixedip Chain is good!
```

**Los tres motores de NSS imponen la restricción sobre `iPAddress`, y la imponen aunque la
extensión viva en el ancla de confianza.** No hace falta una jerarquía de dos niveles.

### El código que lo hace

`lib/certdb/genname.c` tiene el caso explícito en el `switch` por tipo de `CERTGeneralName` de
`cert_CompareNameWithConstraints`, y delega en un comparador dedicado:

```c
case certIPAddress: /* type 8 */
    matched = compareIPaddrN2C(&name->name.other, &current->name.name.other);
    break;
```

`compareIPaddrN2C` es literalmente la semántica del RFC 5280: 4 octetos de dirección contra 8 de
restricción (dirección + máscara) en IPv4, 16 contra 32 en IPv6, comparados con
`(name ^ constraint) & mask`. Longitudes discordantes —una IPv4 contra una restricción IPv6— no
casan nunca. Conviene ver el contraste: los tipos que NSS **no** sabe comparar
(`certX400Address`, `certEDIPartyName`, `certRegisterID`) están resueltos con laxitud explícita
en ese mismo `switch`. `certIPAddress` **no** está en ese grupo.

La imposición sobre la cadena está en `lib/certhigh/certvfy.c`
(`cert_VerifyCertChainOld`), contra las restricciones **del emisor**, sea raíz o no:

```c
rv = CERT_CompareNameSpace(issuerCert, namesList, certsList, arena, &badCert);
if (rv != SECSuccess || badCert != NULL) {
    PORT_SetError(SEC_ERROR_CERT_NOT_IN_NAME_SPACE);
```

Y libpkix **no reimplementa nada**: `pkix_pl_nameconstraints.c` llama a `CERT_CheckNameSpace`,
que es el mismo `compareIPaddrN2C`. Por eso las tres columnas de la tabla dan lo mismo.

### Dos avisos que salen del propio experimento

**El código de error que imprime `vfychain` no es el bueno.** El rechazo por `nameConstraints` se
enseña así:

```
Chain is bad!
PROBLEM WITH THE CERT CHAIN:
CERT 1. localca [Certificate Authority]:
  ERROR -8157: Certificate extension not found.
```

−8157 es `SEC_ERROR_EXTENSION_NOT_FOUND` (`SEC_ERROR_BASE + 35`), **no**
`SEC_ERROR_CERT_NOT_IN_NAME_SPACE`, que es −8080 (`SEC_ERROR_BASE + 112`). Es el registro de
verificación recogiendo un error posterior al que de verdad tumbó la cadena; según cómo se invoque
la herramienta el número cambia (pasando también la CA por la línea de órdenes sale −8187,
`SEC_ERROR_INVALID_ARGS`). Es el mismo número para una violación de `DNS:` y para una de `IP:`.
**Diagnosticar por el número que imprime `vfychain` lleva a un sitio equivocado**; lo que vale es
el binario bueno/malo, y el código real hay que verlo en Firefox (§3) o en el `PORT_GetError` de
la propia biblioteca. `certutil -V` es aún peor: informa `SEC_ERROR_UNKNOWN_ISSUER`, porque al
descartar al emisor por la restricción el constructor de cadenas se queda sin ninguno.

**NSS y OpenSSL no tratan igual el CN.** Con la misma CA restringida:

| Hoja | `openssl verify` | `vfychain -u 1` |
| --- | --- | --- |
| `CN=example.com`, SAN `IP:127.0.0.1` | error 47, *permitted subtree violation* | `Chain is good!` |
| `CN=127.0.0.1`, SAN `IP:127.0.0.1` | error 47 | `Chain is good!` |
| `CN=example.com`, SAN `DNS:localhost` | `OK` | `Chain is good!` |
| `CN=localhost`, SAN `DNS:localhost`,`IP:127.0.0.1` | `OK` | `Chain is good!` |

OpenSSL contrasta el CN como nombre DNS cuando la hoja no trae ningún `DNS:` en la SAN; NSS, en
la medición, no lo hizo ni siquiera en ese caso. **Consecuencia práctica para rfirma, y es la
única que hay que recordar de este apartado: el certificado de servidor lleva `CN=localhost` y
*las dos* entradas en la SAN (`DNS:localhost` e `IP:127.0.0.1`).** Esa forma —la fila `ok`— pasa
en OpenSSL, en NSS, en Firefox y en Chrome. Poner `CN=127.0.0.1` la rompe en OpenSSL sin ganar
nada.

---

## 3. Firefox (`mozilla::pkix`): sí, y con interstitial no salteable

Perfil nuevo, la CA metida con `certutil -A -t CT,C,C` en su `cert9.db`, Firefox headless
conducido por Marionette contra los tres servidores:

| Servidor | Resultado |
| --- | --- |
| `ok` | la página carga; navegación sin error |
| `outip` | `insecure certificate`; interstitial de `about:certerror` |
| `outdns` | `insecure certificate`; interstitial de `about:certerror` |

Con `browser.xul.error_pages.expert_bad_cert` puesto, la página de error de `outip` dice
exactamente:

```
Error Code: SEC_ERROR_CERT_NOT_IN_NAME_SPACE

What makes the site look dangerous?
The Certifying Authority for this certificate is not permitted to issue a
certificate with this name.
```

y ofrece **un solo botón: «Go back (Recommended)»**. La comparación con un control —un
autofirmado desconocido en el mismo Firefox— es la que cierra el punto 4 del ticket:

| Caso | Código | Botón para continuar |
| --- | --- | --- |
| `outip` (IP fuera de ámbito) | `SEC_ERROR_CERT_NOT_IN_NAME_SPACE` | **no hay** |
| autofirmado desconocido | `MOZILLA_PKIX_ERROR_SELF_SIGNED_CERT` | «Proceed to localhost:63120 (Risky)» |

En el código, `lib/mozpkix/lib/pkixnames.cpp` despacha el tipo 7 a
`MatchPresentedIPAddressWithConstraint`, que valida longitudes 8/32, deja claro en su comentario
que «*an IPv4 address never matches an IPv6 constraint, and vice versa*» y aplica
`presented ^ constraintAddress & constraintMask`. `pkixnss.cpp` mapea
`ERROR_CERT_NOT_IN_NAME_SPACE` a `SEC_ERROR_CERT_NOT_IN_NAME_SPACE`, y ese código **no está** en
la lista de errores anulables de `CategorizeCertificateError()`
(`security/manager/ssl/SSLServerCertVerification.cpp`) — de ahí la ausencia del botón. Que la
restricción se aplique también a raíces de terceros importadas está en `pkixbuild.cpp`:
`PathBuildingStep::Check()` mira `potentialIssuer.GetNameConstraints()` para **cualquier**
candidato a emisor, anclas incluidas. Sin exención para raíces empresariales, y la medición lo
confirma.

---

## 4. Chrome: lee el almacén NSS, valida con lo suyo, y también lo impone

Chrome en Linux **lee** el almacén NSS de la persona usuaria —`TrustStoreNSS`, que en
`net/cert/internal/system_trust_store.cc` se compone con `TrustStoreChrome`— pero **no valida con
NSS**: la validación la hace `bssl::CertPathBuilder` dentro de `CertVerifyProcBuiltin`. Son dos
implementaciones distintas de la misma regla, así que había que medir las dos por separado. El
Chrome Root Store está activado por omisión en Linux desde Chrome 114.

Medido con `HOME` apuntando a un directorio desechable, la CA en `~/.pki/nssdb` y en
`~/.local/share/pki/nssdb` (Chrome 146 movió la ruta por omisión), Chrome 152 headless:

| Servidor | Resultado | Código | ¿Enlace para continuar? |
| --- | --- | --- | --- |
| `ok` | la página carga | — | — |
| `outip` | *Privacy error* | `ERR_CERT_INVALID` | **no** (`proceed-link` ausente del DOM) |
| `outdns` | *Privacy error* | `ERR_CERT_INVALID` | **no** |
| autofirmado desconocido | *Privacy error* | `ERR_CERT_AUTHORITY_INVALID` | sí |

Que `ok` cargue demuestra de paso que Chrome sí toma el ancla del `nssdb` del usuario; que
`outip` no cargue, que impone la restricción sobre IP.

En BoringSSL, `pki/name_constraints.cc` tiene `NameConstraints::IsPermittedIP()` con
`IPAddressMatchesWithNetmask`, e `IsPermittedCert()` recorre `subject_alt_names->ip_addresses`
emitiendo `kNotPermittedByNameConstraints`. La raíz también cuenta:
`pki/verify_certificate_chain.cc` aplica el RFC 5937 §3.2 y empuja las restricciones del ancla a
la lista.

**El código merece una nota, porque es contraintuitivo.** Existe un
`ERR_CERT_NAME_CONSTRAINT_VIOLATION` (−212) en `net/base/net_error_list.h`, y **no es** el que
sale aquí: ese está reservado a la lista negra histórica por hash de clave pública
(`HasNameConstraintsViolation` en `cert_verify_proc.cc`). La violación real de la extensión no
tiene mapeo explícito en `MapPathBuilderErrorsToCertStatus()` y cae en la red de seguridad
—«*If the path was invalid for a reason that was not explicitly checked above, set a general
error*»— que pone `CERT_STATUS_INVALID`, o sea `ERR_CERT_INVALID`. Y eso es lo que salva el
punto 4: `IsCertErrorFatal()` devuelve `true` para `ERR_CERT_INVALID` y `false` para
`ERR_CERT_NAME_CONSTRAINT_VIOLATION`. **El error que de verdad ocurre es el no salteable; el que
lleva el nombre bonito sería el salteable.** Buscar «name constraint violation» en los registros
de Chrome no encuentra nada: hay que buscar `ERR_CERT_INVALID`.

---

## 5. Cómo se comprueba en una grada, y en cuál

La prueba que vale es la que el ticket pide: fabricar una CA restringida y verificar que **emitir
para `example.com` se rechaza**. Se hace entera con `openssl` para fabricar y `certutil`/`vfychain`
para verificar, en un directorio temporal, **sin red, sin GraalVM y sin SoftHSM**.

Es **grada B** del [ADR-0014](../adr/0014-gradas-de-prueba-y-puerta-de-calidad.md), y sale gratis:
`libnss3-tools` —que trae `certutil`, `pk12util` y también `vfychain`— ya está instalado en el
carril rápido de `ci.yml` desde el #99, y `libnss3` ya viene en el *runner*. No hay ninguna
dependencia nueva que añadir, ni en CI ni en los tres canales de empaquetado. Cada ejecución tarda
segundos.

La forma de la prueba, para cuando se escriba:

1. Generar la CA con la `nameConstraints` **que produzca el código de rfirma**, no una escrita a
   mano en el fichero de prueba. Si no, se prueba el fichero de prueba.
2. Emitir dos hojas con esa CA: una dentro de ámbito y otra que difiera **en una sola entrada de
   la SAN**, fuera de ámbito. Sin el par, un fallo del montaje se lee como éxito.
3. Verificar las dos con `vfychain -u 1` sobre un `sql:` desechable con la CA en `CT,C,C`.
4. **Aserción sobre el resultado, no sobre el código de error.** El apartado 2 explica por qué:
   `vfychain` imprime −8157 o −8187 según cómo se le llame, y ninguno de los dos es
   `SEC_ERROR_CERT_NOT_IN_NAME_SPACE`. Una prueba que fije el número se rompe sin que nada esté
   mal.
5. Merece la pena una tercera hoja con **la IP fuera de ámbito y el DNS dentro**, que es la que
   distingue «se impone sobre `iPAddress`» de «se impone sobre `dNSName`». Es la única que
   protege la promesa del ADR-0005 de verdad.

Lo que **no** puede estar en una grada es la mitad de navegador de este informe: exige Firefox y
Chrome instalados y varios minutos por ejecución. Queda como medición fechada aquí.

---

## 6. Qué se lleva el ADR-0005

- La promesa de `nameConstraints` con `IP:127.0.0.1` **se sostiene**, y se sostiene en los dos
  navegadores, no en uno solo. El residuo de la CA local **sí queda acotado por forma**: una CA
  de rfirma que aparezca emitiendo para cualquier otro nombre no sirve para suplantar nada, ni en
  Firefox ni en Chrome.
- El acotamiento por forma **no sustituye a la caducidad de 90 días, la refuerza**. Cubren cosas
  distintas: la forma limita el daño de la CA mientras es válida; la fecha limita el tiempo. Una
  CA huérfana restringida sigue siendo una clave privada olvidada en el `$HOME` de alguien.
- Cuando algo va mal, **va mal en voz alta**: `SEC_ERROR_CERT_NOT_IN_NAME_SPACE` en Firefox y
  `ERR_CERT_INVALID` en Chrome, los dos con interstitial **sin botón de continuar**. Esto tiene
  una cara incómoda que conviene escribir: si rfirma emitiera alguna vez un certificado mal
  formado —una SAN de más, un `CN=127.0.0.1` sin `DNS:` — **la persona usuaria se queda sin salida
  manual**. No hay «acepto el riesgo». La forma del certificado de servidor no es un detalle de
  implementación: es la diferencia entre un fallo recuperable y uno que no lo es.
- La forma que hay que emitir, y la única que pasa en los cuatro verificadores medidos:
  `CN=localhost`, SAN `DNS:localhost` **e** `IP:127.0.0.1`.

---

## Fuentes

**NSS**

- [`lib/certdb/genname.c`](https://searchfox.org/nss/source/lib/certdb/genname.c) —
  `cert_CompareNameWithConstraints`, `compareIPaddrN2C`, `CERT_CheckNameSpace`,
  `CERT_GetConstrainedCertificateNames`.
- [`lib/certhigh/certvfy.c`](https://github.com/nss-dev/nss/blob/master/lib/certhigh/certvfy.c) —
  `cert_VerifyCertChainOld`, `CERT_CompareNameSpace`, `SEC_ERROR_CERT_NOT_IN_NAME_SPACE`.
- [`lib/libpkix/pkix/checker/pkix_nameconstraintschecker.c`](https://github.com/nss-dev/nss/blob/master/lib/libpkix/pkix/checker/pkix_nameconstraintschecker.c)
  y [`lib/libpkix/pkix_pl_nss/pki/pkix_pl_nameconstraints.c`](https://github.com/nss-dev/nss/blob/master/lib/libpkix/pkix_pl_nss/pki/pkix_pl_nameconstraints.c)
  — libpkix delega en `CERT_CheckNameSpace`.
- [`cmd/vfychain/vfychain.c`](https://github.com/nss-dev/nss/blob/master/cmd/vfychain/vfychain.c) —
  significado de `-a`, `-u`, `-p`, `-pp`.
- `secerr.h` de `libnss3-dev` 3.120 — `SEC_ERROR_BASE`, `SEC_ERROR_EXTENSION_NOT_FOUND` (+35),
  `SEC_ERROR_CERT_NOT_IN_NAME_SPACE` (+112).

**Firefox**

- [`lib/mozpkix/lib/pkixnames.cpp`](https://searchfox.org/mozilla-central/source/security/nss/lib/mozpkix/lib/pkixnames.cpp)
  — `CheckNameConstraints`, `MatchPresentedIPAddressWithConstraint`.
- [`lib/mozpkix/lib/pkixbuild.cpp`](https://github.com/nss-dev/nss/blob/master/lib/mozpkix/lib/pkixbuild.cpp)
  — `PathBuildingStep::Check()`, restricciones de cualquier emisor candidato.
- [`lib/mozpkix/lib/pkixnss.cpp`](https://github.com/nss-dev/nss/blob/master/lib/mozpkix/lib/pkixnss.cpp)
  — mapeo a `SEC_ERROR_CERT_NOT_IN_NAME_SPACE`.
- [`security/manager/ssl/SSLServerCertVerification.cpp`](https://searchfox.org/mozilla-central/source/security/manager/ssl/SSLServerCertVerification.cpp)
  — `CategorizeCertificateError()`, la lista de errores salteables.
- [`security/certverifier/NSSCertDBTrustDomain.cpp`](https://searchfox.org/mozilla-central/source/security/certverifier/NSSCertDBTrustDomain.cpp)
  — raíces de terceros por el mismo camino.

**Chrome**

- [`pki/name_constraints.cc`](https://boringssl.googlesource.com/boringssl/+/refs/heads/master/pki/name_constraints.cc)
  — `IsPermittedIP`, `IPAddressMatchesWithNetmask`, `IsPermittedCert`.
- [`pki/verify_certificate_chain.cc`](https://boringssl.googlesource.com/boringssl/+/refs/heads/master/pki/verify_certificate_chain.cc)
  — RFC 5937 §3.2, restricciones del ancla.
- [`net/cert/internal/system_trust_store.cc`](https://source.chromium.org/chromium/chromium/src/+/main:net/cert/internal/system_trust_store.cc)
  — `TrustStoreNSS` sólo como fuente de anclas.
- [`net/cert/cert_verify_proc_builtin.cc`](https://source.chromium.org/chromium/chromium/src/+/main:net/cert/cert_verify_proc_builtin.cc)
  y [`net/cert/cert_status_flags.cc`](https://source.chromium.org/chromium/chromium/src/+/main:net/cert/cert_status_flags.cc)
  — `CERT_STATUS_INVALID` → `ERR_CERT_INVALID`.
- [`components/security_interstitials/core/ssl_error_options_mask.cc`](https://source.chromium.org/chromium/chromium/src/+/main:components/security_interstitials/core/ssl_error_options_mask.cc)
  — `IsCertErrorFatal()`.
- [`docs/linux/cert_management.md`](https://source.chromium.org/chromium/chromium/src/+/main:docs/linux/cert_management.md)
  — rutas del `nssdb` y el traslado a `~/.local/share/pki/nssdb`.
- [Chrome Root Store FAQ](https://source.chromium.org/chromium/chromium/src/+/main:net/data/ssl/chrome_root_store/faq.md)
  — Linux por omisión desde Chrome 114.

**OpenSSL**

- [`x509v3_config(5)`](https://docs.openssl.org/master/man5/x509v3_config/) — sección *Name
  Constraints*: «*the `IP` form should consist of an IP addresses and subnet mask separated by a
  `/`*».

---

## Apéndice: la sesión, para repetirla

```sh
# --- la CA restringida ---
cat > ca.cnf <<'C'
[req]
distinguished_name=dn
x509_extensions=v3_ca
prompt=no
[dn]
CN=rfirma local CA
[v3_ca]
basicConstraints=critical,CA:TRUE,pathlen:0
keyUsage=critical,keyCertSign,cRLSign
subjectKeyIdentifier=hash
nameConstraints=critical,permitted;DNS:localhost,permitted;IP:127.0.0.1/255.255.255.255
C
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.pem \
  -days 90 -config ca.cnf -sha256

# --- las tres hojas que se diferencian en una entrada de la SAN ---
# ok      -> subjectAltName=DNS:localhost,IP:127.0.0.1
# outip   -> subjectAltName=DNS:localhost,IP:127.0.0.1,IP:192.0.2.1
# outdns  -> subjectAltName=DNS:localhost,IP:127.0.0.1,DNS:example.com
openssl req -newkey rsa:2048 -nodes -keyout outip.key -out outip.csr -subj /CN=localhost
openssl x509 -req -in outip.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out outip.pem -days 60 -sha256 -extfile outip.ext

# --- OpenSSL ---
openssl verify -CAfile ca.pem outip.pem      # error 47: permitted subtree violation

# --- NSS: la CA es el ancla ---
certutil -N -d sql:nssdb --empty-password
certutil -A -d sql:nssdb -n localca -t CT,C,C -i ca.pem
vfychain -d sql:nssdb     -u 1 -a outip.pem   # Chain is bad!
vfychain -d sql:nssdb -p  -u 1 -a outip.pem   # idem (CERT_VerifyCertificate)
vfychain -d sql:nssdb -pp -u 1 -a outip.pem   # idem (libpkix)

# --- tres servidores TLS de verdad ---
openssl s_server -accept 63117 -cert ok.pem     -key ok.key     -www -quiet &
openssl s_server -accept 63118 -cert outip.pem  -key outip.key  -www -quiet &
openssl s_server -accept 63119 -cert outdns.pem -key outdns.key -www -quiet &

# --- Firefox: perfil nuevo con la CA, y el panel experto desplegado ---
certutil -N -d sql:ffprofile --empty-password
certutil -A -d sql:ffprofile -n localca -t CT,C,C -i ca.pem
echo 'user_pref("browser.xul.error_pages.expert_bad_cert", true);' >> ffprofile/prefs.js
firefox --headless --profile ffprofile --screenshot ff.png https://localhost:63118/
# no escribe la captura: la carga falla. Para ver la pagina de error hace falta
# Marionette (MOZ_MARIONETTE_PORT=2828, WebDriver:Navigate + WebDriver:TakeScreenshot).

# --- Chrome: HOME desechable con la CA en las dos rutas de nssdb ---
certutil -A -d sql:"$H/.pki/nssdb"              -n localca -t C,, -i ca.pem
certutil -A -d sql:"$H/.local/share/pki/nssdb"  -n localca -t C,, -i ca.pem
HOME="$H" google-chrome --headless=new --user-data-dir="$H/profile" \
  --virtual-time-budget=8000 --dump-dom https://localhost:63118/ \
  | grep -oE 'ERR_CERT_[A-Z_]+'                 # ERR_CERT_INVALID
HOME="$H" google-chrome ... --dump-dom https://localhost:63118/ \
  | grep -c 'id="proceed-link"'                 # 0 -> no salteable
```
