# Windows como segunda plataforma: dependencias por target y adaptadores no disponibles

rFirma compila también en Windows (`x86_64-pc-windows-msvc`). Linux sigue siendo la plataforma de
referencia: en Linux no cambian ni el código activo, ni el comportamiento, ni lo que se compila.
Windows entra por fases, y hasta que una fase lo implementa, cada pieza que solo existe en un
escritorio Linux tiene en Windows un **adaptador pendiente**.

## Las dependencias de escritorio Linux van por target

`gtk`, `glib`, `gio` y `oo7` se declaran en `[target.'cfg(target_os = "linux")'.dependencies]`:
sin GTK ni D-Bus no compilan en Windows, y en Linux la resolución es la misma de antes. `libc`
se queda común porque compila en las dos, aunque en Windows no se use.

## OpenSSL: compilado desde las fuentes, y solo en Windows

El crate `openssl` es común (la CA local y la lectura de certificados lo usan en `domain/`), y
Windows no trae OpenSSL. En `[target.'cfg(windows)'.dependencies]` se le añade la
característica `vendored`: `openssl-src` compila OpenSSL y lo enlaza estático, así que el
binario no arrastra ninguna DLL más. En Linux esa característica no se activa y se sigue usando
el OpenSSL del sistema.

Compilarlo exige un **Perl nativo de Windows** (Strawberry Perl, que traen los runners de Windows
de GitHub) y las Build Tools de Visual Studio. El Perl de Git for Windows **no sirve**: es de
Cygwin, le faltan módulos y el `Configure` de OpenSSL rechaza sus rutas para `VC-WIN64A`.

El `Cargo.lock` es el mismo para todas las plataformas, así que gana la entrada de
`openssl-src`, y `packaging/flatpak/cargo-sources.json` con ella: el flatpak la descarga para
resolver el grafo, pero no la compila.

## Los condicionales de sistema viven en una lista cerrada

El ADR-0010 dejaba todo el conocimiento del sistema en `paths.rs`. Con Windows ya no cabe en un
fichero, pero sigue sin repartirse: la guarda `tests/single_cfg_os_site.rs` enumera en
`AUTHORISED_SITES` los únicos ficheros con un `cfg` de sistema operativo, y exige que cada uno
lo lleve de verdad. Un fichero nuevo en la lista se justifica en la PR que lo añade.

Donde la diferencia es solo de tipos, no se pone condicional: el parámetro de PSS es un
`c_ulong`, que en Linux es el mismo `u64` de siempre.

## Los adaptadores pendientes dicen su nombre

Mientras una pieza que solo existe en un escritorio Linux no tiene su versión de Windows, su
adaptador **pendiente** respeta el puerto, falla como fallaría Linux sin el servicio, y su nombre
dice qué falta. La fase 4 sustituyó todos menos uno:

| Pieza en Linux | En Windows |
| --- | --- |
| Diálogo GTK del PIN | `PendingWindowsPinDialog`: falla con un mensaje en castellano. Solo lo alcanza un módulo PKCS#11, porque con CNG el PIN lo pide Windows. |
| Llavero `oo7` (ADR-0034) | `WindowsCredentialManager`: una credencial genérica `rfirma/almacen-pin` del usuario en el Administrador de credenciales. |
| Manejadores de `afirma://` por GIO y `mimeapps.list` | `HKCU\Software\Classes\afirma`, leído junto al de `HKLM` (§ *`afirma://` en el registro*). |
| Diálogo GTK de fallo de arranque | `MessageBoxW` con el mismo texto. |
| `mlock` del secreto | `VirtualLock`, y `VirtualUnlock` al soltarlo. |
| Cerrojo de `.parentlock` de Firefox | `parent.lock` abierto sin compartir: mientras Firefox vive, abrirlo falla por violación de uso compartido. |
| CA local en los almacenes NSS (ADR-0005) | `CurrentUser\Root` con CryptoAPI (§ *La CA local en Windows*). |

