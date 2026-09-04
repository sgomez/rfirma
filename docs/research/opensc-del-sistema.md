# El OpenSC del sistema: qué traen las distribuciones y qué cambia fuera del flatpak

Sondeo del [#226](https://github.com/sgomez/rfirma/issues/226), hijo del mapa
[#217](https://github.com/sgomez/rfirma/issues/217). El mapa decide que el flatpak y los
paquetes nativos `.deb`/`.rpm` **convivan**, y con ello acepta una asimetría: dentro del
flatpak el módulo PKCS#11 lo compila el propio manifiesto y sabemos exactamente cuál es;
en un `.deb` o un `.rpm` es el del sistema, una versión que no controlamos. Este informe
son los hechos para escribir esa decisión bien.

**Respuesta corta.** La asimetría de **versión** es grande —de OpenSC 0.22.0 (agosto de
2021) a 0.27.1 (marzo de 2026), casi cinco años— pero **no es el riesgo principal**: entre
0.22.0 y 0.27.1 el registro de cambios de OpenSC no trae ni una sola entrada sobre el DNIe
o la FNMT, y las regresiones graves conocidas contra la firma con DNIe son todas anteriores
a 0.22.0. Los tres riesgos reales son otros, y los tres son de **empaquetado**, no de
versión:

1. **`opensc-pkcs11` no depende de `pcscd` ni de `libpcsclite1`** — OpenSC abre
   `libpcsclite.so.1` con `dlopen`, así que ninguna de las dos aparece en su `Depends`
   (medido más abajo). Un `.deb` que solo dependa de `opensc-pkcs11` puede acabar en una
   máquina sin PC/SC ninguno, y **el fallo es silencioso**: `C_Initialize` devuelve
   `CKR_OK` y `C_GetSlotList` devuelve cero ranuras, exactamente igual que si no hubiera
   lector conectado.
2. **Los nombres de paquete no coinciden y ninguno de los dos que supone el ticket es
   exacto.** En Debian/Ubuntu el módulo está en `opensc-pkcs11` (correcto), pero en Fedora
   **no está en `opensc`: está en `opensc-libs`**.
3. **La lista de rutas de `stores.rs` cubre las seis distribuciones estudiadas en x86_64 y
   no le falta ninguna**, pero está escrita solo para `x86_64-linux-gnu`: en un `.deb` de
   `arm64` no encontraría nada.

Y una cuarta cosa, que es la que decide la pregunta 4: **sin tarjeta no se puede firmar
contra OpenSC**, ni real ni emulada barata. La grada B contra el OpenSC del sistema solo
puede ser una prueba de humo, y llegar más lejos cuesta la receta completa de la CI del
propio OpenSC.

---

## 0. El punto de partida: lo que hay dentro del flatpak

Del manifiesto `packaging/flatpak/me.sgomez.rfirma.yml`, que es lo que midió el
[#22](https://github.com/sgomez/rfirma/issues/22) y está en
`docs/research/flatpak-canal-unico.md` §3:

| | Dentro del flatpak | Fuera (`.deb`/`.rpm`) |
| --- | --- | --- |
| OpenSC | **0.27.1**, compilado por el manifiesto | del sistema: **0.22.0 … 0.27.1** |
| pcsc-lite | **2.5.1**, solo la librería cliente | del sistema: **1.9.5 … 2.4.1** |
| `pcscd` | del anfitrión, por `--socket=pcsc` | del anfitrión, directo |
| Ruta del módulo | `/app/lib/pkcs11/opensc-pkcs11.so` | seis rutas distintas (§2) |
| `opensc.conf` | `/app/etc/opensc/opensc.conf`, de solo lectura | `/etc/opensc/opensc.conf`, editable |

Esa última fila es una asimetría que no estaba en el ticket y merece un párrafo propio en
§3: el flatpak compila OpenSC con prefijo `/app`, así que autotools le deja el `sysconfdir`
en `/app/etc`. Un usuario que ajuste `/etc/opensc/opensc.conf` en su anfitrión —que es
exactamente lo que dicen los manuales del DNIe cuando el PIN se pide en cada firma— **no
tiene ningún efecto dentro del flatpak**, y el fichero de dentro es de solo lectura. En el
`.deb` y el `.rpm` sí lo tiene. Es la asimetría de mayor consecuencia práctica de todo el
informe, porque afecta a un ajuste que la gente sí toca.

---

## 1. Qué versión trae cada distribución, y cómo se llama el paquete

### Las versiones de OpenSC upstream, con fecha

Del [listado de etiquetas del repositorio](https://github.com/OpenSC/OpenSC/tags), que es
la única página que muestra el año:

| Versión | Fecha |
| --- | --- |
| 0.22.0 | 2021-08-10 |
| 0.23.0 | 2022-11-29 |
| 0.24.0 | 2023-12-13 |
| 0.25.0 | 2024-03-06 |
| 0.25.1 | 2024-04-05 |
| 0.26.0 | 2024-11-13 |
| 0.26.1 | 2025-01-14 |
| 0.27.0-rc1 | 2026-02-24 |
| 0.27.0-rc2 | 2026-03-13 |
| 0.27.0 | 2026-03-30 |
| **0.27.1** | **2026-03-31** ← la del flatpak |

### Lo que trae cada distribución

| Distribución | Publicada | Paquete del módulo | Versión de OpenSC | Fuente |
| --- | --- | --- | --- | --- |
| Debian 12 *bookworm* | 2023-06-10 | `opensc-pkcs11` | `0.23.0-0.3+deb12u2` | [packages.debian.org](https://packages.debian.org/bookworm/opensc-pkcs11) |
| Debian 13 *trixie* | 2025-08-09 | `opensc-pkcs11` | `0.26.1-2` | [packages.debian.org](https://packages.debian.org/trixie/opensc-pkcs11) |
| Ubuntu 22.04 LTS | 2022-04-21 | `opensc-pkcs11` (*universe*) | `0.22.0-1ubuntu2` | [packages.ubuntu.com](https://packages.ubuntu.com/jammy/opensc-pkcs11) |
| Ubuntu 24.04 LTS | 2024-04-25 | `opensc-pkcs11` (*universe*) | **`0.25.0~rc1-1ubuntu0.2`** | [packages.ubuntu.com](https://packages.ubuntu.com/noble/opensc-pkcs11) |
| Ubuntu 26.04 LTS | 2026-04-23 | `opensc-pkcs11` (*universe*) | **`0.27.0~rc1-1`** | medido en esta máquina (`dpkg -l`) |
| Fedora 43 | 2025-10-28 | **`opensc-libs`** | `0.26.1-3.fc43` en GA, **`0.27.1-2.fc43`** hoy vía *updates* | [packages.fedoraproject.org](https://packages.fedoraproject.org/pkgs/opensc/opensc/) |
| Fedora 44 | — | **`opensc-libs`** | `0.27.1-2.fc44` | ídem |

Tres cosas que salen de esta tabla y que el ticket no anticipaba:

**Dos de las tres LTS vigentes de Ubuntu congelan un *release candidate*.** 24.04 lleva
`0.25.0~rc1` cuando la 0.25.0 final salió el 2024-03-06, cinco semanas antes de la
publicación de la distribución; 26.04 lleva `0.27.0~rc1` cuando la 0.27.0 final salió el
2026-03-30 y la 0.27.1 el 2026-03-31, ambas antes del 2026-04-23. No es un descuido nuestro
ni de esta máquina: es lo que hay en el archivo. Cualquier frase del spec que diga «la
versión de OpenSC de Ubuntu 24.04» tiene que decir *release candidate*, porque a la hora de
reportar un fallo upstream la diferencia importa.

**En Ubuntu, OpenSC está en `universe`.** Es decir, sin compromiso de soporte de seguridad
de Canonical; lo mantiene la comunidad. Que la aplicación de firma dependa de un paquete de
`universe` es un hecho que el ADR debe decir en voz alta, aunque no cambie la decisión.

**En Fedora el nombre correcto no es `opensc`.** El SRPM `opensc` produce dos subpaquetes:
`opensc`, que solo lleva las herramientas de `/usr/bin` (`pkcs11-tool`, `opensc-tool`…), y
**`opensc-libs`, que es el que lleva `opensc-pkcs11.so`**. Comprobado en la lista de
ficheros y en las dependencias de
[`opensc` en Fedora 43](https://packages.fedoraproject.org/pkgs/opensc/opensc/fedora-43.html)
y de
[`opensc-libs` en Fedora 43](https://packages.fedoraproject.org/pkgs/opensc/opensc-libs/fedora-43.html).

---

## 2. Dónde queda el módulo, y si la lista de `stores.rs` está completa

### Las rutas, verificadas contra las listas de ficheros de los paquetes

| Distribución | Rutas del módulo | Fichero de p11-kit |
| --- | --- | --- |
| Debian 12/13, Ubuntu 22.04/24.04/26.04 | `/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so` **y** `/usr/lib/x86_64-linux-gnu/pkcs11/opensc-pkcs11.so` | `/usr/share/p11-kit/modules/opensc-pkcs11.module` |
| Fedora 43/44 | `/usr/lib64/opensc-pkcs11.so` **y** `/usr/lib64/pkcs11/opensc-pkcs11.so` | `/usr/share/p11-kit/modules/**opensc**.module` |

En Debian/Ubuntu la segunda ruta es un **enlace simbólico** a la primera. Medido en esta
máquina:

```
$ ls -l /usr/lib/x86_64-linux-gnu/pkcs11/opensc-pkcs11.so
lrwxrwxrwx 1 root root 19 /usr/lib/x86_64-linux-gnu/pkcs11/opensc-pkcs11.so -> ../opensc-pkcs11.so
```

Eso ya está resuelto: `present_among` canonicaliza cada candidato antes de quedárselo, así
que las dos rutas colapsan en un solo almacén y la ventana no enseña el mismo módulo dos
veces. La prueba que lo fija es
`lists_the_same_module_once_even_under_two_names`, en `stores.rs`.

En Fedora el `%{_libdir}/pkcs11/` es **política de empaquetado**, no una casualidad: la
guía de Fedora obliga a instalar ahí los proveedores PKCS#11
([`PackagingDrafts/Pkcs11Support`](https://fedoraproject.org/wiki/PackagingDrafts/Pkcs11Support)).

Nótese el nombre del `.module`: en Debian/Ubuntu es `opensc-pkcs11.module` y en Fedora
`opensc.module`. No nos afecta hoy —no leemos p11-kit— pero sí afectaría a la variante que
se propone abajo.

### Veredicto sobre `CANDIDATE_MODULES`

De las nueve entradas de la lista (`rfirma-app/src-tauri/src/pkcs11/stores.rs:26`), las seis
de OpenSC cubren **las seis distribuciones estudiadas, en x86_64, sin que falte ninguna**.
Las dos que no salen en la tabla —`/usr/lib/opensc-pkcs11.so` y
`/usr/lib/pkcs11/opensc-pkcs11.so`— cubren Arch, y la de `/usr/lib64/` cubre además
openSUSE. La respuesta a la pregunta 2 del ticket es: **la lista está completa para lo que
el mapa va a empaquetar.**

Con dos huecos que sí conviene anotar:

**La arquitectura está clavada a mano.** `x86_64-linux-gnu` aparece literal en la constante,
y en Debian/Ubuntu la ruta *multiarch* depende de la arquitectura: en `arm64` sería
`/usr/lib/aarch64-linux-gnu/`. Hoy no importa —el flatpak no declara arquitectura y todo lo
que se construye es amd64— pero el `.deb` de `arm64` sí existiría el día que alguien lo
pida, y el fallo sería «no aparece ningún certificado» sin más pista. Se arregla con un
`std::env::consts::ARCH` o, más simple, añadiendo `/usr/lib/aarch64-linux-gnu/…` a la lista;
el coste de una entrada de más en un array es cero, porque `present_among` la descarta si no
existe.

**No se consulta p11-kit.** Las seis distribuciones instalan un `.module` en
`/usr/share/p11-kit/modules/`, y `libp11-kit.so` está en todas ellas y también en
`org.gnome.Platform`. Cargar `p11-kit-proxy.so` daría *todos* los módulos que el sistema
declara —OpenSC, SoftHSM, gnome-keyring y el del fabricante que el usuario haya instalado a
mano— sin lista de rutas ninguna. **No se recomienda hacerlo**, y por la razón que ya
escribe el propio `stores.rs`: sería «dejar que el entorno decida con qué se firma», y
además el proxy fusiona los módulos en un espacio de ranuras único, lo que enturbia el
«qué módulo firmó esto» que la aplicación necesita poder decir. Se anota aquí para que la
próxima persona que lo proponga encuentre el argumento ya escrito, no para adoptarlo.

---

## 3. Qué se rompe entre versiones

### El registro de cambios no dice nada del DNIe desde 2018

Revisadas las notas de versión de 0.22.0 a 0.27.1 y el
[`NEWS`](https://github.com/OpenSC/OpenSC/blob/master/NEWS) del repositorio: **ninguna
entrada menciona DNIe, DNI, España, FNMT, CERES ni FNMT-RCM** en ese rango. La única
aparición de `dnie` en toda la franja es en 0.22.0, dentro de una entrada genérica sobre
*fuzzing* que enumera drivers retocados por informes de oss-fuzz y Coverity.

Hay que retroceder a 2017–2018 para encontrar entradas sobre el DNIe: «Added support for
DNIe 3.0» en **0.17.0** (2017-07-18), y «fixed DNIe UI compatibility» en **0.18.0** y
**0.19.0**.

### Las roturas graves conocidas son todas anteriores a 0.22.0

| Fallo | Qué pasaba | Cuándo se arregló |
| --- | --- | --- |
| [#2105 / PR #2109](https://github.com/OpenSC/OpenSC/pull/2109) | La Dirección General de la Policía cambió la estructura de CA del canal seguro en los DNIe con serie a partir de `BMP100001`; el login y la firma devolvían `CKR_DATA_INVALID` | fusionado el **2020-09-25** |
| [#1246](https://github.com/OpenSC/OpenSC/issues/1246) | Tras la revocación masiva por CVE-2017-15361, el DNIe 3.0 dejó de firmar (`CKR_FUNCTION_NOT_SUPPORTED` en `C_SignFinal`) | 2018 |
| [#1036](https://github.com/OpenSC/OpenSC/issues/1036) | El `logout` marcaba cerrado el canal seguro CWA14890 sin cerrarlo, rompiendo la segunda firma del mismo proceso | 2017 |

Las tres son anteriores a 0.22.0, o sea que **están arregladas en todas las versiones que
traen las distribuciones de la tabla de §1**.

**No se ha encontrado ninguna versión, entre las que traen esas seis distribuciones, con un
fallo documentado que impida firmar con DNIe o con una tarjeta de la FNMT.** Es la respuesta
directa a la pregunta 3 del ticket, y es una respuesta tranquilizadora. Se dice con la
cautela debida: significa «no está documentado en las fuentes primarias consultadas», no
«se ha comprobado que las seis firman».

### Dos cosas que sí conviene saber, y que no son fallos

**El contactless del DNIe no está soportado, y no lo estará.** El issue
[#2258](https://github.com/OpenSC/OpenSC/issues/2258) reporta que `card-dnie.c` solo
reconoce los ATR de contacto, y **se cerró como *not planned***. OpenSC integra
`libeac`/OpenPACE (es la `libeac.so.3` que el #22 vio faltar dentro del flatpak), pero ese
soporte PACE es para el nPA alemán, no para el DNIe. Consecuencia para rfirma: un lector NFC
no vale; hace falta lector de contacto.

**OpenSC no clasifica el DNIe como tarjeta plenamente soportada.** La
[wiki de hardware soportado](https://github.com/OpenSC/OpenSC/wiki/Supported-hardware-%28smart-cards-and-USB-tokens%29)
sitúa el *Spanish eID* entre las eID de soporte «unclear/unsupported», y la
[página del driver](https://github.com/OpenSC/OpenSC/wiki/DNIe-(OpenDNIe)) documenta
limitaciones del propio chip (sin canales lógicos ISO 7816-4; clave pública y privada en el
mismo fichero, lo que rompe `pkcs15-tool --read-public-keys`). No hay alternativa mejor
—el propio portal oficial [dnielectronico.es](https://www.dnielectronico.es) distribuye un
*Manual de Instalación y Configuración de OpenSC-DNIe* como la vía para Unix— pero la
expectativa hay que calibrarla.

**La FNMT publica su propio módulo, distinto de OpenSC**, para sus tarjetas CERES:
`libpkcs11-fnmtdnie`, en `.deb`, desde
[sede.fnmt.gob.es](https://www.sede.fnmt.gob.es/descargas/descarga-software). Quien lo tenga
instalado no lo pondrá en ninguna de las rutas de `CANDIDATE_MODULES`, así que hoy rfirma no
lo encontraría. Es material para el **selector real de módulo PKCS#11** (ficha 17b del mapa),
no para este informe: la escotilla existe (`RFIRMA_PKCS11_MODULE`), lo que falta es que se
pueda elegir desde la ventana.

### El ajuste que sí cambia entre canales: `opensc.conf`

`opensc.conf` tiene cuatro claves que tocan directamente al DNIe, documentadas en
`etc/opensc.conf.example.in` y usadas por `src/libopensc/card-dnie.c`:

- `user_consent_enabled` — el driver del DNIe pide confirmación antes de cada firma.
- `user_consent_app` — la herramienta que la pide (por omisión `/usr/bin/pinentry`), solo si
  se compiló con `--enable-dnie-ui`.
- `pin_cache_ignore_user_consent` y `use_pin_caching` — la pareja que los manuales
  recomiendan activar para que una aplicación que no maneja `CKA_ALWAYS_AUTHENTICATE` no
  tenga que pedir el PIN en cada operación.

No se ha podido fijar en qué versión concreta cambió el comportamiento por defecto de estas
claves; el `NEWS` no lo dice y no se ha encontrado el issue. **La premisa del ticket sobre un
cambio en 0.23/0.24 no se da por confirmada.**

Lo que sí está medido es dónde vive el fichero. En esta máquina:

```
$ cat /etc/opensc/opensc.conf
app default {
	# debug = 3;
	# debug_file = opensc-debug.txt;
}
```

Cuatro líneas, todo comentado: los valores en vigor son los compilados. Y en el flatpak ese
fichero **no es ese**, sino `/app/etc/opensc/opensc.conf`, de solo lectura. Un usuario que
siga un manual del DNIe y edite el de su sistema verá el cambio en el `.deb` y no lo verá en
el flatpak, sin ningún mensaje que se lo explique. Si el spec quiere cerrar ese agujero, la
palanca es la variable `OPENSC_CONF`, que el manifiesto no toca hoy.

---

## 4. Cómo se prueba contra el OpenSC del sistema

### Lo que se puede probar sin tarjeta, medido

En esta máquina (Ubuntu 26.04.1, OpenSC 0.27.0rc1, `pcscd.socket` activo, **sin lector**):

```
$ pkcs11-tool --module /usr/lib/x86_64-linux-gnu/opensc-pkcs11.so --show-info
No slots.
Cryptoki version 3.0
Manufacturer     OpenSC Project
Library          OpenSC smartcard framework (ver 0.27)

$ pkcs11-tool --module /usr/lib/x86_64-linux-gnu/opensc-pkcs11.so --list-slots
No slots.
Available slots:
$ echo $?
0
```

El módulo **carga, `C_Initialize` devuelve `CKR_OK`, `C_GetInfo` contesta y `C_GetSlotList`
devuelve cero ranuras sin error**. Es el mismo resultado que midió el #22 dentro del
flatpak.

Y lo mismo ocurre si `pcscd` no está corriendo siquiera. En `reader-pcsc.c`, `pcsc_init`
solo hace el `dlopen` de `libpcsclite`; el fallo llega después, en `pcsc_detect_readers`,
cuando `SCardEstablishContext` devuelve `SCARD_E_NO_SERVICE`, que se traduce a
`SC_ERROR_NO_READERS_FOUND`; y en `pkcs11-global.c` la lista de ranuras vacía se reporta
como `*pulCount = 0` con `CKR_OK`
([código](https://github.com/OpenSC/OpenSC/blob/master/src/pkcs11/pkcs11-global.c)).

**Consecuencia dura: no hay forma de distinguir «no hay `pcscd`» de «hay `pcscd` pero no hay
lector» ni de «hay lector pero no hay tarjeta» mirando el retorno de `C_GetSlotList`.** Las
tres son cero ranuras y `CKR_OK`. Cualquier diagnóstico que rfirma quiera dar sobre esto
tendrá que salir de otro sitio (existencia del socket, por ejemplo), no del módulo.

### La costura ya existe

`tests/pkcs11_token.rs:86` ya lee **`RFIRMA_PKCS11_MODULE`** para elegir el módulo, con
SoftHSM por omisión, y `stores::from_environment` respeta la misma variable y, cuando está
puesta, manda ella sola. No hay que inventar ningún mecanismo: hay que decidir **qué se
ejecuta** cuando esa variable apunta a OpenSC.

Lo que no es reutilizable son los datos: los cinco certificados, sus etiquetas
`FNMT-*-99999999R`, sus `CKA_ID` y el PIN `1234` son del token `rfirma-test` de SoftHSM y no
existen en ninguna otra parte. Contra OpenSC sin tarjeta, cada una de esas pruebas fallaría
en la primera aserción.

### Las tres opciones, con su coste

| Opción | Qué prueba | Coste en CI | Veredicto |
| --- | --- | --- | --- |
| **A. Prueba de humo** contra el `opensc-pkcs11.so` del sistema | que el `.so` de la distro carga, que `C_Initialize` no revienta contra nuestra versión de `cryptoki`, que `C_GetInfo` devuelve una versión de Cryptoki que sabemos manejar, y que cero ranuras **no es un error** para nuestro código sino una lista vacía | ~0 s; `opensc` ya está instalado en el carril rápido (`ci.yml` lo pone) | **recomendada** |
| **B. Tarjeta emulada**, la receta de la CI de OpenSC | el ciclo completo: login, listado, firma, contra un `opensc-pkcs11.so` de verdad | alto (ver abajo) | fuera del carril rápido; decisión propia |
| **C. p11-kit remoting** | nada nuevo | medio | descartada |

**Sobre B.** La CI del propio OpenSC (`.github/workflows/linux.yml`, objetivo `cac`) no usa
`vpicc` a secas ni `sc-hsm-embedded`: encadena **`vpcd`** (el lector PC/SC virtual de
`vsmartcard`) + **`libcacard`** (la tarjeta CAC emulada de SPICE/QEMU) + **`virt_cacard`**
(el puente entre ambos) + **SoftHSM2** como fuente de los certificados que se cargan dentro
de la tarjeta emulada, con `pcscd` relanzado en primer plano (`sudo /usr/sbin/pcscd -f`) y
una regla de `polkit` para que el usuario del runner pueda hablar con él sin sesión gráfica.
De esos cuatro, solo `vsmartcard-vpcd` está empaquetado (Ubuntu 26.04, `universe`,
`3.3+dfsg-2ubuntu2`); `libcacard` y `virt_cacard` se compilan desde el fuente en cada
ejecución. Y la tarjeta resultante es una **CAC**, no un DNIe ni una FNMT: se ejercita el
driver PKCS#15 genérico de OpenSC, no `card-dnie.c`. No se ha encontrado ningún proyecto que
emule un DNIe en CI.

**Sobre C.** `p11-kit server` + `p11-kit-client.so` mueven un módulo a través de un socket,
pero **sigue haciendo falta una tarjeta detrás**: resuelve dónde corre el proceso, no la
ausencia de tarjeta. No sirve para lo que aquí se busca.

### La recomendación

Que la grada B se parta en dos, sin cambiar el ADR-0014 más que para nombrarlo:

- **B (SoftHSM)** — lo de hoy, sin tocar. Firma de verdad, carril rápido, segundos.
- **B′ (OpenSC del sistema)** — un fichero nuevo y pequeño, del orden de tres o cuatro
  pruebas, que corre en el mismo carril rápido y comprueba la opción A de la tabla. No
  necesita `pcscd` ni un solo paquete nuevo: `ci.yml` ya instala `opensc` en los dos jobs de
  Rust, y `just tools` ya lo exige para `pkcs11-tool`.

Lo que B′ compra no es poco: es **la única prueba automática de que el `.deb` y el `.rpm`
declaran las dependencias correctas y de que la lista de rutas encuentra el módulo que la
distribución instaló**. Y es la que se pondría roja el día que una distribución mueva el
`.so`.

Lo que B′ **no** compra, y hay que escribirlo donde se lea: que se pueda firmar con una
tarjeta. Eso sigue siendo manual, con un lector y un DNIe encima de la mesa, igual que la
puerta manual de VALIDe que el ADR-0014 ya reconoce. La opción B de la tabla es la única vía
automática, y es una decisión propia con su propio coste; no debe colarse dentro de este
ticket.

---

## 5. `pcscd`: quién lo arranca y qué tiene que decir el paquete

### Nombres y versiones

| Distribución | Demonio | Librería cliente | Versión |
| --- | --- | --- | --- |
| Debian 12 | `pcscd` | `libpcsclite1` | 1.9.9-2 |
| Debian 13 | `pcscd` | `libpcsclite1` | 2.3.3-1 |
| Ubuntu 22.04 | `pcscd` | `libpcsclite1` | 1.9.5-3 |
| Ubuntu 24.04 | `pcscd` | `libpcsclite1` | 2.0.3-1build1 |
| Ubuntu 26.04 | `pcscd` | `libpcsclite1` | 2.4.1-1 (medido aquí) |
| Fedora 43 | **`pcsc-lite`** | **`pcsc-lite-libs`** | 2.3.3-2.fc43 |
| Fedora 44 | **`pcsc-lite`** | **`pcsc-lite-libs`** | 2.4.1-2.fc44 |

### ¿Se arranca solo?

Sí, por **activación de socket de systemd**, en todas. El paquete instala
`/usr/lib/systemd/system/pcscd.service` y `/usr/lib/systemd/system/pcscd.socket`
(verificado en la lista de ficheros de `pcscd` en trixie y medido en esta máquina), y el
socket queda habilitado al instalar: `dh_installsystemd` en Debian/Ubuntu, `%systemd_post`
en Fedora. Medido aquí:

```
$ systemctl is-enabled pcscd.socket   → enabled
$ systemctl is-active  pcscd.socket   → active
```

Hubo un fallo histórico en Ubuntu por el que `pcscd.socket` quedaba **deshabilitado** tras
instalar, porque `dh_installsystemd` no gestionaba las unidades bajo `/usr/lib/systemd/system`
en vez de `/lib/systemd/system`
([Launchpad #1971984](https://bugs.launchpad.net/bugs/1971984)); se corrigió por SRU en julio
de 2023 en jammy y lunar. Está arreglado en todo lo que empaquetaríamos, pero explica por
qué medio internet dice «acuérdate de `systemctl enable pcscd.socket`».

### El hallazgo que decide las dependencias del paquete

**OpenSC abre `libpcsclite.so.1` con `dlopen`, no enlaza contra ella.** Medido aquí:

```
$ ldd /usr/lib/x86_64-linux-gnu/opensc-pkcs11.so | grep -c pcsclite
0
$ strings /usr/lib/x86_64-linux-gnu/libopensc.so.13 | grep libpcsclite
libpcsclite.so.1
$ opensc-tool --info
OpenSC 0.27.0rc1 [gcc  15.2.0]
Enabled features: locking zlib readline openssl pcsc(libpcsclite.so.1)
```

Como no hay enlace, `dpkg-shlibdeps` no genera ninguna dependencia, y el paquete lo
confirma:

```
$ apt-cache show opensc-pkcs11 | grep ^Depends
Depends: libc6, libeac3, libglib2.0-0t64, libssl3t64, zlib1g
```

Ni `libpcsclite1` ni `pcscd`. Igual en Debian 13
([`opensc-pkcs11` 0.26.1-2](https://packages.debian.org/trixie/opensc-pkcs11)). El paquete
`opensc` (las herramientas) sí añade `Recommends: pcscd` — pero es un *Recommends*, y
`--no-install-recommends` lo salta, que es exactamente lo que hace nuestro propio `ci.yml`.

En Fedora la cosa está mejor repartida pero también hay que mirarla: `opensc-libs` requiere
**`pcsc-lite-libs`** (la librería cliente, no el demonio) y `opensc` requiere
**`pcsc-lite`** (el demonio).

De ahí salen las dependencias que el spec tiene que escribir:

| Paquete | Depende de | Por qué |
| --- | --- | --- |
| `.deb` | `opensc-pkcs11` | el `.so`; `opensc` a secas ya lo arrastra, pero nombrarlo es más honesto |
| `.deb` | `pcscd` — **`Depends`, no `Recommends`** | nadie más lo va a traer, y sin él no hay tarjeta |
| `.rpm` | `opensc-libs` | el `.so`; **no `opensc`**, que solo lleva las herramientas |
| `.rpm` | `pcsc-lite` | `opensc-libs` solo pide `pcsc-lite-libs`, que es la librería, no el demonio |

Se recomienda `Depends`/`Requires` y no `Recommends`, contra la costumbre: un cliente de
firma sin PC/SC **no puede hacer su trabajo**, y el modo de fallo es cero ranuras con
`CKR_OK`, o sea silencio. El coste de la dependencia dura es un demonio activado por socket
que no consume nada mientras no haya lector; el coste de la blanda es una persona mirando una
lista de certificados vacía sin saber por qué.

---

## Lo que este informe deja decidido

1. **La asimetría es aceptable, y la de versión no es el problema.** Ninguna de las seis
   versiones que empaquetaríamos tiene un fallo documentado que impida firmar con DNIe o
   FNMT, y el registro de cambios no toca esos drivers desde antes de 0.22.0.
2. **La asimetría de configuración sí es real y no estaba en el ticket**: el
   `opensc.conf` del flatpak es `/app/etc/opensc/opensc.conf`, de solo lectura, y el ajuste
   del anfitrión no le llega.
3. **`CANDIDATE_MODULES` no necesita entradas nuevas** para lo que el mapa va a empaquetar,
   solo la variante de arquitectura si algún día hay `.deb` de `arm64`.
4. **Los nombres de paquete**: `opensc-pkcs11` + `pcscd` en el `.deb`; `opensc-libs` +
   `pcsc-lite` en el `.rpm`. Todos como dependencia dura.
5. **La grada B se amplía con una B′** de prueba de humo contra el OpenSC del sistema, en el
   carril rápido y a coste cero. Firmar contra una tarjeta emulada es otra decisión, con la
   receta de la CI de OpenSC como única vía conocida y ningún proyecto que emule un DNIe.

## Lo que no se ha medido

- **No hay lector ni tarjeta.** Todo lo de §4 sobre «cero ranuras» está medido; nada sobre
  el camino `opensc → libpcsclite → pcscd → tarjeta` lo está. Es la misma laguna honesta que
  ya declara `docs/research/flatpak-canal-unico.md`.
- **No se ha instalado ninguna de las seis distribuciones** para comprobar las rutas: salvo
  Ubuntu 26.04, que es esta máquina, las rutas vienen de las listas de ficheros de los
  paquetes publicados.
- **La versión de Fedora 43** aparece como `0.26.1-3.fc43` en la ficha por versión y como
  `0.27.1-2.fc43` en la tabla general de la misma web; se interpreta como GA frente a
  *updates*, pero no se ha contrastado con una tercera fuente.
- **Las claves `user_consent_*` de `opensc.conf`** están confirmadas en el fuente, pero no la
  versión en la que cambió su valor por omisión.
