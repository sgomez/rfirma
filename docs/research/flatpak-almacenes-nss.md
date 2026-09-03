# ¿Puede el flatpak escribir en los almacenes NSS del usuario?

Medición para el issue [#243](https://github.com/sgomez/rfirma/issues/243). Sale de la reserva del
§9 de la resolución del [#238](https://github.com/sgomez/rfirma/issues/238), que decidió que **la
confianza de la CA local la instala la aplicación en la sesión de la persona**, escribiendo en los
almacenes NSS de su `$HOME` —el `cert9.db` de cada perfil declarado en `profiles.ini`, y
`~/.pki/nssdb` / `~/.local/share/pki/nssdb` para Chrome— y dejó sin medir si el flatpak puede
hacerlo. Si no puede, no sirve para la v0.5 y eso toca al
[ADR-0004](../adr/0004-libreria-nativa-distribuida-en-el-paquete.md) y al
[ADR-0005](../adr/0005-servidor-local-https-y-ca-en-el-almacen-del-sistema.md).

Todo lo de aquí está medido **dentro del bundle ya instalado**, con sus permisos reales, por el
recipiente que documenta `CLAUDE.md`:

```bash
flatpak run --command=python3 me.sgomez.rfirma - < sonda.py
```

Entorno: Ubuntu 26.04, flatpak 1.16.6, `me.sgomez.rfirma` 0.1.0 instalado en modo `user`
(commit `a4e013ef…`), runtime `org.gnome.Platform//50` y `org.gnome.Sdk//50` con **NSS 3.101.4**.
Firefox del sistema (`/usr/lib/firefox`, no snap), un solo perfil `plyif2tp.default`.
`certutil` del anfitrión es de `libnss3-tools`.

Permisos vigentes del bundle instalado, que son los del manifiesto
(`flatpak info --show-permissions me.sgomez.rfirma`):

```
[Context]
shared=ipc;
sockets=x11;wayland;fallback-x11;pcsc;
devices=dri;
filesystems=xdg-documents;~/.mozilla/firefox:ro;~/.pki/nssdb:ro;
```

**Ninguna medición ha tocado los almacenes NSS reales de la persona.** Lo que se escribe se escribe
en almacenes de usar y tirar creados para la ocasión y borrados al terminar; las pruebas de permiso
ampliado usan `flatpak run --filesystem=…`, que vale **para una sola ejecución y no persiste**
(no se ha usado `flatpak override` en ningún punto). Al final se verifica desde el anfitrión que
`~/.pki/nssdb` conserva sus tres ficheros con la misma marca de tiempo.

## Veredicto

**Sí puede, y sin salir del sandbox ni empaquetar nada nuevo.** El flatpak escribió un `cert9.db`
real con la confianza `CT,C,C` puesta, usando **sólo la NSS que ya trae el runtime**, y `certutil`
del anfitrión lo confirmó desde fuera. La v0.5 no queda descartada por esta vía: «la v0.4 es la
puerta» **no** recupera su segunda pata por aquí.

Lo que cuesta es un permiso, y es un permiso concreto y acotable:

1. **Hoy no.** Las dos rutas declaradas están montadas `:ro` de verdad —`EROFS` al escribir, no un
   fallo silencioso— y `~/.local/share/pki/nssdb` **ni siquiera existe** dentro. Peor: escribir en
   una ruta no declarada **contesta OK y no llega a ninguna parte**, que es la trampa medida en el
   [#27](https://github.com/sgomez/rfirma/issues/27) reproducida sobre los almacenes.
2. **Hay granularidad, pero no toda la que uno querría.** `~/.pki/nssdb` y
   `~/.local/share/pki/nssdb:create` son permisos exactos y baratos. Firefox no: los nombres de
   perfil son aleatorios, así que hay que conceder `~/.mozilla/firefox` **entero y en escritura**.
   El `:ro` de hoy ya deja leer `logins.json`, `key4.db` y `cookies.sqlite`; lo que se añade es
   poder escribirlos. Sigue siendo mucho menos que `--filesystem=home`.
3. **`certutil` no hace falta.** No está en el Platform, pero **la biblioteca NSS entera sí**, con
   todos los símbolos del trabajo. Y `certutil` sí está en el **SDK**, así que copiarlo a `/app/bin`
   cuesta 215.976 bytes y ninguna dependencia nueva — medido funcionando dentro del sandbox. La
   opción preferible es la FFI: **no hay ninguna crate viva que sirva**, pero son siete símbolos.
4. **Los navegadores confinados no se detectan** con los permisos de hoy, ni por `~/snap` ni por
   `~/.var/app` ni por `/run/host`. Detectarlos exige un `:ro` por ruta, que es barato y suficiente.
5. **No hay vía por portal.** No existe ningún portal de certificados ni de confianza, en ninguna
   versión. `flatpak-spawn --host` sí funciona, pero exige `--talk-name=org.freedesktop.Flatpak`,
   que es **salir del sandbox entero**: medido, con ese permiso el flatpak ejecuta órdenes
   arbitrarias en el anfitrión con los privilegios de la persona.

---

## 1. Qué ve el sandbox hoy

Sonda sobre las rutas del #238, con los permisos vigentes y nada más:

```
HOME = /home/sergio
uid  = 1000
--- ~/.mozilla -> /home/sergio/.mozilla
   exists: True | isdir: True
   R_OK: True W_OK: True X_OK: True
   listdir: ['firefox']
--- ~/.mozilla/firefox
   exists: True | R_OK: True W_OK: False X_OK: True
   listdir: ['Crash Reports', 'Pending Pings', 'Profile Groups', 'firefox-mpris',
             'firefox-themes', 'installs.ini', 'plyif2tp.default', 'profiles.ini']
--- ~/.mozilla/firefox/profiles.ini
   exists: True | R_OK: True W_OK: False
--- ~/.pki/nssdb
   exists: True | R_OK: True W_OK: False X_OK: True
   listdir: ['cert9.db', 'key4.db', 'pkcs11.txt']
--- ~/.local/share/pki        -> exists: False
--- ~/.local/share/pki/nssdb  -> exists: False
--- ~/snap/firefox/common/.mozilla -> exists: False
--- ~/.var/app/org.mozilla.firefox/.mozilla -> exists: False
--- ~/.config -> exists: False
--- ~/ -> listdir: ['.local', '.mozilla', '.pki', '.var', 'Documents']
```

Tres cosas que conviene no confundir:

* **`~/.mozilla` aparece escribible y no sirve de nada.** Es un directorio sintetizado por el
  sandbox para poder colgar de él el montaje de `firefox`; en el anfitrión ese mismo directorio
  tiene cuatro entradas (`extensions`, `firefox`, `native-messaging-hosts`,
  `systemextensionsdev`) y dentro se ve **una**. Un `mkdir ~/.mozilla/otrodir` **contesta OK** y no
  aparece en el anfitrión. Lo mismo pasa con `~/.local/share`, donde sólo se ve `flatpak`.
* **Los dos almacenes declarados están en un sistema de ficheros de sólo lectura de verdad.**
  `statvfs` da `ST_RDONLY=True` y cualquier escritura da `EROFS` (30), no `EACCES`:

  ```
  crear ~/.pki/nssdb/rfirma-probe:        OSError errno=30 (EROFS) Read-only file system
  abrir cert9.db en r+b:                  OSError errno=30 (EROFS) Read-only file system
  crear ~/.mozilla/firefox/rfirma-probe:  OSError errno=30 (EROFS) Read-only file system
  ```

  Y la lectura funciona entera, incluidos los ficheros que no son certificados:

  ```
  leer ~/.pki/nssdb/cert9.db:                          OK 45056
  leer …/plyif2tp.default/cert9.db:                    OK 720896
  leer …/plyif2tp.default/key4.db:                     OK 294912
  leer …/plyif2tp.default/cookies.sqlite:              OK 2097152
  leer …/plyif2tp.default/logins.json:                 OK 258521
  ```

* **La trampa del #27, reproducida sobre los almacenes.** En la ruta *no declarada* de Chrome, todo
  contesta que sí:

  ```
  mkdir ~/.local/share/pki:                  OK
  mkdir -p ~/.local/share/pki/nssdb:         OK
  listdir ~/.local/share/pki:                OK ['nssdb']
  escribir ~/.local/share/pki/nssdb/x:       OK 1
  ```

  y en el anfitrión `~/.local/share/pki/` sigue vacío. Una implementación que creara el almacén de
  Chrome cuando no existe **creería haberlo hecho** y no habría hecho nada. El remedio está en el
  apartado 2 (`:create`), no en el código.

## 2. Qué hay que añadir al manifiesto, y qué cuesta

Cada línea, probada con `flatpak run --filesystem=…` (una sola ejecución, sin persistir):

| Permiso de una ejecución | `~/.pki/nssdb` | `~/.mozilla/firefox` | perfil | `~/.local/share/pki/nssdb` |
|---|---|---|---|---|
| *(ninguno: los de hoy)* | ro | ro | ro | no existe |
| `--filesystem=~/.pki/nssdb` | **rw** | ro | ro | no existe |
| `--filesystem=~/.local/share/pki/nssdb:create` | ro | ro | ro | **rw, creado** |
| `--filesystem=~/.mozilla/firefox` | ro | **rw** | **rw** | no existe |

(«ro» = `os.access(W_OK)` falso y `statvfs` con `ST_RDONLY`; «rw» = lo contrario.)

**Chrome sale barato.** Las dos rutas del #238 son exactas y estables, y `:create` resuelve además
la trampa del apartado anterior: con `--filesystem=~/.local/share/pki/nssdb:create` el directorio
**aparece creado en el anfitrión** (comprobado; se borró después). Es justo lo que documenta
flatpak para ese sufijo: «read/write access, and create the directory if it doesn't exist»
([docs.flatpak.org, *Sandbox Permissions*](https://docs.flatpak.org/en/latest/sandbox-permissions.html)).

**Firefox no tiene granularidad menor que el directorio del almacén.** Dos medidas:

* **Los comodines se aceptan en silencio y no conceden nada.** Con
  `--filesystem=~/.mozilla/firefox/*/cert9.db` el flatpak arranca sin quejarse y ni el perfil ni el
  fichero quedan escribibles. Es una trampa: parece un permiso estrecho y es un permiso nulo.
* **Un permiso a fichero suelto tampoco basta**, porque NSS necesita escribir en el *directorio*.
  Medido contra un almacén de usar y tirar en `~/.local/share/rfirma-nss-probe`, concediendo el
  directorio `:ro` y sólo `cert9.db` en escritura:

  ```
  dir NO escribible
  cert9.db escribible
  certutil: function failed: SEC_ERROR_READ_ONLY: security library: read-only database.
  rc=255
  ```

  y con el directorio entero en escritura, la misma orden:

  ```
  dir escribible
  cert9.db escribible
  rc=0
  solo-fichero      CT,C,C
  ```

  Concuerda con lo que el #238 §10 ya sabía: los bits de confianza son atributos autenticados
  firmados con HMAC contra `key4.db`, así que el trabajo toca **dos** ficheros, y SQLite necesita
  además crear sus auxiliares en el mismo directorio.

Como los nombres de perfil son aleatorios (`plyif2tp.default` aquí) y sólo se conocen leyendo
`profiles.ini` **en tiempo de ejecución**, no hay ninguna forma de declarar en el manifiesto los
`cert9.db` concretos. El permiso mínimo que sirve es:

```yaml
  - --filesystem=~/.mozilla/firefox          # rw: sustituye al :ro de hoy
  - --filesystem=~/.pki/nssdb                # rw: sustituye al :ro de hoy
  - --filesystem=~/.local/share/pki/nssdb:create
```

**El coste, contrastado con el `--filesystem=home` del [#240](https://github.com/sgomez/rfirma/issues/240).**
El #240 midió que `home` lleva de «no existe» a legible *y escribible* `~/.ssh` (9 entradas),
`~/.gnupg` (18), `~/.local/share/keyrings`, `~/.aws`, `~/.kube`, `~/.git-credentials`, `~/.netrc`,
`~/.config` (128) y `~/.bash_history`. Nada de eso cambia aquí: con las tres líneas de arriba
`~/.config` **sigue sin existir** dentro del sandbox. Lo que sí cambia es el perfil de Firefox:
son **104 entradas**, y entre ellas están `logins.json`, `key4.db`, `cookies.sqlite`,
`places.sqlite`, `formhistory.sqlite` y `sessionstore-backups`. **Todas ellas ya son legibles hoy**
por el `:ro` que el manifiesto declara desde el #95/#101; lo que se añade es escritura sobre el
perfil de navegación. Dicho en una línea, como pide el ADR: *un atacante que ya ejecuta código en
rfirma pasa de leer las contraseñas y las cookies de Firefox a poder también sustituirlas, y a
poder plantar una raíz de confianza propia en los tres almacenes.* Es exactamente la capacidad que
hay que ejercer, así que no es un extra: es el enunciado.

Una nota de honestidad sobre el `:ro` de hoy: el manifiesto lo justifica diciendo que «`:ro` es la
mitad importante» porque rfirma no tiene motivo para escribir en el perfil. Con el #238 ese motivo
existe. La concesión hay que reescribirla, no ampliarla en silencio.

## 3. `certutil`: no está, no hace falta, y si se quiere sale casi gratis

### 3.1. Qué trae el runtime

Sonda dentro del bundle:

```
certutil: None        pk12util: None      modutil: None
openssl:  /usr/bin/openssl                p11-kit: /usr/bin/p11-kit
/usr/bin: ['certtool', 'p11-kit', 'p11tool']
```

Pero la biblioteca está entera:

```
/usr/lib/x86_64-linux-gnu/libnss3.so       (1347576)
/usr/lib/x86_64-linux-gnu/libnssutil3.so   (223872)
/usr/lib/x86_64-linux-gnu/libsmime3.so     (198056)
/usr/lib/x86_64-linux-gnu/libssl3.so       (451416)
/usr/lib/x86_64-linux-gnu/libsoftokn3.so   (356616)
/usr/lib/x86_64-linux-gnu/libfreebl3.so    (15696)
/usr/lib/x86_64-linux-gnu/libnssckbi.so    (15544)
/usr/lib/x86_64-linux-gnu/libnspr4.so      (311024)
libnss3.so: cargada        NSS_GetVersion: b'3.101.4'
  NSS_Initialize: presente        NSS_InitReadWrite: presente
  PK11_ImportDERCert: presente    CERT_ChangeCertTrust: presente
  CERT_DecodeTrustString: presente  PK11_GetInternalKeySlot: presente
  CERT_NewTempCertificate: presente
```

No es una sorpresa: `libsoftokn3.so` ya es dependencia de este proyecto —`pkcs11::stores` lo carga
con `cryptoki` para *leer* los certificados de Firefox y de `~/.pki/nssdb`—, así que la familia NSS
ya estaba en el runtime por otro motivo.

### 3.2. La medida que decide: escribir la confianza con la NSS del runtime

Sonda `ctypes` dentro del sandbox, sin ninguna herramienta añadida, contra un almacén de usar y
tirar creado en `~/Documents` (que sí es escribible por `--filesystem=xdg-documents`):

```
CA de usar y tirar: 793 bytes DER; almacen: /home/sergio/Documents/rfirma-nss-probe
NSS_Initialize(rw) -> 0
PK11_NeedUserInit: 1
PK11_InitPin('') -> 0
PK11_Authenticate -> 0
CERT_NewTempCertificate -> ok
PK11_ImportCert -> 0
CERT_DecodeTrustString('CT,C,C') -> 0  152 24 24
CERT_ChangeCertTrust -> 0
ficheros creados: ['cert9.db', 'key4.db', 'pkcs11.txt']
```

Y verificado **desde el anfitrión**, con el `certutil` de `libnss3-tools`, que es un tercero
independiente de la sonda:

```
$ certutil -L -d sql:$HOME/Documents/rfirma-nss-probe
Certificate Nickname                    Trust Attributes
rfirma probe CA                         CT,C,C
```

Siete símbolos y una estructura de tres enteros (`CERTCertTrust`) es todo el trabajo. **El flatpak
puede instalar la confianza.**

Segunda medida, sobre el almacén real montado `:ro` —sin escribir nada, sólo abriéndolo—:

```
NSS_Initialize(rw, ~/.pki/nssdb) -> -1  -8018 (Unknown PKCS #11 error.)
```

`-8018` es `SEC_ERROR_UNKNOWN_PKCS11_ERROR` (`SEC_ERROR_BASE + 174`, en `/usr/include/nss/secerr.h`
del SDK). **NSS no dice «sólo lectura»: dice un error opaco.** Quien implemente esto tiene que
comprobar la escribibilidad del directorio por su cuenta antes de creerse el diagnóstico de NSS,
igual que el #238 §10 ya obliga a no fiarse del código de salida de `certutil`.

### 3.3. Empaquetar `certutil`, si aun así se quisiera

No hace falta construir NSS: **`certutil` ya está en el SDK**, que es donde corren los
`build-commands` del manifiesto.

```
$ flatpak run --devel --command=sh org.gnome.Sdk//50 -c '…'
/usr/bin/certutil   215976 bytes
/usr/bin/modutil    185720
/usr/bin/pk12util   112952
pkg-config --modversion nss  ->  3.101.4
pkg-config --libs nss -> -lssl3 -lsmime3 -lnss3 -lnssutil3 -lplds4 -lplc4 -lnspr4
/usr/include/nss/cert.h  /usr/include/nss/certdb.h
ldd /usr/bin/certutil:
    libnss3.so libnssutil3.so libsmime3.so libssl3.so libc.so.6
    libplds4.so libplc4.so libnspr4.so
```

Sus siete dependencias están **todas** en el Platform (apartado 3.1), así que un
`install -Dm755 /usr/bin/certutil /app/bin/certutil` en un módulo `simple` basta. Comprobado que
funciona de verdad: copiado el `certutil` del SDK a una ruta escribible y ejecutado **dentro del
sandbox de la aplicación**, sobre un almacén de usar y tirar:

```
+ certutil -A -d sql:…/store -n 'rfirma probe CA 2' -t CT,C,C -a -i /tmp/ca.pem
rc=0
+ certutil -L -d sql:…/store
rfirma probe CA 2                       CT,C,C
```

y, sobre el almacén real montado `:ro`, falla limpiamente y **sin escribir nada**:

```
certutil: function failed: SEC_ERROR_READ_ONLY: security library: read-only database.
rc=255
```

(Comprobado después en el anfitrión: `~/.pki/nssdb` conserva sus tres ficheros con la marca de
tiempo intacta, `2026-09-03 13:59:28`.)

Aun así, **la FFI es preferible a copiar el binario**, por tres motivos y ninguno es de gusto:

1. El binario copiado es una foto de la NSS del SDK del día de la construcción; el runtime se
   actualiza por su cuenta. La biblioteca cargada en ejecución es siempre la del runtime, así que
   la FFI no puede descuadrarse y el binario sí.
2. `certutil` bloquea leyendo del tty si el almacén tiene contraseña maestra y no se le da `-f`
   (#238 §10). Por FFI eso es `PK11_Authenticate` con la contraseña que se decida, y el bloqueo no
   existe.
3. NSS es MPL-2.0 y rfirma es EUPL-1.2: distribuir un binario de NSS dentro del bundle añade una
   obligación de fuentes que enlazar contra la biblioteca del runtime no añade.

### 3.4. Crates de Rust: no hay ninguna que sirva

Consultado el registro (`crates.io/api/v1/crates?q=…`, septiembre de 2026):

| Crate | Última versión | Fecha | Qué es |
|---|---|---|---|
| `nss` | 0.7.1 | 2016-10-12 | «Bindings for the NSS cryptography library». Muerta hace diez años. |
| `nss-sys` | 0.1.9 | 2016-10-12 | Igual, la capa `-sys`. Muerta. |
| `nss-gk-api` | 0.3.0 | 2023-06-14 | «Gecko API for NSS». Viva en Gecko, pero **no cubre la base de certificados**. |
| `koi-truststore` | 0.4.1 | 2026-06-15 | «Platform trust store integration for installing and removing local CA certificates». |

* `nss-gk-api` está descargada y revisada: su `bindings/bindings.toml` sólo declara `NSS_Initialize`
  de todo lo que hace falta; el resto es SSL, PKCS#11 y NSPR para *neqo* (QUIC). `CERT_ChangeCertTrust`,
  `PK11_ImportCert` y `CERT_NewTempCertificate` **no están**.
* `koi-truststore` promete justo lo que buscamos y **no lo hace en Linux**: su `src/linux.rs` son 34
  líneas que copian a `/usr/local/share/ca-certificates/` y llaman a `update-ca-certificates`
  (`Command::new("update-ca-certificates")`); su único uso de `certutil` es el de **Windows**, que
  es otra herramienta con el mismo nombre. Es decir, hace exactamente lo que el
  [#225](https://github.com/sgomez/rfirma/issues/225) midió que **no llega a ningún navegador** en
  Debian/Ubuntu, y encima pide root. Sus dependencias (`koi-common`, `thiserror`, `tracing`)
  confirman que no toca NSS.

Conclusión: **FFI a mano**, con `bindgen` sobre `/usr/include/nss/` (el SDK trae los cabeceros y el
`.pc`) o con siete `extern "C"` escritos a mano. La sonda de `ctypes` del 3.2 es la lista completa
de lo que hay que declarar.

## 4. Perfiles confinados

Con los permisos de hoy, **no son alcanzables ni detectables**:

```
~/.var/app          -> ['me.sgomez.rfirma']        (el anfitrión tiene nueve)
~/.var/app/com.stremio.Stremio -> NO
~/snap              -> NO
~/.local/share/flatpak -> []                        (vacío: sintetizado)
/var/lib/flatpak    -> NO       /var/lib/snapd -> NO      /snap -> NO
~/.local/share/applications -> NO
/run/host/share       -> ['icons']
/run/host/user-share  -> ['icons']
/usr/share/applications -> ['gcr-prompter.desktop', 'gcr-viewer.desktop',
                            'mimeinfo.cache', 'org.gnome.Yelp.desktop']
```

No hay ningún canal indirecto: `/run/host` sólo proyecta tipografías e iconos, no las
`applications` del anfitrión, así que **tampoco se puede detectar un navegador por su `.desktop`**.

Con un permiso explícito sí son alcanzables, y en los dos casos:

```
$ flatpak run --filesystem=~/.var/app/com.stremio.Stremio …
~/.var/app -> ['com.stremio.Stremio', 'me.sgomez.rfirma']
~/.var/app/com.stremio.Stremio -> ['.ld.so', '.local', '.stremio-server', 'cache', 'config', 'data']

$ flatpak run --filesystem=~/snap …
~/snap -> ['scc']
```

Es decir: **flatpak no protege de forma especial el `$HOME` privado de otro flatpak**. Un
`--filesystem=~/.var/app/org.mozilla.firefox/.mozilla` funcionaría igual que cualquier otra ruta.
Que se pueda no cambia lo que decidió el #238 —confinados se **detectan y avisan, no se tocan**—,
pero sí contesta la pregunta que hacía falta: **detectarlos es posible y cuesta dos permisos `:ro`**,

```yaml
  - --filesystem=~/snap/firefox/common/.mozilla:ro
  - --filesystem=~/.var/app/org.mozilla.firefox/.mozilla:ro
```

y la detección no da falsos positivos: probadas esas dos líneas en una máquina donde **ninguna de
las dos rutas existe** en el anfitrión, dentro del sandbox siguen dando «no existe». El sandbox no
sintetiza el directorio de un montaje `:ro` inexistente (a diferencia de lo que hace con los
*padres*, apartado 1).

*No medido*: esta máquina no tiene ni Firefox snap ni Firefox flatpak instalados, así que se ha
medido la **alcanzabilidad de la ruta** con otro flatpak (`com.stremio.Stremio`) y con otro snap
(`scc`), no un `cert9.db` de Firefox confinado real. Escribir dentro no se ha intentado, porque el
#238 ya decidió no hacerlo.

## 5. La vía por portal: no existe

`gdbus introspect` sobre `org.freedesktop.portal.Desktop` desde dentro del bundle da 30 interfaces:
`Account`, `Background`, `Camera`, `Clipboard`, `DynamicLauncher`, `Email`, `FileChooser`,
`GameMode`, `GlobalShortcuts`, `Inhibit`, `InputCapture`, `Location`, `MemoryMonitor`,
`NetworkMonitor`, `Notification`, `OpenURI`, `PowerProfileMonitor`, `Print`, `ProxyResolver`,
`Realtime`, `RemoteDesktop`, `ScreenCast`, `Screenshot`, `Secret`, `Settings`, `Trash`, `Usb`,
`WebExtensions`, más `org.freedesktop.host.portal.Registry` y las de DBus.

**Ninguna toca certificados ni confianza.** Y no es cosa de esta versión: la
[referencia de API de xdg-desktop-portal](https://flatpak.github.io/xdg-desktop-portal/docs/api-reference.html)
documenta la misma lista (más `Documents`, `File Transfer`, `Request` y `Session`, que son de otro
objeto) y no incluye ningún portal de certificados, de confianza TLS ni de PKI. `Secret` es para
guardar un secreto de la aplicación, no para tocar el almacén de nadie.

El portal de ficheros tampoco abre camino: entrega descriptores por su FUSE en
`/run/user/1000/doc/…` y el #240 ya midió que ni siquiera con `--filesystem=home` contesta
`Documents.Info`/`Documents.Lookup` dentro del sandbox. Un `cert9.db` que entrase por ahí sería una
copia en el árbol del portal, no el almacén del navegador.

**`flatpak-spawn --host` sí es una vía, y es la vía de salir del sandbox.** El binario está en el
runtime (`/usr/bin/flatpak-spawn`), pero sin permiso no habla con nadie:

```
flatpak-spawn --host true
  Portal call failed: org.freedesktop.DBus.Error.ServiceUnknown
  Hint: --host only works when the Flatpak is allowed to talk to org.freedesktop.Flatpak
```

Con `--talk-name=org.freedesktop.Flatpak` en una sola ejecución:

```
flatpak-spawn --host true                          -> rc 0
flatpak-spawn --host sh -c 'command -v certutil'   -> /usr/bin/certutil
flatpak-spawn --host sh -c 'ls ~/.mozilla'         -> extensions
                                                      firefox
                                                      native-messaging-hosts
                                                      systemextensionsdev
```

Las cuatro entradas del `~/.mozilla` real, frente a la única que se ve desde dentro: es el
anfitrión, sin sandbox. **Pedir ese permiso es pedirlo todo** —ejecución arbitraria con los
privilegios de la persona— a cambio de no pedir tres `--filesystem` acotados. Es peor negocio por
un orden de magnitud, y hay que dejarlo escrito para que nadie lo proponga como «la vía limpia».

## 6. Lo que no se ha medido

* **Un Firefox snap y un Firefox flatpak reales.** Esta máquina lleva el Firefox del sistema
  (`/usr/lib/firefox`). Medida la alcanzabilidad de las rutas con otros confinamientos, no un
  `cert9.db` de Firefox confinado.
* **Escribir en un almacén con contraseña maestra.** Todos los almacenes de usar y tirar se crearon
  sin contraseña. El caso que el #238 §10 documenta —`-t` deja la confianza en `,,`, y `certutil`
  bloquea en el tty— **no se ha reproducido aquí**; se hereda de allí tal cual.
* **Que Chrome o Firefox lean de verdad la CA así instalada.** Se ha comprobado que la confianza
  queda escrita y que `certutil -L` la ve; no se ha arrancado un navegador contra un servidor
  `wss://127.0.0.1`. Eso es de la v0.5, con el servidor.
* **El módulo del manifiesto que copia `certutil`, construido de verdad.** El binario se probó
  copiándolo del SDK a mano y ejecutándolo dentro del sandbox; no se ha reconstruido el flatpak con
  ese módulo (`just flatpak` sale de aquí y no era necesario para contestar la pregunta).
* **La FFI en Rust.** Lo medido es `ctypes` desde Python contra la misma `libnss3.so` que cargaría
  Rust. Los símbolos, las firmas y la estructura son los mismos; el `bindgen` no se ha escrito.
* **aarch64.**

## 7. Lo que queda abierto para el ADR (aquí no se decide)

Ninguna de estas opciones queda descartada por la medición, y el informe no elige:

1. **Ampliar el permiso**: `~/.mozilla/firefox` rw + `~/.pki/nssdb` rw + `~/.local/share/pki/nssdb:create`,
   más dos `:ro` para detectar confinados. **Coste**: escritura sobre el perfil de Firefox entero,
   que hoy ya es legible. **Sigue sin ser `--filesystem=home`.**
2. **No ampliarlo y que el flatpak no instale confianza**: el flatpak queda como canal de firma y no
   de protocolo, y la v0.5 sale por `.deb`/`.rpm`. Coste: dos comportamientos distintos según canal,
   que es justo lo que el #238 §3 quiso evitar.
3. **Ampliarlo sólo para Chrome** (`~/.pki/nssdb` y `~/.local/share/pki/nssdb:create` son exactos y
   baratos) y tratar Firefox como confinado: detectado y avisado. Coste: el navegador más probable
   del usuario español de sede electrónica queda fuera.

Y una restricción firme para las tres: **la vía de `--talk-name=org.freedesktop.Flatpak` no está
sobre la mesa**, por el apartado 5.

## Reproducir

Las sondas son cuatro ficheros sueltos que se meten por la entrada estándar; no hay nada que
instalar. El esqueleto:

```bash
# 1. Qué se ve y qué se puede escribir hoy
flatpak run --command=python3 me.sgomez.rfirma - < sonda-visibilidad.py

# 2. Granularidad del permiso, sin persistir nada
flatpak run --filesystem=~/.pki/nssdb                    --command=python3 me.sgomez.rfirma - < sonda-permisos.py
flatpak run --filesystem=~/.local/share/pki/nssdb:create --command=python3 me.sgomez.rfirma - < sonda-permisos.py
flatpak run --filesystem=~/.mozilla/firefox              --command=python3 me.sgomez.rfirma - < sonda-permisos.py

# 3. Escribir la confianza con la NSS del runtime, en un almacén de usar y tirar
flatpak run --command=python3 me.sgomez.rfirma - < sonda-nss.py
certutil -L -d sql:$HOME/Documents/rfirma-nss-probe   # verificación desde el anfitrión
rm -rf $HOME/Documents/rfirma-nss-probe

# 4. certutil del SDK, ejecutado dentro del sandbox de la aplicación
flatpak run --devel --filesystem=$HOME/Documents --command=cp org.gnome.Sdk//50 \
    /usr/bin/certutil $HOME/Documents/rfirma-certutil

# 5. Portales y salida del sandbox
flatpak run --command=python3 me.sgomez.rfirma - < sonda-portales.py
flatpak run --talk-name=org.freedesktop.Flatpak --command=python3 me.sgomez.rfirma - < sonda-portales.py
```

**Ni un `flatpak override`.** Todo lo que amplía permisos es `flatpak run --…`, que vale para esa
ejecución y no deja rastro en la instalación. Y todo lo que se escribe se escribe en almacenes
creados para la prueba: los de la persona no se tocan.
