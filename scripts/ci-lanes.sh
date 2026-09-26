#!/usr/bin/env bash
# Lee por stdin los ficheros de un PR y dice que carriles del CI corren (`java=true`...).
# Un carril se salta solo si todos caen en su lista de ajenos; sin ficheros, corren todos.
set -euo pipefail

inert() {
    case "$1" in
        docs/adr/* | docs/design/*) return 1 ;;
        docs/* | rfirma-conformance/* | changelog.d/* | .claude/* | .agents/* | skills-lock.json) return 0 ;;
        */*) return 1 ;;
        *.md) return 0 ;;
    esac
    return 1
}

java_ignores() {
    case "$1" in
        scripts/bootstrap.sh | scripts/ci-lanes.sh) return 1 ;;
        docs/* | rfirma-app/* | packaging/* | scripts/* | testdata/site-driver/*) return 0 ;;
    esac
    return 1
}

web_ignores() {
    case "$1" in
        rfirma-native-bridge/pom.xml | testdata/site-driver/*) return 1 ;;
        rfirma-app/src-tauri/*.rs | rfirma-app/src-tauri/*.md | rfirma-app/src-tauri/*.png) return 0 ;;
        rfirma-app/src-tauri/tests/*.snapshot | rfirma-app/src-tauri/tests/*.baseline) return 0 ;;
        rfirma-app/src-tauri/.config/nextest.toml | rfirma-app/src-tauri/clippy.toml) return 0 ;;
        docs/adr/* | rfirma-native-bridge/* | testdata/*) return 0 ;;
    esac
    return 1
}

rust_ignores() {
    case "$1" in
        rfirma-native-bridge/testbench/* | scripts/ci-lanes.sh) return 1 ;;
        docs/design/* | packaging/* | scripts/* | rfirma-native-bridge/*) return 0 ;;
    esac
    return 1
}

native_ignores() {
    case "$1" in
        docs/* | packaging/* | rfirma-app/src/* | rfirma-app/po/*) return 0 ;;
    esac
    return 1
}

lanes=(java web rust native)
declare -A runs=([java]=false [web]=false [rust]=false [native]=false)
seen=false

while IFS= read -r file || [ -n "$file" ]; do
    [ -n "$file" ] || continue
    seen=true
    inert "$file" && continue
    for lane in "${lanes[@]}"; do
        "${lane}_ignores" "$file" || runs[$lane]=true
    done
done

for lane in "${lanes[@]}"; do
    if [ "$seen" = false ]; then
        echo "$lane=true"
    else
        echo "$lane=${runs[$lane]}"
    fi
done
