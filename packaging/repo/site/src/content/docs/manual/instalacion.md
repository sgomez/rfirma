---
title: Instalación
description: "Cómo instalar rFirma en Linux (apt, rpm y flatpak), Windows y macOS, y cómo comprobar la huella de la clave GPG y los ficheros que descargas."
---

rFirma se instala desde un repositorio de tu sistema operativo, de modo que las
actualizaciones llegan con el gestor de paquetes. No necesitas tener Java: el motor
criptográfico va dentro del paquete.

**Elige un solo canal.** Si instalas rFirma por dos vías, tendrás dos aplicaciones
con memorias separadas: no comparten los documentos recientes, ni la rúbrica, ni las
preferencias.

## Linux con apt (Debian, Ubuntu y derivadas)

Añade la clave pública y el repositorio, y instala el paquete:

```bash
curl -fsSL https://rfirma.sgomez.me/rfirma.asc | sudo tee /usr/share/keyrings/rfirma.asc >/dev/null
sudo tee /etc/apt/sources.list.d/rfirma.sources <<'EOF'
Types: deb
URIs: https://rfirma.sgomez.me/apt/
Suites: stable
Components: main
Signed-By: /usr/share/keyrings/rfirma.asc
EOF
sudo apt update && sudo apt install rfirma
```

Las actualizaciones llegan con `sudo apt upgrade`.

## Linux con rpm (Fedora y derivadas)

```bash
sudo tee /etc/yum.repos.d/rfirma.repo <<'EOF'
[rfirma]
name=rfirma
baseurl=https://rfirma.sgomez.me/rpm/
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=https://rfirma.sgomez.me/rfirma.asc
EOF
sudo dnf install rfirma
```

Con `gpgcheck=1` y `repo_gpgcheck=1`, `dnf` comprueba la firma de los paquetes y la de
los metadatos del repositorio.

## Linux con flatpak

Es la opción para las distribuciones que no usan apt ni dnf. Necesitas tener
instalados `flatpak` y `xdg-desktop-portal`. El remoto de rFirma se da de alta
como un repositorio más, después del de Flathub, de donde sale el runtime
`org.gnome.Platform`:

```bash
flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak remote-add --if-not-exists rfirma https://rfirma.sgomez.me/rfirma.flatpakrepo
flatpak install rfirma me.sgomez.rfirma
```

Con el remoto dado de alta así, rFirma aparece al buscar en GNOME Software y las
versiones nuevas llegan con las actualizaciones del sistema.

Bajo flatpak, rFirma solo ve las carpetas que le das a través del portal del
sistema. Por eso, en [Preferencias](/manual/preferencias/), el destino de los
documentos firmados es siempre una carpeta elegida: no existe «junto al original».

### Por qué GNOME Software dice «potencialmente insegura»

Si abres rFirma en GNOME Software verás la etiqueta «Potencialmente insegura». Es
esperable y no indica que haya un problema: GNOME Software la pone a cualquier
aplicación que pide acceso a carpetas concretas de tu equipo, y no distingue para qué.

rFirma pide esos accesos porque los necesita para firmar:

* **Certificados de tus navegadores.** Para ofrecerte los que ya tienes instalados en
  Firefox (también si es Snap o Flatpak), LibreWolf, Chrome y Chromium.
* **Lector de tarjetas.** Para hablar con él cuando firmas con el DNIe.
* **Tu navegador.** Para que pueda llegar a rFirma en tu equipo cuando una web te pide
  firmar.
* **Versiones nuevas.** Para avisarte cuando hay una disponible.
* **La carpeta Documentos.** Para guardar en ella los documentos firmados.

## Windows

Descarga el instalador `rFirma_*_x64-setup.exe` de la
[última Release](https://github.com/sgomez/rfirma/releases/latest). Se instala para tu
usuario, sin permisos de administrador, y usa directamente el almacén de certificados
de Windows.

El instalador no está firmado con Authenticode, así que SmartScreen avisará al abrirlo.
Antes de ejecutarlo, comprueba que el fichero descargado es el publicado: baja
`SHA256SUMS` de la misma Release y compara el hash con la línea del instalador.

```powershell
Get-FileHash .\rFirma_*_x64-setup.exe -Algorithm SHA256
Select-String rFirma_ .\SHA256SUMS
```

Una vez instalado, rFirma se actualiza desde la propia aplicación: cada versión nueva
llega con su firma minisign, que se comprueba antes de instalarla.

## macOS

La versión para macOS está en preparación y todavía no se puede instalar. Se
integrará con el llavero de Apple. Puedes seguir el avance en el
[repositorio](https://github.com/sgomez/rfirma).

## La huella de la clave GPG

Los paquetes y los repositorios de rFirma se firman con una clave GPG. Su parte
pública está en [rfirma.asc](https://rfirma.sgomez.me/rfirma.asc). Tras descargarla,
comprueba su huella:

```bash
gpg --show-keys rfirma.asc
```

La huella debe ser esta:

```text
C8D6 A81C 1ED4 3A28 D426  8112 A6E0 EE02 2344 6A16
```

Si no coincide, no la uses.

## Verificar las descargas

Con los ficheros `SHA256SUMS` y `SHA256SUMS.asc` de la Release, importada ya la clave:

```bash
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check SHA256SUMS
```

Un `.rpm` suelto lleva además la firma dentro:

```bash
sudo rpm --import https://rfirma.sgomez.me/rfirma.asc
rpm --checksig rfirma-*.rpm
```

Los `.deb` no se firman uno a uno, porque apt firma el índice del repositorio: un
`.deb` suelto se verifica con `SHA256SUMS.asc`.

## Después de instalar

La primera vez que abras rFirma, un asistente instala el certificado que hace segura la
conexión del navegador con la aplicación y deja rFirma como el programa que abren las sedes
electrónicas. Después, [firma tu primer PDF](/manual/firmar-un-pdf/).

## Antes de desinstalar

Para no dejar el certificado de rFirma en el navegador, pulsa *Retirar certificado* en el panel de estado antes de desinstalar.
