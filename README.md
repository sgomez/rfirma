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
just native      # los seis .so con GraalVM CE 25; tarda minutos
just dev         # levanta la aplicacion contra esa libreria
```

`just tools` comprueba las herramientas y falla nombrando la que falte. `just
check` es lo mismo que ejecuta el CI.

`just dev` y `just build` **no** construyen la librería nativa: si falta, fallan
diciendo que ejecutes `just native`. Es deliberado — `native-image` tarda minutos
y no debe dispararse por sorpresa al tocar una línea de la interfaz.

> **Hoy solo existen `bootstrap`, `tools`, `lint`, `build`, `test`, `check`,
> `native` y `clean`, y ninguna toca la interfaz: el repositorio todavía no
> tiene código de producción.** Las recetas de arriba son la rejilla que fija el
> ADR-0013 y las construye el
> [issue #10](https://github.com/sgomez/rfirma/issues/10).

---

## 📄 Licencia
Este proyecto es software libre y está licenciado bajo las mismas condiciones que el Cliente @firma original (**GPL 2.0+** y **EUPL 1.1**).
