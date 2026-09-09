# Dónde busca AutoFirma a Firefox y a NSS, y qué de eso alcanza rFirma

Barrido del código de [`ctt-gob-es/clienteafirma`](https://github.com/ctt-gob-es/clienteafirma)
en el tag **v1.9.2** (`b4fe147c3`), de sus PR e issues abiertas sobre el asunto,
y contraste con lo que hace hoy `pkcs11::stores` bajo el sandbox del flatpak.
Nace de la sospecha de que rFirma no cubre todas las variantes de empaquetado de
Firefox que sí cubre el original.

**Respuesta corta: la sospecha es cierta y el hueco tiene un solo nombre, Snap.**
De todo el catálogo del upstream, lo único que a rFirma le falta y le importa es
`~/snap/firefox/common/.mozilla/firefox` —el perfil de Firefox en Ubuntu desde
la 22.04, que es el Firefox por omisión de la distribución con más usuarios del
público de rFirma— y su gemelo de Chromium, cuyo almacén se mudó a la ruta XDG
y ya no está donde el upstream lo busca (ver 5.1). Se ha medido **sobre un
Ubuntu con los dos snaps instalados** que el sandbox llega ahí: basta un
`--filesystem=~/snap/firefox/common/.mozilla/firefox` en el manifiesto, y no hace
falta `--filesystem=home`. Todo lo demás del catálogo del upstream —las quince
rutas de `libsoftokn3.so`, `/opt/firefox-<versión>`, `firefox-esr`,
`compatibility.ini`, la precarga de dependencias, el `ElfParser`— **no aplica a
rFirma por construcción**: rFirma carga el `libsoftokn3.so` del runtime del
flatpak, no el del anfitrión, y esa decisión ya está tomada (ADR-0004).

Y hay una vuelta al argumento que conviene decir entera: **en la localización de
perfiles rFirma va por delante del original, no por detrás.** AutoFirma se queda
con **un** perfil (el activo, o el marcado por omisión, o el más reciente, o el
primero), y eso es exactamente el
[issue 233](https://github.com/ctt-gob-es/clienteafirma/issues/233) y el
[86](https://github.com/ctt-gob-es/clienteafirma/issues/86) del upstream, sin
resolver desde 2022. `stores::nss_profiles` los recorre **todos**, en los dos
layouts, y descarta los que no tienen `cert9.db`. La ruta XDG de Firefox 147
(`~/.config/mozilla/firefox`) tampoco está en v1.9.2: es el
[issue 560](https://github.com/ctt-gob-es/clienteafirma/issues/560) del upstream
y dos PR sin mergear, y rFirma la lleva desde el #246.

---

## 1. Qué hace AutoFirma hoy (v1.9.2)

### 1.1 Dónde busca las bibliotecas NSS

El punto de entrada es `MozillaKeyStoreUtilities.getSystemNSSLibDir()`
(`afirma-keystores-mozilla/src/main/java/es/gob/afirma/keystores/mozilla/MozillaKeyStoreUtilities.java:215-262`):
mira primero una variable de entorno, y luego deriva al fichero de cada sistema.

**Variable de entorno `AFIRMA_NSS_HOME_ENV`** (`MozillaKeyStoreUtilities.java:58`,
usada en las líneas 226-243). Solo se consulta si la propiedad de sistema Java
`es.gob.afirma.keystores.mozilla.UseEnvironmentVariables` está a `true`
(`MozillaKeyStoreUtilities.java:62`, condición en la 222). Y aun consultada,
**no sirve de nada**: las líneas 245-253 sobrescriben `nssLibDir` con el
resultado del método por plataforma sin mirar si ya tenía valor. Es el
[issue 90](https://github.com/ctt-gob-es/clienteafirma/issues/90), abierto desde
2019 y todavía cierto en v1.9.2.

**Linux/Solaris**, `MozillaKeyStoreUtilitiesUnix.getNssPaths()`
(`MozillaKeyStoreUtilitiesUnix.java:45-103`). La lista se construye en este orden
y luego se poda quitando lo que no sea directorio (líneas 97-101):

| Orden | Ruta | Línea |
|---|---|---|
| 1 | `/usr/lib/x86_64-linux-gnu/nss` (solo si el JRE es de 64) | 51 |
| 2 | `/usr/lib/x86_64-linux-gnu` | 52 |
| 3 | `/usr/lib/i386-linux-gnu/nss` (solo si el JRE es de 32) | 55 |
| 4 | `/usr/lib/i386-linux-gnu` | 56 |
| 5 | `<libdir>/nss` | 59 |
| 6 | `<libdir>` | 60 |
| 7 | `<libdir>/firefox` | 61 |
| 8 | `<libdir>/firefox-<última versión encontrada>` | 64-67 |
| 9 | `<libdir>/thunderbird` | 68 |
| 10 | `/lib64` o `/lib32`, y si no `/lib` | 70-75 |
| 11 | `/opt/firefox` | 77 |
| 12 | `/usr/lib64` o `/usr/lib32`, y si no `/usr/lib` | 80-85 |
| 13 | `/opt/firefox-<última versión encontrada>` | 87-90 |
| 14 | `/opt/fedora-ds/clients/lib` | 92 |
| 15 | `/opt/google/chrome` | 95 |

`<libdir>` es `Platform.getSystemLibDir()`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/Platform.java:327-330`):
`/usr/lib64` o `/usr/lib32` si existe, y si no `/usr/lib`.

El «`<última versión encontrada>`» de las filas 8 y 13 es
`searchLastFirefoxVersion()` (`MozillaKeyStoreUtilitiesUnix.java:149-181`):
lista el directorio, se queda con las entradas que empiezan por `firefox-`,
parsea el resto como versión numérica separada por puntos y devuelve la mayor.
Es el método que revienta con `firefox-esr` —el sufijo no es numérico
([issue 252](https://github.com/ctt-gob-es/clienteafirma/issues/252))— y el que
lanza un `NullPointerException` si `File.list()` devuelve `null` porque el
directorio no se puede leer (PR 313 y 367, ninguna mergeada).

Encontrado el directorio, `getNSSLibDirUnix()`
(`MozillaKeyStoreUtilitiesUnix.java:105-143`) exige dos cosas: que contenga
**`libsoftokn3.so`** (línea 30) y que `ElfParser.archMatches()` diga que la
arquitectura del ELF coincide con la del JRE (línea 111). Después precarga a
mano la primera biblioteca de SQLite que encuentre entre
`mozsqlite3.so`, `libmozsqlite3.so`, `libsqlite3.so.0`, `libnspr4.so`
(`SQLITE_LIBS`, líneas 34-39). Y `getSoftkn3DependenciesUnix()`
(líneas 189-199) declara el orden de precarga completo:
`libnspr4.so`, `libplds4.so`, `libplc4.so`, `libnssutil3.so`, `libsqlite3.so`,
`libmozsqlite3.so`, `libsqlite3.so.0`.

Hay un caso especial de Fedora antiguo de 32 bits en
`MozillaKeyStoreUtilities.loadNSSDependencies()` (líneas 401-427): si existen a
la vez `/usr/lib/libsoftokn3.so` y `/lib/libnspr4.so`, precarga las dos copias
de cada dependencia (`/lib/libmozglue.so`, `/usr/lib/libmozglue.so`, y así con
`libplds4`, `libplc4`, `libnssutil3`, `libsqlite3`, `libmozsqlite3`).

**Windows**, `MozillaKeyStoreUtilitiesWindows.getSystemNSSLibDirWindows()`
(`MozillaKeyStoreUtilitiesWindows.java:87`). No hay lista de rutas: se lee la
clave `LastPlatformDir=` del fichero `compatibility.ini` del perfil activo de
Firefox (`MozillaKeyStoreUtilities.getNssPathFromCompatibilityFile()`, líneas
178-207). Después convierte la ruta a nombre corto 8.3, y si aun así contiene
caracteres fuera de `P11_CONFIG_VALID_CHARS`
(`MozillaKeyStoreUtilitiesWindows.java:35`) copia todo NSS a un directorio
temporal, porque el bug 6581254 de Java no traga esas rutas. Las dieciocho
dependencias (`getSoftkn3DependenciesWindows()`, líneas 221-241) incluyen los
runtimes de Visual C 10, 12 y 14 además de `mozglue.dll`, `nss3.dll`,
`nspr4.dll`, `plc4.dll`, `plds4.dll`, `nssutil3.dll`, `mozsqlite3.dll`,
`sqlite3.dll`, `nssdbm3.dll` y `freebl3.dll`. Si nada de eso vale, cae a un NSS
que la propia aplicación descomprime en `~/.afirma/nss/`
(`BundledNssHelper.java:30-31`, recurso `/nss/<SO>/nss<arch>.zip`, línea 55).

**macOS**, `MozillaKeyStoreUtilitiesOsX.getSystemNSSLibDirMacOsX()`
(`MozillaKeyStoreUtilitiesOsX.java:121-145`), buscando `libsoftokn3.dylib` en:
`/Applications/Firefox.app/Contents/MacOS`, `/lib`, `/usr/lib`, `/usr/lib/nss`,
`/Applications/Minefield.app/Contents/MacOS`. Si la carga directa falla,
`configureMacNSS()` (líneas 36-119) crea **enlaces simbólicos en
`/usr/local/lib` ejecutando un AppleScript como administrador**.

### 1.2 Dónde busca el perfil del usuario

`MozillaKeyStoreUtilities.getProfilesIniPath()` (líneas 482-531):

| Plataforma | Ruta | Línea |
|---|---|---|
| Todas | `$AFIRMA_NSS_PROFILES_INI` (con `%APPDATA%` expandido) | 494-517 |
| Windows | `<AppData>\Mozilla\Firefox\profiles.ini` | 521 |
| macOS | `~/Library/Application Support/Firefox/profiles.ini` | 524 |
| Linux | `~/snap/firefox/common/.mozilla/firefox/profiles.ini`, **si el fichero existe** | 527-528 |
| Linux | `~/.mozilla/firefox/profiles.ini` | 530 |

La variable `AFIRMA_NSS_PROFILES_INI` (línea 60) sí funciona, a diferencia de su
hermana de NSS, pero sigue detrás de la misma propiedad
`UseEnvironmentVariables` (línea 488), que por omisión está apagada.

**La rama de Snap entró en `7975c8fcb`, del 2022-09-07, «Compatibilidad Ubuntu
22.04.1»**, en respuesta al
[issue 282](https://github.com/ctt-gob-es/clienteafirma/issues/282), cuyo
*workaround* era un enlace simbólico a mano. La ruta XDG de Firefox 147,
`~/.config/mozilla/firefox/profiles.ini`, **no está en v1.9.2**; sí está ya en
`master`.

Si `profiles.ini` no aparece, se cae a `SharedNssUtil.getSharedUserProfileDirectory()`
(`shared/SharedNssUtil.java:71-81`), que prueba en este orden:

1. `~/.pki/nssdb` (línea 29)
2. `~/snap/chromium/current/.pki/nssdb` (línea 30)
3. `/etc/pki/nssdb` (línea 28)

y acepta el primero que contenga algún fichero `key*.db` (`isNssProfileDirectory()`,
líneas 46-64).

### 1.3 Cuál de los perfiles elige

`NSPreferences.getFireFoxUserProfileDirectory()` (`NSPreferences.java:52-131`)
**devuelve uno solo**, en esta cascada:

1. El perfil «activo»: en `profiles.ini` v1, el primero con un `lock` o
   `parent.lock` (`ProfilesIni.java:177-179`); en v2 o superior, el que nombra la
   sección `[Install…]` (`NSPreferences.java:166-181`).
2. El que tenga `Default=1` (líneas 92-106).
3. El de fecha de modificación más reciente (líneas 108-122).
4. El primero de la lista (líneas 124-127).

Con un descarte previo: `isDummyProfile()` (líneas 190-209) tira los perfiles con
menos de diez ficheros o sin `key4.db` ni `key3.db` legible.

Del `profiles.ini` lee `Name=`, `IsRelative=`, `Path=`, `Default=`, `Version=`,
`StartWithLastProfile=`, `Locked=` (`ProfilesIni.java:22-28`), y resuelve los
`Path=` relativos contra el directorio del propio `profiles.ini`
(`ProfilesIni.java:171-173`).

El prefijo `sql:/` del `configdir` se decide por la variable de entorno
**`NSS_DEFAULT_DB_TYPE`** o por la presencia de `pkcs11.txt` en el perfil
(`MozillaKeyStoreUtilities.java:303` y `717-718`).

---

## 2. Las PR e issues del upstream

Ninguna de las PR relevantes de la lista está en el checkout local, salvo la 91:
`git -C ~/Developer/SideProjects/clienteafirma log -1` da `b4fe147c3`, tag
`v1.9.2`, con la copia de trabajo limpia. **Ninguna PR ni ningún issue del
upstream menciona el Firefox de Flatpak** (`~/.var/app/org.mozilla.firefox`); ahí
no hay trabajo previo que aprovechar.

| PR/issue | Estado | Qué aporta | ¿Nos sirve? |
|---|---|---|---|
| [PR 487](https://github.com/ctt-gob-es/clienteafirma/pull/487) | Abierta | Convierte las constantes de `RestoreConfigFirefox` y `ConfiguratorFirefoxLinux` en arrays con `.mozilla`, `.config/mozilla` y `snap/…` | **No**: rFirma ya recorre los tres layouts salvo el de Snap, y esos ficheros no tienen equivalente |
| [PR 553](https://github.com/ctt-gob-es/clienteafirma/pull/553) | Abierta | Que el configurador **no corte en el primer layout que acierte**: con Snap presente, `.mozilla` y `.config/mozilla` nunca se evaluaban | **La idea sí**: es la trampa exacta que hay que evitar al añadir Snap a `firefox_layouts` |
| [PR 554](https://github.com/ctt-gob-es/clienteafirma/pull/554) | Abierta | `getMultiarchNssPaths()`: enumera `/usr/lib`, se queda con los subdirectorios cuyo nombre contenga `-linux-` y que tengan `libsoftokn3.so` (p. ej. `/usr/lib/aarch64-linux-gnu`) | **La idea sí, para otra cosa**: `CANDIDATE_SOFTOKENS` de rFirma tiene `x86_64-linux-gnu` a fuego; la enumeración es más barata que una lista |
| [PR 312](https://github.com/ctt-gob-es/clienteafirma/pull/312) | Abierta | Añade `<libdir>/firefox-esr` a las rutas de NSS. Cierra el issue 252 | **No**: rFirma no carga el NSS del anfitrión (ADR-0004) |
| [PR 313](https://github.com/ctt-gob-es/clienteafirma/pull/313) | Abierta | Exige `canRead()` y `canExecute()` antes de listar, contra el NPE de `searchLastFirefoxVersion` | **No**: en Rust `read_dir` devuelve `Err`, no se puede desreferenciar un nulo |
| [PR 367](https://github.com/ctt-gob-es/clienteafirma/pull/367) | Abierta | Mismo bug, comprobando el resultado de `list()` en vez de los permisos | **No**, por lo mismo |
| [PR 435](https://github.com/ctt-gob-es/clienteafirma/pull/435) | Abierta (contra `develop`) | `ElfParser.archMatches()` acepta ARM64: sin esto, la 554 no sirve en aarch64 | **No**: rFirma no compara arquitecturas de ELF, `dlopen` ya lo hace |
| [PR 490](https://github.com/ctt-gob-es/clienteafirma/pull/490) | Abierta | Borra `getFirefoxProfilesDir()`, que en Linux iteraba rutas de `C:\Users\…`. Complementa a la 487 | **No**: sin equivalente |
| [PR 555](https://github.com/ctt-gob-es/clienteafirma/pull/555) | **Mergeada** 2026-08-17 | Registra el esquema `afirma:` en `/usr/lib/firefox/defaults/pref/AutoFirma.js` con `network.protocol-handler.expose.afirma=false`; mide qué rutas de preferencias lee Firefox 153 | **Sí, pero es otro asunto** (`docs/research/contrato-protocolo-afirma.md`), no la localización de almacenes |
| [PR 556](https://github.com/ctt-gob-es/clienteafirma/pull/556) | **Mergeada** 2026-08-22 | No fabricar el almacén NSS de un perfil declarado pero nunca abierto: `certutil -d` lo crea aunque falle. Guarda por `cert9.db`, `pkcs11.txt` o `cert8.db` | **La idea ya la tenemos**: `stores.rs:96-106` filtra por `cert9.db`. Lo que sí falta es el reconocimiento de `cert8.db` |
| [PR 91](https://github.com/ctt-gob-es/clienteafirma/pull/91) | **Mergeada** 2020-04-13, está en v1.9.2 | Instaura `Platform.getSystemLibDir()` (`/usr/lib64` → `/usr/lib32` → `/usr/lib`) y la lista computada de `NSS_PATHS` | **No** |
| [PR 6](https://github.com/ctt-gob-es/clienteafirma/pull/6) | **Mergeada** 2016-10-22 | Origen histórico de `SQLITE_LIBS` y de las rutas `*-linux-gnu` | **No** |
| [PR 561](https://github.com/ctt-gob-es/clienteafirma/pull/561) | Abierta | Un perfil NSS contiene `SecretKeyEntry` (la clave con que Firefox cifra las contraseñas) sin cadena de certificados, y el casteo directo a `PrivateKeyEntry` revienta con `ClassCastException` | **Aviso útil**: al enumerar objetos de un perfil hay que filtrar los que no tienen certificado |
| [PR 13](https://github.com/ctt-gob-es/clienteafirma/pull/13) | **Cerrada sin mergear** | Deducir el prefijo `sql:/` de la presencia de `cert9.db` y no solo de `NSS_DEFAULT_DB_TYPE` | **No**: `Store::nss` ya pone `sql:` siempre |
| [PR 184](https://github.com/ctt-gob-es/clienteafirma/pull/184) | **Cerrada sin mergear** | `os.arch` puede valer `amd64` y no `x86_64` | **No** |
| [Issue 560](https://github.com/ctt-gob-es/clienteafirma/issues/560) | Abierto | Pide `~/.config/mozilla/firefox/profiles.ini`, la ruta XDG de Firefox 147 | **Ya cubierto** desde el #246 |
| [Issue 282](https://github.com/ctt-gob-es/clienteafirma/issues/282) | Abierto | Ubuntu 22.04: el perfil está en `~/snap/firefox/common/.mozilla/firefox`. El *workaround* era un enlace simbólico | **Sí, es el hueco** |
| [Issue 233](https://github.com/ctt-gob-es/clienteafirma/issues/233) | Abierto | AutoFirma se queda con el primer perfil de `profiles.ini` y no mira los demás | **rFirma ya está por delante**: `nss_profiles` los recorre todos |
| [Issue 86](https://github.com/ctt-gob-es/clienteafirma/issues/86) | Cerrado | El mismo problema con dos perfiles | Ídem |
| [Issue 90](https://github.com/ctt-gob-es/clienteafirma/issues/90) | Abierto | `AFIRMA_NSS_HOME_ENV` se lee y luego se pisa: no hay forma de fijar NSS a mano | **Aviso**: rFirma tiene `RFIRMA_PKCS11_MODULE` y sí cortocircuita de verdad |
| [Issue 252](https://github.com/ctt-gob-es/clienteafirma/issues/252) | Abierto | Firefox ESR: `searchLastFirefoxVersion` avisa «versión no soportada: esr» y la carga muere | **No** |
| [Issue 469](https://github.com/ctt-gob-es/clienteafirma/issues/469) | Abierto | `ExceptionInInitializerError` en `getSystemNSSLibDir` en Ubuntu 24: no encuentra NSS del anfitrión | **No, y es el argumento del ADR-0004**: el flatpak no depende del NSS del anfitrión y por eso no puede tener este fallo |
| [Issue 446](https://github.com/ctt-gob-es/clienteafirma/issues/446) | Abierto | Pide soporte para **LibreWolf**, que guarda su perfil en `~/.librewolf`. Ninguna PR lo implementa | Anotado; ver *Preguntas abiertas* |
| [Issue 168](https://github.com/ctt-gob-es/clienteafirma/issues/168) | Abierto | Un usuario de Fedora arregla la carga con `sudo ln -s /usr/lib64 /opt/firefox-`, explotando el sufijo colgante de la fila 13 de la tabla 1.1 | Confirma que esas rutas están vivas; no aplica |
| [Issue 27](https://github.com/ctt-gob-es/clienteafirma/issues/27) | Cerrado | Fedora de 64: NSS en `/usr/lib64` y no en `/usr/lib`. Propone además inspeccionar `java.library.path`. Origen de la PR 91 | No |
| [Issue 164](https://github.com/ctt-gob-es/clienteafirma/issues/164) | Abierto | macOS: un `libnss3.dylib` de Homebrew en `/usr/local/lib` rompe la carga del de Firefox | n/a |
| [Issues 122](https://github.com/ctt-gob-es/clienteafirma/issues/122), [190](https://github.com/ctt-gob-es/clienteafirma/issues/190), [484](https://github.com/ctt-gob-es/clienteafirma/issues/484) | Abiertos | Piden un flatpak oficial de AutoFirma; el 484 anuncia uno no oficial en Flathub | Contexto, no código |

Descartadas por no tocar la localización de rutas: PR 8 y 116 (mergeadas, manejo
de errores y certificado raíz), PR 491 y 249 (cerradas), PR 2 (abierta, JNLP).

Además del `AFIRMA_NSS_HOME_ENV` y el `AFIRMA_NSS_PROFILES_INI` de la sección 1,
el original tiene otras dos parejas propiedad/variable que gobiernan **qué se
carga**, no dónde: `es.gob.afirma.keystores.mozilla.LoadSscdOnly` /
`AFIRMA_NSS_LOAD_SSCD_ONLY` y
`es.gob.afirma.keystores.mozilla.IncludeNativeDniePkcs11` /
`AFIRMA_NSS_INCLUDE_NATIVE_DNIE_PKCS11`
(`MozillaUnifiedKeyStoreManager.java:31-40`). rFirma no tiene equivalente porque
no soporta tarjeta ni DNIe en la v0.4.

---

## 3. Qué hace rFirma hoy

Toda la localización vive en un solo fichero,
`rfirma-app/src-tauri/src/identity/adapters/pkcs11/stores.rs`, y el ADR-0005 se
niega a propósito a enumerarla en ninguna otra parte. El puente Java no toca
almacenes: `grep -rn 'nss\|mozilla\|firefox' rfirma-native-bridge/src` no
devuelve nada, coherente con el ADR-0001.

**Softoken** (`CANDIDATE_SOFTOKENS`, `stores.rs:14-21`), primera que exista:
`/usr/lib/x86_64-linux-gnu/libsoftokn3.so`,
`/usr/lib/x86_64-linux-gnu/nss/libsoftokn3.so`, `/usr/lib64/libsoftokn3.so`,
`/usr/lib64/nss/libsoftokn3.so`, `/usr/lib/libsoftokn3.so`,
`/usr/lib/nss/libsoftokn3.so`. Y `CANDIDATE_NSS`
(`identity/adapters/pkcs11/nss.rs:15-19`) para `libnss3.so`:
`/usr/lib/x86_64-linux-gnu/libnss3.so`, `/usr/lib64/libnss3.so`,
`/usr/lib/libnss3.so`. En el flatpak la primera de cada lista siempre acierta:
el runtime `org.gnome.Platform//50` trae `libsoftokn3.so`, `libnss3.so`,
`libnssutil3.so`, `libnssckbi.so`, `libsmime3.so`, `libfreebl3.so`, `libnspr4.so`,
`libplc4.so` y `libplds4.so` en `/usr/lib/x86_64-linux-gnu`
(`docs/research/p12-en-almacen-nss.md`, y comprobado de nuevo aquí).

**Layouts de Firefox** (`firefox_layouts`, `stores.rs:71-79`), pares
(configuración → datos):

1. `$HOME/.mozilla/firefox` → `$HOME/.mozilla/firefox`
2. `$HOME/.config/mozilla/firefox` → `$HOME/.local/share/mozilla/firefox`

De cada uno se lee `<config>/profiles.ini`, se sacan **todas** las secciones que
empiezan por `Profile` con su clave `Path=` (`profiles_declared_in`,
`stores.rs:112-137`) y se resuelven las relativas contra el directorio de datos
(`resolve_under`, `stores.rs:140-147`).

**Almacenes de Chromium** (`stores.rs:91-92`): `$HOME/.pki/nssdb` y
`$HOME/.local/share/pki/nssdb`.

**Filtro final** (`stores.rs:96-106`): se descarta todo perfil sin `cert9.db` y
se deduplica por `canonicalize`.

**Variables de entorno**: una sola, `RFIRMA_PKCS11_MODULE`
(`rfirma-app/src-tauri/src/lib.rs:23`), y cortocircuita todo el descubrimiento
(`stores.rs:25-27`). No hay forma de fijar el perfil ni el softoken.

**Permisos del flatpak**, `packaging/flatpak/me.sgomez.rfirma.yml:98-102`:
`--filesystem=~/.mozilla/firefox`, `~/.pki/nssdb`, `~/.config/mozilla`,
`~/.local/share/mozilla/firefox`, `~/.local/share/pki/nssdb`. Los cinco de
lectura y escritura, sin `:create`. **No hay `--filesystem=home`, ni `host`, ni
`host-os`.**

---

## 4. La brecha, ruta a ruta

| Ruta o heurística | AutoFirma v1.9.2 | rFirma | Comentario |
|---|---|---|---|
| `~/.mozilla/firefox/profiles.ini` | Sí | **Sí** | |
| `~/.config/mozilla/firefox/profiles.ini` (Firefox 147, XDG) | **No** (sí en `master`) | **Sí** | rFirma va por delante |
| `~/snap/firefox/common/.mozilla/firefox/profiles.ini` | Sí | **No** | **El hueco** |
| `~/.var/app/org.mozilla.firefox/.mozilla/firefox/profiles.ini` | No | No | Nadie lo cubre |
| `%APPDATA%\Mozilla\Firefox\profiles.ini` | Sí | n/a | rFirma no tiene canal Windows |
| `~/Library/Application Support/Firefox/profiles.ini` | Sí | n/a | Ni macOS |
| Recorre **todos** los perfiles de `profiles.ini` | **No**, elige uno | **Sí** | rFirma va por delante (issues 233 y 86) |
| Resuelve `Path=` relativa contra el directorio de datos | Sí | Sí | |
| Descarta perfiles sin almacén | Sí (`key4.db`/`key3.db`) | Sí (`cert9.db`) | rFirma no reconoce el `cert8.db` antiguo |
| `~/.pki/nssdb` | Sí | Sí | |
| `~/.local/share/pki/nssdb` (Chrome M146) | **No** | **Sí** | rFirma va por delante |
| `~/snap/chromium/current/.pki/nssdb` | Sí | **No** | Vale para Chromium antiguo; el moderno se mudó a la ruta XDG (ver 5.1) |
| `~/snap/chromium/current/.local/share/pki/nssdb` | **No** | **No** | Donde está hoy el almacén del snap de Chromium (ver 5.1) |
| `/etc/pki/nssdb` | Sí | No | Deliberado: el ADR-0005 retiró el almacén de sistema |
| Quince rutas de `libsoftokn3.so` del anfitrión | Sí | No | **Deliberado**: ADR-0004, el softoken es el del runtime |
| `<libdir>/firefox-esr`, `/opt/firefox`, `/opt/firefox-<ver>` | Parcial (PR 312 sin mergear) | No | No aplica |
| `/usr/lib/<triplete>-linux-gnu` por enumeración | No (PR 554 sin mergear) | No | rFirma solo tiene `x86_64` a fuego |
| Precarga manual de `libnspr4`/`libplds4`/`libplc4`/`libnssutil3`/`libmozsqlite3` | Sí | No | No hace falta: `dlopen` resuelve por `DT_NEEDED` dentro del runtime |
| Comprobación de arquitectura del ELF | Sí (`ElfParser`) | No | `dlopen` ya falla si no encaja |
| `compatibility.ini` → `LastPlatformDir=` | Sí (Windows) | n/a | |
| Enlaces simbólicos en `/usr/local/lib` vía AppleScript | Sí (macOS) | n/a | |
| NSS empaquetado de repuesto en `~/.afirma/nss/` | Sí (Windows) | No | El ADR-0004 lo prohíbe |
| Variable para fijar el directorio de NSS | `AFIRMA_NSS_HOME_ENV` (rota, issue 90) | `RFIRMA_PKCS11_MODULE` (funciona) | rFirma va por delante |
| Variable para fijar `profiles.ini` | `AFIRMA_NSS_PROFILES_INI` | **No** | Hueco discutible |

---

## 5. Qué de eso alcanza de verdad el sandbox

Dos preguntas, y las dos se han medido en este equipo con Flatpak 1.16.6, usando
`org.love2d.love2d` como conejillo porque no es rFirma y no contamina su
instalación.

### 5.1 ¿Puede el flatpak leer `~/snap/firefox/common/.mozilla/firefox`? **Sí.**

`~/snap` es un directorio corriente del `$HOME` de la persona: modo `0700`, pero
propiedad del mismo usuario que ejecuta el flatpak, así que el modo no estorba —
el sandbox de Flatpak es de *bind mounts*, no de cambio de identidad. No aparece
en ninguna de las listas de rutas reservadas de la
[documentación de permisos de Flatpak](https://docs.flatpak.org/en/latest/sandbox-permissions.html),
que solo reserva rutas de primer nivel del sistema (`/app`, `/bin`, `/dev`,
`/etc`, `/lib`, `/lib32`, `/lib64`, `/proc`, `/run/flatpak`, `/run/host`,
`/sbin`, `/usr`).

Medido **en un Ubuntu con el snap de Firefox de verdad instalado**
(`firefox 154.0-1`, revisión 8763, del editor `mozilla`), negativo y positivo en
la misma sesión, con `org.gnome.Calculator` de cobaya:

```
$ flatpak run --command=sh --nofilesystem=host org.gnome.Calculator \
    -c 'ls ~/snap/firefox/common/.mozilla/firefox/'
ls: no se puede acceder a '/home/…/snap/firefox/common/.mozilla/firefox/':
    No existe el fichero o el directorio

$ flatpak run --command=sh --nofilesystem=host \
    --filesystem=~/snap/firefox/common/.mozilla/firefox org.gnome.Calculator \
    -c 'cat ~/snap/firefox/common/.mozilla/firefox/profiles.ini'
[General]
StartWithLastProfile=1
Version=2

[Profile0]
Name=default
IsRelative=1
Path=v00g2c63.default
Default=1
```

Sin el permiso el directorio **no existe** dentro del sandbox; con él se lee el
`profiles.ini` entero. Y el perfil tiene exactamente la forma que
`stores::nss_profiles` espera: `IsRelative=1` con `Path=` relativo —que es lo
que `resolve_under` ancla— y el almacén en formato moderno, `cert9.db` y
`key4.db`, junto a `pkcs11.txt` y `compatibility.ini`.

Es decir: **un `--filesystem` estrecho basta, no hace falta `--filesystem=home`**.
Es exactamente la misma concesión, en forma y en tamaño, que las cinco que ya
tiene el manifiesto, y encaja con el argumento del ADR-0005: se declara la ruta
del almacén, no el `$HOME`.

Escribir la CA local ahí dentro también funciona por el mismo motivo: quien
escribe es el usuario, y el confinamiento AppArmor del snap regula lo que hace
*Firefox*, no lo que hacen los demás procesos del usuario sobre sus ficheros.

**El gemelo de Chromium ha cambiado de sitio, y hay que soportar los dos.** En
un Ubuntu 26.04 con el snap de Chromium (revisión 3520),
`~/snap/chromium/current/.pki/nssdb` —la constante de `SharedNssUtil.java:30`,
repetida en `ConfiguratorFirefoxLinux.java:44` y en
`RestoreConfigFirefox.java:65`— **no existe**; el almacén está en
`~/snap/chromium/current/.local/share/pki/nssdb`. No es un error del upstream:
es la misma mudanza a la ruta XDG que la tabla de la sección 4 anota como
«Chrome M146», y en las Ubuntu de 2022, cuando se escribió la constante, `.pki`
era lo correcto. Como no se ha medido en qué versión exacta de Chromium cambia
—esta máquina solo tiene la nueva—, **se declaran las dos formas**, que cuesta
lo mismo.

Dos detalles de forma, medidos: el almacén cuelga de `SNAP_USER_DATA`
(versionado: `~/snap/chromium/3520/`), no de `common` como el de Firefox, pero
`~/snap/chromium/current` es un enlace simbólico vivo a la revisión en uso, así
que **basta con declarar `current`** y no hace falta barrer revisiones.

Y esto es **Chromium, no Chrome**: Google no publica snap de Chrome, lo
distribuye en `.deb` y `.rpm`, así que no hay ningún `~/snap/chrome`. El caso de
Chrome ya lo cubren `~/.pki/nssdb` y `~/.local/share/pki/nssdb`, que rFirma
tiene.

Que el perfil de Firefox está ahí lo dice la fuente primaria: el
[`snapcraft.yaml` de `canonical/firefox-snap`](https://github.com/canonical/firefox-snap/blob/stable/snapcraft.yaml)
pone `HOME: "$SNAP_USER_COMMON"`, que para este snap es `~/snap/firefox/common`;
de ahí que el perfil caiga en `~/snap/firefox/common/.mozilla/firefox`.

### 5.2 ¿Y el Firefox de Flatpak, `~/.var/app/org.mozilla.firefox`? **Sí, pero con un pero.**

La documentación dice que `--filesystem=home` concede el `$HOME` **«except
`~/.var/app`»**, y que ese directorio «requires explicit request». Medido, las
dos mitades se confirman: con `--filesystem=host:ro` —que es más ancho que
`home`— la ruta **no existe** dentro del sandbox; con
`--filesystem=~/.var/app/org.mozilla.firefox` se lee sin problema.

El pero no es técnico: es que declarar el directorio de datos de **otra**
aplicación de Flatpak es justo lo que la revisión de Flathub mira con lupa, y
además rompe la garantía que el sistema le da a esa otra aplicación. El ADR-0004
ya asumió que no compartir perfil con el Firefox de Flatpak es «conducta normal
de Linux». Esta medición no cambia ese razonamiento; solo quita la excusa de que
fuera imposible. **Recomendación: no hacerlo.**

### 5.3 ¿Y cargar un `libsoftokn3.so` del anfitrión? **No, y no hace ninguna falta.**

Sin `--filesystem=host-os` el `/usr` del anfitrión sencillamente no está montado.
Y con él, lo que aparece bajo `/run/host/usr` está compilado contra la glibc del
anfitrión, que no es la del runtime; `docs/research/flatpak-canal-unico.md` ya
midió que los módulos PKCS#11 del anfitrión no cargan dentro del sandbox. Es un
callejón sin salida, y además vacío: el runtime trae su propio softoken, que es
la premisa del ADR-0004. **Toda la sección 1.1 de este documento es, para
rFirma, arqueología.**

---

## 6. Qué merece la pena hacer

Por orden, y con el criterio de que cada punto o cierra un caso real de usuario o
no se hace. Los dos primeros son un solo cambio y van en el
[#637](https://github.com/sgomez/rfirma/issues/637).

1. **Snap de Firefox.** Añadir a `firefox_layouts` el par
   `$HOME/snap/firefox/common/.mozilla/firefox` → `$HOME/snap/firefox/common/.mozilla/firefox`
   y al manifiesto `--filesystem=~/snap/firefox/common/.mozilla/firefox`. Es el
   Firefox por omisión de Ubuntu desde la 22.04 y hoy rFirma **no ve ni un
   certificado** en esa máquina. Cuidado con la trampa que denuncia la PR 553 del
   upstream: el layout de Snap **se añade a los otros dos, no los sustituye**;
   hay quien tiene el snap y el `~/.mozilla` de antes de migrar, con perfiles
   distintos en cada uno. `nss_profiles` ya itera y deduplica, así que la trampa
   no llega a darse: basta con no escribir un `if/else`.
2. **Snap de Chromium.** Mismo argumento, menos usuarios, y **dos rutas bajo
   `~/snap/chromium/current/`**: `.local/share/pki/nssdb` para el Chromium
   moderno y `.pki/nssdb` para el antiguo (ver 5.1). El enlace `current` sirve;
   no hay que barrer revisiones.
3. **`cert8.db` como señal de almacén: no.** La PR 556 del upstream amplía la
   señal a `cert9.db`, `pkcs11.txt` o `cert8.db`, y parecía barato de copiar,
   pero **el runtime del flatpak no trae `libnssdbm3.so`** —solo
   `libsoftokn3.so`, `libfreebl3.so` y `libfreeblpriv3.so`—, así que el lector
   del formato *dbm* antiguo no existe de nuestro lado y un perfil con `cert8.db`
   no se puede abrir aunque se detecte. Aceptarlo solo cambiaría un descarte
   silencioso por un fallo al abrir. Al upstream le vale porque carga el NSS del
   anfitrión, que en muchas distribuciones sí lleva `nssdbm3`.
4. **`CANDIDATE_SOFTOKENS` por enumeración**, al estilo de la PR 554: buscar en
   los subdirectorios de `/usr/lib` cuyo nombre contenga `-linux-`, en vez de
   tener `x86_64-linux-gnu` a fuego. Dentro del flatpak da igual —el runtime es
   siempre `x86_64` y siempre acierta la primera—, pero **el `.deb` y el `.rpm`
   del ADR-0004 corren sobre el NSS del anfitrión**, y ahí un `aarch64` hoy no
   arranca. Es la única fila de la tabla de la brecha que muerde fuera del
   flatpak.
5. **Firefox de Flatpak: no.** Medido que se puede, decidido que no. Si algún día
   se reconsidera, que sea reescribiendo el ADR-0004, no añadiendo una línea al
   manifiesto.

En el tracker de rFirma **no hay ningún issue sobre Snap, sobre el Firefox de
Flatpak, sobre ESR ni sobre otras arquitecturas**. Los que rondan el asunto
—#95, #101, #243, #246, #255, #344, #413, #435— están todos cerrados y tratan de
lo que ya está hecho. Los puntos 1 y 2 caben en un issue conjunto («el perfil de
Firefox y el nssdb de Chromium cuando vienen por Snap»); el 3 en otro; el 4 en un
tercero, y ese es el único que no es de sandbox sino de los otros dos canales.

---

## Preguntas abiertas

Tres cosas que este sondeo **no** ha medido, con el experimento escrito para que
lo ejecute quien tenga la máquina adecuada.

**1. Que el certificado del snap se pueda usar para firmar.** Lo medido en 5.1
es que la ruta se lee y que el perfil tiene la forma que `nss_profiles` espera;
lo que no se ha medido es el trámite entero —abrir el `cert9.db` del snap desde
el bundle instalado y firmar con su certificado—, porque exige rFirma instalado
en esa máquina.

```bash
flatpak override --user --filesystem=~/snap/firefox/common/.mozilla/firefox me.sgomez.rfirma
flatpak run me.sgomez.rfirma    # ¿aparece el certificado del snap en la lista?
flatpak override --user --reset me.sgomez.rfirma
```

**2. Cargar un `libsoftokn3.so` del anfitrión.** Se ha razonado que no procede
—runtime propio, glibc distinta, y `flatpak-canal-unico.md` ya midió que los
módulos PKCS#11 del anfitrión no cargan— pero **no se ha ejecutado el
experimento**, porque exige reconstruir el bundle con `--filesystem=host-os` y
eso no cabe en el presupuesto de este sondeo. Si algún día hace falta zanjarlo:

```bash
flatpak run --command=sh --filesystem=host-os me.sgomez.rfirma \
  -c 'python3 -c "import ctypes; ctypes.CDLL(\"/run/host/usr/lib/x86_64-linux-gnu/libsoftokn3.so\")"'
```

Mirar en la salida: si el `OSError` cita `GLIBC_` o un símbolo indefinido, el
razonamiento queda confirmado y el asunto cerrado.

**3. LibreWolf y los demás forks.** El issue 446 del upstream pide LibreWolf
(`~/.librewolf`), y del mismo palo salen Waterfox y el Firefox de Flathub. Antes
de añadir rutas conviene saber si alguna tiene usuarios entre el público de
rFirma; ninguna se ha medido aquí.

---

## Reproducir

```bash
# el catálogo del upstream, a tag fijado
cd ~/Developer/SideProjects/clienteafirma && git log -1   # b4fe147c3, v1.9.2
grep -rn 'snap\|profiles.ini\|libsoftokn3' --include='*.java' .

# fechar la entrada del soporte de Snap
git log -1 -S 'snap/firefox/common' -- \
  afirma-keystores-mozilla/src/main/java/es/gob/afirma/keystores/mozilla/MozillaKeyStoreUtilities.java

# lo que el sandbox alcanza (en una maquina con el snap de Firefox instalado,
# con cualquier flatpak de cobaya)
flatpak run --command=sh --nofilesystem=host \
  org.gnome.Calculator -c 'ls ~/snap/firefox/common/.mozilla/firefox/'
flatpak run --command=sh --nofilesystem=host \
  --filesystem=~/snap/firefox/common/.mozilla/firefox \
  org.gnome.Calculator -c 'cat ~/snap/firefox/common/.mozilla/firefox/profiles.ini'

# donde esta de verdad el almacen del snap de Chromium (y que 'current' vive)
ls -ld ~/snap/chromium/current
ls -d ~/snap/chromium/*/.local/share/pki/nssdb

# qué NSS trae el runtime
ls /var/lib/flatpak/runtime/org.gnome.Platform/x86_64/50/active/files/lib/x86_64-linux-gnu \
  | grep -E 'nss|softokn|nspr|plc4|plds4|smime'
```

## Fuentes

* Checkout de `ctt-gob-es/clienteafirma` en `b4fe147c3` (tag `v1.9.2`), rutas y
  líneas citadas una a una en el cuerpo.
* PR e issues del upstream, leídos por `gh pr diff` y `gh issue view`, con su
  número y su estado en la tabla de la sección 2.
* [Flatpak — Sandbox Permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html),
  para las rutas reservadas y la excepción de `~/.var/app`.
* [`canonical/firefox-snap`, `snapcraft.yaml`](https://github.com/canonical/firefox-snap/blob/stable/snapcraft.yaml),
  para `HOME: "$SNAP_USER_COMMON"`.
* `docs/research/p12-en-almacen-nss.md`, `flatpak-canal-unico.md`,
  `ca-en-los-almacenes-de-confianza.md`, `rutas-reales-con-filesystem-home.md`,
  y el ADR-0004 y el ADR-0005.
