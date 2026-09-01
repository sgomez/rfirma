# rfirma: Firma Electrónica Nativa

`rfirma` es una reimplementación moderna y nativa de la herramienta de firma de la administración española **AutoFirma**. Combina el rendimiento y la ligereza de **Tauri v2 (Rust + React)** para la interfaz, con la madurez del motor criptográfico original de Java compilado a código nativo mediante **GraalVM Native Image**.

---

## 🚀 Características Clave
* **Sin Dependencia de JRE:** Se ejecuta directamente como código de máquina nativo sin necesidad de tener Java instalado en el equipo del usuario.
* **Distribución en flatpak:** Canal único, para cualquier distribución de Linux. El motor criptográfico compilado se instala junto a la aplicación en `/app/lib/rfirma/` y se carga dinámicamente al arrancar (ver [ADR-0004](docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)).
* **Arranque Instantáneo:** Reduce los tiempos de arranque de ~3 segundos a menos de 100ms y el consumo de RAM a ~30-50MB.
* **Integración del Sistema Operativo:** Acceso rápido y nativo a almacenes de certificados (DNI electrónico, FNMT) mediante APIs del sistema y PKCS#11.
* **Interfaz Moderna:** Rediseño completo en React sobre un sistema de diseño propio en CSS (`docs/design/design-system.md`), en reemplazo de la interfaz Swing obsoleta.

---

## 🏛️ Arquitectura del Proyecto

El proyecto está diseñado de forma modular para desacoplar la interfaz y la integración con el sistema de la lógica criptográfica pesada:

1. **Frontend (React + Vite):** Interfaz gráfica minimalista para la selección y filtrado de certificados e introducción de PIN.
2. **Tauri Backend (Rust):**
   * Levanta un servidor local HTTPS/WS seguro (`127.0.0.1:63117`) para comunicarse con las sedes electrónicas.
   * Maneja el protocolo deep link `rfirma://` y `afirma://`.
   * Realiza la lectura y firma nativa de los certificados locales (incluyendo tarjetas inteligentes PKCS#11).
3. **GraalVM FFI Bridge (Java Core):** Librería nativa compilada (`librfirma_crypto.so` más cinco auxiliares, ver [ADR-0004](docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)) que recibe los datos en formato JSON mediante FFI y procesa las fases de **Prefirma** y **Postfirma** (generación de los contenedores CAdES, PAdES, XAdES y FacturaE).

---

## 🛠️ Instrucciones de Construcción y Ejecución

Todo pasa por `just`, que es el único punto de entrada del repositorio
([ADR-0013](docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md)).
`just --list` enseña el resto de recetas.

```bash
just bootstrap   # dependencias de AutoFirma en ~/.m2 (no estan en Maven Central)
just native      # librfirma_crypto.so con GraalVM CE 25; tarda minutos
just dev         # levanta la aplicacion contra esa libreria
```

`just tools` comprueba las herramientas y falla nombrando la que falte. `just
check` es lo mismo que ejecuta el CI.

`just dev` y `just build` **no** construyen la librería nativa: si falta, fallan
diciendo que ejecutes `just native`. Es deliberado — `native-image` tarda minutos
y no debe dispararse por sorpresa al tocar una línea de la interfaz.

`just native` produce **un solo fichero**, `librfirma_crypto.so`, y el manifiesto
instala ese fichero por su nombre. Los cinco auxiliares de AWT que `native-image`
sigue dejando en su directorio de construcción **no se copian nunca**: con
`libawt.so` al lado, un JPEG con perfil ICC deja de dar un error recuperable y
**aborta el proceso entero**
([ADR-0012](docs/adr/0012-normalizacion-de-la-rubrica-en-rust.md),
[`docs/research/exclusion-afirma-ui-utils.md`](docs/research/exclusion-afirma-ui-utils.md)).

## 📦 Instalación

El **flatpak es el único canal soportado**
([ADR-0004](docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md),
[ADR-0015](docs/adr/0015-canal-de-distribucion-propio.md)). No hace falta tener
Java: el motor criptográfico va compilado dentro.

El bundle **no trae el runtime**: se consume del remoto de **Flathub**, así que
añadirlo es requisito de instalación. Es de un solo uso, y `--user` no pide
permisos de administración:

```bash
flatpak remote-add --user --if-not-exists \
    flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

Con el remoto puesto, `flatpak install` resuelve `org.gnome.Platform//50` solo:

```bash
just flatpak                                          # produce el .flatpak
flatpak install --user packaging/flatpak/me.sgomez.rfirma.flatpak
flatpak run me.sgomez.rfirma
```

> **v0.1 no se publica** ([ID-42](https://github.com/sgomez/rfirma/issues/46)):
> el entregable es ese fichero `.flatpak`, instalable en local. El canal del
> ADR-0015 —Releases con `SHA256SUMS`, repositorio ostree firmado con GPG en
> `rfirma.sgomez.me` y `.flatpakref`— exige una clave GPG y un secreto de
> despliegue que crea una persona, y queda fuera de este hito.

---

## 📄 Licencia
Este proyecto es software libre y está licenciado bajo las mismas condiciones que el Cliente @firma original (**GPL 2.0+** y **EUPL 1.1**).