El cerrojo de la carpeta de paso (ADR-0024) no quedó nunca pendiente: en Windows es abrir el
fichero sin compartirlo, que cumple lo mismo que `flock`. Los adaptadores de NSS y de p11-kit
compilan sin cambios. En Windows no encuentran sus bibliotecas y fallan en tiempo de ejecución:
el Almacén de rFirma (ADR-0034), que es NSS, no existe todavía en Windows, aunque su llavero ya
sí.

Las pruebas intrínsecamente de Unix (enlaces simbólicos, `fork`, `flock`, `OsString` no UTF-8)
se marcan con `cfg(unix)`, no se borran; las del cerrojo de Firefox tienen su gemela de Windows.

## La CA local en Windows: el almacén raíz del usuario

El ADR-0005 instala la CA local en los almacenes NSS **de la persona**, nunca en el del sistema.
En Windows, el almacén de la persona es `CurrentUser\Root`: lo leen Edge y Chrome, y lo escribe
el usuario sin privilegios. `WindowsUserStores` sirve el puerto `TrustStores` sobre los almacenes
de sistema de `CurrentUser` con CryptoAPI: instalar es `CertAddEncodedCertificateToStore`,
consultar es buscar el certificado exacto (`CERT_FIND_EXISTING`) y retirar es
`CertDeleteCertificateFromStore`. Estar en `Root` es la confianza entera, así que la consulta
responde con los bits de CA TLS de NSS y el caso de uso no cambia.

El «perfil» de ese almacén es la ruta `cryptoapi:CurrentUser/Root`, que ningún perfil NSS puede
tener, igual que `cng:CurrentUser/MY` en la firma. La interfaz lo presenta con su propia marca,
`windows`, que Linux nunca produce.

Windows **pregunta** antes de añadir o borrar un certificado de `CurrentUser\Root`, con su propio
diálogo de seguridad. Es lo esperado: la instalación la pide la persona desde el asistente o el
panel de estado, nunca el arranque (el arranque de un trámite no toca almacenes, ADR-0005). Si
la persona dice que no, el almacén queda como «no instalado» con el motivo, y el asistente
ofrece reintentar. La renovación (ADR-0005) pregunta una vez por la CA siguiente y otra al
retirar la vieja.

**Firefox** no tiene su almacén aparte en Windows: desde la versión 120 importa por omisión los
certificados raíz que el usuario o el administrador han añadido al almacén de Windows
(`security.enterprise_roots.enabled`), `CurrentUser\Root` incluido. rFirma no busca sus perfiles
en `%APPDATA%\Mozilla\Firefox\Profiles` ni escribe en ellos.

## `afirma://` en el registro

Windows mezcla `HKCU\Software\Classes` sobre `HKLM\Software\Classes`, y la rama del usuario
gana. rFirma se registra solo en la del usuario, sin privilegios: `afirma` con `URL Protocol`, su
icono y `shell\open\command` = `"<rfirma.exe>" "%1"`. AutoFirma, instalado para la máquina,
queda en `HKLM` y no se toca.

El puerto `HandlerRegistry` se cumple así:

- **Candidatos**: rFirma siempre, más el programa de la orden `open` de cada rama. El de otro
  programa se nombra por su ejecutable (`AutoFirma.exe` es «AutoFirma»), y su identificador es
  la ruta.
- **Elegido**: el de la rama del usuario o, si no hay, el de la máquina. Es rFirma solo si la
  orden apunta al ejecutable que está en marcha: una entrada de otra copia de rFirma sale como
  otro programa y la señal pide reparar, que reescribe la rama con el ejecutable actual.
- **Elegir** rFirma escribe la rama del usuario; elegir el de la máquina la borra.
- **Retirar** borra la rama del usuario solo si es de rFirma.

Cada trámite de sede es su propio proceso (ADR-0024), así que Windows lanza un `rfirma.exe`
nuevo por cada `afirma://`; la instancia única sigue siendo solo la del escritorio. Los fallos
del registro se cuentan con las situaciones de `mimeapps.list`, porque el dominio no distingue
dónde vive la lista.

## La firma en Windows: CNG para el almacén del usuario, PKCS#11 para lo demás

En Windows, el puerto `Token` lo sirve `WindowsToken`. El almacén personal del usuario
(`CurrentUser\MY`) entra como un `Store` más, con una ruta que ningún módulo PKCS#11 puede
tener (`cng:CurrentUser/MY`), y va el primero de la lista. Cualquier otro almacén se lo pasa a
`RealToken`, el mismo PKCS#11 de Linux.

