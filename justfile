# Punto de entrada del proyecto.
#
# Es la interfaz que encuentra quien llega al repositorio sin contexto: una
# persona nueva, o el agente revisor, que segun docs/agents/code-host.md
# siempre instala y ejecuta las comprobaciones el mismo. `just` lista las
# recetas disponibles.
#
# Decidido en https://github.com/sgomez/rfirma/issues/11
#
# Requisitos: just, maven, git y un GraalVM CE 25 para `native`.
#   apt-get install -y just maven

# GraalVM CE 25: lo fijo el issue #6. La linea 21 aborta dentro del JNI_OnLoad
# de libawt.so con cualquier firma visible, asi que no sirve para construir.
# El pom sigue compilando a release 21: cambia el JDK que construye, no el
# lenguaje de destino.
graalvm_por_defecto := "$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce"

bridge := justfile_directory() / "rfirma-native-bridge"

# Lista las recetas.
default:
    @just --list --unsorted

# Comprueba que estan las herramientas, y falla nombrando la que falte.
tools:
    #!/usr/bin/env bash
    set -euo pipefail
    fallan=0
    for t in mvn git java; do
        command -v "$t" >/dev/null || { echo "falta: $t"; fallan=1; }
    done
    graal="${GRAALVM_HOME:-{{graalvm_por_defecto}}}"
    if [ ! -x "$graal/bin/native-image" ]; then
        echo "falta native-image en $graal"
        echo "  (solo hace falta para 'just native'; instala GraalVM CE 25)"
    fi
    [ "$fallan" = 0 ] || exit 1
    echo "herramientas: correcto"

# No estan en Maven Central: hay que compilarlas desde el repositorio oficial.
# La etiqueta v1.9.1 es inmutable, asi que esto se ejecuta una vez y la cache
# acierta siempre despues.
#
# Instala las dependencias de AutoFirma en ~/.m2 si no estan.
bootstrap:
    ./bootstrap.sh

# Compila el puente Java.
build: bootstrap
    cd {{bridge}} && mvn -B package -DskipTests

# Compila con todos los avisos del compilador activos.
lint:
    cd {{bridge}} && mvn -B clean compile

# AVISO: hoy no hay ninguna prueba. El repositorio todavia no tiene codigo de
# produccion -- NativeBridge.java es el puente de medicion de los issues #2 y
# #13, no el puente real. Esta receta existe para que la primera sub-issue que
# escriba codigo de verdad tenga donde colgar sus pruebas, y para que el
# workflow no tenga que cambiar cuando eso pase.
#
# Ejecuta las pruebas (hoy: ninguna, ver aviso arriba).
test: build
    cd {{bridge}} && mvn -B test

# Lo que ejecutan el CI y el agente revisor.
check: tools lint build test

# Tarda minutos y consume mucha memoria; por eso el workflow no la construye
# en cada PR (ver .github/workflows/ci.yml).
#
# Construye la libreria nativa compartida con GraalVM CE 25.
native: build
    #!/usr/bin/env bash
    set -euo pipefail
    graal="${GRAALVM_HOME:-{{graalvm_por_defecto}}}"
    dir="{{bridge}}/target/native"
    mkdir -p "$dir" && cd "$dir"
    "$graal/bin/native-image" --shared -H:Name=librfirma_crypto --no-fallback \
        -cp "{{bridge}}/target/rfirma-native-bridge-0.1.0.jar:$(cat {{bridge}}/target/cp.txt)"
    ls -la "$dir"/*.so

# Borra lo construido.
clean:
    cd {{bridge}} && mvn -B clean
