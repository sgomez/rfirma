> [!IMPORTANT]
> **rFirma no tiene ninguna relación con la Administración pública española.**
>
> No es un proyecto oficial, no está respaldado, autorizado ni supervisado por ningún
> organismo público, y no lo desarrolla ni lo mantiene el equipo de AutoFirma. Es un
> proyecto personal e independiente, sin ningún vínculo con el Gobierno de España, con
> el Ministerio para la Transformación Digital y de la Función Pública ni con la
> FNMT-RCM. Los nombres AutoFirma y FNMT se citan solo para describir con qué es
> compatible.
>
> Para el programa oficial, acude a la web de la Administración.

# rFirma: Firma Electrónica Nativa

**rFirma** —`rfirma` como identificador: binario, paquete y `.desktop`— es una reimplementación moderna y nativa de la herramienta de firma de la administración española **AutoFirma**. Combina el rendimiento y la ligereza de **Tauri v2 (Rust + React)** para la interfaz, con la madurez del motor criptográfico original de Java compilado a código nativo mediante **GraalVM Native Image**.

---

## 🚀 Características Clave
* **Sin Dependencia de JRE:** Se ejecuta directamente como código de máquina nativo sin necesidad de tener Java instalado en el equipo del usuario.
* **Tres canales:** flatpak para cualquier distribución de Linux, y `.deb` y `.rpm` para las nativas. El motor criptográfico compilado se instala junto a la aplicación —`/app/lib/rfirma/` en el flatpak, `/usr/lib/rfirma/` en los nativos— y se carga dinámicamente al arrancar (ver [ADR-0004](docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)).
* **Arranque Instantáneo:** Reduce los tiempos de arranque de ~3 segundos a menos de 100ms y el consumo de RAM a ~30-50MB.
* **Integración del Sistema Operativo:** Acceso rápido y nativo a almacenes de certificados (DNI electrónico, FNMT) mediante APIs del sistema y PKCS#11.
* **Interfaz Moderna:** Rediseño completo en React sobre un sistema de diseño propio en CSS (`docs/design/design-system.md`), en reemplazo de la interfaz Swing obsoleta.

---

## 🏛️ Arquitectura del Proyecto

El proyecto está diseñado de forma modular para desacoplar la interfaz y la integración con el sistema de la lógica criptográfica pesada:

1. **Frontend (React + Vite):** Interfaz gráfica minimalista para la selección y filtrado de certificados e introducción de PIN.
2. **Tauri Backend (Rust):**
   * Levanta un servidor local HTTPS/WS en `127.0.0.1` para comunicarse con las sedes electrónicas: el `63117` fijo con el protocolo 3, uno de los puertos que sortea la sede con el 4 (ver [ADR-0005](docs/adr/0005-servidor-local-https-y-ca-en-los-almacenes-nss.md)).
   * Maneja el protocolo `afirma://`.
   * Realiza la lectura y firma nativa de los certificados locales (incluyendo tarjetas inteligentes PKCS#11).
3. **GraalVM FFI Bridge (Java Core):** Librería nativa compilada (**un solo fichero**, `librfirma_crypto.so`, ver [ADR-0004](docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)) que recibe los datos en formato JSON mediante FFI y procesa las fases de **Prefirma** y **Postfirma** (generación de los contenedores CAdES, PAdES, XAdES y FacturaE).

---

## 🛠️ Instrucciones de Construcción y Ejecución

Todo pasa por `just`, que es el único punto de entrada del repositorio
([ADR-0013](docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md)).
`just --list` enseña el resto de recetas.

```bash
just bootstrap   # dependencias de AutoFirma en ~/.m2 (no estan en Maven Central)
just native      # librfirma_crypto.so (rfirma_crypto.dll en Windows, librfirma_crypto.dylib en macOS) con GraalVM CE 25; tarda minutos
just dev         # levanta la aplicacion contra esa libreria
```

`just tools` comprueba las herramientas y falla nombrando la que falte. `just
check` es lo mismo que ejecuta el CI.

En Windows las recetas se lanzan desde Git Bash, con GraalVM en `GRAALVM_HOME`
o `JAVA_HOME`, las Build Tools de Visual Studio (con C++ y el Windows SDK),
Strawberry Perl, Node con pnpm y WebView2
([ADR-0035](docs/adr/0035-windows-como-segunda-plataforma-dependencias-por-target-y-adaptadores-no-disponibles.md)):

```bash
git config --global core.autocrlf false                # antes de clonar: los .sh necesitan LF
pnpm config set script-shell "C:/Program Files/Git/bin/bash.exe"
export OPENSSL_SRC_PERL=C:/Strawberry/perl/bin/perl.exe  # la primera compilacion de OpenSSL
just native                                            # rfirma_crypto.dll
just bundle                                            # instalador NSIS en $CARGO_TARGET_DIR/release/bundle/nsis
```

Con una GraalVM CE 25.4, `just native` necesita además
`NATIVE_IMAGE_OPTIONS=--initialize-at-build-time=es.gob.afirma.signers.tsp.pkcs7.TsaParams`;
el CI usa la de `GRAALVM_VERSION` de `versions.env`. El instalador es por usuario y no pide administrador: deja
`rfirma.exe`, `rfirma.com` (el binario de consola), `rfirma_crypto.dll` y el runtime de Visual C++ en
`%LOCALAPPDATA%\rfirma`, añade esa carpeta al PATH del usuario y registra `afirma://` si ningún otro
programa lo tiene.

En macOS (Apple Silicon) hacen falta las Command Line Tools de Xcode
(`xcode-select --install`), GraalVM CE 25 en `GRAALVM_HOME`, `JAVA_HOME` o
registrada en `/usr/libexec/java_home`, Maven y Node con pnpm; gettext solo
para `just po`:

```bash
just native        # librfirma_crypto.dylib
just dev
just bundle        # rfirma.app y el .dmg en target/aarch64-apple-darwin/release/bundle
```

La `.app` lleva `librfirma_crypto.dylib` en `Contents/Frameworks`, registra
`afirma://` en su `Info.plist` y va firmada ad hoc, sin notarizar: Gatekeeper
la bloquea la primera vez y hay que permitirla en Ajustes del Sistema ›
Privacidad y seguridad.

`just dev` **no** construye la librería nativa: si falta, falla diciendo que
ejecutes `just native`. Es deliberado — `native-image` tarda minutos y no debe
dispararse por sorpresa al tocar una línea de la interfaz.

`just native` produce **un solo fichero**, `librfirma_crypto.so`, y el manifiesto
instala ese fichero por su nombre. Los cinco auxiliares de AWT que `native-image`
sigue dejando en su directorio de construcción **no se copian nunca**: con
`libawt.so` al lado, un JPEG con perfil ICC deja de dar un error recuperable y
**aborta el proceso entero**
([ADR-0012](docs/adr/0012-normalizacion-de-la-rubrica-en-rust.md),
[`docs/research/exclusion-afirma-ui-utils.md`](docs/research/exclusion-afirma-ui-utils.md)).

## ⌨️ Línea de órdenes

`rfirma` atiende en la terminal las órdenes de AutoFirma, siempre como primer
argumento y sin unirse a la ventana abierta: `sign`, `cosign`, `listaliases` y
`verify`, con `-i`, `-o`, `--format`, `--store`, `--alias`, `--filter`, `--algorithm`,
`--config` y `--xml`. `rfirma --help` las describe todas, `rfirma <orden> --help`
da la sintaxis de cada una, y `rfirma --version` dice la versión de rFirma y la de
AutoFirma de la que salen los validadores. El código de salida es 0 si termina bien; por la
salida estándar solo sale lo que se consume (el XML de `--xml`) y los mensajes van
a la de errores.

```bash
rfirma listaliases
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --alias mi-certificado
```

**La contraseña no se acepta en la línea de órdenes** (`--password` se rechaza:
la ve cualquier usuario del equipo y queda en el historial). Si el almacén no
resuelve el PIN solo, se pide en la terminal sin eco, o se lee de un descriptor
con `--password-fd`. Con `secret-tool`, el llavero del escritorio (GNOME Keyring,
KWallet, KeePassXC) lo entrega sin pasar por argv:

```bash
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --alias mi-certificado \
    --password-fd 3 3< <(secret-tool lookup service rfirma)
```

### Ejemplo con aerc

[aerc](https://aerc-mail.org) puede firmar el adjunto que tengas seleccionado
con un atajo en `binds.conf`, ejecutando `rfirma` por `:pipe`. Aquí se firma el
PDF recibido y el resultado queda en `~/Documents` (cada firma sobrescribe la
anterior: cambia el nombre de salida si no quieres eso). Hace falta `bash`, no
`sh`, por la sustitución de procesos:

```ini
[view]
S = :pipe -b bash -c 'f=$(mktemp --suffix=.pdf) && trap "rm -f $f" EXIT && cat > "$f" && rfirma sign -i "$f" -o ~/Documents/firmado.pdf --alias mi-certificado --password-fd 3 3< <(secret-tool lookup service rfirma)'<Enter>
```

### En el flatpak

Dentro del sandbox, `rfirma` solo ve la carpeta de documentos (`xdg-documents`). Para un fichero que esté
fuera, `--file-forwarding` y `@@` hacen que flatpak lo exponga por el portal:

```bash
flatpak run --file-forwarding me.sgomez.rfirma sign -i @@ ~/Descargas/contrato.pdf @@ -o ~/Documents/firmado.pdf
```

### En Windows

El instalador deja `rfirma.com`, el binario de consola, junto a `rfirma.exe`, y pone la carpeta de
rFirma en el PATH del usuario: en una consola abierta después de instalar, `rfirma` resuelve al
`.com`, que escribe en la consola y devuelve el código de salida. En cmd y en PowerShell se escribe
igual que en Linux:

```powershell
rfirma listaliases
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --alias mi-certificado
```

`--store windows` (o `auto`) acota al almacén de certificados de Windows, y el PIN de sus
certificados lo pide Windows. **En Git Bash dentro de Windows Terminal hay que escribir
`rfirma.com`**: bash no prueba la extensión `.com` y encuentra `rfirma.exe`, el de la ventana, que
ahí no responde en la consola. En la ventana propia de Git Bash (mintty), `rfirma` funciona tal
cual, porque `rfirma.exe` responde por las tuberías de mintty.

`--password-fd` y `--certtui` todavía no están en Windows.

---

## 🌐 Traducciones

Quien traduce revisa sus textos en la pantalla real, sin montar el entorno, en el
[Storybook publicado](https://sgomez.github.io/rfirma/): se reconstruye con cada
push a `main`, y su barra de herramientas cambia el idioma y el tema. Los textos
viven en `rfirma-app/po/`; en local, `just storybook`.

## 📦 Instalación

Los canales son **tres** —flatpak, `.deb` y `.rpm`—, todos servidos desde
`rfirma.sgomez.me` y desde las Releases de GitHub
([ADR-0004](docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md),
[ADR-0015](docs/adr/0015-canal-de-distribucion-propio.md)). No hace falta tener
Java: el motor criptográfico va compilado dentro.

### Elige un canal

**Elige uno solo.** Instalar rFirma por dos vías son dos aplicaciones con
memorias separadas: ni los documentos recientes, ni la rúbrica, ni las
preferencias se comparten, y no se migran. Es la conducta normal de Linux —el
Firefox flatpak y el `.deb` tampoco comparten perfil—.

> **Los canales nativos todavía no existen**: los construye el hito del canal
> propio y aún no se han publicado. Hasta que la primera Release los sirva, lo
> único instalable es el flatpak que produce `just flatpak`, en local.

**La vía recomendada es añadir el repositorio**, no descargar un fichero suelto:
una vez dado de alta, las actualizaciones llegan solas con el gestor de
paquetes del sistema (ADR-0015). Elige tu canal en
<https://rfirma.sgomez.me>, que da la orden completa para flatpak, apt y dnf.

El bundle de flatpak **no trae el runtime**: se consume del remoto de
**Flathub**, así que añadirlo es requisito de instalación. Es de un solo uso, y
`--user` no pide permisos de administración:

```bash
flatpak remote-add --user --if-not-exists \
    flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

#### Descarga suelta (excepción)

Si no quieres dar de alta el repositorio, cada canal también se sirve como
fichero suelto en las Releases de GitHub. Es la vía sin actualizaciones
automáticas: hay que repetir la descarga a mano en cada versión. Las
descargas van **siempre a la última publicación**, sin número de versión en
el enlace: así el README no envejece y no hay que sincronizarlo con nada
(ID-151). Los nombres de los ficheros publicados no llevan versión, que es lo
que hace que estos enlaces resuelvan:

* flatpak: <https://github.com/sgomez/rfirma/releases/latest/download/me.sgomez.rfirma.flatpak>
* `.deb`: <https://github.com/sgomez/rfirma/releases/latest/download/rfirma_amd64.deb>
* `.rpm`: <https://github.com/sgomez/rfirma/releases/latest/download/rfirma.x86_64.rpm>

Las candidatas (`-rc.N`) publican **solo el flatpak**: el campo `Version` de un
RPM no admite guiones, así que un `.deb` o un `.rpm` de una candidata no existe.

Con el remoto de Flathub puesto (arriba), `flatpak install` resuelve
`org.gnome.Platform//50` solo:

```bash
just flatpak                                          # produce el .flatpak
flatpak install --user packaging/flatpak/me.sgomez.rfirma.flatpak
flatpak run me.sgomez.rfirma
```

---

## 📄 Licencia
Este proyecto es software libre y está licenciado bajo las mismas condiciones que el Cliente @firma original (**GPL 2.0+** y **EUPL 1.1**).
