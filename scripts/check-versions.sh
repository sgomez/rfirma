#!/usr/bin/env bash
# Guarda de `versions.env`: ninguna version fijada se escribe fuera de el, y los pom arrancan con su AutoFirma.
#
# Uso: check-versions.sh [raiz]
set -euo pipefail

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"

versions=versions.env
poms=(rfirma-native-bridge/pom.xml rfirma-native-bridge/testbench/reference-signer/pom.xml)
failures=0

fail() {
    echo "$*" >&2
    failures=$((failures + 1))
}

tooling_files() {
    local f
    for f in justfile lefthook.yml; do
        [ ! -f "$f" ] || echo "$f"
    done
    [ ! -d .github ] || find .github -type f
    find scripts rfirma-native-bridge/testbench -maxdepth 1 -name '*.sh' 2>/dev/null || true
}

literal_pattern() {
    printf '(^|[^0-9.])%s(\\.?[^0-9.]|$)' "$(printf '%s' "$1" | sed 's/\./\\./g')"
}

if [ ! -f "$versions" ]; then
    echo "falta $versions" >&2
    exit 1
fi
[ ! -e .graalvm-version ] || fail ".graalvm-version: la version de GraalVM vive en $versions"

files="$(tooling_files | sort)"
while IFS= read -r line || [ -n "$line" ]; do
    line="${line%$'\r'}"
    if [[ ! "$line" =~ ^[A-Z][A-Z0-9_]*=[^[:space:]\"\'\$]+$ ]]; then
        fail "$versions: '$line' no es CLAVE=valor, sin comillas ni export"
        continue
    fi
    [ -n "$files" ] || continue
    while IFS= read -r hit; do
        fail "$hit"
        fail "    literal de ${line%%=*}: se lee de $versions"
    done < <(printf '%s\n' "$files" | xargs grep -nE "$(literal_pattern "${line#*=}")" /dev/null || true)
done <"$versions"

autofirma="$(sed -n 's/^AUTOFIRMA_VERSION=//p' "$versions" | tr -d '\r')"
for pom in "${poms[@]}"; do
    if [ ! -f "$pom" ]; then
        fail "falta $pom"
        continue
    fi
    default="$(sed -n 's:.*<autofirma.version>\(.*\)</autofirma.version>.*:\1:p' "$pom")"
    [ "$default" = "$autofirma" ] ||
        fail "$pom: autofirma.version vale '$default' y $versions fija '$autofirma'"
done

[ "$failures" -eq 0 ] || exit 1
echo "OK  versiones fijadas: solo en $versions, y los pom con AutoFirma $autofirma"
