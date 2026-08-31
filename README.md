# rfirma: Firma Electrónica Nativa

`rfirma` es una reimplementación moderna y nativa de la herramienta de firma de la administración española **AutoFirma**. Combina el rendimiento y la ligereza de **Tauri v2 (Rust + Svelte)** para la interfaz y el servidor local, con la madurez del motor criptográfico original de Java compilado a código nativo mediante **GraalVM Native Image**.

---

## 🚀 Características Clave
* **Sin Dependencia de JRE:** Se ejecuta directamente como código de máquina nativo sin necesidad de tener Java instalado en el equipo del usuario.
* **Distribución en `.deb`:** El motor criptográfico compilado se instala junto a la aplicación en `/usr/lib/rfirma/` y se carga dinámicamente al arrancar (ver [ADR-0004](docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)).
* **Arranque Instantáneo:** Reduce los tiempos de arranque de ~3 segundos a menos de 100ms y el consumo de RAM a ~30-50MB.
* **Integración del Sistema Operativo:** Acceso rápido y nativo a almacenes de certificados (DNI electrónico, FNMT) mediante APIs del sistema y PKCS#11.
* **Interfaz Moderna:** Diseño moderno basado en Svelte y Tailwind CSS en reemplazo de la interfaz Swing obsoleta.

---

## 🏛️ Arquitectura del Proyecto

El proyecto está diseñado de forma modular para desacoplar la interfaz y la integración con el sistema de la lógica criptográfica pesada:

1. **Frontend (Svelte + Tailwind):** Interfaz gráfica minimalista para la selección y filtrado de certificados e introducción de PIN.
2. **Tauri Backend (Rust):**
   * Levanta un servidor local HTTPS/WS seguro (`127.0.0.1:63117`) para comunicarse con las sedes electrónicas.
   * Maneja el protocolo deep link `rfirma://` y `afirma://`.
   * Realiza la lectura y firma nativa de los certificados locales (incluyendo tarjetas inteligentes PKCS#11).
3. **GraalVM FFI Bridge (Java Core):** Librería nativa compilada (`libautofirma_crypto.so`) que recibe los datos en formato JSON mediante FFI y procesa las fases de **Prefirma** y **Postfirma** (generación de los contenedores CAdES, PAdES, XAdES y FacturaE).

---

## 🛠️ Instrucciones de Construcción y Ejecución

Para compilar el proyecto por primera vez, sigue estos pasos en orden:

### Paso 1: Instalar dependencias Java oficiales de Autofirma
Dado que las librerías oficiales de Autofirma no están publicadas en Maven Central, el proyecto incluye un script de automatización (`bootstrap.sh`) que las clona, compila e instala localmente en tu caché de Maven (`~/.m2`):
```bash
./bootstrap.sh
```
Este script se encargará de comprobar si ya dispones de las dependencias; si faltan, las obtendrá directamente del repositorio oficial de la administración ([ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma)), registrará los paquetes en tu máquina local y luego limpiará el directorio temporal de descarga.

### Paso 2: Compilar el Bridge Nativo
Compila el bridge intermedio que genera la librería dinámica nativa usando GraalVM:
```bash
cd rfirma-native-bridge
mvn clean package
```
Esto creará el archivo `target/libautofirma_crypto.so`.

### Paso 3: Lanzar la aplicación en modo desarrollo
Ve a la aplicación de Tauri y lánzala:
```bash
cd ../rfirma-app
npm install
npm run tauri dev
```

---

## 📄 Licencia
Este proyecto es software libre y está licenciado bajo las mismas condiciones que el Cliente @firma original (**GPL 2.0+** y **EUPL 1.1**).
