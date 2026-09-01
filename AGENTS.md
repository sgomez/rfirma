# Instrucciones para Agentes de IA: rfirma

Este archivo contiene el contexto técnico esencial, las restricciones de diseño y el estado actual del proyecto **rfirma** para guiar a los agentes autónomos de codificación que continúen con la implementación de esta aplicación.

---

## 🎯 Objetivo del Proyecto
Reemplazar la interfaz Swing y el servidor sockets en Java de **AutoFirma** (cuyo repositorio oficial es [ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma)) por una aplicación nativa en **Tauri v2 (Rust + React)**. La lógica criptográfica pesada (CAdES, PAdES, XAdES, FacturaE) se delega a una biblioteca compartida compilada con **GraalVM Native Image** a partir de la base de código original de Autofirma.

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

3. **Distribución de la librería nativa (ADR-0004):**
   * Es **un solo fichero**, `librfirma_crypto.so` (27,7 MB), y cubre los cuatro casos: sin rúbrica, rúbrica de texto y rúbrica de imagen. Los cinco auxiliares de AWT desaparecieron al excluir `afirma-ui-utils` del `pom.xml` (ADR-0012); si ves esa exclusión, **no la quites**. Por eso una firma visible con rúbrica emite siempre un `WARNING` de `ClassNotFoundException: es.gob.afirma.ui.utils.ImageUtils` en el registro de las pruebas: es la exclusión haciendo su trabajo, no un fallo que arreglar.
   * **No instales nunca los auxiliares «por si acaso».** Si `libawt.so` está en el directorio, un JPEG con perfil ICC **aborta el proceso** en vez de dar un error recuperable, y se lleva la aplicación entera. Medido en `docs/research/exclusion-afirma-ui-utils.md`.
   * **No uses `include_bytes!` ni extraigas nada a `~/.cache/rfirma/`.** El **flatpak es el único canal soportado**: el fichero se instala en `/app/lib/rfirma/`. El manifiesto y su verificación viven en `packaging/flatpak/`.
   * Rust lo carga por una ruta **relativa al ejecutable** (`../lib/rfirma`) con `libloading`. No hace falta `LD_LIBRARY_PATH` ni tocar `RPATH`. La ruta es sobreescribible con `RFIRMA_LIB_DIR`.
   * El puente **exige un JPEG ya normalizado y sin perfil ICC**: la normalización de la rúbrica es de Rust (ADR-0012). Un PNG que llegue hasta aquí falla con «no está codificada en JPEG», y eso es lo correcto.
   * El arenero cambia dos cosas que fuera eran gratis: el **módulo PKCS#11 lo empaqueta el propio flatpak** (los del anfitrión no cargan dentro), y **toda entrada y salida de ficheros pasa por portales**, así que la aplicación nunca conoce la ruta original de un documento. Ver `docs/research/flatpak-canal-unico.md`.

4. **Firma Trifásica (Triphase Signing):**
   * La clave privada **nunca** debe pasar al isolate de Java/GraalVM (especialmente si es un certificado no exportable en un DNIe o tarjeta física).
   * Java se utiliza únicamente para el **Pre-proceso** (generar los hashes a firmar) y el **Post-proceso** (ensamblar la firma con el PDF/XML final).
   * La **Firma (Sign)** del hash se ejecuta nativamente en el backend de Rust usando el módulo PKCS#11 / CNG / Keychain del sistema operativo.

5. **Idioma: castellano para la prosa, inglés para el código:**
   * En **castellano**: documentación (`README`, `CONTEXT.md`, `docs/`, ADR), comentarios del código, mensajes de commit, descripciones de issues y de PR.
   * En **inglés**: todo el identificador — nombres de variables, funciones, tipos, módulos, ficheros y ramas — y también los nombres de los tests (`fn signs_pdf_without_rubric()`, `it('rejects a PNG rubric')`).
   * Los textos que ve la persona usuaria (etiquetas de la UI, mensajes de error mostrados) van en castellano; las claves de i18n que los identifican, en inglés.

---