- **Listar**: CryptoAPI enumera `MY` y se queda con los certificados que tienen
  `CERT_KEY_PROV_INFO_PROP_ID`, sin abrir la clave ni tocar la tarjeta. La cadena se completa
  con `MY`, `CA` y `Root` del usuario. El `CKA_ID` de la referencia es la huella SHA-1, y la
  etiqueta, el nombre descriptivo o, si no hay, el sujeto.
- **Firmar (ADR-0001)**: Rust resume lo que manda Java y firma el resumen con `NCryptSignHash`
  sobre la clave de `CryptAcquireCertificatePrivateKey` con `CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG`.
  RSA con PKCS#1 v1.5 o PSS (sal del tamaño del resumen, como en PKCS#11) y ECDSA, cuyo `r||s`
  se reempaqueta en DER igual que el de PKCS#11. La clave no sale de su proveedor, y las claves
  de un CSP antiguo también se abren por CNG.
- **El PIN lo pide Windows**: el almacén del usuario responde `NotNeeded` a `secret_of`, así que
  rFirma no enseña su diálogo, y el proveedor de la tarjeta (KSP o minidriver) pide el PIN con
  su propia ventana. Esa ventana es modal sobre la de rFirma: la clave se abre con
  `CRYPT_ACQUIRE_WINDOW_HANDLE_FLAG` y se le pone `NCRYPT_WINDOW_HANDLE_PROPERTY`, con la ventana
  en primer plano si es del proceso o, si no, la primera visible del proceso. Cancelarla sigue
  llegando como `Situation::Unknown`, con el detalle «has cancelado la petición del PIN de
  Windows»: ninguna situación del catálogo dice «cancelado» sin mentir, y añadirla cambia el
  dominio también en Linux. El diálogo propio del PIN solo haría falta en el camino PKCS#11, y
  ahí sigue pendiente.

El **DNIe** llega por las dos vías. Con el minidriver que Windows instala para la tarjeta, sus
certificados aparecen en `CurrentUser\MY` y se firman por CNG con el PIN pedido por Windows: es
la vía preferente. Además se buscan los módulos PKCS#11 de OpenSC y del DNIe en
`%ProgramFiles%` y `%SystemRoot%\System32` (`RFIRMA_PKCS11_MODULE` los sustituye a todos, como
en Linux); si el mismo certificado sale por los dos, la fila única se queda con la copia de CNG
porque su clase, `Windows`, se prefiere a `Card`, y la ventana la rotula «Almacén de Windows».

## Las recetas corren en Git Bash

El `justfile` es el mismo en las dos plataformas. En Windows sus recetas corren en el `bash.exe`
de Git for Windows, fijado por ruta absoluta en `windows-shell`: buscado por el `PATH`, Windows
encontraría antes el `bash.exe` de WSL. Las recetas con shebang necesitan además `cygpath`, así
que `just` se lanza desde Git Bash, no desde PowerShell ni desde cmd.

La raíz del repositorio se toma con barras normales (`root`), porque `bash` se come las
invertidas de `justfile_directory()`. GraalVM se busca en `GRAALVM_HOME` y, en Windows, en
`JAVA_HOME`, que es donde lo deja su instalador; en Linux sigue la ruta de SDKMAN. `just tools`
comprueba en cada sistema lo suyo: en Windows, `cl.exe` por `vswhere`, el runtime de WebView2 y
un Perl nativo para OpenSSL, y no pide ni las bibliotecas del WebView de Linux, ni softhsm, ni
NSS; gettext pasa a aviso porque solo lo usan `just po` y el carril del CI.

## La biblioteca nativa se llama como manda la plataforma

`librfirma_crypto.so` en Linux y `rfirma_crypto.dll` en Windows. Rust compone el nombre con
`DLL_PREFIX` y `DLL_SUFFIX` de `std` (`library_file`), sin `cfg`; el `justfile` hace lo mismo
en `native_lib_name`. Se busca primero en `RFIRMA_LIB_DIR`, como en el ADR-0004, y después
donde la deja el paquete de cada plataforma: `../lib/rfirma` en Linux y el directorio del
propio ejecutable en Windows. Ese segundo sitio lo decide `Platform::native_library_directory`,
en `paths.rs`, sin `cfg` en `ffi/location.rs`.

