#!/bin/bash
# rfirma Bootstrap Script
#
# Este script automatiza la instalación de las dependencias Java oficiales de
# Autofirma en la caché de Maven local (~/.m2). Esto permite que el módulo
# rfirma-native-bridge compile correctamente sin necesidad de duplicar
# código fuente ni usar sub-módulos de Git persistentes.

set -e

UPSTREAM_REPO="https://github.com/ctt-gob-es/clienteafirma.git"
UPSTREAM_VERSION="1.9.2"
TEMP_DIR="target/upstream-clone"

echo "=== rfirma Bootstrap: Comprobando dependencias de Autofirma ==="

# Verificar herramientas requeridas
if ! command -v mvn &> /dev/null; then
    echo "Error: Maven (mvn) no está instalado. Por favor, instálalo primero."
    exit 1
fi

if ! command -v java &> /dev/null; then
    echo "Error: Java (java) no está instalado. Por favor, instálalo primero."
    exit 1
fi

# Verificar si el artefacto principal ya está registrado en ~/.m2
M2_AFIRMA_PATH="$HOME/.m2/repository/es/gob/afirma/afirma-core/$UPSTREAM_VERSION/afirma-core-$UPSTREAM_VERSION.jar"

if [ -f "$M2_AFIRMA_PATH" ]; then
    echo "✔ Las dependencias de Autofirma (versión $UPSTREAM_VERSION) ya están en la caché de Maven local."
    echo "Ruta: $M2_AFIRMA_PATH"
    echo "No es necesario recompilar. Puedes proceder a compilar rfirma-native-bridge."
else
    echo "✘ No se encontraron las dependencias de Autofirma en la caché de Maven."
    echo "Clonando temporalmente el repositorio original de: $UPSTREAM_REPO..."
    
    # Limpiar clonados anteriores
    rm -rf "$TEMP_DIR"
    mkdir -p "target"
    
    # SOLO el tag, sin reserva. La reserva a la rama por defecto que habia aqui
    # instalaba jar ETIQUETADOS "$UPSTREAM_VERSION" construidos desde `master`,
    # que es como el .so acabo llevando dentro codigo de Autofirma sin publicar
    # (#330). Si el tag no se puede clonar, se para.
    git clone --branch "v$UPSTREAM_VERSION" --depth 1 "$UPSTREAM_REPO" "$TEMP_DIR"
    
    echo "Compilando e instalando dependencias de Autofirma en el repositorio local..."
    cd "$TEMP_DIR"
    # -Dclienteafirma.version OBLIGATORIO: el pom del tag v1.9.2 declara
    # <clienteafirma.version>1.9</clienteafirma.version> con el proyecto en
    # 1.9.2, asi que sin esto los 42 pom de modulo se bajan afirma-core:1.9 de
    # Central en vez de usar el modulo del reactor. `afirma-crypto-cades` no
    # compila (Map<String,byte[]> contra byte[] en getASiCSData) y, peor, lo que
    # si compila lo hace contra la 1.9 en silencio. Medido en el #330.
    mvn clean install -DskipTests -Dclienteafirma.version="$UPSTREAM_VERSION"
    cd ../..
    
    # Limpiar el clon temporal
    rm -rf "$TEMP_DIR"
    echo "✔ Dependencias de Autofirma compiladas e instaladas con éxito en ~/.m2."
fi

echo "=== rfirma Bootstrap: Proceso completado ==="
