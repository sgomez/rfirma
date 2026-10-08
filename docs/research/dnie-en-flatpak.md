# El DNIe en el flatpak: el `pcscd` del anfitrión y OpenSC empaquetado

Qué hace falta para que el flatpak trate las tarjetas y el DNIe como el `.deb`, medido para el
[issue #1765](https://github.com/sgomez/rfirma/issues/1765). Parte de lo que ya midieron
[`flatpak-canal-unico.md`](flatpak-canal-unico.md) §3 (el runtime no trae OpenSC ni `libpcsclite`, y
p11-kit solo proyecta la confianza) y [`dnie-en-linux.md`](dnie-en-linux.md) (el DNIe con el OpenSC
del sistema), y no lo repite.

Cada apartado separa lo **leído** en el código fuente, lo **medido** en esta máquina y lo que queda
**sin medir** porque necesita a una persona con el lector y el DNIe. Ninguna salida de esta nota
lleva datos de una tarjeta.

## Veredicto

**El camino aguanta, y la versión del cliente de pcsc-lite ya es la buena.** Un cliente 2.5.1 dentro
del sandbox negocia con todo `pcscd` desde la 1.8.24, que cubre todas las distribuciones con soporte;
solo falla contra la 1.8.23 o anteriores, y lo hace con un error limpio que el vigilante cuenta como
cero lectores. `--socket=pcsc` monta el socket del anfitrión, y polkit autoriza al proceso del sandbox
por la sesión gráfica del usuario. Con el DNIe en el lector (apartado 5), la sonda lee la tarjeta, ve sus
tres certificados sin PIN y sigue en caliente al lector y a la tarjeta en un mismo sandbox abierto,
contra un `pcscd` 4:5 y contra uno 4:4; y el flatpak de rFirma firma con el DNIe, con el PIN en su
propio diálogo.

Tres cosas cambian el manifiesto o lo que se promete:

1. **OpenSC tiene que compilarse con zlib, OpenSSL y SM obligatorios**, no detectados: sin zlib el
   driver `dnie` carga, pero no puede leer los certificados, que el DNIe guarda comprimidos.
2. **El flatpak no enseña la ventana «Signature Requested» de OpenSC**: Debian y Ubuntu compilan
   OpenSC con `--enable-dnie-ui` y el valor por omisión de OpenSC, que es el que se empaqueta, no la
   compila. Es lo mismo que hace Fedora.
3. **El socket se monta al arrancar**: si el anfitrión no tiene `pcscd`, o reinicia `pcscd.socket`
   con rFirma abierto, rFirma se queda sin lector hasta que se vuelve a abrir.

## Entorno

| Qué | Valor |
| --- | --- |
| Sistema | Ubuntu 26.04.1, núcleo 7.0, sesión GNOME en Wayland |
| `pcscd` del anfitrión | 2.4.1-1, activado por `pcscd.socket`, con polkit (`pcscd --version`: `polkit systemd`) |
| polkit | 127-2ubuntu1.1; `org.debian.pcsc-lite.access_pcsc` y `access_card` con `allow_active=yes` |
| Flatpak | flatpak 1.16.6, flatpak-builder 1.4.8, `org.gnome.Platform//50` y su SDK |
| Sonda | `me.sgomez.RfirmaDnieProbe`, desechable: pcsc-lite 2.5.1 (solo cliente) y OpenSC 0.27.1, con `--socket=pcsc` |
| Demonios de prueba | `pcscd` 1.8.23, 1.9.9, 2.3.3, 2.4.1 y 2.5.1 de las etiquetas de [LudovicRousseau/PCSC](https://github.com/LudovicRousseau/PCSC), compilados en el SDK sin USB ni polkit y lanzados como usuario |
| Lector y tarjeta | Las mediciones de los apartados 1 a 4 se hicieron sin lector: se desenchufó antes de entrar en el sandbox (`usb 1-1: USB disconnect` en el journal). Las del apartado 5 las hizo una persona con la sonda y con el flatpak de rFirma, con un lector Alcor Micro AU9540 y un DNIe |
| Equipo 4:4 | Ubuntu 24.04.5 en una VM de libvirt, `pcscd` 2.0.3-1build1 con polkit, flatpak-builder 1.4.2; el lector, por `hostdev` USB |
| Flatpak de rFirma | El bundle de `just flatpak` del #1765, lanzado con `WEBKIT_DISABLE_DMABUF_RENDERER=1`: sin ella, en esta máquina (NVIDIA 580, Wayland) se cierra al abrirse con `Error 71`, aun con el paliativo del manifiesto |

## 1. El protocolo entre `libpcsclite` y `pcscd`

### Leído

La versión del protocolo vive en `src/winscard_msg.h`. A lo largo de las etiquetas:

| pcsc-lite | Protocolo | Qué acepta |
| --- | --- | --- |
| 1.8.9 – 1.8.23 | 4:3 | Solo 4:3 |
| 1.8.24 – 2.2.3 | 4:4 | Solo 4:4 |
| 2.3.0 – 2.4.0 | 4:5 | Solo 4:5 ([`53f57ed`](https://github.com/LudovicRousseau/PCSC/commit/53f57ed700bcd0bc47d970dc674ba3fd5ee5b387) añade `CMD_GET_READER_EVENTS`) |
| 2.4.1 | 4:5 | Servidor y cliente aceptan también 4:4 ([`db459f9`](https://github.com/LudovicRousseau/PCSC/commit/db459f95b855ff5a4bd03e46aebad0eac6414cc5), [`18e16b3`](https://github.com/LudovicRousseau/PCSC/commit/18e16b3d)) |
| 2.5.0 – 2.5.2 | 4:6 | 4:4 a 4:6 ([`92c8565`](https://github.com/LudovicRousseau/PCSC/commit/92c85658); 4:6 añade `CMD_GET_READERS_STATE_SIZE` y `_ARRAY`) |

En 2.5.1, `PROTOCOL_VERSION_MAJOR 4`, `PROTOCOL_VERSION_MINOR 6`, y los dos `_BACKWARD` a 4
([`winscard_msg.h:50-56`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_msg.h#L50-L56)).

**La regla del demonio no es la misma en todas las versiones**, y es lo que decide:

- **Hasta 2.4.0**, cualquier diferencia de mayor o menor es `SCARD_E_SERVICE_STOPPED`, y la respuesta
  lleva la versión del servidor
  ([2.2.3, `winscard_svc.c:396-416`](https://github.com/LudovicRousseau/PCSC/blob/2.2.3/src/winscard_svc.c#L396-L416);
  igual en [1.9.5](https://github.com/LudovicRousseau/PCSC/blob/1.9.5/src/winscard_svc.c#L376-L396)).
- **2.4.1 acepta todo menor ≥ 4**, también uno **más nuevo** que el suyo: solo rechaza
  `veStr.minor < PROTOCOL_VERSION_MINOR_SERVER_BACKWARD`
  ([`winscard_svc.c:411-414`](https://github.com/LudovicRousseau/PCSC/blob/2.4.1/src/winscard_svc.c#L411-L414)).
- **2.5.x acepta su menor y los anteriores hasta 4**, no los posteriores
  ([`winscard_svc.c:404-426`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_svc.c#L404-L426)).

**El cliente 2.5.1 reintenta con la versión del servidor.** Si recibe `SCARD_E_SERVICE_STOPPED` con el
mismo mayor y un menor ≥ 4, vuelve a mandar `CMD_VERSION` con el menor que le propone el servidor, y
guarda en `Protocol_version` el que se acuerda
([`winscard_clnt.c:592-642`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_clnt.c#L592-L642)).
Con 4:5 o menos usa la tabla fija de lectores
([`:3684-3720`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_clnt.c#L3684-L3720)),
y con 4:4 no pide el número de eventos de lector
([`:3661`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_clnt.c#L3661)): la
notificación `\\?PnP?\Notification` sigue saltando, pero por el cambio en el número de lectores
([`:1900-1918`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_clnt.c#L1900-L1918)).
El autor lo resume en
[«pcsc-lite backward & forward compatible with itself»](https://blog.apdu.fr/posts/2026/01/pcsc-lite-backward-forward-compatible-with-itself/):
un cliente 2.4.1 o posterior habla con cualquier demonio desde la 1.8.24, y el cambio nació de la
petición de Flatpak en [PCSC#199](https://github.com/LudovicRousseau/PCSC/issues/199).

Entre 2.5.1 y 2.5.2 el cliente solo cambia un `strncpy` y una comprobación de Coverity
(`git diff 2.5.1 2.5.2 -- src/winscard_clnt.c`): nada del protocolo.

### Lo que traen las distribuciones

| Distribución | pcsc-lite | Protocolo | Fuente |
| --- | --- | --- | --- |
| Debian 12 bookworm (oldstable) | 1.9.9-2 | 4:4 | [sources.debian.org](https://sources.debian.org/api/src/pcsc-lite/) |
| Debian 13 trixie (stable) | 2.3.3-1 | 4:5 estricto | ídem |
| Debian forky / sid | 2.5.2-1 | 4:6 | ídem |
| Ubuntu 22.04 jammy | 1.9.5-3ubuntu1 | 4:4 | [madison](https://people.canonical.com/~ubuntu-archive/madison.cgi?package=pcsc-lite&text=on) |
| Ubuntu 24.04 noble | 2.0.3-1build1 | 4:4 | ídem |
| Ubuntu 26.04 resolute | 2.4.1-1 | 4:5 tolerante | ídem |
| Ubuntu en desarrollo (`stonking`) | 2.5.1-1 | 4:6 | ídem |
| Fedora 43 | 2.3.3 | 4:5 estricto | [`pcsc-lite.spec` de f43](https://src.fedoraproject.org/rpms/pcsc-lite/blob/f43/f/pcsc-lite.spec) |
| Fedora 44 | 2.4.1 | 4:5 tolerante | [ídem, f44](https://src.fedoraproject.org/rpms/pcsc-lite/blob/f44/f/pcsc-lite.spec) |
| Fedora 45 (ramificada) y rawhide | 2.5.2 | 4:6 | [ídem, f45](https://src.fedoraproject.org/rpms/pcsc-lite/blob/f45/f/pcsc-lite.spec) |

Bodhi da como versiones vigentes de Fedora la 43 y la 44
([`/releases/?state=current`](https://bodhi.fedoraproject.org/releases/?state=current)). Ninguna de
estas versiones toca el protocolo: la serie de parches de Debian 2.3.3-1 y 1.9.9-2 está vacía, el
`.spec` de Fedora no lleva ningún `Patch`, y los cambios de Ubuntu son de empaquetado (el
`changelog` de 1.9.5-3ubuntu1 y de 2.0.3-1build1). El parche que Fedora llevó en 2022 y cambiaba
`PCSCLITE_MAX_READERS_CONTEXTS` sin subir el protocolo
([PCSC#118](https://github.com/LudovicRousseau/PCSC/issues/118)) ya no está: el valor es 16 en
todas las etiquetas de la tabla.

### Medido

El cliente 2.5.1 del sandbox contra cada demonio, con `PCSCLITE_DEBUG=0` y `testpcsc`
(`/tmp/dnie-flatpak/scripts/matrix.sh`, en «Reproducir»):

| `pcscd` | Lado del cliente | Lado del demonio | Resultado |
| --- | --- | --- | --- |
| 1.8.23 (4:3) | `Server is protocol version 4:3` y nada más | `Client protocol is 4:6`, `Server protocol is 4:3` | `SCardEstablishContext: Service not available.` |
| 1.9.9 (4:4) | `Using backward compatibility`, segundo `CMD_VERSION` | `mismatch`, después `Client is protocol version 4:4` | Contexto; `SCardListReaders`: sin lector |
| 2.3.3 (4:5 estricto) | `Using backward compatibility` | `mismatch`, después `Client is protocol version 4:5` | Contexto; sin lector |
| 2.4.1 (4:5 tolerante) | Sin reintento: `Server is protocol version 4:5` | `mismatch` y `Enable backward compatibility` | Contexto; sin lector |
| 2.4.1 del anfitrión, por `/run/pcscd/pcscd.comm` | Igual que la fila anterior | `Communication protocol mismatch!` en el journal, sin rechazo | Contexto; sin lector |
| 2.5.1 (4:6) | Sin aviso | Sin aviso | Contexto; sin lector |

«Sin lector» es `SCARD_E_NO_READERS_AVAILABLE`, lo que se esperaba con el lector desenchufado: el
protocolo ya se ha acordado para entonces. El `Communication protocol mismatch!` que deja en el journal
del anfitrión cada arranque de rFirma con 2.4.1 es un `PCSC_LOG_CRITICAL`, pero la conexión sigue.

**El vigilante ya hace lo correcto con el único caso que falla.** Contra 1.8.23,
`Context::establish` devuelve error, `PcscSource::context` se queda en `None` y `StatusWatch` cuenta
cero lectores y lo vuelve a intentar cada cinco segundos (`RETRY_WITHOUT_PCSC` en
`rfirma-app/src-tauri/src/identity/adapters/pcsc.rs`). Esa versión solo la trae ya Ubuntu 18.04.

## 2. `--socket=pcsc`

### Leído

`flatpak_run_add_pcsc_args`
([`common/flatpak-run-sockets.c:75-101`](https://github.com/flatpak/flatpak/blob/1.16.6/common/flatpak-run-sockets.c#L75-L101),
idéntica en `main`):

- Si el anfitrión tiene `PCSCLITE_CSOCK_NAME`, usa esa ruta; si no, `/run/pcscd/pcscd.comm`.
- **Si la ruta no existe al arrancar, no monta nada.** Con la variable definida además la quita del
  entorno del sandbox; sin ella, simplemente vuelve.
- Si existe, la monta con `--ro-bind` en `/run/pcscd/pcscd.comm` y pone
  `PCSCLITE_CSOCK_NAME=/run/pcscd/pcscd.comm` dentro.

El cliente lee esa variable antes que la ruta compilada
([`winscard_msg.c:86-99`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_msg.c#L86-L99)),
así que el `-Dipcdir=/run/pcscd` del manifiesto solo importa si la variable falta, y apunta al mismo
sitio.

### Medido

Dentro del sandbox, con el `pcscd` del anfitrión activo:

```
srw-rw-rw- 1 nfsnobody nfsnobody 0 oct  8 07:55 pcscd.comm
PCSCLITE_CSOCK_NAME=/run/pcscd/pcscd.comm
```

El directorio `/run/pcscd` del sandbox solo tiene el socket: `pcscd.pid` no cruza, y el cliente no lo
necesita. Con `PCSCLITE_CSOCK_NAME=<ruta>` en el anfitrión, flatpak monta esa otra ruta: así se
conectaron los demonios de prueba del apartado 1.

**Un socket recreado deja al sandbox abierto sin servicio.** Con un `pcscd` 2.5.1 de usuario: el sandbox
conecta, el demonio se reinicia y recrea su socket, y el mismo sandbox recibe
`SCardEstablishContext: Service not available.`; un sandbox nuevo conecta a la primera
(`/tmp/dnie-flatpak/scripts/stale.sh`). El montaje fija el inodo, no la ruta, como explica smcv en
[flatpak#4723](https://github.com/flatpak/flatpak/issues/4723). Con activación por socket el fichero es
de systemd y no del demonio, así que no pasa mientras `pcscd.socket` siga vivo: en esta máquina el
socket conserva la hora en que arrancó la unidad, las 07:55.

## 3. polkit

### Leído

`IsClientAuthorized` toma el pid y el uid de `SO_PEERCRED`, construye un sujeto con
`polkit_unix_process_new_for_owner(pid, 0, uid)` y pregunta por `org.debian.pcsc-lite.<acción>`
([`auth.c:91-140`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/auth.c#L91-L140)). El demonio
pregunta `access_pcsc` al aceptar la conexión, antes de leer `CMD_VERSION`, y `access_card` en cada
`SCardConnect`
([`winscard_svc.c:359`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_svc.c#L359),
[`:571`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_svc.c#L571)).

**El pid que ve `pcscd` es el del anfitrión, no el del sandbox.** El núcleo traduce el pid de
`SO_PEERCRED` al espacio de nombres de quien llama a `getsockopt`, y el uid a su espacio de usuarios
(`cred_to_ucred`: `pid_vnr(pid)` y `from_kuid_munged(current_user_ns(), …)`,
[`net/core/sock.c:1704-1715`](https://github.com/torvalds/linux/blob/6c377d19d4a5/net/core/sock.c#L1704-L1715),
llamada en [`:1910`](https://github.com/torvalds/linux/blob/6c377d19d4a5/net/core/sock.c#L1910)).
`pcscd` vive en el espacio inicial, así que recibe el pid real del proceso de rFirma y el uid 1000.

**Ese proceso no está en ninguna sesión de logind**, pero polkit no lo necesita: si `sd_pid_get_session`
falla, toma el uid del proceso y le asigna la sesión gráfica del usuario con `sd_uid_get_display`
([`polkitbackendsessionmonitor-systemd.c:343-431`](https://github.com/polkit-org/polkit/blob/d3ba690/src/polkitbackend/polkitbackendsessionmonitor-systemd.c#L343-L431)).
Ese respaldo existe desde polkit 0.113
([`a68f5df`](https://github.com/polkit-org/polkit/commit/a68f5df), para el modelo de un bus de sesión
por usuario), y Ubuntu 22.04, que se quedó en 0.105, lo lleva parcheado
(`0.113/sessionmonitor-systemd-prepare-for-D-Bus-user-bus-mo.patch` en la
[serie de jammy](https://git.launchpad.net/ubuntu/+source/policykit-1/plain/debian/patches/series?h=ubuntu/jammy-updates)).
Es el mismo camino que recorre cualquier aplicación que GNOME lanza en un `app-gnome-*.scope`: el
flatpak no empeora nada respecto al `.deb`.

Un rechazo de `access_pcsc` no llega como error de permisos: el demonio cierra antes de contestar a
`CMD_VERSION`, el cliente lo registra como `Your pcscd is too old and does not support CMD_VERSION`
([`winscard_clnt.c:606-612`](https://github.com/LudovicRousseau/PCSC/blob/2.5.1/src/winscard_clnt.c#L606-L612))
y el vigilante lo cuenta como cero lectores. El de `access_card` sí sale como
`SCARD_W_SECURITY_VIOLATION`.

### Medido

| Qué | Valor |
| --- | --- |
| pid de la shell dentro del sandbox | `2`, en `pid:[4026533004]`; el anfitrión está en `pid:[4026531836]` |
| cgroup del sandbox | `…/user@1000.service/app.slice/app-flatpak-me.sgomez.RfirmaDnieProbe-….scope`: fuera de toda sesión |
| Sesión gráfica del usuario | `loginctl show-user 1000`: `Display=4`, `State=active`; la sesión 4, `Type=wayland`, `Active=yes`, `Remote=no` |
| `access_pcsc` desde el sandbox | Concedido: el `pcscd` del anfitrión, con polkit, dio contexto, y el journal no tiene ningún `NOT authorized` ni `Rejected` en las ejecuciones del sandbox |
| `access_card` | Concedido: `SCardConnect` lee el ATR desde el sandbox (apartado 5) |

## 4. OpenSC para el DNIe

### Leído: qué versión

| Versión | Qué cambia para el DNIe |
| --- | --- |
| 0.17.0 (2017) | Soporte del DNIe 3.0, canal del PIN ([`1d051db`](https://github.com/OpenSC/OpenSC/commit/1d051dba)) |
| 0.21.0 (2020) | El login de los DNIe 3.0 nuevos fallaba con `CKR_DATA_INVALID`: nueva estructura de CA para el canal seguro ([OpenSC#2105](https://github.com/OpenSC/OpenSC/issues/2105)) |
| 0.23.0 (2022) | Canal seguro portado a la API de OpenSSL 3 ([PR #2438](https://github.com/OpenSC/OpenSC/pull/2438)) |
| 0.24.0 (2023) | La descompresión de certificados pasa del driver a la capa común (`e1c0029`, `8352486`) |
| 0.27.0 (2026) | Caché de ficheros apagada por omisión, que servía el certificado viejo tras renovar ([OpenSC#3276](https://github.com/OpenSC/OpenSC/issues/3276)); el MF se selecciona sin SM al iniciar y al cerrar, que arregla el fallo de `dnie-tool` y de Firefox al dar el PIN ([OpenSC#3466](https://github.com/OpenSC/OpenSC/issues/3466), [PR #3476](https://github.com/OpenSC/OpenSC/pull/3476)) |

**0.27.0 es el mínimo razonable, y 0.27.1 es la última versión publicada** (31 de marzo de 2026, un día
después de la 0.27.0). Es la que fija el manifiesto con su `sha256`, y la que traen Fedora 43, 44 y 45 y
Debian forky. El DNIe 3.0 y el 4.0 entran por la misma tabla de ATR, que enmascara los bytes de versión
([`card-dnie.c:129-144`](https://github.com/OpenSC/OpenSC/blob/0.27.1/src/libopensc/card-dnie.c#L129-L144)).
En `master` hay ya arreglos del DNIe sin publicar (un desbordamiento al procesar `EF(CFD)`, `c930b22`, y
otro del canal CWA con claves RSA grandes, `3353ccb`): conviene subir de versión cuando salga la
siguiente, no parchear.

### Leído: con qué se compila

| Dependencia | ¿Hace falta? | Por qué |
| --- | --- | --- |
| OpenSSL y SM | **Sí** | `card-dnie.c` queda vacío sin `ENABLE_OPENSSL` y `ENABLE_SM` ([`card-dnie.c:31`](https://github.com/OpenSC/OpenSC/blob/0.27.1/src/libopensc/card-dnie.c#L31)) |
| zlib | **Sí** | Los certificados del DNIe van comprimidos; sin `ENABLE_ZLIB`, `dnie_is_compressed` dice siempre que no y `decompress_file` devuelve `SC_ERROR_NOT_SUPPORTED` ([`card-dnie.c:910-943`](https://github.com/OpenSC/OpenSC/blob/0.27.1/src/libopensc/card-dnie.c#L910-L943), [`pkcs15.c:2414-2440`](https://github.com/OpenSC/OpenSC/blob/0.27.1/src/libopensc/pkcs15.c#L2414-L2440)) |
| `libpcsclite` | En ejecución, por `dlopen` | `DEFAULT_PCSC_PROVIDER="libpcsclite.so.1"` en Linux ([`configure.ac:916-930`](https://github.com/OpenSC/OpenSC/blob/0.27.1/configure.ac#L916-L930)); al compilar, solo sus cabeceras |
| readline | No | Solo la usa `opensc-explorer` |
| OpenPACE (`libeac`) | No | PACE no se usa con el DNIe por contacto, y OpenSC no lo implementa para el DNIe ([`dnie-en-linux.md`](dnie-en-linux.md)) |
| `--enable-dnie-ui` | No, y está apagado por omisión | Es la ventana de confirmación por `pinentry` ([`configure.ac:291-296`](https://github.com/OpenSC/OpenSC/blob/0.27.1/configure.ac#L291-L296)) |

**zlib, OpenSSL y SM se detectan por omisión, así que hay que pedirlos**: `--enable-zlib`,
`--enable-openssl` y `--enable-sm` hacen que la construcción falle si falta alguno, en vez de producir
un OpenSC que lista el DNIe y no lee sus certificados.

**La ventana «Signature Requested» depende de quién compile OpenSC.** Debian la enciende
(`--enable-dnie-ui` en [`debian/rules`](https://sources.debian.org/src/opensc/0.27.1-3/debian/rules/#L10-L18)),
y por eso la enseña el `.deb` en Ubuntu; Fedora no
([`opensc.spec` de f44](https://src.fedoraproject.org/rpms/opensc/blob/f44/f/opensc.spec#_97));
el flatpak, con el valor por omisión, tampoco. Encenderla dentro del sandbox no es gratis:
`dnie_ask_user_consent` hace `exec` de `/usr/bin/pinentry`, y si el programa no está, el hijo aborta
([`card-dnie.c:294-308`](https://github.com/OpenSC/OpenSC/blob/0.27.1/src/libopensc/card-dnie.c#L294-L308)).
El runtime trae `pinentry` y `pinentry-gnome3`, pero el segundo necesita el diálogo de gcr por D-Bus,
que el manifiesto no concede; no se ha medido.

**El `.module` lo instala OpenSC si se le da el directorio.** Por omisión lo lleva al
`p11_system_config_modules` del `p11-kit-1.pc` del SDK, `/usr/share/p11-kit/modules`, que en la
construcción es de solo lectura
([`configure.ac:957-966`](https://github.com/OpenSC/OpenSC/blob/0.27.1/configure.ac#L957-L966),
[`src/pkcs11/Makefile.am:33-37`](https://github.com/OpenSC/OpenSC/blob/0.27.1/src/pkcs11/Makefile.am#L33-L37)).
Con `--enable-p11_system_config_modules=/app/share/p11-kit/modules` queda en
`/app/share/p11-kit/modules/opensc.module`, con `module: opensc-pkcs11.so` relativo. El manifiesto
retirado en `ab4bb61b` hacía lo mismo con `p11kitdir=` en `make-install-args`.

### Medido

Construido en el SDK de GNOME 50 con las opciones de la sonda (`--enable-zlib --enable-openssl
--enable-sm --disable-readline --disable-openpace --disable-notify`), el resumen de `configure` dice
`zlib support: yes`, `OpenSSL support: yes`, `SM support: yes`, `DNIe UI support: no`,
`p11_system_config: /app/share/p11-kit/modules`. Dentro del sandbox:

```
OpenSC 0.27.1 [gcc  15.2.0]
Enabled features: locking zlib openssl pcsc(libpcsclite.so.1)
  dnie             DNIe: Spanish eID card
```

- `opensc-pkcs11.so` y `libopensc.so` resuelven `libcrypto.so.3` (OpenSSL 3.5.8) y `libz.so.1` en el
  runtime, sin ningún `not found`.
- `/app/lib/pkcs11/opensc-pkcs11.so` es un enlace a `../opensc-pkcs11.so`; `pkcs11-tool --module …
  -I` dice `OpenSC smartcard framework (ver 0.27)` y `-L`, `No slots.`, porque no había lector.
- `/app/etc/opensc.conf` es `app default { }`: la configuración por omisión.
- El `.module` relativo resuelve como lo hace `p11kit::installed` con raíz `/app`: `module_directories`
  incluye `/app/lib/pkcs11`, y `is_file` sigue el enlace. Dentro del sandbox,
  `/usr/share/p11-kit/modules` solo tiene `p11-kit-trust.module`, que `module_for_rfirma` descarta por
  `trust-policy: yes`, y `~/.config/pkcs11/modules` no se ve sin permiso sobre el `$HOME`.
- El `/app` de la sonda, con las herramientas de OpenSC enteras, ocupa 18 MB; el manifiesto real ya
  descarta casi todas ([`flatpak-canal-unico.md`](flatpak-canal-unico.md) §7).

## 5. Con el DNIe en el lector

### Medido

Una ejecución de [`card-probes/flatpak_probe.sh`](card-probes/flatpak_probe.sh) contra el `pcscd` 2.4.1
del anfitrión (4:5 tolerante), con el lector enchufado y el DNIe dentro:

| Paso | Resultado |
| --- | --- |
| Socket y variable | `/run/pcscd/pcscd.comm` montado, `PCSCLITE_CSOCK_NAME=/run/pcscd/pcscd.comm` |
| Protocolo | `Server is protocol version 4:5`, `Client is protocol version 4:6`; contexto concedido. El journal del anfitrión deja un `Communication protocol mismatch!` por conexión, sin rechazo (apartado 1) |
| Lectores (`access_pcsc`) | `Alcor Micro AU9540 00 00`, con tarjeta |
| `SCardConnect` (`access_card`) | ATR leído; `opensc-tool -n` dice `dnie` |
| PKCS#11 sin login | Una ranura con `token label : DNI electrónico` y `token flags : login required, rng, token initialized, PIN initialized` |
| Certificados sin login | `CertAutenticacion`, `CertCAIntermediaDGP` y `CertFirmaDigital`: el canal seguro y la descompresión funcionan dentro del sandbox |
| En caliente, un sandbox abierto | `Yes` → `No` (tarjeta fuera) → `sin lector` (lector fuera) → `No` (lector dentro) → `Yes` (tarjeta dentro), cada cambio en menos de 2 s |

La misma sonda en el equipo 4:4, con el `pcscd` 2.0.3:

| Paso | Resultado |
| --- | --- |
| Protocolo | `Server is protocol version 4:4`, `Client is protocol version 4:6`, `Using backward compatibility`; contexto concedido. Un `Communication protocol mismatch!` por conexión en el journal, sin rechazo |
| Lectura sin PIN | La misma que contra la 2.4.1: el lector con tarjeta, ATR leído, driver `dnie`, la ranura del `DNI electrónico` y sus tres certificados |
| En caliente, un sandbox abierto | `Yes` → `No` (t=4 s) → `sin lector` (t=19 s) → `No` (t=29 s) → `Yes` (t=35 s). El lector se quitó y se devolvió a la VM a los 19 s y a los 29 s exactos: la notificación por el número de lectores llega en menos de un segundo |

El flatpak de rFirma, a mano en esta máquina:

| Comprobación | Resultado |
| --- | --- |
| La línea del lector | Sin tarjeta, leyendo, «DNIe listo», y vuelve al sacarla |
| La fila «Lector de tarjetas» del panel «Estado de rFirma» | «detectado» y «no detectado» al enchufar y desenchufar, sin «Volver a comprobar»; nunca «no soportado» |
| La lista de certificados | El DNIe aparece con su chip y desaparece al sacarlo |
| Una firma PAdES con el certificado de firma | El PIN lo pide el diálogo de rFirma, sin la ventana de OpenSC; `pdfsig` y VALIDe la dan por válida |

### Lo que no se ha medido

- **La firma con PIN desde el flatpak de rFirma contra un `pcscd` 4:4**: en ese equipo solo corrió la
  sonda.
- **Una sesión inactiva** (otro usuario en primer plano o una sesión remota): `allow_active` no la
  cubre, igual que en el `.deb`.
- **Qué hace cada distribución con `pcscd.socket` al actualizar el paquete.** Si lo reinicia, los
  flatpak abiertos se quedan sin lector hasta reabrirse (apartado 2).
- **Si cada distribución instala `pcscd` y `libccid` por omisión.** El `.deb` los recomienda; el
  flatpak no puede.

### La sonda para una persona

[`card-probes/flatpak_probe.sh`](card-probes/flatpak_probe.sh) construye la misma sonda en un
directorio temporal, la instala con `--user`, la recorre y la desinstala. No hace `C_Login` ni pide un
PIN, y redacta los números de serie del token y del lector. Pasos:

1. Enchufar el lector, meter el DNIe y comprobar en el anfitrión que `opensc-tool -l` lo ve con
   `Card: Yes`.
2. `sh docs/research/card-probes/flatpak_probe.sh`. Del paso 1 al 7 no hay que tocar nada: socket y
   variable, protocolo, `SCardConnect`, el nombre del driver (`opensc-tool -n` debe decir `dnie`), las
   ranuras con `token label : DNI electrónico`, las etiquetas `CertAutenticacion`, `CertFirmaDigital` y
   `CertCAIntermediaDGP`, y los avisos del `pcscd` del anfitrión.
3. En el paso 8, durante los 40 s de la cuenta: sacar el DNIe, desenchufar el lector, enchufarlo y
   meter el DNIe. Cada cambio debe salir en una línea `t=…s`, en el mismo sandbox.
4. Repetirla en un equipo con Debian 12 o Ubuntu 24.04 mide el caso 4:4. En una VM, el `pcscd` del
   anfitrión tiene que estar parado antes de pasarle el lector (si lo tiene abierto, la VM se queda
   en `can't set config #1, error -32`), y el lector se pasa después de iniciar sesión, porque GDM
   con un lector a la vista solo ofrece el inicio con tarjeta.
5. Con el flatpak de rFirma del #1765, a mano: la línea de estado del lector sigue al lector y a la
   tarjeta, el DNIe aparece en la lista con su chip de tarjeta, y una firma con
   `CertFirmaDigital` pide el PIN en el diálogo de rFirma, sin la ventana de OpenSC, y valida.

## Las decisiones del #1765

| Decisión | Estado | Qué cambia |
| --- | --- | --- |
| El flatpak hace con tarjetas lo mismo que el `.deb` | **Se sostiene, con dos diferencias** | No hay ventana «Signature Requested» de OpenSC (como en Fedora, no como en Debian y Ubuntu), y un `pcscd.socket` reiniciado con rFirma abierto obliga a reabrirlo. Contra un `pcscd` 4:4 la notificación PnP va por el número de lectores, y llega igual de rápido. Con el DNIe, medidas la lectura sin PIN y la detección en caliente contra un `pcscd` 4:5 y uno 4:4, y la firma con PIN desde el flatpak de rFirma contra el 4:5 |
| `pcscd` del anfitrión por `--socket=pcsc`, ni demonio propio ni `--device=all` | **Se sostiene** | Medidos el socket, la variable, el protocolo y `access_pcsc`. Sin `pcscd` en el anfitrión, el estado es «sin lector» desde el arranque y hasta reabrir |
| Solo se empaqueta OpenSC, con versión y `sha256` fijos | **Se sostiene** | 0.27.1 con el `sha256` de [`flatpak-canal-unico.md`](flatpak-canal-unico.md), comprobado. Añadir `--enable-zlib --enable-openssl --enable-sm` para que falte lo que falte la construcción se pare; `--disable-openpace` y `--disable-readline` quitan lo que el DNIe no usa. Subir de versión con la siguiente publicación |
| rFirma lo descubre por un `.module` en `/app/share/p11-kit/modules`, con raíz `/app` solo en el canal flatpak | **Se sostiene** | El `.module` lo instala el propio OpenSC con `--enable-p11_system_config_modules=/app/share/p11-kit/modules`; no hace falta escribirlo en el manifiesto. Resuelve a `/app/lib/pkcs11/opensc-pkcs11.so` con el código actual de `p11kit` |
| Con el protocolo de `pcscd` incompatible, «sin lector»; la versión del cliente cubre las distribuciones con soporte | **Se sostiene, y 2.5.1 vale** | Cubre todo `pcscd` desde la 1.8.24; el único caso incompatible, la 1.8.23 o anteriores, da «sin lector» con el vigilante actual. La regla para el futuro: no bajar el cliente de la 2.4.1, que es la primera que reintenta con la versión del servidor |
| `opensc.conf` por omisión | **Se sostiene** | Es `app default { }`; con él la caché de ficheros va apagada (0.27) y la ventana de confirmación no existe, porque no se compila |

## Reproducir

Nada de esto toca el árbol: todo vive en `/tmp/dnie-flatpak`.

```bash
# Fuentes
git clone https://github.com/LudovicRousseau/PCSC.git /tmp/dnie-flatpak/src/pcsc
git clone --filter=blob:none https://github.com/flatpak/flatpak.git /tmp/dnie-flatpak/src/flatpak
git clone --filter=blob:none https://github.com/OpenSC/OpenSC.git /tmp/dnie-flatpak/src/opensc
git clone --filter=blob:none https://github.com/polkit-org/polkit.git /tmp/dnie-flatpak/src/polkit

# Protocolo por etiqueta (en zsh, ${t} entre llaves: "$t:src" es un modificador)
for t in 1.8.23 1.9.9 2.2.3 2.3.0 2.4.1 2.5.1; do
  git -C /tmp/dnie-flatpak/src/pcsc show "${t}:src/winscard_msg.h" | grep 'define PROTOCOL_VERSION_'
done

# La sonda: el manifiesto de card-probes/flatpak_probe.sh, instalado con --user
flatpak-builder --user --install --force-clean builddir me.sgomez.RfirmaDnieProbe.yml

# Un pcscd de una etiqueta, en el SDK, sin USB ni polkit, con ipcdir propio
flatpak run --filesystem=/tmp/dnie-flatpak --command=sh org.gnome.Sdk//50 \
  /tmp/dnie-flatpak/scripts/build-pcscd.sh 1.9.9

# El cliente del sandbox contra ese demonio
PCSCLITE_CSOCK_NAME=/tmp/dnie-flatpak/run/1.9.9/pcscd.comm \
  flatpak run --env=PCSCLITE_DEBUG=0 --command=testpcsc me.sgomez.RfirmaDnieProbe

flatpak uninstall --user -y me.sgomez.RfirmaDnieProbe
```

`build-pcscd.sh` exporta la etiqueta con `git archive` y la compila con meson
(`-Dlibudev=false -Dlibusb=false -Dlibsystemd=false -Dpolkit=false -Dusb=false -Dipcdir=…`) o, antes de
la 2.2, con autotools y las mismas opciones. `matrix.sh` lanza cada demonio con `--foreground --debug`
y lo recorre con `testpcsc`; `stale.sh` reinicia un demonio con un sandbox abierto. La sonda de la
medición instalaba además `testpcsc` en `/app/bin` (`post-install` del módulo `pcsc-lite`), que no
hace falta en el manifiesto de rFirma.