## 🛠️ Herramientas y Estado de Configuración del Entorno
* **GraalVM JDK: `25.3.4+1.r25-graalce`**, decidido en el ADR-0004. Hay dos instalados por SDKMAN y `21-graalce` sigue siendo el de por defecto de SDKMAN, así que **fija `GRAALVM_HOME` a la ruta de la 25** para construir. La línea 21 aborta dentro del `JNI_OnLoad` de `libawt.so` con cualquier firma visible. El `pom.xml` sigue compilando a `release 21`: cambia el JDK que construye, no el lenguaje de destino. Ver `docs/research/graalvm-libawt-shared.md`.
* **Maven:** Instalado y configurado en el PATH.
* **Token PKCS#11 de pruebas:** `softhsm2` con el token `rfirma-test` (PIN `1234`), módulo en `/usr/lib/softhsm/libsofthsm2.so`, cargado con un certificado **de pruebas de la FNMT** emitido por su CA de producción. El kit completo vive en `~/.local/share/rfirma-test-certs`. **El certificado personal del titular no se usa en ningún punto del proyecto.** Ver `docs/research/token-pkcs11-pruebas.md`.
* **Cargo (Rust):** Instalado en `~/.cargo/bin`, pero **no está en el `PATH` de una shell no interactiva** de este entorno. `command -v cargo` falla y `just tools` lo denuncia. Exporta el `PATH` antes de cualquier receta de Rust.
* **Cadena de Tauri (`rfirma-app/src-tauri/`):** **ya compila en este equipo de desarrollo** (confirmado de forma independiente en #49 y #50) — `pkg-config --exists` encuentra `webkit2gtk-4.1`, `javascriptcoregtk-4.1` y `libsoup-3.0`, y `cargo build`, `cargo test`, `cargo clippy`, `cargo llvm-cov` y `cargo crap` corren enteros. Dos peros siguen en pie: exporta el `PATH` de Cargo primero (ver la entrada de arriba), y `cargo` necesita `rfirma-app/dist` ya construido (`pnpm install` + `vite build` dentro de `rfirma-app/`) antes de compilar nada, porque `tauri-build` lee `frontendDist` dentro de su propio `build.rs`. Si en algún equipo vuelve a faltar una de esas bibliotecas de sistema, el fallo aparece como `pkg-config exited with status code 1` dentro del `build.rs` de `javascriptcore-rs-sys` sin nombrar el paquete que falta; la lista completa vive en el paso «Dependencias de sistema de Tauri» de `.github/workflows/ci.yml`.
* **`cargo-crap` (puerta CRAP, ADR-0014):** `--fail-above` es un interruptor sin valor; el umbral va aparte, en `--threshold 30`. El ADR-0014 los escribe juntos (`--fail-above` con «umbral 30» al lado) y confunde — la invocación real está en el `justfile`, no copies la del ADR literalmente.
* **Prueba de Concepto FFI:** Se encuentra una PoC funcional del enlace FFI en `clienteafirma/autofirma-native-bridge/rust-poc`. Compila y se ejecuta con éxito.

---

## 📍 Archivos de Interés y Rutas
* **Especificación de Desarrollo:** el [issue #46](https://github.com/sgomez/rfirma/issues/46) y sus dieciséis sub-issues. Es la fuente de verdad de lo que hay que construir: sus *Implementation Decisions* (`ID-01`…`ID-42`) y *Testing Decisions* (`TD-01`…`TD-09`) las copia cada sub-issue en su `## Spec extract`. El borrador `rfirma_development_spec.md` que vivía en la raíz **ya no existe**: tenía errores comprobados y se borró al publicar el #46.
* **Bridge Java:** `rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/NativeBridge.java`
* **App Rust/Tauri:** `rfirma-app/src-tauri/`
* **App Frontend:** `rfirma-app/src/` (React 19 + Vite + TypeScript, pnpm)
* **Empaquetado:** `packaging/flatpak/`
* **Punto de entrada de todo:** `justfile` — ver el [ADR-0013](docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md).



---

## Agent skills

### Issue tracker

Los issues viven en GitHub Issues de `sgomez/rfirma` (CLI `gh`). Ver `docs/agents/issue-tracker.md`.

### Triage labels

Vocabulario canónico por defecto: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. Ver `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` en la raíz. Ver `docs/agents/domain.md`.

### Prototyping

Un canvas de Claude Design por caso de uso ([proyecto `c0ddbfa7`](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132)) para prototipar; al validarlo, una ficha `docs/design/<pantalla>.md` por pantalla. Ver `docs/agents/prototyping.md`.
