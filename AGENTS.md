# Instrucciones para agentes: rfirma

## Objetivo

Sustituir la interfaz Swing y el servidor de sockets en Java de **AutoFirma** ([ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma)) por una aplicación nativa en **Tauri v2 (Rust + React)**. La criptografía pesada (CAdES, PAdES, XAdES, FacturaE) la hace una biblioteca compartida compilada con **GraalVM Native Image** desde el código de AutoFirma.

## Restricciones de diseño

Las decisiones están en `docs/adr/`; cada zona tiene su mapa con sus trampas. Las que hay que conocer antes de tocar nada:

1. **Firma trifásica (ADR-0001):** la clave privada **nunca** pasa al aislado de Java. Java hace el preproceso y el postproceso; la firma del hash es de Rust, con PKCS#11 / CNG / Keychain.
2. **Dependencias Java desde `~/.m2` (ADR-0002):** no se copian ni se enlazan los módulos de AutoFirma. Cómo se construyen, en `rfirma-native-bridge/AGENTS.md`.
3. **Memoria en la frontera FFI (ADR-0003):** las cadenas que devuelve Java se reservan con `UnmanagedMemory.malloc` y las libera Rust con `autofirma_free_string`.
4. **Un solo `.so` instalado por el paquete (ADR-0004):** ni `include_bytes!`, ni extracción a `~/.cache/`, ni auxiliares de AWT «por si acaso». La carga en desarrollo y `RFIRMA_LIB_DIR`, en `rfirma-app/src-tauri/src/AGENTS.md`; el manual del flatpak, en `packaging/flatpak/README.md`.
5. **Idioma:** castellano para la prosa —documentación, ADR, comentarios, commits, issues y PR—, sin traducir términos como branch, sandbox, tag o HEAD. Inglés para todo identificador —variables, funciones, tipos, módulos, ficheros, ramas— y para los nombres de los tests (`fn signs_pdf_without_rubric()`). Los textos que ve la persona usuaria, en castellano; sus claves de i18n, en inglés.
6. **Comentarios: el defecto es ninguno** (capítulo «Comments» de *Clean Code*): el nombre es el comentario, y un bloque que necesita explicación se extrae a una función con ese nombre. Formas máximas: `//!` de módulo, una frase; `///` de elemento público, una línea; `//` en un cuerpo, solo si dice lo que el código no puede. La única excepción es el enlace a un ADR por número (`ADR-0005`), en una línea, donde el código parece un error y no lo es. El destino de lo demás, en `rfirma-app/src-tauri/src/AGENTS.md`.
7. **No escribas lo que se pudre.** Ni en un documento ni en un comentario: contadores («66 historias»), enumeraciones («los primitivos son A, B, C») ni listas que ya existen en el código. Apunta a la fuente en vez de copiarla. Y un cambio no toca documentación por un detalle: una PR cambia lo que se pidió, y solo edita README, ADR, `AGENTS.md` o cualquier otro documento si el cambio los contradice de verdad o si la persona lo pide.

## Entorno

* **Un comando por llamada, sin prefijos:** `just <receta>` a secas, sin `PATH=…`, `CARGO_TARGET_DIR=…`, `cd … &&` ni `; echo $?`. Las recetas ya ponen `~/.cargo/bin` en el `PATH` y el árbol de compilación.
* **GraalVM JDK 25** (`GRAALVM_HOME`, ADR-0004); la trampa del 21 de SDKMAN, en `rfirma-native-bridge/AGENTS.md`.
* **Token PKCS#11 de pruebas:** `softhsm2`, tokens `rfirma-test` (RSA) y `rfirma-test-ecc` (curva elíptica), PIN `1234`, módulo `/usr/lib/softhsm/libsofthsm2.so`, con certificados de pruebas de la FNMT; el kit, en `~/.local/share/rfirma-test-certs` (`docs/research/token-pkcs11-pruebas.md`). **El certificado personal del titular no se usa en ningún punto del proyecto.**
* Un `pkg-config exited with status code 1` en `javascriptcore-rs-sys` es una biblioteca de sistema de Tauri que falta: la lista, en el paso «Dependencias de sistema de Tauri» de `.github/workflows/ci.yml`.
* Desde un worktree, las recetas compilan en `.claude/worktrees/target`, compartido (ADR-0014).

## Qué ejecutar y cuándo

`just check` es la puerta del CI, que la reparte en tres runners (ADR-0014). En local se sube esta escalera, y no tiene más peldaños:

| Cuándo | Qué |
| --- | --- |
| En cada rojo → verde | Solo la prueba que tocas: `just test-one-rust <filtro>`, `just test-one-ts <fichero>` |
| Antes de commitear | `just fmt`. El commit corre con lefthook `just structural-guards` y, si tocas `rfirma-app/src/`, tipos, i18n y knip. El pre-push los repite y añade formato y biome, y `just check-rust` si tocas `rfirma-app/src-tauri/` |
| Al abrir la PR | Push: el veredicto de `just check` es del CI |
| Si el CI sale en rojo | Vuelve al primer peldaño con lo que falló. `IO failure on output stream` o `No space left on device` en local es el disco: `just clean-coverage` |
| Al revisar una PR | Nada, si el CI está verde para ese head sha (`docs/agents/code-host.md`) |
| `just check` entero en local | Nunca, ni al tocar el `justfile` o `.github/` |

`just --list` agrupa las recetas: `checklist` es esta tabla; `ci`, lo que llaman los workflows; `dev` y `release`, lo que se usa a mano.

* **Itera con `just test-one-rust <filtro>`.** `just check-rust`, `just coverage` y `just crap` compilan un árbol instrumentado aparte; la primera ya la paga el pre-push.
* **Un `just test-one-rust` suelto necesita `rfirma-app/dist` y el token.** Desde un árbol limpio: `pnpm install` → `just po-import` → `just build-ts` → `just certs install`.
* **Filtra la salida:** el reportero más callado de cada cadena; en rojo, vuelve a correr solo el fichero o el nombre que falló.
* **Si tocas `Cargo.lock`, corre `just flatpak-sources`**, que reescribe el sello de `packaging/flatpak/sources.lock`; sin él, la «Cadena TypeScript» del CI sale en rojo. Si la receta no corre, el sello es el `sha256sum` de `Cargo.lock`.

## Rutas

* **Especificación:** el [issue #46](https://github.com/sgomez/rfirma/issues/46); cada sub-issue copia sus decisiones (`ID-NN`, `TD-NN`) en su `## Spec extract`.
* **Puente Java:** `rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/NativeBridge.java`
* **App Rust/Tauri:** `rfirma-app/src-tauri/` · **Frontend:** `rfirma-app/src/` (React 19 + Vite + TypeScript, pnpm)
* **Empaquetado:** `packaging/flatpak/`, `packaging/windows/` (ADR-0035, ADR-0040), `packaging/macos/`
* **Punto de entrada de todo:** `justfile` (ADR-0013).

### Mapas: lee el índice antes que el código

Antes de abrir código, en este orden:

1. Lee entero el mapa de tu zona (lista de abajo) y, en el backend, el de su contexto.
2. Pide el índice de la carpeta que vas a tocar, sin `| head` ni `2>/dev/null` (unas 40 líneas): `just outline rfirma-app/src-tauri/src/signing/` en el backend (también `site`, `documents`, `identity`, `desktop`, `crossing`; `signing/domain/` si ya sabes la capa) o `just outline rfirma-app/src/signing/` en la interfaz.
3. Abre el módulo cuya línea es el tuyo. Si el issue ya nombra un símbolo, `grep -rln '<símbolo>'` lo localiza; sus vecinos —vistas, tests, contrato— salen del índice de su carpeta, no de más `grep`.

* `rfirma-app/src-tauri/src/AGENTS.md` — backend Rust.
* `rfirma-app/src/AGENTS.md` — interfaz.
* `rfirma-native-bridge/AGENTS.md` — puente Java.
* `scripts/AGENTS.md` — los arneses del `justfile`.
* `rfirma-conformance/AGENTS.md` — la suite de conformidad.
* `docs/AGENTS.md` — ADR, research, fichas de diseño y contratos de proceso.

### Presupuesto de exploración

* **De un fichero, `just outline <fichero>...`**: el esqueleto con números de línea (`.rs`, `.ts`, `.tsx`), de varios a la vez.
* **Abre los tramos de todos los ficheros en una llamada**: `just outline a.rs:10-40,80-120 b.tsx:5-30`.
* **El fichero entero, solo si `just outline` marca menos de 300 líneas** y vas a tocarlo entero.
* **De los tests, los nombres:** `grep -n 'fn \|it('`. Se abren para tocarlos.
* **Los documentos de `docs/` se abren por `grep`**; los de `research/`, solo si vas a cambiar la decisión que sostienen.
* Si lees entero un fichero que el índice no anunciaba, **arregla el índice** en la misma PR, en `## Discoveries`.

## Agent skills

* **Issue tracker:** GitHub Issues de `sgomez/rfirma`, con `gh` (`docs/agents/issue-tracker.md`).
* **Triage labels:** `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix` (`docs/agents/triage-labels.md`).
* **Domain docs:** `CONTEXT-MAP.md` apunta a los glosarios; los ADR, en `docs/adr/` y `rfirma-conformance/docs/adr/`, con la misma numeración. Al escribir un ADR, `docs/agents/domain.md`.
* **Prototyping:** se explora en Claude Design con los componentes reales (proyecto «rFirma Components», sincronizado con `/design-sync`) y, validado, se escribe como código, historias y una ficha `docs/design/<pantalla>.md` (`docs/agents/prototyping.md`).