`native-image` sigue construyendo con las banderas de `native-image.properties`, que no cambian,
así que en Windows emite `librfirma_crypto.dll`: `just native` la instala renombrada. El nombre
del fichero no lo lee nadie dentro de la imagen, y los trece símbolos que resuelve Rust salen
exportados igual. Como en Linux, emite al lado los auxiliares de AWT (`awt.dll`, `java.dll`,
`jvm.dll`, `lcms.dll`…), la `.lib` de importación y las cabeceras, y no se copia ninguno: la
invariante de un solo fichero del ADR-0012 vale igual. La `.dll` solo importa bibliotecas del
sistema y el runtime de Visual C++.

## El instalador es NSIS, por usuario y sin privilegios

`just bundle-windows` construye con el *bundler* de Tauri un instalador NSIS
(`rfirma_<versión>_x64-setup.exe`) que instala para la persona que lo ejecuta, en
`%LOCALAPPDATA%\rfirma`, sin pedir administrador. Es coherente con todo lo demás de Windows:
la CA va a `CurrentUser\Root`, `afirma://` a `HKCU` y la firma al almacén del usuario, así que
nada de rFirma necesita la máquina.

La configuración de Windows vive en `packaging/windows/tauri.windows.json` y la receta se la pasa
a `tauri build --config`. No es un `tauri.windows.conf.json` junto a `tauri.conf.json` porque
`tauri-build` lo fusionaría en cualquier compilación de Windows, y sus recursos (la `.dll` y el
runtime) tendrían que existir también para `cargo test` y `just dev`. `tauri.conf.json`, que es
el de Linux, no cambia.

- **La `.dll` va junto al ejecutable**: `%LOCALAPPDATA%\rfirma\rfirma.exe` y
  `%LOCALAPPDATA%\rfirma\rfirma_crypto.dll`. Es el sitio donde Windows busca primero las
  dependencias de un programa, y el instalador no tiene un `bin/` desde el que `../lib/rfirma`
  tenga sentido. Ese directorio es el mismo que el de estado de rFirma
  (`%LOCALAPPDATA%\rfirma`), sin choque de nombres: el estado son ficheros propios.
- **El runtime de Visual C++ va al lado, copiado**: `native-image` enlaza la `.dll` contra
  `VCRUNTIME140.dll` y `VCRUNTIME140_1.dll`, que Windows no garantiza. La receta los copia del
  directorio `VC\Redist\MSVC\<versión>\x64\Microsoft.VC14x.CRT` de las Build Tools (la
  instalación local que Microsoft permite redistribuir) y el instalador los deja junto a la
  `.dll`. El UCRT (`api-ms-win-crt-*`) ya es parte de Windows 10. El ejecutable no los
  necesita: `tauri build` enlaza estático el `vcruntime` de Rust.
- **WebView2** llega con el *bootstrapper* que descarga Tauri si falta
  (`downloadBootstrapper`); Windows 11 y los Windows 10 al día ya lo traen.
- **`afirma://` se registra al instalar solo si nadie lo tiene**: si ni `HKCU` ni `HKLM` tienen
  `afirma\shell\open\command`, el instalador escribe la rama del usuario con los mismos
  valores que el adaptador del registro. Es lo que hace el `.desktop` en Linux, que declara
  `x-scheme-handler/afirma` y gana solo si no hay otro. Si AutoFirma ya está, no se toca y la
  elección se hace en el asistente, como hasta ahora.
- **La CA local no se instala al instalar**: Windows pregunta antes de tocar `CurrentUser\Root`
  y el ADR-0005 deja la instalación en manos de la persona, desde el asistente.
