#!/bin/bash
# rfirma Bootstrap Script
#
# Este script automatiza la instalación de las dependencias Java oficiales de
# Autofirma en la caché de Maven local (~/.m2). Esto permite que el módulo
# rfirma-native-bridge compile correctamente sin necesidad de duplicar
# código fuente ni usar sub-módulos de Git persistentes.

set -e

UPSTREAM_REPO="https://github.com/ctt-gob-es/clienteafirma.git"
UPSTREAM_VERSION="1.9.1"
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
    
    # Intentar clonar la versión tag específica, fallback a rama por defecto si falla
    git clone --branch "v$UPSTREAM_VERSION" --depth 1 "$UPSTREAM_REPO" "$TEMP_DIR" || \
    git clone --depth 1 "$UPSTREAM_REPO" "$TEMP_DIR"
    
    echo "Compilando e instalando dependencias de Autofirma en el repositorio local..."
    cd "$TEMP_DIR"
    mvn clean install -DskipTests
    cd ../..
    
    # Limpiar el clon temporal
    rm -rf "$TEMP_DIR"
    echo "✔ Dependencias de Autofirma compiladas e instaladas con éxito en ~/.m2."
fi

echo "=== rfirma Bootstrap: Proceso completado ==="
