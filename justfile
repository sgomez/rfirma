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
# Requisitos: just, maven, git, pnpm, cargo y un GraalVM CE 25 para `native`,
# mas las librerias -dev del WebView (ver `system_libs` mas abajo) y softhsm2 +
# opensc para el token de la grada B (ver la receta `token`).
#   apt-get install -y just maven softhsm2 opensc
#
# `just tools` los comprueba TODOS y falla nombrando lo que falte, con la orden
# de apt lista para copiar. Ejecutalo antes que nada si algo no compila.

# GraalVM CE 25: lo fijo el issue #6. La linea 21 aborta dentro del JNI_OnLoad
# de libawt.so con cualquier firma visible, asi que no sirve para construir.
# El pom sigue compilando a release 21: cambia el JDK que construye, no el
# lenguaje de destino.
default_graalvm := "$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce"

bridge := justfile_directory() / "rfirma-native-bridge"
app := justfile_directory() / "rfirma-app"
tauri := app / "src-tauri"

# Ruta canonica de la libreria nativa (ADR-0013). `native` la produce aqui y el
# manifiesto flatpak la instala desde aqui. NO es target/native ni
# target/ce25-noui: esos eran los dos rivales que el ADR resolvio.
native_lib := bridge / "target/lib/rfirma/librfirma_crypto.so"

# cargo-crap tiene un solo mantenedor y cuatro meses de vida (ADR-0014), asi
# que la version va FIJADA: una publicacion suya no puede poner en rojo un PR
# que no la ha tocado. Si se abandona, la puerta se quita borrando la receta
# `crap` de `test` y esta linea.
crap_version := "0.4.3"

# El modulo FFI, oculto para la puerta CRAP del carril rapido. cargo-crap
# puntua con `--missing pessimistic`, o sea que una funcion SIN datos de
# cobertura vale 0 %, y la cobertura del carril rapido no incluye la grada C.
# Sin esta exclusion los peores CRAP del repositorio serian justo el codigo que
# SI esta probado, solo que en el otro carril. El carril lento repite la
# medicion sin ella (`just crap-full`).
ffi_allow := "src/ffi.rs"

# Las librerias de sistema que necesita el WebView de Tauri, como pares
# "<modulo de pkg-config>:<paquete apt>". NO son dependencias de cargo: son
# paquetes -dev del sistema, y sin ellas `cargo build` muere dentro del
# build script de webkit2gtk-sys con un error de pkg-config que no menciona
# ningun paquete instalable.
#
# ESTA ES LA LISTA CANONICA: .github/workflows/ci.yml instala exactamente
# estos paquetes en sus dos carriles. Si tocas una, tocas las tres.
system_libs := "webkit2gtk-4.1:libwebkit2gtk-4.1-dev javascriptcoregtk-4.1:libjavascriptcoregtk-4.1-dev libsoup-3.0:libsoup-3.0-dev"

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
quick: lint

# ---------------------------------------------------------------------------
# Herramientas y dependencias
# ---------------------------------------------------------------------------

