#!/usr/bin/env bash
# Instala en ~/.m2 las dependencias Java de AutoFirma que no estan en Maven Central (ADR-0002).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

upstream_repo="https://github.com/ctt-gob-es/clienteafirma.git"
upstream_version="1.9.2"
temp_dir="target/upstream-clone"

echo "=== rfirma Bootstrap: Comprobando dependencias de Autofirma ==="

if ! command -v mvn &> /dev/null; then
    echo "Error: Maven (mvn) no esta instalado. Por favor, instalalo primero."
    exit 1
fi

if ! command -v java &> /dev/null; then
    echo "Error: Java (java) no esta instalado. Por favor, instalalo primero."
    exit 1
fi

m2_afirma_path="$HOME/.m2/repository/es/gob/afirma/afirma-core/$upstream_version/afirma-core-$upstream_version.jar"

if [ -f "$m2_afirma_path" ]; then
    echo "Las dependencias de Autofirma (version $upstream_version) ya estan en la cache de Maven local."
    echo "Ruta: $m2_afirma_path"
else
    echo "No se encontraron las dependencias de Autofirma en la cache de Maven."
    echo "Clonando temporalmente el repositorio original de: $upstream_repo..."

    rm -rf "$temp_dir"
    mkdir -p "target"

    # ADR-0002: solo la etiqueta, sin reserva a la rama por defecto.
    git clone --branch "v$upstream_version" --depth 1 "$upstream_repo" "$temp_dir"

    echo "Compilando e instalando dependencias de Autofirma en el repositorio local..."
    (
        cd "$temp_dir"
        # ADR-0002: sin esta propiedad el reactor resuelve afirma-core:1.9 de
        # Maven Central en vez de usar el modulo del propio tag.
        mvn clean install -DskipTests -Dclienteafirma.version="$upstream_version"
    )

    rm -rf "$temp_dir"
    echo "Dependencias de Autofirma compiladas e instaladas con exito en ~/.m2."
fi

echo "=== rfirma Bootstrap: Proceso completado ==="
