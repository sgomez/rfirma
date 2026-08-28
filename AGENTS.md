# Instrucciones para Agentes de IA: rfirma

Este archivo contiene el contexto técnico esencial, las restricciones de diseño y el estado actual del proyecto **rfirma** para guiar a los agentes autónomos de codificación que continúen con la implementación de esta aplicación.

---

## 🎯 Objetivo del Proyecto
Reemplazar la interfaz Swing y el servidor sockets en Java de **AutoFirma** (cuyo repositorio oficial es [ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma)) por una aplicación nativa en **Tauri v2 (Rust + Svelte)**. La lógica criptográfica pesada (CAdES, PAdES, XAdES, FacturaE) se delega a una biblioteca compartida compilada con **GraalVM Native Image** a partir de la base de código original de Autofirma.

---

## ⚠️ Restricciones Críticas de Diseño (Must-Know)

1. **Sin Duplicación de Código de dependencias Java:**
   * **No copies ni hagas enlaces simbólicos** de los submódulos originales de Java (`afirma-core`, `afirma-crypto-*`, etc.) en este repositorio.
   * Deben consumirse estrictamente desde la caché local de Maven (`~/.m2`) como dependencias ordinarias en el `pom.xml` de `rfirma-native-bridge`.
   * El repositorio original se compila mediante `mvn clean install` en su propia ubicación.
   
2. **Gestión de Memoria en la Frontera FFI (Evitar Double-Free):**
   * GraalVM Native Image libera automáticamente la memoria de los strings C creados con `CTypeConversion.toCString(...)` al salir del bloque de recurso (`try-with-resources`).
   * Para retornar cadenas JSON desde Java a Rust de forma segura, se debe usar la asignación manual en el C-heap mediante `UnmanagedMemory.malloc(bytes.length + 1)`.
   * Rust es responsable de llamar al método FFI `autofirma_free_string(thread, ptr)` una vez leído el JSON para evitar fugas de memoria.

3. **Incrustación Dinámica del `.so` (Distribución Portable):**
   * No enlaces la librería criptográfica `.so`/`.dll`/`.dylib` en tiempo de compilación mediante `build.rs` o variables `LD_LIBRARY_PATH`.
   * El backend de Rust debe usar la macro `include_bytes!` para guardar la librería dentro del binario.
   * Al iniciar, comprueba y extrae el archivo en el directorio de la caché del usuario (ej. `~/.cache/rfirma/libautofirma_crypto.so`) y cárgalo dinámicamente con `libloading`.

4. **Firma Trifásica (Triphase Signing):**
   * La clave privada **nunca** debe pasar al isolate de Java/GraalVM (especialmente si es un certificado no exportable en un DNIe o tarjeta física).
   * Java se utiliza únicamente para el **Pre-proceso** (generar los hashes a firmar) y el **Post-proceso** (ensamblar la firma con el PDF/XML final).
   * La **Firma (Sign)** del hash se ejecuta nativamente en el backend de Rust usando el módulo PKCS#11 / CNG / Keychain del sistema operativo.

---

## 🛠️ Herramientas y Estado de Configuración del Entorno
* **GraalVM JDK:** Instalado en el sistema (versión Java 21). La herramienta `native-image` está disponible en el PATH.
* **Maven:** Instalado y configurado en el PATH.
* **Cargo (Rust):** Instalado y configurado.
* **Prueba de Concepto FFI:** Se encuentra una PoC funcional del enlace FFI en `clienteafirma/autofirma-native-bridge/rust-poc`. Compila y se ejecuta con éxito.

---

## 📍 Archivos de Interés y Rutas
* **Especificación de Desarrollo:** **[rfirma_development_spec.md](rfirma_development_spec.md)** (contiene la arquitectura detallada y planos de código para los módulos de Rust, Java y Svelte).
* **Bridge Java:** `rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/NativeBridge.java`
* **App Rust/Tauri:** `rfirma-app/src-tauri/`
* **App Frontend:** `rfirma-app/src/`



---

## Agent skills

### Issue tracker

Los issues viven en GitHub Issues de `sgomez/rfirma` (CLI `gh`). Ver `docs/agents/issue-tracker.md`.

### Triage labels

Vocabulario canónico por defecto: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. Ver `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` en la raíz. Ver `docs/agents/domain.md`.