# Comprueba que estan las herramientas, y falla nombrando la que falte.
tools:
    #!/usr/bin/env bash
    set -euo pipefail
    failures=0
    for t in mvn git java pnpm cargo; do
        command -v "$t" >/dev/null || { echo "falta: $t"; failures=1; }
    done
    # Un cargo instalado pero fuera del PATH es el falso negativo mas caro de
    # esta receta: "falta: cargo" manda a reinstalar rustup a quien solo tiene
    # que cargar el env. rustup lo deja en ~/.cargo/env, que ~/.profile carga
    # y zsh NO lee en shells interactivas.
    if ! command -v cargo >/dev/null && [ -x "$HOME/.cargo/bin/cargo" ]; then
        echo "  cargo esta en ~/.cargo/bin pero no en el PATH:"
        echo "    anade '. \"$HOME/.cargo/env\"' a tu ~/.zshrc (o ~/.bashrc)"
    fi
    # El token de la grada B (ADR-0014). No es opcional: sus pruebas corren en
    # el carril rapido, asi que sin estas tres ordenes `test-rust` falla.
    softhsm_apt=""
    command -v softhsm2-util >/dev/null || { echo "falta: softhsm2-util"; softhsm_apt="$softhsm_apt softhsm2"; failures=1; }
    command -v pkcs11-tool  >/dev/null || { echo "falta: pkcs11-tool";  softhsm_apt="$softhsm_apt opensc";   failures=1; }
    command -v openssl      >/dev/null || { echo "falta: openssl";      softhsm_apt="$softhsm_apt openssl";  failures=1; }
    if [ -n "$softhsm_apt" ]; then
        echo
        echo "Instalalos con:"
        echo "  sudo apt install -y$softhsm_apt"
        echo "y monta el token con: just token"
        echo
    fi
    # Las librerias de sistema del WebView. pkg-config es quien decide, porque
    # es quien consulta el build script que falla: el paquete de runtime puede
    # estar instalado y faltar solo el -dev, que es el que trae el .pc.
    if command -v pkg-config >/dev/null; then
        missing_apt=""
        for pair in {{ system_libs }}; do
            module="${pair%%:*}"
            package="${pair#*:}"
            pkg-config --exists "$module" || {
                echo "falta la libreria de sistema: $module"
                missing_apt="$missing_apt $package"
                failures=1
            }
        done
        if [ -n "$missing_apt" ]; then
            echo
            echo "Instalalas con:"
            echo "  sudo apt install -y$missing_apt"
            echo
        fi
    else
        echo "falta: pkg-config"
        failures=1
    fi
    # Opcionales: no rompen `check`, pero si la receta que los usa.
    graal="${GRAALVM_HOME:-{{ default_graalvm }}}"
    if [ ! -x "$graal/bin/native-image" ]; then
        echo "aviso: falta native-image en $graal"
        echo "  (solo hace falta para 'just native'; instala GraalVM CE 25)"
    fi
    command -v flatpak-builder >/dev/null || \
        echo "aviso: falta flatpak-builder (solo hace falta para 'just flatpak')"
    cargo llvm-cov --version >/dev/null 2>&1 || \
        echo "aviso: falta cargo-llvm-cov (cargo binstall cargo-llvm-cov)"
    cargo crap --version >/dev/null 2>&1 || \
        echo "aviso: falta cargo-crap (cargo binstall cargo-crap@{{ crap_version }})"
    [ "$failures" = 0 ] || exit 1
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

# El token de la GRADA B (ADR-0014). Es idempotente y tarda segundos, asi que
# `test-rust` lo llama siempre: la grada B corre en el carril rapido por
# definicion, y una prueba que se salta en silencio porque falta el token no es
# una prueba.
#
# Provisiona el token SoftHSM `rfirma-test` desde testdata/fnmt/.
token:
    ./testdata/softhsm/provision-token.sh

# ---------------------------------------------------------------------------
# Lint
# ---------------------------------------------------------------------------

# Las tres cadenas, y falla si falla cualquiera.
#
# `check-flatpak-sources` va PRIMERA a proposito: no necesita ni bootstrap ni
# deps, tarda milisegundos, y lo que detecta —un fichero de bloqueo tocado sin
# regenerar las fuentes vendorizadas— no lo encuentra ninguna de las otras.
lint: check-flatpak-sources lint-java lint-ts lint-rust

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

# Depende de build-ts porque tauri-build lee frontendDist (../dist) ya en
# build.rs: sin el, clippy se cae antes de mirar una sola linea de Rust.
#
# clippy y rustfmt sobre rfirma-app/src-tauri.
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
build: check-native build-java build-ts build-rust

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
# --features custom-protocol NO ES OPCIONAL, y es justo lo que se pierde al no
# usar `cargo tauri build`, que la pasa el solo. Sin ella el `dev` de Tauri
# queda encendido y el binario apunta la ventana a devUrl en vez de servir el
# frontal empotrado. Ver el bloque [features] de src-tauri/Cargo.toml.
#
# Compila el binario de la aplicacion.
build-rust: build-ts
    cd {{ tauri }} && cargo build --release --features custom-protocol

# La libreria nativa NO SE ENCADENA (ADR-0013): `dev` y `build` comprueban que
# esta y, si falta, fallan nombrando `just native`. Encadenarla metaria 1 m 22 s
# de native-image en cada compilacion.
#
# RFIRMA_SKIP_NATIVE=1 salta la comprobacion. Existe por el carril rapido del CI,
# que corre `just check` sin construir la imagen nativa a proposito (son tres
# minutos que el carril lento ya paga). Ponerla a mano en local es decir "se lo
# que hago y no voy a ejecutar nada".
#
# Falla nombrando `just native` si la libreria nativa no esta.
check-native:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "${RFIRMA_SKIP_NATIVE:-0}" = "1" ]; then
        echo "check-native: omitida (RFIRMA_SKIP_NATIVE=1)"
        exit 0
    fi
    if [ ! -f "{{ native_lib }}" ]; then
        echo "falta la libreria nativa:" >&2
        echo "  {{ native_lib }}" >&2
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