- **Al desinstalar** (no en una actualización, que el *bundler* marca con `/UPDATE`) se borra
  la rama `HKCU\Software\Classes\afirma` si apunta a ese ejecutable, y se retiran de
  `CurrentUser\Root` los certificados «rFirma CA local» con `certutil -user -delstore`, que
  pasa por el mismo diálogo de Windows. Una raíz de confianza sin la aplicación que la usa es
  un resto que no se deja. También se borra `HKCU\Software\sgomez\rfirma`, donde el
  *bundler* recuerda la carpeta de instalación. Si se marca «borrar los datos de la aplicación»,
  se borran además `%APPDATA%\rfirma` y `%LOCALAPPDATA%\rfirma`.

Los ganchos están en `packaging/windows/hooks.nsh` (`installerHooks`); la plantilla de Tauri no
se sustituye.

## El CI tiene un carril de Windows

El job `windows` de `ci.yml` corre en `windows-latest` cuando corre el carril de Rust o el
nativo: compila la `.dll` (cacheada con la misma clave que la de Linux y otro `runner.os`), pasa
`rustfmt` y `clippy`, las pruebas de `--lib` y las del canal local (`channel_client`,
`channel_operations`, `service_acknowledgement`) en una pasada instrumentada (`just
test-windows`). Las gradas B y C no corren: faltan softhsm, NSS y poppler.

El instalador no sale de `ci.yml` sino de la release, como el `.deb`, el `.rpm` y el flatpak: el
job `windows` de `build.yml` compila la `.dll` y ejecuta `just bundle-windows`, y el `.exe` pasa
por la misma puerta del contenido (`packaging/verifica-contenido.sh`, que lo abre con `7z`), entra
en el `SHA256SUMS` firmado y en la atestación de procedencia. `release.yml` le añade la firma
minisign del *updater* (ADR-0015) y los deja los dos en la Release, y `publish.yml` los sirve
además desde el servidor propio, en `https://rfirma.sgomez.me/windows/`, junto a apt, dnf y
ostree y con la misma cadena de verificación. Ahí también está el `latest.json` que consulta
la aplicación instalada para actualizarse en modo `passive`, sin pedir administrador. Para
ensayarlo sin etiquetar, `gh workflow run build.yml --ref <rama>`.

Una etiqueta solo lee las cachés de `main`, así que la compilación de release de Windows iría
siempre en frío. El job `windows-release-cache` de `ci.yml` corre la misma receta en `main` con
el cron semanal y a mano, y guarda la caché de Rust con la clave compartida `windows-release`,
que el job de `build.yml` solo lee. Las PR y los push a `main` no lo pagan.

La puerta CRAP del carril rápido de Linux puntúa como 0 % de cobertura lo que Linux no compila, y
el adaptador CNG (`identity/adapters/windows_store/cng.rs`) tiene funciones de complejidad 6 y 7
que así pasan de 30. Es el caso de `--allow` del ADR-0014, cobertura que se mide en otro carril:
`windows_allow` en el `justfile` lo oculta en Linux, y `just test-windows` lo mide con
`cargo crap --path` sobre el lcov de Windows. Otro fichero solo de Windows que cruce el umbral
en Linux entra en la misma variable.

GraalVM CE se fija a **una versión exacta, escrita en un solo sitio**: `.graalvm-version`, hoy
la **25.3.4.1** (JDK 25.0.4.1). La lee el `justfile`, que deriva de ella la ruta de SDKMAN, y la
acción local `.github/actions/setup-graalvm`, por la que pasan todos los jobs de `ci.yml` y
`build.yml`: pide a `setup-graalvm` la etiqueta `graal-<versión>` exacta de `graalvm-ce-builds`,
no la última publicada. Así la librería que se entrega, que lleva el runtime dentro, se construye
con la misma GraalVM con la que se prueba en local. GraalVM CE ya solo publica versiones
*Innovation*, así que la versión se sube a propósito, en una PR propia, cuando sale una que
compila la imagen y pasa la grada C en Linux y en Windows. `check-workflows.sh` falla si un
workflow o el `justfile` escriben otra versión o instalan GraalVM sin esa acción, y la clave de
caché de la librería nativa lleva la versión, así que subirla la reconstruye. En local, con una
25.4, `native-image` se esquiva con
`NATIVE_IMAGE_OPTIONS=--initialize-at-build-time=es.gob.afirma.signers.tsp.pkcs7.TsaParams`.

