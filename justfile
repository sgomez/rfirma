# Punto de entrada del proyecto.
#
# Es la interfaz que encuentra quien llega al repositorio sin contexto: una
# persona nueva, o el agente revisor, que segun docs/agents/code-host.md
# siempre instala y ejecuta las comprobaciones el mismo. `just` lista las
# recetas disponibles.
#
# La rejilla la fija el ADR-0013; QUE se ejecuta dentro de `lint` y `test`, y
# en que carril cae cada cosa, lo fija el ADR-0014.
#
# EL REPOSITORIO ES POLIGLOTA Y LA RAIZ NO PERTENECE A NINGUNA CADENA (ID-01):
#
#   rfirma-native-bridge/   Maven -> GraalVM CE 25 -> librfirma_crypto.so
#   rfirma-app/src-tauri/   Cargo
#   rfirma-app/             pnpm (React 19 + Vite + TypeScript)
#   packaging/flatpak/      manifiesto y verificacion
#
# `just` es el UNICO orquestador. build.rs no invoca a Maven ni a
# native-image jamas: un `cargo build` que dispare por sorpresa 1 m 22 s de
# native-image arruina el bucle de realimentacion que protegio el issue #11.
#
# Requisitos: just, maven, git, pnpm, cargo y un GraalVM CE 25 para `native`.
#   apt-get install -y just maven

# GraalVM CE 25: lo fijo el issue #6. La linea 21 aborta dentro del JNI_OnLoad
# de libawt.so con cualquier firma visible, asi que no sirve para construir.
# El pom sigue compilando a release 21: cambia el JDK que construye, no el
# lenguaje de destino.
graalvm_por_defecto := "$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce"

bridge := justfile_directory() / "rfirma-native-bridge"
app := justfile_directory() / "rfirma-app"
tauri := app / "src-tauri"

# Ruta canonica de la libreria nativa (ADR-0013). `native` la produce aqui y el
# manifiesto flatpak la instala desde aqui. NO es target/native ni
# target/ce25-noui: esos eran los dos rivales que el ADR resolvio.
lib_nativa := bridge / "target/lib/rfirma/librfirma_crypto.so"

# cargo-crap tiene un solo mantenedor y cuatro meses de vida (ADR-0014), asi
# que la version va FIJADA: una publicacion suya no puede poner en rojo un PR
# que no la ha tocado. Si se abandona, la puerta se quita borrando la receta
# `crap` de `test` y esta linea.
version_crap := "0.4.3"

# El modulo FFI, oculto para la puerta CRAP del carril rapido. cargo-crap
# puntua con `--missing pessimistic`, o sea que una funcion SIN datos de
# cobertura vale 0 %, y la cobertura del carril rapido no incluye la grada C.
# Sin esta exclusion los peores CRAP del repositorio serian justo el codigo que
# SI esta probado, solo que en el otro carril. El carril lento repite la
# medicion sin ella (`just crap-completo`).
ffi_allow := "src/ffi.rs"

# Lista las recetas.
default:
    @just --list --unsorted

# ---------------------------------------------------------------------------
# Contrato
# ---------------------------------------------------------------------------

# `check` ES UN CONTRATO (ID-03): es exactamente `tools lint build test`, y
# docs/agents/code-host.md promete que el CI ejecuta eso y nada mas. Crece por
# dentro; su nombre y su papel no cambian.
#
# Lo que ejecutan el CI y el agente revisor.
check: tools lint build test

# El bucle corto de quien quiera formatear antes de commitear. Voluntaria, y
# deliberadamente NO es un hook de pre-commit (ADR-0014): en un repositorio
# movido por agentes un hook desconocido se esquiva con --no-verify o explota
# sin que nadie entienda por que.
#
# Solo lint, sin build ni test.
rapido: lint

# ---------------------------------------------------------------------------
# Herramientas y dependencias
# ---------------------------------------------------------------------------

# Comprueba que estan las herramientas, y falla nombrando la que falte.
tools:
    #!/usr/bin/env bash
    set -euo pipefail
    fallan=0
    for t in mvn git java pnpm cargo; do
        command -v "$t" >/dev/null || { echo "falta: $t"; fallan=1; }
    done
    # Opcionales: no rompen `check`, pero si la receta que los usa.
    graal="${GRAALVM_HOME:-{{ graalvm_por_defecto }}}"
    if [ ! -x "$graal/bin/native-image" ]; then
        echo "aviso: falta native-image en $graal"
        echo "  (solo hace falta para 'just native'; instala GraalVM CE 25)"
    fi
    command -v flatpak-builder >/dev/null || \
        echo "aviso: falta flatpak-builder (solo hace falta para 'just flatpak')"
    cargo llvm-cov --version >/dev/null 2>&1 || \
        echo "aviso: falta cargo-llvm-cov (cargo binstall cargo-llvm-cov)"
    cargo crap --version >/dev/null 2>&1 || \
        echo "aviso: falta cargo-crap (cargo binstall cargo-crap@{{ version_crap }})"
    [ "$fallan" = 0 ] || exit 1
    echo "herramientas: correcto"

