# Instrucciones para Agentes de IA: rfirma

Este archivo contiene el contexto técnico esencial, las restricciones de diseño y el estado actual del proyecto **rfirma** para guiar a los agentes autónomos de codificación que continúen con la implementación de esta aplicación.

---

## 🎯 Objetivo del Proyecto
Reemplazar la interfaz Swing y el servidor sockets en Java de **AutoFirma** (cuyo repositorio oficial es [ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma)) por una aplicación nativa en **Tauri v2 (Rust + React)**. La lógica criptográfica pesada (CAdES, PAdES, XAdES, FacturaE) se delega a una biblioteca compartida compilada con **GraalVM Native Image** a partir de la base de código original de Autofirma.

---

## ⚠️ Restricciones Críticas de Diseño (Must-Know)

Las decisiones están en `docs/adr/` y no se repiten aquí; cada zona tiene su
mapa con sus trampas. Las cuatro que hay que conocer antes de tocar nada:

1. **Firma trifásica (ADR-0001):** la clave privada **nunca** pasa al aislado de Java. Java hace el preproceso y el postproceso; la firma del hash es de Rust, con PKCS#11 / CNG / Keychain.
2. **Dependencias Java desde `~/.m2` (ADR-0002):** no se copian ni se enlazan los módulos de AutoFirma. Cómo se construyen, en `rfirma-native-bridge/AGENTS.md`.
3. **Memoria en la frontera FFI (ADR-0003):** las cadenas que devuelve Java se reservan con `UnmanagedMemory.malloc` y las libera Rust con `autofirma_free_string`.
4. **Un solo `.so` instalado por el paquete (ADR-0004):** ni `include_bytes!`, ni extracción a `~/.cache/`, ni auxiliares de AWT «por si acaso». La carga en desarrollo y `RFIRMA_LIB_DIR`, en `rfirma-app/src-tauri/src/AGENTS.md`; el manual del flatpak, en `packaging/flatpak/README.md`.
5. **Idioma: castellano para la prosa, inglés para el código:**
   * En **castellano**: documentación (`README`, `CONTEXT.md`, `docs/`, ADR), comentarios del código, mensajes de commit, descripciones de issues y de PR. Excepciones: no se traducen términos técnicos como branch, sandbox, tag, HEAD, etc. 
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
* **GraalVM JDK 25** (`GRAALVM_HOME`), decidido en el ADR-0004; la trampa del 21 de SDKMAN está en `rfirma-native-bridge/AGENTS.md`.
* **Token PKCS#11 de pruebas:** `softhsm2` con los tokens `rfirma-test` y `rfirma-test-ecc` (PIN `1234` los dos), módulo en `/usr/lib/softhsm/libsofthsm2.so`, cargados con certificados **de pruebas de la FNMT** emitidos por su CA de producción: RSA en el primero, curva elíptica en el segundo. El kit completo vive en `~/.local/share/rfirma-test-certs`. **El certificado personal del titular no se usa en ningún punto del proyecto.** Ver `docs/research/token-pkcs11-pruebas.md`.
* **Cargo (Rust):** Instalado en `~/.cargo/bin`, pero **no está en el `PATH` de una shell no interactiva** de este entorno. `command -v cargo` falla y `just tools` lo denuncia. Exporta el `PATH` antes de cualquier receta de Rust.
* **Cadena de Tauri:** compila en este equipo. Si en otro faltan las bibliotecas de sistema, el fallo aparece como `pkg-config exited with status code 1` dentro del `build.rs` de `javascriptcore-rs-sys` sin nombrar el paquete; la lista completa vive en el paso «Dependencias de sistema de Tauri» de `.github/workflows/ci.yml`.
* **`target/` compartido entre worktrees:** el `justfile` apunta `CARGO_TARGET_DIR` a `.claude/worktrees/target` cuando corre desde un worktree, y deja fuera el checkout principal (ADR-0014).

---

## 🚦 Qué ejecutar y cuándo

La puerta del repositorio es `just check`, y **no es tuya: es del CI**, que la
reparte en tres runners simultáneos y por eso paga el carril más lento. En un
portátil se pagan los tres sumados, y repetirla tras cada arreglo es el gasto
más grande de una ronda de entrega. La escalera es esta y no tiene más
peldaños:

