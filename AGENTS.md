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

## Entorno

* **Cargo** está en `~/.cargo/bin`, fuera del `PATH` de una shell no interactiva: exporta el `PATH` antes de cualquier receta de Rust (`just tools` lo comprueba).
* **GraalVM JDK 25** (`GRAALVM_HOME`, ADR-0004); la trampa del 21 de SDKMAN, en `rfirma-native-bridge/AGENTS.md`.
* **Token PKCS#11 de pruebas:** `softhsm2`, tokens `rfirma-test` (RSA) y `rfirma-test-ecc` (curva elíptica), PIN `1234`, módulo `/usr/lib/softhsm/libsofthsm2.so`, con certificados de pruebas de la FNMT; el kit, en `~/.local/share/rfirma-test-certs` (`docs/research/token-pkcs11-pruebas.md`). **El certificado personal del titular no se usa en ningún punto del proyecto.**
* Un `pkg-config exited with status code 1` en `javascriptcore-rs-sys` es una biblioteca de sistema de Tauri que falta: la lista, en el paso «Dependencias de sistema de Tauri» de `.github/workflows/ci.yml`.
* Desde un worktree, `CARGO_TARGET_DIR` es `.claude/worktrees/target`, compartido (ADR-0014).

## Qué ejecutar y cuándo

`just check` es la puerta del CI, que la reparte en tres runners (ADR-0014). En local se sube esta escalera, y no tiene más peldaños:

| Cuándo | Qué |
| --- | --- |
| En cada rojo → verde | Solo la prueba que tocas: `cargo test <filtro>`, `pnpm exec vitest run <fichero> --reporter=dot` |
| Antes de commitear | `just fmt`. El resto lo corre lefthook en el pre-push: formato, biome y `just structural-guards`; tipos, i18n y knip si tocas `rfirma-app/src/`; `just check-rust` si tocas `rfirma-app/src-tauri/` |
| Al abrir la PR | Push: el veredicto de `just check` es del CI |
| Si el CI sale en rojo | Vuelve al primer peldaño con lo que falló. `IO failure on output stream` o `No space left on device` en local es el disco: `just clean-coverage` |
| Al revisar una PR | Nada, si el CI está verde para ese head sha (`docs/agents/code-host.md`) |
| `just check` entero en local | Nunca, ni al tocar el `justfile` o `.github/` |

`just --list` agrupa las recetas: `checklist` es esta tabla; `ci`, lo que llaman los workflows; `dev` y `release`, lo que se usa a mano.

* **Itera con `cargo test <filtro>`.** `just check-rust`, `just coverage` y `just crap` compilan un árbol instrumentado aparte; la primera ya la paga el pre-push.
* **Un `cargo test` suelto necesita `rfirma-app/dist` y el token.** Desde un árbol limpio: `pnpm install` → `just po-import` → `just build-ts` → `just certs install`.
* **Filtra la salida:** el reportero más callado de cada cadena; en rojo, vuelve a correr solo el fichero o el nombre que falló.
* **Si tocas `Cargo.lock`, corre `just flatpak-sources`**, que reescribe el sello de `packaging/flatpak/sources.lock`; sin él, la «Cadena TypeScript» del CI sale en rojo. Si la receta no corre, el sello es el `sha256sum` de `Cargo.lock`.

## Rutas

* **Especificación:** el [issue #46](https://github.com/sgomez/rfirma/issues/46); cada sub-issue copia sus decisiones (`ID-NN`, `TD-NN`) en su `## Spec extract`.
* **Puente Java:** `rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/NativeBridge.java`
* **App Rust/Tauri:** `rfirma-app/src-tauri/` · **Frontend:** `rfirma-app/src/` (React 19 + Vite + TypeScript, pnpm)
* **Empaquetado:** `packaging/flatpak/`, `packaging/windows/` (ADR-0035, ADR-0040), `packaging/macos/`
* **Punto de entrada de todo:** `justfile` (ADR-0013).

### Mapas: lee el índice antes que el código

En el backend y en la interfaz cada módulo dice qué es en su primera línea `//!`, y `just outline <directorio>/` junta esas líneas en el índice de una carpeta, para que sepas cuál abrir sin explorar el árbol; su mapa guarda las carpetas y las trampas. Los demás mapas lo dicen en una fila por fichero. Es la primera lectura de cualquier trabajo. Cómo se escribe una cabecera o una fila, en `docs/AGENTS.md`.

* `rfirma-app/src-tauri/src/AGENTS.md` — mapa del backend Rust.
* `rfirma-app/src/AGENTS.md` — mapa de la interfaz.
* `rfirma-native-bridge/AGENTS.md` — mapa del puente Java.
* `scripts/AGENTS.md` — mapa de los arneses que llama el `justfile`.
* `rfirma-conformance/AGENTS.md` — mapa de la suite de conformidad y su consola web.
* `docs/AGENTS.md` — índice de ADR, research, fichas de diseño y contratos de proceso.

### Presupuesto de exploración

Lo leído se queda en el contexto y se reenvía en cada turno: una lectura cuesta su tamaño por los turnos que vienen detrás, y la unidad de coste es el turno.

* **Para situarte, `just outline <ruta>...`**: el esqueleto con números de línea (`.rs`, `.ts`, `.tsx`), de varios ficheros a la vez; para lo demás, `grep -n '<símbolo>'`.
* **Abre los tramos de todos los ficheros en una llamada**: `just outline a.rs:10-40,80-120 b.tsx:5-30`.
* **El fichero entero, solo si `just outline` marca menos de 300 líneas** y vas a tocarlo entero.
* **De los tests, los nombres:** `grep -n 'fn \|it('`. Se abren para tocarlos.
* **Los documentos de `docs/` se abren por `grep`**; los de `research/`, solo si vas a cambiar la decisión que sostienen.
* Si lees entero un fichero que el índice no anunciaba, **arregla el índice** en la misma PR, en `## Discoveries`.

## Agent skills

* **Issue tracker:** GitHub Issues de `sgomez/rfirma`, con `gh` (`docs/agents/issue-tracker.md`).
* **Triage labels:** `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix` (`docs/agents/triage-labels.md`).
* **Domain docs:** `CONTEXT-MAP.md` apunta a los glosarios; los ADR, en `docs/adr/` y `rfirma-conformance/docs/adr/`, con la misma numeración. Al escribir un ADR, `docs/agents/domain.md`.
* **Prototyping:** un canvas de Claude Design por caso de uso ([proyecto `c0ddbfa7`](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132)) y, validado, una ficha `docs/design/<pantalla>.md` (`docs/agents/prototyping.md`).