## `lefthook` llama a scripts de una línea

En Windows `lefthook` le pasa a `sh` solo la primera línea de un `run:` de varias. Cada trabajo
del `pre-push` es ahora una línea que llama a `scripts/pre-push-fmt.sh rust|ts|python`, que
hace lo mismo que hacían los bloques.

## Considered Options

**MSI con WiX** en vez de NSIS: es el formato que prefieren las empresas para desplegar por
directiva, pero el MSI de Tauri instala para la máquina y pide administrador, cuando nada de
rFirma lo necesita. Si alguien lo pide, se puede añadir como segundo formato con la misma
configuración.

**Servir el instalador solo desde la Release**, como al principio: la persona tenía que buscar
el fichero en la página de Releases, descargarlo y ejecutarlo a mano cada vez, y la mayoría se
quedaba en la versión con la que instaló. Linux lo resuelve con apt, dnf y ostree; en Windows
solo lo resuelve un *updater* con un `latest.json` estable en un origen propio.

**Ejecutar `VC_redist.x64.exe` al instalar**: instala el runtime para todo el equipo y pide
administrador. La copia local son dos ficheros, unos 170 KB, junto a la `.dll`.

**Seguir en la GraalVM CE 25.0.2**, a la que resolvía `'25'`: es la última de la línea 25.0.x
(enero de 2026) y después no ha salido ninguna, así que la librería se quedaba sin los parches
de seguridad del JDK. Además no era la de local, y dos compilaciones de la 25 dejan alcanzables
clases distintas: la grada C pasaba en local y fallaba en el CI.

**Pedir `java-version: '25'` o `version: '25.3'`** a `setup-graalvm`: resuelven a la última
publicada el día que corre el CI, que deja de construir con la de local sin que cambie una línea.

**La 25.4**: `native-image` muere con un error interno del compilador en
`PdfTimestamper.initialize()`. Pasar a ella es otro trabajo.

**Descargar el tarball de `graalvm-ce-builds` y comprobar su sha256** en vez de `setup-graalvm`:
la acción ya fija la *Innovation* exacta con `version:`, y en Windows deja el entorno de Visual
Studio con el que enlazan `native-image` y `rustc`. No comprueba el sha256: baja el fichero de
la Release por HTTPS, como antes.

**Una versión que sirva también a macOS Intel**: GraalVM CE solo publica macOS x64 hasta la
25.0.1, y el Mac Intel queda fuera de alcance, así que no condiciona la versión.

**Llevar `--initialize-at-build-time=...TsaParams` a `native-image.properties`**: arreglaría la
25.4 también en local, pero cambia cuándo se inicializa una clase de AutoFirma en la imagen de
Linux, que hoy funciona. Fijar la versión no toca la imagen.

**Registrar siempre `afirma://` al instalar**: la rama del usuario gana a la de la máquina, así
que taparía a AutoFirma sin preguntar.

**Retirar la CA desde la propia aplicación al desinstalar** (`rfirma.exe --uninstall`): sabría
exactamente qué certificado es, pero añade un modo de línea de órdenes solo para el
desinstalador. `certutil` por el nombre común hace lo mismo con lo que trae Windows.


**OpenSSL del sistema en Windows** (`OPENSSL_DIR`, vcpkg): deja el `Cargo.lock` intacto, pero
obliga a cada equipo y al CI a instalarlo, y a distribuir sus DLL junto al binario. Se descartó
porque la construcción deja de ser reproducible con solo `cargo`.

**Sustituir `openssl` por crates de Rust puro**: reescribe código que en Linux funciona. Queda
fuera de una fase que solo busca compilar.

**Pasar `-o rfirma_crypto` a `native-image` en Windows**: evita renombrar, pero devuelve banderas
sueltas al `justfile`, y `native-image.properties` es el único sitio de las banderas de la imagen.

**Recetas duplicadas con `[windows]` y `[linux]`**, o PowerShell como shell de Windows: dos
versiones de cada receta que envejecen por separado. Git Bash ya está en cualquier equipo que
tenga Git, y los scripts de `scripts/` son de `bash`.