# No estan en Maven Central: hay que compilarlas desde el repositorio oficial.
# La etiqueta v1.9.1 es inmutable, asi que esto se ejecuta una vez y la cache
# acierta siempre despues.
#
# bootstrap.sh NO CRECE (ID-04): resuelve ~/.m2 y nada mas. Instalar GraalVM,
# flatpak-builder o el token de pruebas son cosas con sudo o SDKMAN que un
# script no debe hacer a espaldas de nadie; quien las comprueba es `tools`.
#
# Instala las dependencias de AutoFirma en ~/.m2 si no estan.
bootstrap:
    ./bootstrap.sh

# Instala las dependencias de node de rfirma-app.
deps:
    cd {{ app }} && pnpm install --frozen-lockfile

# ---------------------------------------------------------------------------
# Lint
# ---------------------------------------------------------------------------

# Las tres cadenas, y falla si falla cualquiera.
lint: lint-java lint-ts lint-rust

# -Xlint:all, como decidio el issue #11.
lint-java: bootstrap
    cd {{ bridge }} && mvn -B clean compile

# Biome, no eslint + prettier (ADR-0014): un binario que formatea y lintea en
# milisegundos, y aqui el ecosistema de plugins de eslint no cobra porque no
# hay router, ni tabla de datos, ni biblioteca de componentes.
#
# Biome sobre rfirma-app.
lint-ts: deps
    cd {{ app }} && pnpm exec biome ci .

# clippy y rustfmt sobre rfirma-app/src-tauri.
#
# Depende de build-ts porque tauri-build lee frontendDist (../dist) ya en
# build.rs: sin el, clippy se cae antes de mirar una sola linea de Rust.
lint-rust: build-ts
    cd {{ tauri }} && cargo fmt --all -- --check
    cd {{ tauri }} && cargo clippy --all-targets --all-features -- -D warnings

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

# `tsc -b` va DENTRO de build, no en una receta aparte (ID-03): un build que
# compila TypeScript sin comprobar tipos miente sobre lo que ha comprobado.
#
# Compila las tres cadenas.
build: comprueba-nativa build-java build-ts build-rust

# Compila el puente Java.
build-java: bootstrap
    cd {{ bridge }} && mvn -B package -DskipTests

# tsc -b y vite build.
build-ts: deps
    cd {{ app }} && pnpm exec tsc -b
    cd {{ app }} && pnpm exec vite build

# Sin `cargo tauri build`: bundle.active es false (ID-05) y el binario lo
# instala el manifiesto flatpak. `vite build` tiene que haber corrido antes,
# porque tauri-build lee frontendDist.
#
# Compila el binario de la aplicacion.
build-rust: build-ts
    cd {{ tauri }} && cargo build --release

# La libreria nativa NO SE ENCADENA (ADR-0013): `dev` y `build` comprueban que
# esta y, si falta, fallan nombrando `just native`. Encadenarla metaria 1 m 22 s
# de native-image en cada compilacion.
#
# RFIRMA_SIN_NATIVA=1 salta la comprobacion. Existe por el carril rapido del CI,
# que corre `just check` sin construir la imagen nativa a proposito (son tres
# minutos que el carril lento ya paga). Ponerla a mano en local es decir "se lo
# que hago y no voy a ejecutar nada".
#
# Falla nombrando `just native` si la libreria nativa no esta.
comprueba-nativa:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "${RFIRMA_SIN_NATIVA:-0}" = "1" ]; then
        echo "comprueba-nativa: omitida (RFIRMA_SIN_NATIVA=1)"
        exit 0
    fi
    if [ ! -f "{{ lib_nativa }}" ]; then
        echo "falta la libreria nativa:" >&2
        echo "  {{ lib_nativa }}" >&2
        echo >&2
        echo "Ejecuta 'just native' (tarda unos tres minutos y necesita" >&2
        echo "GraalVM CE 25). No se construye sola a proposito: ver ADR-0013." >&2
        exit 1
    fi

# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------

# Las tres cadenas mas la puerta CRAP. Las gradas las fija el ADR-0014: A (nada)
# y B (SoftHSM) corren aqui; C (la libreria nativa) se marca #[ignore] y solo la
# ejecuta el carril lento, pero AQUI SE COMPILA.
#
# Ejecuta las pruebas de las tres cadenas y la puerta CRAP.
test: test-java test-ts test-rust crap

# AVISO: el puente todavia no tiene pruebas propias. La receta existe para que
# el sub-issue que escriba el puente real tenga donde colgarlas.
#
# Pruebas del puente Java.
test-java: build-java
    cd {{ bridge }} && mvn -B test

# vitest.
test-ts: deps
    cd {{ app }} && pnpm exec vitest run --reporter=dot

# cargo test, mas la compilacion de las pruebas de grada C.
test-rust: build-ts
    cd {{ tauri }} && cargo test --all-features
    # El punto ciego de #[ignore] es que una prueba de grada C que deja de
    # compilar contra la FFI se salta EN SILENCIO. Esto lo cierra: el carril
    # rapido las compila aunque no las ejecute (TD-02).
    cd {{ tauri }} && cargo test --all-features --no-run

