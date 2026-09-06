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
   * El repositorio original se compila **en el tag `v1.9.2`** y con
     `mvn clean install -Dclienteafirma.version=1.9.2` en su propia ubicación. **La propiedad no es
     opcional**: el `pom.xml` de ese tag declara `<clienteafirma.version>1.9</clienteafirma.version>`
     con el proyecto en `1.9.2`, así que sin ella los 42 POM de módulo se bajan `afirma-core:1.9` de
     Maven Central en vez de usar el módulo del reactor —`afirma-crypto-cades` no compila y, peor,
     lo que sí compila lo hace contra la 1.9 **en silencio**—. Lo automatiza `bootstrap.sh`; medido
     en el [#330](https://github.com/sgomez/rfirma/issues/330).
   * **La 1.9.2 no está en Maven Central** (la 1.9 y la 1.9.1 sí). No hay red: si no está en
     `~/.m2`, hay que construirla.
   
2. **Gestión de Memoria en la Frontera FFI (Evitar Double-Free):**
   * GraalVM Native Image libera automáticamente la memoria de los strings C creados con `CTypeConversion.toCString(...)` al salir del bloque de recurso (`try-with-resources`).
   * Para retornar cadenas JSON desde Java a Rust de forma segura, se debe usar la asignación manual en el C-heap mediante `UnmanagedMemory.malloc(bytes.length + 1)`.
   * Rust es responsable de llamar al método FFI `autofirma_free_string(thread, ptr)` una vez leído el JSON para evitar fugas de memoria.

3. **Distribución de la librería nativa (ADR-0004):**
   * Los canales son **tres** desde el hito v0.4 —flatpak, `.deb` y `.rpm`—, y la librería vive en `/app/lib/rfirma/` en el primero y en `/usr/lib/rfirma/` en los otros dos. Es la **misma** ruta relativa al ejecutable, así que no hay ni una línea condicional: no añadas rutas a `ffi.rs`.
   * Es **un solo fichero**, `librfirma_crypto.so` (27,7 MB), y cubre los cuatro casos: sin rúbrica, rúbrica de texto y rúbrica de imagen. Los cinco auxiliares de AWT **dejan de instalarse** al copiar solo ese fichero: `native-image` los sigue produciendo en `target/native/`, así que verlos ahí es lo normal y no un fallo. Lo que consigue excluir `afirma-ui-utils` del `pom.xml` (ADR-0012) es otra cosa —que `javax.imageio` acabe con cero métodos alcanzables dentro de la imagen—; si ves esa exclusión, **no la quites**. Por eso una firma visible con rúbrica emite siempre un `WARNING` de `ClassNotFoundException: es.gob.afirma.ui.utils.ImageUtils` en el registro de las pruebas: es la exclusión haciendo su trabajo, no un fallo que arreglar.
   * **No instales nunca los auxiliares «por si acaso».** Si `libawt.so` está en el directorio, un JPEG con perfil ICC **aborta el proceso** en vez de dar un error recuperable, y se lleva la aplicación entera. Medido en `docs/research/exclusion-afirma-ui-utils.md`.
   * **No uses `include_bytes!` ni extraigas nada a `~/.cache/rfirma/`.** El fichero lo instala el paquete. El manifiesto del flatpak y su verificación viven en `packaging/flatpak/`; el `.deb` y el `.rpm` los produce el *bundler* de Tauri (ADR-0013), y la invariante «un solo `.so`, `libawt.so` en ninguna parte» la comprueba `packaging/verifica-contenido.sh` sobre cualquiera de los tres.
   * Rust lo carga por una ruta **relativa al ejecutable** (`../lib/rfirma`) con `libloading`. No hace falta `LD_LIBRARY_PATH` ni tocar `RPATH`. La ruta es sobreescribible con `RFIRMA_LIB_DIR`, y eso es lo que te ahorra reconstruir la imagen nativa desde un worktree: para correr la grada C, `RFIRMA_LIB_DIR=<checkout principal>/rfirma-native-bridge/target/lib/rfirma` reutiliza el `.so` ya compilado allí, unos tres minutos menos que `just native`.
   * El puente **exige un JPEG ya normalizado y sin perfil ICC**: la normalización de la rúbrica es de Rust (ADR-0012). Un PNG que llegue hasta aquí falla con «no está codificada en JPEG», y eso es lo correcto.
   * El sandbox —**del flatpak, no de rfirma**— cambia una cosa que fuera es gratis: **toda entrada y salida de ficheros pasa por portales**, así que ahí la aplicación no conoce la ruta original de un documento. En el `.deb` y el `.rpm` eso se enciende solo. Ver `docs/research/flatpak-canal-unico.md`. La fontanería de tarjeta (el cliente PC/SC y el módulo PKCS#11 de OpenSC) se retiró del flatpak en el #256: nunca se había publicado, y tarjetas y DNIe no están soportados en la v0.4.
   * **`just flatpak-sources` no corre en este entorno**: `flatpak-cargo-generator.py` no está versionado y `pip install aiohttp` no resuelve contra PyPI. Cuando `Cargo.lock` cambia por una dependencia nueva, `packaging/flatpak/cargo-sources.json` se reproduce a mano: dos entradas por crate de `registry+…crates.io-index`, ordenadas por nombre y por **versión semver** (`0.9.6` antes que `0.10.2`, no orden lexicográfico). El `sha256` de cada crate **no se calcula**: ya está en `Cargo.lock`, en el campo `checksum` de ese paquete, y es el mismo valor que va en la entrada `archive` y dentro de la `inline`. Lo único que sí se sella con `sha256sum` es `sources.lock`, que guarda el hash de los dos ficheros de bloqueo. Si el cambio es solo **hacer directa una dependencia que ya estaba en el árbol transitivo**, `Cargo.lock` cambia en una sola línea y `cargo-sources.json` **no** se toca — basta con regenerar el sha de `sources.lock` (`sha256sum` de los dos ficheros de bloqueo), sin la regeneración manual completa.
   * La guardia del ADR-0011 (`the_portal_path_never_crosses_to_the_window`, en `commands/guards.rs`) construye cada tipo de salida desde su caso de uso con un enlace del portal y recorre el JSON resultante campo a campo. Descubre sola cada tipo que derive `Serialize` en `commands/`, así que uno nuevo **no pasa desapercibido**, pero sí hay que decidir a mano dónde entra: o se construye en `crossings_from_a_portal_document`, o se declara en `OUTPUTS_WITH_NO_DOCUMENT_BEHIND` (solo si detrás no puede haber ningún documento). No hay lista de ficheros: las guardas descubren todo `.rs` de `commands/` y de cualquier `<contexto>/adapters/`; hay guardas hermanas que se ponen rojas si se olvida cualquiera de las dos cosas.
   * Un comando de Tauri (`#[tauri::command]`) sobre una `fn` **no** `async` corre en `ExecutionContext::Blocking`, en el hilo del bucle de eventos: si dentro llama a un `blocking_*` de un plugin (p. ej. el de diálogo), se cuelga ahí para siempre y sin error visible. La forma correcta es `#[tauri::command(async)]`; conviene fijarla con una prueba porque ninguna otra guardia la vigila.
   * Para medir el comportamiento del sandbox **sin GUI**, `flatpak run --command=python3 me.sgomez.rfirma -` mete un script por la entrada estándar dentro del bundle ya instalado, con sus permisos reales. `org.gnome.Platform` trae `python3` con PyGObject y `gdbus`, pero **no** `strings` ni `busctl`.

4. **Firma Trifásica (Triphase Signing):**
   * La clave privada **nunca** debe pasar al isolate de Java/GraalVM (especialmente si es un certificado no exportable en un DNIe o tarjeta física).
   * Java se utiliza únicamente para el **Pre-proceso** (generar los hashes a firmar) y el **Post-proceso** (ensamblar la firma con el PDF/XML final).
   * La **Firma (Sign)** del hash se ejecuta nativamente en el backend de Rust usando el módulo PKCS#11 / CNG / Keychain del sistema operativo.

5. **Idioma: castellano para la prosa, inglés para el código:**
   * En **castellano**: documentación (`README`, `CONTEXT.md`, `docs/`, ADR), comentarios del código, mensajes de commit, descripciones de issues y de PR.
   * En **inglés**: todo el identificador — nombres de variables, funciones, tipos, módulos, ficheros y ramas — y también los nombres de los tests (`fn signs_pdf_without_rubric()`, `it('rejects a PNG rubric')`).
   * Los textos que ve la persona usuaria (etiquetas de la UI, mensajes de error mostrados) van en castellano; las claves de i18n que los identifican, en inglés.

6. **Comentarios: el defecto es ninguno.**
   * Se aplica el capítulo «Comments» de *Clean Code* sin excepciones locales: el nombre es el comentario, y un bloque que necesita explicación se extrae a una función con ese nombre.
   * Formas máximas: la cabecera `//!` de un módulo es **una frase** —qué es y qué no es—; el `///` de un elemento público es **una línea**; dentro de un cuerpo, un `//` solo si dice algo que el código no puede decir.
   * Cada cosa tiene su destino, y no es el comentario: el porqué de una decisión va a un ADR; una advertencia a agentes de alcance general va al mapa del backend (`rfirma-app/src-tauri/src/AGENTS.md`); lo que ya dice la tabla del mapa no se repite; un conteo, un número de PR o la interfaz del otro lado no van a ninguna parte.
   * **El enlace a un ADR es la excepción**: solo donde el código hace algo que parece un error y no lo es, una línea, citando el ADR por número (`ADR-0005`), nunca por ruta ni por epígrafe. El número es estable porque un ADR se reescribe en su sitio y nunca se supera.
   * Una cita a un identificador de especificación (`ID-NN`, `TD-NN`, `RD-NN`, `RT-NN`) que ya exista se tolera como cita corta mientras la poda no pase por su zona; al podar, o pasa a citar el ADR que recoja la decisión, o se borra. **No se escriben citas nuevas** a identificadores de spec en código. Una guarda de grada A (`tests/adr_citations_resolve.rs`) vigila que cada `ADR-NNNN` citado tenga fichero en `docs/adr/`, y nada más.
   * Aplica al código nuevo **y al movido**: mover un fichero es la ocasión de podarlo, no de trasladar su prosa.
   * En revisión, un comentario nuevo de más de dos líneas pide justificación en el PR.

---

## 🛠️ Herramientas y Estado de Configuración del Entorno
* **GraalVM JDK: `25.3.4+1.r25-graalce`**, decidido en el ADR-0004. Hay dos instalados por SDKMAN y `21-graalce` sigue siendo el de por defecto de SDKMAN, así que **fija `GRAALVM_HOME` a la ruta de la 25** para construir. La línea 21 aborta dentro del `JNI_OnLoad` de `libawt.so` con cualquier firma visible. El `pom.xml` sigue compilando a `release 21`: cambia el JDK que construye, no el lenguaje de destino. Ver `docs/research/graalvm-libawt-shared.md`.
* **Maven:** Instalado y configurado en el PATH.
* **Token PKCS#11 de pruebas:** `softhsm2` con el token `rfirma-test` (PIN `1234`), módulo en `/usr/lib/softhsm/libsofthsm2.so`, cargado con un certificado **de pruebas de la FNMT** emitido por su CA de producción. El kit completo vive en `~/.local/share/rfirma-test-certs`. **El certificado personal del titular no se usa en ningún punto del proyecto.** Ver `docs/research/token-pkcs11-pruebas.md`.
* **Cargo (Rust):** Instalado en `~/.cargo/bin`, pero **no está en el `PATH` de una shell no interactiva** de este entorno. `command -v cargo` falla y `just tools` lo denuncia. Exporta el `PATH` antes de cualquier receta de Rust.
* **Cadena de Tauri (`rfirma-app/src-tauri/`):** **ya compila en este equipo de desarrollo** (confirmado de forma independiente en #49 y #50) — `pkg-config --exists` encuentra `webkit2gtk-4.1`, `javascriptcoregtk-4.1` y `libsoup-3.0`, y `cargo build`, `cargo test`, `cargo clippy`, `cargo llvm-cov` y `cargo crap` corren enteros. Dos peros siguen en pie: exporta el `PATH` de Cargo primero (ver la entrada de arriba), y `cargo` necesita `rfirma-app/dist` ya construido antes de compilar nada —**sin excepción, tampoco `cargo test --lib` ni `cargo clippy --lib`**, porque `lib.rs` llama a `tauri::generate_context!()` y revienta con «The `frontendDist` configuration is set to `"../dist"` but this path doesn't exist»—. Y en un árbol limpio `pnpm install` + `vite build` **no basta**: `src/i18n/locales/*.ts` se genera desde `po/*.po` y no está versionado, así que el arranque es `pnpm install` → `just po-import` → recién entonces cualquier receta de Rust. Si en algún equipo vuelve a faltar una de esas bibliotecas de sistema, el fallo aparece como `pkg-config exited with status code 1` dentro del `build.rs` de `javascriptcore-rs-sys` sin nombrar el paquete que falta; la lista completa vive en el paso «Dependencias de sistema de Tauri» de `.github/workflows/ci.yml`.
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

### 🗺️ Mapas: lee el índice antes que el código

Hay un índice por zona, y **cada uno da el tamaño de cada fichero antes de que
lo abras**. Son la primera lectura de cualquier trabajo, y en la mayoría de los
casos la única que hace falta además del fichero que vas a tocar:

* `rfirma-app/src-tauri/src/AGENTS.md` — mapa del backend Rust.
* `rfirma-app/src/AGENTS.md` — mapa de la interfaz.
* `docs/AGENTS.md` — índice de ADR, research, fichas de diseño y contratos de proceso.

**Presupuesto de exploración.** Explorar es lo que agota el contexto, no
escribir código: en una sesión medida, leer ficheros se llevó el 58 % del
contexto y escribir el parche el 3 %. Y **lo que cuesta una lectura no es su
tamaño, sino su tamaño multiplicado por las peticiones que vienen después**: lo
leído se queda en el contexto y se reenvía en cada turno. Un `cat` de 4k tokens
en la llamada 15 de 120 se paga ciento y pico veces. Por eso:

* **Para situarte en un fichero, `just outline <ruta>`, nunca `cat`.** Imprime
  el esqueleto —cada elemento público y cada prueba, con su número de línea y la
  primera línea de su documentación— y desde ahí abres los tramos que te hagan
  falta. Funciona con `.rs`, `.ts` y `.tsx`; para lo demás,
  `grep -n '<símbolo>'`. El pie de la salida dice lo que habría costado leerlo
  entero, medido en ese momento.
* **Los tramos se abren todos de golpe, `sed -n 'A,Bp;C,Dp'`, no de uno en
  uno.** La unidad de coste es el turno, no el carácter. Medido sobre las
  ejecuciones de un día: el esqueleto bajó un tercio lo que crece el contexto
  por turno, y aun así los tickets salieron más caros, porque el agente pasó de
  diez lecturas sueltas a veinticinco y cada ida y vuelta se arrastra todo el
  contexto anterior. Si `outline` te ha dicho dónde están las cinco cosas que
  buscas, pídelas en una llamada.
* **`cat` solo de ficheros que el pie de `just outline` marque por debajo de 300
  líneas**, y solo si vas a tocarlos enteros.
* **Los tests no se leen para entender el código**, solo para tocarlos. Sus
  nombres son frases y se listan con un `grep -n 'fn \|it('` que cuesta cien
  veces menos.
* **Un documento de `docs/` se abre por `grep`, no por `cat`.** Los de
  `research/` llegan a 32 KB y solo se consultan si vas a cambiar la decisión
  que sostienen.
* Si acabas leyendo entero un fichero que el índice no anunciaba, **el índice
  está mal**: arréglalo en la misma PR, en `## Discoveries`.



---

## Agent skills

### Issue tracker

Los issues viven en GitHub Issues de `sgomez/rfirma` (CLI `gh`). Ver `docs/agents/issue-tracker.md`.

### Triage labels

Vocabulario canónico por defecto: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. Ver `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` en la raíz. Ver `docs/agents/domain.md`.

**Un ADR es la ley vigente, escrita una sola vez, no un acta con fecha.** Cuando una decisión cambia, **se reescribe o se enmienda el ADR que dejó de ser cierto** —ajustando su nombre de fichero si el título deja de describirla—; **nunca** se añade uno nuevo que lo contradiga ni se marca el viejo como `Superseded`. Quien lee no es una persona, es un modelo, y dos ficheros que se contradicen sobre lo mismo gastan contexto y siembran dudas. El histórico ya lo guardan GitHub, los issues y las PR. Lo que sí se conserva dentro del ADR reescrito es **por qué se descartó la opción anterior**, en `Considered Options`.

### Prototyping

Un canvas de Claude Design por caso de uso ([proyecto `c0ddbfa7`](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132)) para prototipar; al validarlo, una ficha `docs/design/<pantalla>.md` por pantalla. Ver `docs/agents/prototyping.md`.