**Un alias de `cfg` desde `build.rs`** (`cfg(gtk_desktop)`) para no tocar la guarda: esconde el
sistema operativo tras un nombre que la guarda no ve. Se descartó por eso mismo.

**El DNIe solo por PKCS#11 en Windows**, como en Linux: reutiliza todo el código, pero exige
instalar OpenSC o el módulo de la Policía y el diálogo propio del PIN, que en Windows no existe.
El minidriver ya lo trae el sistema y su PIN lo pide Windows, así que PKCS#11 queda como segunda
vía.

**`CRYPT_ACQUIRE_PREFER_NCRYPT_KEY_FLAG`** en vez de `ONLY`: devolvería un `HCRYPTPROV` de
CryptoAPI para las claves de un CSP antiguo, y habría que firmar también con `CryptSignHash`.
Con `ONLY`, CNG abre esas claves por su capa de compatibilidad y hay un único camino de firma.

**El crate `windows`** en vez de `windows-sys`: envuelve los tipos, pero pesa más en compilación
y aquí bastan una docena de funciones.

**Una clase de almacén propia para `CurrentUser\MY`**: cambia el dominio, las vistas y la
interfaz también en Linux. Se aplaza; mientras, el almacén del usuario se presenta como `Card`.

## Consequences

- `tauri-build` exige en Windows `icons/icon.ico`; se empaqueta con los PNG que ya existían.
- `tauri-build` solo pone el manifiesto de Common Controls v6 al binario, y los ejecutables de
  prueba que enlazan Tauri mueren al arrancar con `STATUS_ENTRYPOINT_NOT_FOUND`. En Windows,
  `build.rs` enlaza `windows-app-manifest.xml` en todo lo que se enlaza, y por eso está en
  `AUTHORISED_SITES`.
- `just tools`, `just bootstrap`, `just native`, `just dev`, `just fmt` y `just bundle-windows`
  funcionan en Windows; el resto de recetas (`check`, `flatpak`, `bundle`, `certs`…) sigue siendo
  de Linux.
- En Windows la aplicación busca la `.dll` junto al ejecutable; en Linux, en `../lib/rfirma`.
- El instalador no está firmado con Authenticode: SmartScreen avisa al abrirlo la primera vez.
  Las actualizaciones las verifica el *updater* con la minisign, no Windows.
- Un cambio solo en `packaging/windows/` no enciende el carril de Windows en un PR, porque
  `ci-lanes.sh` no tiene un carril propio para él.
- `cargo test` en Windows no corre las pruebas de grada B y C que necesitan softhsm o NSS.
- Las pruebas de `identity/adapters/windows_store/tests.rs` crean certificados autofirmados en
  `Cert:\CurrentUser\My` con `New-SelfSignedCertificate` y los borran con su clave al acabar.
  La que firma XAdES, CAdES y PAdES con el puente está `#[ignore]` y necesita `RFIRMA_LIB_DIR`.
- El canal TLS local de Windows es de rustls (ADR-0036).
- Las pruebas del almacén raíz, del registro y del Administrador de credenciales trabajan sobre
  un almacén `rfirma-test-*`, una rama `HKCU\Software\rfirma-test-*` y una credencial
  `rfirma-test/*` propios, y los borran al acabar. Instalar de verdad en `CurrentUser\Root` abre
  el diálogo de Windows y no se automatiza: se prueba a mano.
- Una asociación elegida por la persona en `UrlAssociations\afirma\UserChoice` manda sobre
  `Classes`, y rFirma no la lee todavía.
- `site/adapters/mod.rs`, `desktop/adapters/registry.rs` y `site/adapters/channel/acceptor.rs`
  entran en `AUTHORISED_SITES`: son el punto donde cada puerto elige su adaptador de plataforma.
- `documents/domain/dropped/tests.rs` también entra: una URL `file://` nombra una ruta distinta en
  cada sistema (con unidad y barras invertidas en Windows), y la prueba lleva un caso por sistema.
- El certificado de un servicio de sede que no se reconoce (ADR-0039) llega en Windows como un
  código de Schannel y no como el `certificate verify failed` de OpenSSL: la detección acepta los
  dos sin condicional de sistema, porque los códigos no se confunden con nada en Linux.