# Las de grada C, que el carril lento ejecuta con --include-ignored.
test-nativo: comprueba-nativa build-ts
    cd {{ tauri }} && cargo test --all-features -- --include-ignored

# ---------------------------------------------------------------------------
# CRAP: solo en Rust (ADR-0014)
# ---------------------------------------------------------------------------
#
# En Java no entra —lo unico en Maven Central es un plugin de Hudson de 2010 y
# el puente es codigo que reenvia— y en TypeScript tampoco: la complejidad
# ciclomatica de un componente React es JSX condicional, que no es lo que la
# metrica mide. El codigo de riesgo de este proyecto esta todo en Rust.
#
# Umbral ABSOLUTO en 30 (el de Savoia), sin --baseline ni --fail-regression: el
# trinquete exige versionar un JSON que cambia en casi cada PR, y su unica
# ventaja —amnistiar deuda existente— no aplica cuando no hay deuda.

# Genera lcov.info con cargo llvm-cov.
cobertura: build-ts
    cd {{ tauri }} && cargo llvm-cov --all-features --lcov --output-path lcov.info

# La puerta del carril rapido, con el modulo FFI oculto.
crap: cobertura
    cd {{ tauri }} && cargo crap --lcov lcov.info --threshold 30 --fail-above \
        --allow '{{ ffi_allow }}'

# La misma medicion SIN la exclusion, con la cobertura de la grada C incluida.
# Aqui el modulo FFI da la cara. Carril lento.
crap-completo: comprueba-nativa build-ts
    cd {{ tauri }} && cargo llvm-cov --all-features --lcov --output-path lcov.info \
        -- --include-ignored
    cd {{ tauri }} && cargo crap --lcov lcov.info --threshold 30 --fail-above

# ---------------------------------------------------------------------------
# Imagen nativa, empaquetado y desarrollo
# ---------------------------------------------------------------------------

# Tarda minutos y consume mucha memoria; por eso el workflow no la construye
# en cada PR (ver .github/workflows/ci.yml).
#
# Produce la ruta CANONICA del ADR-0013, y solo librfirma_crypto.so: el
# directorio de construccion sigue teniendo los auxiliares de AWT que
# native-image emite, y un `install *.so` reintroduciria libawt.so — y con el,
# el aborto del proceso ante un JPEG con perfil ICC que midio el #36.
#
# Construye la libreria nativa compartida con GraalVM CE 25.
native: build-java
    #!/usr/bin/env bash
    set -euo pipefail
    graal="${GRAALVM_HOME:-{{ graalvm_por_defecto }}}"
    obra="{{ bridge }}/target/native"
    destino="$(dirname "{{ lib_nativa }}")"
    mkdir -p "$obra" && cd "$obra"
    "$graal/bin/native-image" --shared -H:Name=librfirma_crypto --no-fallback \
        -cp "{{ bridge }}/target/rfirma-native-bridge-0.1.0.jar:$(cat {{ bridge }}/target/cp.txt)"
    mkdir -p "$destino"
    install -m644 "$obra/librfirma_crypto.so" "$destino/librfirma_crypto.so"
    ls -la "$destino"

# Construye el flatpak, el unico canal soportado (ADR-0015).
flatpak: native
    cd {{ justfile_directory() }}/packaging/flatpak && \
        flatpak-builder --force-clean --user --install build-dir me.sgomez.rfirma.yml

# A mano, cuando cambie un fichero de bloqueo: el flatpak se construye SIN red
# (ADR-0013) y el CI comprueba que estos ficheros estan al dia en vez de
# regenerarlos, porque un fichero generado dentro del CI es un fichero que
# nadie ha mirado.
#
# Regenera cargo-sources.json y node-sources.json.
fuentes-flatpak:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}/packaging/flatpak"
    # Los dos generadores viven fuera de este repositorio: son de
    # flatpak/flatpak-builder-tools. No se versionan aqui ni los instala
    # bootstrap.sh (ID-04); se traen a mano la primera vez.
    if [ ! -f flatpak-cargo-generator.py ]; then
        echo "falta packaging/flatpak/flatpak-cargo-generator.py" >&2
        echo "  https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo" >&2
        exit 1
    fi
    command -v flatpak-node-generator >/dev/null || {
        echo "falta flatpak-node-generator" >&2
        echo "  https://github.com/flatpak/flatpak-builder-tools/tree/master/node" >&2
        exit 1
    }
    python3 flatpak-cargo-generator.py \
        ../../rfirma-app/src-tauri/Cargo.lock -o cargo-sources.json
    flatpak-node-generator pnpm ../../rfirma-app/pnpm-lock.yaml -o node-sources.json

# Abre la ventana con recarga en caliente.
dev: comprueba-nativa deps
    cd {{ app }} && RFIRMA_LIB_DIR="$(dirname "{{ lib_nativa }}")" pnpm exec tauri dev

# Borra lo construido.
clean:
    cd {{ bridge }} && mvn -B clean
    cd {{ tauri }} && cargo clean
    rm -rf {{ app }}/dist