| Cuándo | Qué |
| --- | --- |
| En cada rojo → verde | Solo la prueba que estás tocando: `cargo test <filtro>`, `pnpm exec vitest run <fichero> --reporter=dot` |
| Antes de commitear | `just fmt` y **`just check-changed`**, una vez y no por arreglo: deduce de lo que cambia respecto a `origin/main` qué carriles hacen falta |
| Si `check-changed` sale en rojo | Vuelve al primer peldaño con la prueba o el fichero que falló, arréglalo y repite `check-changed` una sola vez; nunca escales a `just check`. Si el rojo es `IO failure on output stream` o `No space left on device`, es el disco, no LLVM: `just clean-coverage` |
| Al revisar una PR | Nada, si el CI está verde para ese head sha: la suite ya respondió y volver a correrla no añade veredicto (`docs/agents/code-host.md`) |
| `just check` entero | Solo si tocas el `justfile` o `.github/` — y entonces `check-changed` ya dispara las tres cadenas sin que tengas que decidirlo |

Tres avisos que ahorran una ronda:

* **`just check-rust`, `just coverage` y `just crap` no son un
  bucle de realimentación: las tres arrastran el árbol instrumentado.**
  `cargo llvm-cov` compila un árbol instrumentado **aparte** del de `cargo
  test` y de `clippy`, así que iterar con ellas paga dos compilaciones completas
  para responder a lo que `cargo test <filtro>` responde en segundos.
* **Un `cargo test` suelto necesita `rfirma-app/dist` y el token**, que es lo
  que le añaden las recetas: desde un árbol limpio el arranque sigue siendo
  `pnpm install` → `just po-import` → `just build-ts` → `just token`.
* **La salida de una suite verde es contexto tirado.** Filtra por nombre y usa
  el reportero más callado de cada cadena; en rojo, vuelve a correr solo el
  fichero o el nombre que falló, nunca la suite.

---

## 📍 Archivos de Interés y Rutas
* **Especificación de Desarrollo:** el [issue #46](https://github.com/sgomez/rfirma/issues/46) y sus dieciséis sub-issues. Es la fuente de verdad de lo que hay que construir: sus *Implementation Decisions* (`ID-01`…`ID-42`) y *Testing Decisions* (`TD-01`…`TD-09`) las copia cada sub-issue en su `## Spec extract`. El borrador `rfirma_development_spec.md` que vivía en la raíz **ya no existe**: tenía errores comprobados y se borró al publicar el #46.
* **Bridge Java:** `rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/NativeBridge.java`
* **App Rust/Tauri:** `rfirma-app/src-tauri/`
* **App Frontend:** `rfirma-app/src/` (React 19 + Vite + TypeScript, pnpm)
* **Empaquetado:** `packaging/flatpak/`
* **Punto de entrada de todo:** `justfile` — ver el [ADR-0013](docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md).

### 🗺️ Mapas: lee el índice antes que el código

Hay un índice por zona, y **cada uno dice en una frase qué es cada fichero, para
que sepas cuál abrir sin explorar el árbol**. No dan tamaños: un número en un
mapa envejece en el commit siguiente y nadie lo vigila — el tamaño lo da
`just outline` en el momento en que lo necesitas, y ahí es exacto. Son la
primera lectura de cualquier trabajo, y en la mayoría de los casos la única que
hace falta además del fichero que vas a tocar:

* `rfirma-app/src-tauri/src/AGENTS.md` — mapa del backend Rust.
* `rfirma-app/src/AGENTS.md` — mapa de la interfaz.
* `rfirma-native-bridge/AGENTS.md` — mapa del puente Java.
* `docs/AGENTS.md` — índice de ADR, research, fichas de diseño y contratos de proceso.

**Una fila de un mapa dice qué es el fichero, y se para ahí.** Una frase, la que
necesitas para saber si es el que buscas; y cuando ayude a no confundirlo, qué
**no** es. El *cómo* funciona no va nunca: ya lo dice el código, y en el mapa
se desincroniza sin que nadie se entere. El porqué de una decisión va a un ADR,
al que se cita por número. Ni tamaños, ni citas a identificadores de spec, ni
números de PR o de issue. Lo vigila
`rfirma-app/src-tauri/tests/agents_map_is_complete.rs`, que además exige que
todo módulo esté en el mapa de su zona.

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