# Las de grada A del puente. Las de grada C llevan @Tag("gradaC") y el pom las
# excluye por omision, porque necesitan poppler (`pdfsig`) y el carril rapido no
# lo instala. Se COMPILAN igual —`mvn test` compila todas—, que es la mitad de la
# TD-02 que le toca a esta cadena.
#
# Pruebas del puente Java.
test-java: build-java
    cd {{ bridge }} && mvn -B test

# vitest.
test-ts: deps
    cd {{ app }} && pnpm exec vitest run --reporter=dot

# cargo test, mas la compilacion de las pruebas de grada C.
test-rust: token build-ts
    cd {{ tauri }} && cargo test --all-features
    # El punto ciego de #[ignore] es que una prueba de grada C que deja de
    # compilar contra la FFI se salta EN SILENCIO. Esto lo cierra: el carril
    # rapido las compila aunque no las ejecute (TD-02).
    cd {{ tauri }} && cargo test --all-features --no-run

# RFIRMA_LIB_DIR por lo mismo que en `dev`: el binario de una prueba vive en
# src-tauri/target/debug/deps/, asi que la ruta relativa al ejecutable que usa
# el cargador (../lib/rfirma) resolveria a src-tauri/target/debug/lib/rfirma y
# no a donde `native` acaba de instalar la libreria.
#
# Las de grada C, que el carril lento ejecuta con --include-ignored.
test-native: token check-native build-ts
    cd {{ tauri }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" cargo test --all-features -- --include-ignored
    # Las de grada C del puente Java: el ciclo trifasico entero validado con
    # `pdfsig` de poppler, que es la puerta automatica de validez del ADR-0014.
    # -DexcludedGroups= levanta la exclusion que el pom pone por omision.
    cd {{ bridge }} && mvn -B test -DexcludedGroups=

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
coverage: token build-ts
    cd {{ tauri }} && cargo llvm-cov --all-features --lcov --output-path lcov.info

# La puerta del carril rapido, con el modulo FFI oculto.
crap: coverage
    cd {{ tauri }} && cargo crap --lcov lcov.info --threshold 30 --fail-above \
        --allow '{{ ffi_allow }}'

# La misma medicion SIN la exclusion, con la cobertura de la grada C incluida.
# Aqui el modulo FFI da la cara. Carril lento.
crap-full: token check-native build-ts
    cd {{ tauri }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" cargo llvm-cov --all-features --lcov --output-path lcov.info \
        -- --include-ignored
    cd {{ tauri }} && cargo crap --lcov lcov.info --threshold 30 --fail-above

# ---------------------------------------------------------------------------
# Imagen nativa, empaquetado y desarrollo
# ---------------------------------------------------------------------------

# Tarda minutos y consume mucha memoria; por eso el workflow no la construye
# en cada PR (ver .github/workflows/ci.yml).
#
# AQUI NO HAY BANDERAS SUELTAS, y es a proposito (ID-06): el nombre de la
# libreria, --no-fallback y los .afm de iText viven VERSIONADOS en
# rfirma-native-bridge/src/main/resources/META-INF/native-image/, que
# native-image recoge del classpath el solo. Asi la imagen se construye igual
# desde un clon limpio que desde aqui. Si vuelve a hacer falta una bandera, va a
# ese fichero, no a esta linea.
#
# Produce la ruta CANONICA del ADR-0013, y solo librfirma_crypto.so: si algun dia
# el directorio de construccion vuelve a tener los auxiliares de AWT, un
# `install *.so` reintroduciria libawt.so — y con el, el aborto del proceso ante
# un JPEG con perfil ICC que midio el #36.
#
# Construye la libreria nativa compartida con GraalVM CE 25.
native: build-java
    #!/usr/bin/env bash
    set -euo pipefail
    graal="${GRAALVM_HOME:-{{ default_graalvm }}}"
    build_dir="{{ bridge }}/target/native"
    dest="$(dirname "{{ native_lib }}")"
    mkdir -p "$build_dir" && cd "$build_dir"
    "$graal/bin/native-image" --shared \
        -cp "{{ bridge }}/target/rfirma-native-bridge-0.1.0.jar:$(cat {{ bridge }}/target/cp.txt)"
    # El directorio de DISTRIBUCION se vacia antes de copiar. No es limpieza
    # cosmetica: sin esto hereda lo que dejase una version anterior de esta
    # receta —las que instalaban los seis .so— y el directorio que el manifiesto
    # empaqueta acabaria con libawt.so dentro sin que nadie lo tocara.
    rm -rf "$dest"
    mkdir -p "$dest"
    install -m644 "$build_dir/librfirma_crypto.so" "$dest/librfirma_crypto.so"
    # Y se comprueba, porque la invariante es "UNO", no "el que acabo de copiar".
    sobran="$(ls -1 "$dest" | grep -v '^librfirma_crypto\.so$' || true)"
    if [ -n "$sobran" ]; then
        echo "sobra algo en $dest:" >&2
        echo "$sobran" >&2
        exit 1
    fi
    ls -la "$dest"

# El manifiesto lee la ruta canonica que produce `native` y el frontend ya
# construido de rfirma-app/dist, porque tauri-build lee `frontendDist` dentro de
# su propio build.rs. Por eso esta receta encadena tambien `build-ts`.
#
# EL ENTREGABLE DEL v0.1 ES EL FICHERO .flatpak (ID-42), no la instalacion: se
# construye contra un repositorio ostree local y de ahi sale el bundle de un
# solo fichero, que se instala con `flatpak install`. No se publica en ningun
# sitio —ni Releases, ni repositorio remoto, ni GPG—: eso es el ADR-0015 y
# queda fuera de este hito.
#
# El runtime NO va dentro del bundle: se consume del remoto de Flathub, que es
# por tanto requisito de instalacion. Ver el README.
#
# Construye el flatpak, el unico canal soportado (ADR-0015).
flatpak: native build-ts
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}/packaging/flatpak"
    flatpak-builder --force-clean --user --install --repo=repo \
        build-dir me.sgomez.rfirma.yml
    flatpak build-bundle repo me.sgomez.rfirma.flatpak me.sgomez.rfirma stable
    echo
    echo "bundle: $PWD/me.sgomez.rfirma.flatpak ($(du -h me.sgomez.rfirma.flatpak | cut -f1))"
    echo "  flatpak install --user me.sgomez.rfirma.flatpak"

# A mano, cuando cambie un fichero de bloqueo: el flatpak se construye SIN red
# (ADR-0013) y el CI comprueba que estos ficheros estan al dia en vez de
# regenerarlos, porque un fichero generado dentro del CI es un fichero que
# nadie ha mirado.
#
# Regenera cargo-sources.json y node-sources.json.
flatpak-sources:
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
    # El sello que lee `check-flatpak-sources`: el sha256 de cada fichero de
    # bloqueo TAL Y COMO estaba al generar los JSON de arriba. Se escribe en el
    # formato de sha256sum para que comprobarlo sea `sha256sum -c` y no un
    # analizador nuestro. Las rutas van relativas a la raiz del repositorio,
    # que es desde donde comprueba el script.
    cd "{{ justfile_directory() }}"
    sha256sum rfirma-app/src-tauri/Cargo.lock rfirma-app/pnpm-lock.yaml \
        > packaging/flatpak/sources.lock
    echo
    echo "regeneradas. Versiona cargo-sources.json, node-sources.json y sources.lock."

# La comprobacion de ID-07, sin regenerar nada. Va dentro de `lint` (y por
# tanto de `check`) en vez de ser un paso suelto del workflow, porque
# docs/agents/code-host.md promete que el CI ejecuta `just check` y nada mas.
#
# Comprueba que las fuentes vendorizadas del flatpak estan al dia.
check-flatpak-sources:
    {{ justfile_directory() }}/packaging/flatpak/check-sources.sh

# Abre la ventana con recarga en caliente.
dev: check-native deps
    cd {{ app }} && RFIRMA_LIB_DIR="$(dirname "{{ native_lib }}")" pnpm exec tauri dev

# Borra lo construido.
clean:
    cd {{ bridge }} && mvn -B clean
    cd {{ tauri }} && cargo clean
    rm -rf {{ app }}/dist
