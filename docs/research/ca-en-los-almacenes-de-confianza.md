# La CA propia y los almacenes de confianza: medición

Sondeo para el issue [#225](https://github.com/sgomez/rfirma/issues/225), hijo del mapa
[#217](https://github.com/sgomez/rfirma/issues/217). El
[ADR-0005](../adr/0005-servidor-local-https-y-ca-en-el-almacen-del-sistema.md) decidió que el
servidor local use HTTPS con una CA propia **en el almacén del sistema**, «en lugar de manipular
las bases de datos `cert9.db` de cada perfil de Firefox». La decisión se tomó sin medir. Aquí se
mide.

Entorno de medición: **Ubuntu 26.04.1 LTS** (`resolute`), `ca-certificates` 20260601~26.04.1,
`p11-kit` 0.26.2-2, `libnss3` 2:3.120-1ubuntu2.1, **Google Chrome 152.0.7977.64**, **Mozilla
Firefox 155.0**, ambos paquetes nativos (ni snap ni flatpak). Oráculo de comportamiento:
AutoFirma en `/home/sergio/Developer/SideProjects/clienteafirma`, commit `0d7f3cf`, con su `.deb`
realmente instalado en esta máquina. Todas las consultas de documentación: **3 de septiembre de
2026**.

## Veredicto

**El ADR-0005 no se sostiene tal y como está escrito, y su frase central es falsa en
Debian/Ubuntu.** Lo medido, con un servidor HTTPS de prueba en `127.0.0.1` y un par CA/servidor
recién generado:

| Almacén donde está la CA | `curl` (OpenSSL) | Chrome 152 | Firefox 155 |
|---|---|---|---|
| Solo almacén del sistema (`/usr/local/share/ca-certificates` + `update-ca-certificates`) | **confía** | **no confía** | **no confía** |
| Solo `~/.pki/nssdb` | no confía | **confía** | no confía |
| Solo `cert9.db` del perfil de Firefox | no confía | no confía | **confía** |

Es decir: **la CA en el almacén del sistema no llega a ningún navegador en Ubuntu**, que es
justamente lo único para lo que el ADR-0005 la quería. Poner la CA ahí sirve para `curl`, `wget`,
GnuTLS y el runtime de Java; no sirve para el cliente real del protocolo, que es el navegador.

Tres hallazgos más, todos con consecuencias:

1. **Chrome 152 ya no usa `~/.pki/nssdb` en instalaciones nuevas.** Con un `HOME` limpio, Chrome
   creó `~/.local/share/pki/nssdb`. La ruta antigua se sigue usando **solo si ya existe**. La ruta
   que AutoFirma tiene cableada (`/.pki/nssdb`) dejará de funcionar para toda cuenta nueva.
2. **`security.enterprise_roots.enabled` no hace nada en Linux.** Medido: Firefox 155 con esa
   preferencia a `true` y la CA únicamente en el almacén del sistema **sigue sin confiar**.
3. **AutoFirma deja residuos en el almacén de confianza tras desinstalar.** En esta máquina hay
   una CA huérfana, `CN=AutoFirmaJA ROOT LOCAL`, válida hasta **2033**, marcada `CT,C,C` en
   `~/.pki/nssdb`, que ningún desinstalador retira porque el `uninstall.sh` borra por *nickname*
   y ese *nickname* ya no es el que usa la versión actual.

La enmienda propuesta está en el §6.

---

## 1. Debian y Ubuntu: `update-ca-certificates`

### 1.1 Cómo se hace

El manual `update-ca-certificates(8)` (paquete `ca-certificates`, Debian trixie 20250419) lo
resume: la orden lee `/etc/ca-certificates.conf` para decidir qué certificados son de confianza y
**acepta certificados en formato PEM con extensión `.crt` desde `/usr/local/share/ca-certificates`
tratándolos como implícitamente confiables**.

Genera dos cosas:

* `/etc/ssl/certs/ca-certificates.crt`, el *bundle* concatenado.
* enlaces con nombre-hash de OpenSSL en `/etc/ssl/certs/` (`6a0e091a.0 -> Autofirma_ROOT.pem`,
  comprobado en esta máquina).

Opciones relevantes: `-f`/`--fresh` («*Remove symlinks in /etc/ssl/certs directory*», es decir,
regenera desde cero y limpia enlaces obsoletos), `--localcertsdir`, `--certsdir`, `--certsconf`.

Hay un mecanismo de *hooks*: `/etc/ca-certificates/update.d/`. Cada ejecutable de ese directorio
se invoca con la lista de certificados añadidos (prefijo `+`) y retirados (`-`) por la entrada
estándar. En esta máquina hay uno, `jks-keystore`, del paquete `ca-certificates-java`, que
regenera el *keystore* JKS que usan los runtimes de Java.

Requiere **root**: escribe en `/usr/local/share/`, en `/etc/ca-certificates.conf` y reescribe
`/etc/ssl/certs/`.

### 1.2 Qué puede y qué no puede hacer un `.deb`

Un `postinst` puede invocar `update-ca-certificates` sin problema (basta declarar la dependencia
en `ca-certificates`). Lo que **no** puede es dejar el certificado en
`/usr/local/share/ca-certificates/`. La Debian Policy, capítulo 9, §9.1.2, es explícita:

> *As mandated by the FHS, packages must not place any files in /usr/local, either by putting them
> in the file system archive to be unpacked by dpkg or by manipulating them in their maintainer
> scripts.*

(Debian Policy Manual v4.7.4.1, `https://www.debian.org/doc/debian-policy/ch-opersys.html`.) La
única excepción admitida es crear directorios vacíos, no ficheros.

La vía correcta para un paquete es:

1. Instalar el PEM en `/usr/share/ca-certificates/<paquete>/<nombre>.crt` (esto sí es territorio
   de paquete).
2. Añadir la línea `<paquete>/<nombre>.crt` a `/etc/ca-certificates.conf`.
3. Ejecutar `update-ca-certificates` desde el `postinst`.
4. Deshacer 2 y 3 desde el `prerm`/`postrm`.

**AutoFirma incumple la Policy.** Su `postinst`
(`afirma-simple-installer/linux/instalador_deb/src/DEBIAN/postinst`) hace las dos cosas a la vez:
copia el certificado a `/usr/share/ca-certificates/Autofirma/` **y** a
`/usr/local/share/ca-certificates/`, y solo la segunda copia es la que acaba en el *bundle* —
comprobado en esta máquina, donde `/etc/ca-certificates.conf` sí tiene la línea
`Autofirma/Autofirma_ROOT.crt` y el enlace de `/etc/ssl/certs/` apunta a `/usr/local/…`. Es una
duplicación que además deja el certificado en una ruta que ningún paquete debería tocar.

### 1.3 Versiones

| Distribución | `ca-certificates` |
|---|---|
| Debian 12 (bookworm) | 20230311+deb12u1, con actualización 20250419~deb12u1 |
| Debian 13 (trixie) | 20250419 |
| Ubuntu 22.04 / 24.04 | 20260601~22.04.1 / 20260601~24.04.1 (canal de seguridad, 2026-09-03) |
| Ubuntu 26.04 | 20260601~26.04.1 (medido en esta máquina) |

---

## 2. Fedora: `update-ca-trust` y p11-kit

### 2.1 Cómo se hace

Dos directorios fuente, por prioridad:

* `/usr/share/pki/ca-trust-source/anchors/` — lo que aportan los paquetes.
* `/etc/pki/ca-trust/source/anchors/` — lo que aporta el administrador. Manda sobre el anterior.

Ambos tienen un `blocklist/` hermano para **desconfiar** explícitamente.

Después hay que ejecutar `update-ca-trust extract` (root), que regenera
`/etc/pki/ca-trust/extracted/{pem,openssl,java,edk2}`. El subcomando vacío (`update-ca-trust` a
secas) está pensado para *scriptlets* de RPM: el propio manual dice que «*may print fewer warnings
when being run during rpm package installation*». `update-ca-trust enable`/`disable` controla si
los ficheros clásicos (`/etc/pki/tls/cert.pem`, `ca-bundle.crt`) son enlaces a los extraídos.

**Fedora acepta más formatos que Debian**: PEM, DER, `BEGIN TRUSTED CERTIFICATE` (extensión de
OpenSSL que codifica *para qué* se confía) y `.p11-kit`. Eso permite **confianza limitada** —
por ejemplo restringir la CA a `serverAuth` — que el PEM plano de Debian no permite: allí es todo
o nada.

En un `.rpm`, el patrón documentado es incluir el `.pem` en `%files` bajo
`/etc/pki/ca-trust/source/anchors/` y llamar a `update-ca-trust extract` en el `%post`. RPM tiene
además *file triggers* (`%transfiletriggerin`, RPM 4.13 / RHEL 8) que permitirían al propio
`ca-certificates` disparar la extracción al detectar ficheros nuevos bajo ese prefijo; **no se ha
podido verificar contra el `.spec` de Fedora** si `ca-certificates` los usa (el servidor
`src.fedoraproject.org` sirve los *blobs* tras un anti-bot). Se anota como pendiente: se resuelve
con `rpm -q --scripts ca-certificates` en una máquina Fedora.

### 2.2 Lo que hace AutoFirma en Fedora

**Nada.** Su `.spec`
(`afirma-simple-installer/linux/instalador_rpm_fedora/rpmbuild/SPECS/autofirma.spec`) **no
menciona `/etc/pki/ca-trust` ni `update-ca-trust` en ningún punto**. El `%post` solo ejecuta el
configurador Java, registra `x-scheme-handler/afirma` en `mimeapps.list` y corre el `script.sh`
generado, que es puro `certutil` contra almacenes NSS. Es decir: en Fedora, AutoFirma se salta
por completo el almacén del sistema y va directo a los navegadores.

Y es la decisión correcta, aunque por accidente: es exactamente lo que funciona.

---

## 3. Los navegadores: aquí es donde se cae el ADR-0005

### 3.1 Método

Se generó una CA de usar y tirar (`CN=rfirma probe ROOT`, RSA 2048, 30 días) y un certificado de
servidor `CN=localhost` con `subjectAltName=DNS:localhost,IP:127.0.0.1`. Un servidor HTTPS en
`127.0.0.1:63118` sirve una página con un marcador. La confianza se comprueba mirando si el
navegador llega a renderizar el marcador (`--dump-dom` en Chrome; en Firefox, un `fetch` de vuelta
al propio servidor que este registra, porque Firefox no tiene volcado de DOM).

Para el caso «CA solo en el almacén del sistema» se aprovechó la CA de AutoFirma ya instalada en
esta máquina (`CN=Autofirma ROOT`, en `/usr/local/share/ca-certificates/`), sirviendo con su
propio certificado de servidor, y se lanzó Chrome con un `HOME` vacío para que su base NSS no
tuviese nada. Todas las mediciones tienen su control positivo.

### 3.2 Resultados

**Chrome 152, CA solo en el almacén del sistema, base NSS vacía:** *no confía*. Control positivo:
añadiendo la misma CA a la base NSS de ese `HOME`, *confía*. Conclusión: en Ubuntu,
`update-ca-certificates` es invisible para Chrome.

**Chrome 152, CA en `~/.pki/nssdb`:** *confía*. El comando clásico sigue vigente:

```sh
certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n "<nombre>" -i ca.crt
certutil -d sql:$HOME/.pki/nssdb -D -n "<nombre>"      # retirar
```

**Chrome 152 con `HOME` limpio crea `~/.local/share/pki/nssdb`, no `~/.pki/nssdb`.** Comprobado:
tras la primera ejecución en un `HOME` recién creado aparecen `cert9.db`, `key4.db` y `pkcs11.txt`
bajo `~/.local/share/pki/nssdb/`. La documentación de Chromium
(`docs/linux/cert_management.md`) lo confirma: desde **M146** la base compartida por defecto es
`$HOME/.local/share/pki/nssdb`, y solo se usa `$HOME/.pki/nssdb` **si ya existe**; no hay
migración automática. En esta máquina existía la antigua, y por eso Chrome la siguió usando —
de ahí que las dos rutas hayan dado resultados coherentes en las dos pruebas.

**Firefox 155, CA solo en el almacén del sistema, perfil nuevo:** *no confía*. **Y tampoco con
`security.enterprise_roots.enabled=true`**, puesta vía `user.js` en el perfil: se repitió la
prueba y el servidor no registró ni una sola visita. Control positivo: metiendo la CA en el
`cert9.db` de ese mismo perfil, *confía* a la primera. Coincide con la documentación de Mozilla,
que lista `ImportEnterpriseRoots` como soportada **solo en Windows y macOS**
(`firefox-admin-docs.mozilla.org/reference/policies/certificates/`) y con el bug 1600509 de
Bugzilla, abierto desde 2019 y sin resolver para Linux.

### 3.3 Por qué: p11-kit sí, NSS no

En Ubuntu el almacén del sistema **sí** está expuesto como módulo PKCS#11. `trust list` en esta
máquina muestra la CA de AutoFirma como `trust: anchor`, y `strings` sobre
`/usr/lib/x86_64-linux-gnu/pkcs11/p11-kit-trust.so` revela una única ruta compilada:
`/etc/ssl/certs/ca-certificates.crt`. Lo que no ocurre es que **NSS cargue ese módulo**:

* `~/.pki/nssdb/pkcs11.txt` en esta máquina declara **un solo módulo**, el interno de NSS. No hay
  `p11-kit-trust`.
* `/usr/lib/x86_64-linux-gnu/libnssckbi.so` es un fichero real de 615.712 bytes —el módulo de
  raíces empotradas de NSS—, **no un enlace** a `p11-kit-trust.so`.

En Fedora sí lo es: la *feature* «Shared System Certificates» (Fedora 19, 2013) sustituye
`libnssckbi.so` por `p11-kit-trust.so` mediante `update-alternatives`
(`fedoraproject.org/wiki/Features/SharedSystemCertificates:TipsAndKnownIssues`). Por eso en Fedora
`update-ca-trust` **sí** llega a Firefox y a Chromium del sistema, y en Debian/Ubuntu no.

En Debian esto se pidió en el **bug #704180**, abierto en marzo de 2013, discutido hasta enero de
2019, y **nunca implementado**. Sigue abierto. El manual de `update-ca-certificates(8)` no
menciona NSS ni p11-kit en ningún punto.

Comprobación adicional: p11-kit en Ubuntu **tampoco admite anclas de usuario**. Un certificado en
`~/.local/share/pki/trust/anchors/` no aparece en `trust list` ni lo ve nadie. La única ruta es la
compilada, y esa exige root.

### 3.4 La política de empresa no es una salida en Linux

* **Firefox** sí tiene una vía: `policies.json` con `Certificates.Install` (Firefox 65+ / ESR
  60.5+), que instala en el almacén NSS con *trust string* fija `CT,CT,`. En Linux se lee de
  `/etc/firefox/policies/policies.json` o de `<instalación>/distribution/policies.json`, y los
  certificados por nombre se buscan en `/usr/lib/mozilla/certificates`,
  `/usr/lib64/mozilla/certificates` y `~/.mozilla/certificates`. **Pero no funciona con Firefox
  como snap ni como flatpak**: el snap es de solo lectura y necesita la interfaz `system-files`
  para leer `/etc/firefox` (bug 1717216); el flatpak necesita que el administrador use la
  extensión `org.mozilla.firefox.systemconfig` (bug 1682462). En Ubuntu, Firefox **es snap por
  defecto** desde 22.04. Esta máquina es la excepción, no la regla.
* **Chrome**: las políticas `CACertificates` y `CACertificatesWithConstraints` (esta última desde
  Chrome 131) **no están soportadas en Linux**. La documentación de Chrome Enterprise lista
  Windows y macOS desde 133 y Android desde 138, y no lista Linux. En Linux la única vía sigue
  siendo `certutil` contra la base NSS del usuario.

Nota tranquilizadora: el **Chrome Root Store** (activo en Linux desde Chrome 114) **no** interfiere.
El FAQ de Chromium (`net/data/ssl/chrome_root_store/faq.md`) dice que el verificador sigue honrando
las decisiones de confianza locales, tanto para añadir como para quitar. Una CA local en NSS se
respeta.

---

## 4. Lo que hace AutoFirma, leído en su código

`ConfiguratorLinux.configure()` genera en la primera instalación un par CA + certificado de
servidor (RSA 2048, `SHA256withRSA`, `CN=Autofirma ROOT`, servidor `CN=127.0.0.1` con
`SAN = IP:127.0.0.1, DNS:127.0.0.1, DNS:localhost`), guarda el PKCS#12 en `autofirma.pfx` con la
contraseña **cableada en el código fuente, `654321`**, y **no instala nada**: escribe dos guiones,
`script.sh` y `uninstall.sh`, que ejecuta después el `postinst` del paquete.

`ConfiguratorFirefoxLinux` compone esos guiones recorriendo todos los `$HOME` obtenidos de
`/etc/passwd`:

* **Almacenes de Chrome/Chromium** (`createScriptsToSystemKeyStore`, nombre engañoso: de «sistema»
  no tiene nada): `~/.pki/nssdb` y `~/snap/chromium/current/.pki/nssdb`, con
  `certutil -A -t "TCP,TCP,TCP"`.
* **Perfiles de Firefox** (`createScriptsToMozillaKeyStore`): los declarados en
  `~/.mozilla/firefox/profiles.ini` y en
  `~/snap/firefox/common/.mozilla/firefox/profiles.ini`, con `certutil -A -t "C,,"`.

Ambos saltan los almacenes que aún no existen, para no crear con `certutil` una base que debería
crear el navegador.

Certificado de la CA en esta máquina: **10 años de validez** (2025-08-28 a 2035-08-26),
`CA:TRUE`, con un `keyUsage` que incluye `Digital Signature`, `Key Encipherment` y `Data
Encipherment` además de `keyCertSign`/`cRLSign`, **sin restricción de nombres**. Es una CA
generada por máquina, válida una década, de confianza total, sin `nameConstraints` que la limiten
a `localhost`. El ADR-0005 ya avisaba de esto en sus consecuencias; conviene que la enmienda lo
convierta en requisito y no en advertencia.

Resumen: **AutoFirma hace las dos cosas** —almacén del sistema *y* NSS por usuario— en Debian, y
**solo NSS** en Fedora. Lo que de verdad hace que el navegador confíe es lo segundo. Lo primero
es decorativo para el caso de uso del navegador.

---

## 5. Qué pasa al desinstalar

**Debian.** El `prerm` de AutoFirma mata Firefox (`pkill firefox`, sin preguntar), ejecuta
`uninstall.sh`, borra `/usr/share/ca-certificates/Autofirma/` y
`/usr/local/share/ca-certificates/Autofirma_ROOT.crt`, y llama a `update-ca-certificates`. Del
almacén del sistema, la limpieza es correcta —aunque lo canónico sería `--fresh`, que además
purga enlaces obsoletos de `/etc/ssl/certs/`.

**El problema está en NSS**, y es real. El `uninstall.sh` que hay instalado en esta máquina dice:

```sh
certutil -D -d sql:<HOME>/.pki/nssdb -n "SocketAutoFirma"
certutil -D -d sql:<HOME>/.mozilla/firefox/<perfil> -n "SocketAutoFirma"
```

Borra **por *nickname***, y solo por el *nickname* que usaba la versión que generó el guión. En
esta máquina, `~/.pki/nssdb` contiene hoy:

| Nickname | Emisor / sujeto | Confianza | Caduca |
|---|---|---|---|
| `SocketAutoFirma` | `CN=Autofirma ROOT` | `CT,C,C` | 2035-08-26 |
| `AutoFirmaJA ROOT LOCAL` | `CN=AutoFirmaJA ROOT LOCAL` | `CT,C,C` | 2033-07-03 |
| *(certificado personal del titular)* | — | `u,u,u` | — |
| `AC FNMT Usuarios - FNMT-RCM` | — | *(sin confianza)* | — |

La segunda entrada es **una CA de una instalación anterior de AutoFirma que ya no existe**, con
confianza total de emisión de certificados de servidor, y con siete años por delante. Ningún
desinstalador la va a tocar nunca, porque su *nickname* no coincide con el que busca el
`uninstall.sh` actual. Es exactamente el problema de seguridad que describe la pregunta 4 del
ticket, y está aquí, medido, en una máquina real.

Hay dos agravantes de diseño:

* El `uninstall.sh` se **genera en la instalación** y enumera los almacenes que existían
  **entonces**. Una reinstalación posterior mete la CA en almacenes nuevos, pero el `uninstall.sh`
  que se ejecutará al desinstalar puede ser otro, o no cubrirlos. La lista se desincroniza en
  cuanto pasa el tiempo.
* Nada retira la CA de un `$HOME` que no existía al instalar, ni de un usuario creado después.

**Fedora.** El `%preun` del RPM ejecuta el configurador con `-uninstall` y luego `uninstall.sh`.
Como en Fedora nunca se tocó `/etc/pki/ca-trust`, no hay nada que limpiar ahí; el residuo posible
es el mismo de NSS.

**Lo correcto** para rfirma: borrar por **huella del certificado**, no por *nickname*, y hacerlo
en el momento de la desinstalación recorriendo los almacenes de entonces, no los de la
instalación. `certutil -L -d <db>` permite enumerar y comparar.

---

## 6. Veredicto sobre el ADR-0005

### 6.1 Qué es falso

La frase «*el instalador registra la CA local en el almacén de CA del sistema **en lugar de**
manipular las bases de datos `cert9.db` de cada perfil de Firefox*» **es incorrecta en
Debian/Ubuntu**, que es la plataforma del hito v0.4. Y la consecuencia que de ella se deriva —
«*la confianza se establece para todos los navegadores y todos los perfiles de usuario a la vez,
y no se rompe al crear un perfil nuevo*»— es **exactamente lo contrario de lo que ocurre**:

* No se establece para ningún navegador.
* Se rompe al crear un perfil nuevo, porque la confianza vive en el perfil.
* Ni siquiera en Fedora, donde `update-ca-trust` sí llega a NSS, sería cierto para un Firefox
  instalado como flatpak o como snap, que trae su propio NSS.

Lo que sí sigue siendo cierto del ADR-0005: el puerto y el protocolo son contrato con las sedes;
la CA no puede distribuirse precompilada; su clave privada solo puede vivir en la máquina del
usuario.

### 6.2 La enmienda

Propuesta de reescritura del ADR-0005 (a decidir en un ticket del mapa #217, aquí solo se
argumenta):

1. **La confianza se instala en los almacenes NSS, uno por uno.** Es la única vía que funciona en
   Linux para los dos navegadores. Concretamente:
   * Chrome/Chromium: `certutil` contra `~/.local/share/pki/nssdb` **y** `~/.pki/nssdb` —la
     primera para Chrome ≥ M146, la segunda si ya existe—, saltando las que no estén inicializadas.
   * Firefox: `certutil` contra cada perfil con `cert9.db` de `profiles.ini`, y, cuando el paquete
     pueda, `policies.json` con `Certificates.Install` en `/etc/firefox/policies/`, que es la vía
     soportada y sobrevive a los perfiles nuevos.
2. **La CA en el almacén del sistema se conserva, pero degradada de «la decisión» a «un extra».**
   Sirve para `curl`, GnuTLS y Java, cuesta poco, y en Fedora (donde `libnssckbi.so` es
   `p11-kit-trust.so`) sí llega a los navegadores del sistema. En el `.deb` debe ir por la vía de
   la Policy (`/usr/share/ca-certificates/rfirma/` + `/etc/ca-certificates.conf`), **nunca** por
   `/usr/local`, que es lo que hace AutoFirma y está prohibido.
3. **La CA debe ir restringida.** Duración corta (meses, no diez años), `keyUsage` reducido a
   `keyCertSign`+`cRLSign`, y `nameConstraints` limitando la emisión a `localhost` y a
   `127.0.0.1`. En Fedora se puede además marcar confianza limitada con
   `BEGIN TRUSTED CERTIFICATE`; en Debian, no.
4. **La desinstalación se hace por huella y en el momento de desinstalar**, no por *nickname* con
   una lista congelada en la instalación. El residuo que se ha medido en esta máquina es la prueba
   de que el otro camino no funciona.
5. **Reconocer el hueco que no se cierra**: Firefox como snap o flatpak, y Chromium como snap,
   quedan fuera del alcance de un `.deb`/`.rpm`. Hay que decidir si eso se documenta, se detecta y
   se avisa, o se ofrece un asistente que lo haga desde la sesión del usuario.

### 6.3 ¿Hay un camino mejor?

Se ha medido la alternativa obvia —**HTTP plano a `127.0.0.1` desde una página HTTPS**— y la
respuesta honesta es **«no del todo, y menos de lo que parece»**.

Lo medido: una página servida por `https://localhost:63118` hace `fetch('http://127.0.0.1:63119/')`
con éxito, tanto en Chrome 152 como en Firefox 155. Es coherente con la especificación de W3C
*Secure Contexts*, que considera `127.0.0.0/8`, `::1/128` y `localhost` orígenes potencialmente
confiables **con independencia del esquema**, de modo que no cuentan como contenido mixto (en
Firefox, desde la 55 para la IP y desde la 84 para el nombre).

**Pero esa medición no es representativa del caso real**, y hay que decirlo: el origen iniciador
de la prueba era él mismo local. En producción el iniciador es una sede electrónica, un origen
público. Y ahí entra **Local Network Access** de Chrome, que sustituye a *Private Network Access*:
según la entrada de Chrome for Developers (`developer.chrome.com/blog/local-network-access`,
lanzamiento en **Chrome 142**, finales de octubre de 2025), toda conexión desde un origen público
hacia loopback o red privada —`fetch`, WebSocket, lo que sea— **dispara un aviso de permiso al
usuario**. LNA **incluye explícitamente `127.0.0.0/8` y `::1/128`**; no hay excepción para
loopback. Esa medición no se ha podido hacer aquí porque exigiría servir la página desde un origen
público real: **queda pendiente**, y es la que decide.

Resumiendo las tres opciones:

| Camino | Coste de instalación | Coste para el usuario al usarlo | Bidireccional |
|---|---|---|---|
| HTTPS local + CA propia | alto: `certutil` por almacén, root, residuos que limpiar | ninguno una vez instalado | sí |
| HTTP plano a loopback | ninguno | aviso de permiso LNA en Chrome ≥142 por cada origen | sí |
| `afirma://` a secas | bajo: `.desktop` con `MimeType=x-scheme-handler/afirma` | diálogo «¿abrir rfirma?» | **no**: sin canal de vuelta |

El esquema `afirma://` no es sustituto: el navegador lanza el proceso con el URI como argumento y
no hay retorno. Sirve para *arrancar* la conversación, que es justo para lo que AutoFirma lo usa,
pero la conversación necesita después el servidor local.

**Conclusión**: el HTTPS local con CA propia sigue siendo el diseño correcto, porque es el único
que da petición y respuesta sin fricción por uso. Lo que hay que corregir del ADR-0005 no es la
elección de HTTPS, sino **dónde se instala la confianza**. Antes de dar por cerrada la enmienda
conviene medir LNA con un origen público: si el aviso de Chrome resultase aceptable, el HTTP plano
ahorraría toda la maquinaria de CA, la de la desinstalación y el problema de los residuos, que no
es poco.

---

## 7. Lo que no se ha medido

* **Fedora.** No hay una máquina Fedora en este entorno; todo lo del §2 viene de documentación, no
  de medición. En particular queda por confirmar con `rpm -q --scripts ca-certificates` si el
  `ca-certificates` de Fedora tiene un `%transfiletriggerin` que extraiga solo.
* **Local Network Access desde un origen público.** Es la medición que decide el §6.3 y necesita
  servir la página desde fuera de loopback.
* **Firefox como snap y como flatpak, y Chromium como snap.** No hay ninguno instalado aquí. Es el
  caso mayoritario en Ubuntu de serie, así que conviene medirlo antes de fijar la enmienda.
* **El `preflight` de CORS de Firefox hacia loopback.** Hay bugs abiertos (1376310, 1440370,
  1535547) que describen bloqueos en peticiones con `preflight` que Chrome sí permite. La prueba
  de aquí usó una petición simple. Si el protocolo de rfirma envía JSON o cabeceras propias, habrá
  `preflight`.

## Fuentes

Documentación consultada el 3 de septiembre de 2026:

* `update-ca-certificates(8)`, Debian trixie — `manpages.debian.org/trixie/ca-certificates/update-ca-certificates.8.en.html`
* Debian Policy Manual v4.7.4.1, cap. 9 — `www.debian.org/doc/debian-policy/ch-opersys.html`
* Debian bug #704180, «p11-kit: provide package that diverts libnssckbi.so» — `bugs.debian.org/704180`
* `update-ca-trust(8)` — `www.mankier.com/8/update-ca-trust`
* `trust(1)` de p11-kit — `manpages.debian.org/testing/p11-kit/trust.1.en.html`
* Fedora, «Shared System Certificates» — `fedoraproject.org/wiki/Features/SharedSystemCertificates:TipsAndKnownIssues`
* RPM file triggers — `rpm.org/docs/latest/manual/file_triggers.html`
* Chromium, «Linux Certificate Management» — `chromium.googlesource.com/chromium/src/+/main/docs/linux/cert_management.md`
* Chrome Root Store FAQ — `chromium.googlesource.com/chromium/src/+/main/net/data/ssl/chrome_root_store/faq.md`
* Chrome Enterprise, `CACertificates` y `CACertificatesWithConstraints` — `chromeenterprise.google/policies/ca-certificates/` y `.../ca-certificates-with-constraints/`
* Chrome for Developers, «New permission prompt for Local Network Access» — `developer.chrome.com/blog/local-network-access`
* Firefox administrator reference, `Certificates` — `firefox-admin-docs.mozilla.org/reference/policies/certificates/`
* Bugzilla 1600509 (enterprise roots en Linux), 1717216 (políticas en el snap), 1682462 (políticas en el flatpak), 903966 y 1488740 (contenido mixto y loopback)
* W3C, *Secure Contexts* — `www.w3.org/TR/secure-contexts/`
* XDG MIME Applications — `wiki.archlinux.org/title/XDG_MIME_Applications`

Código leído de AutoFirma (commit `0d7f3cf`):
`afirma-ui-simple-configurator/src/main/java/es/gob/afirma/standalone/configurator/`
(`ConfiguratorLinux.java`, `ConfiguratorFirefoxLinux.java`, `CertUtil.java`) y
`afirma-simple-installer/linux/` (`instalador_deb/src/DEBIAN/{postinst,prerm,postrm}`,
`instalador_rpm_fedora/rpmbuild/SPECS/autofirma.spec`).
