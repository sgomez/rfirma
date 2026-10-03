#!/usr/bin/env bash
# Lista los ficheros de rfirma-app/src-tauri con un cfg de plataforma y, con '/' final, los modulos que solo compila una.
set -euo pipefail

tauri=rfirma-app/src-tauri

find "$tauri" -path "$tauri/target" -prune -o -name '*.rs' -type f -print | sort | xargs env LC_ALL=C awk '
    function crate_dir(file, dir, base) {
        dir = file; sub(/\/[^\/]*$/, "", dir)
        base = file; sub(/.*\//, "", base)
        if (base == "mod.rs" || base == "lib.rs" || base == "main.rs" || base == "build.rs") return dir
        if (dir ~ /\/(tests|examples|benches)$/ || dir ~ /\/src\/bin$/) return dir
        sub(/\.rs$/, "", file)
        return file
    }
    FNR == 1 { gated = 0; path = ""; dir = FILENAME; sub(/\/[^\/]*$/, "", dir); modules = crate_dir(FILENAME) }
    {
        attribute = $0 ~ /^[[:space:]]*#\[/
        if ($0 ~ /cfg(!|_attr)?[(](.*[^A-Za-z0-9_"-])?(windows|unix|target_os|target_family)([^A-Za-z0-9_"-]|$)/) {
            print FILENAME
            if ($0 ~ /^[[:space:]]*#\[cfg[(]/) gated = 1
        }
        if (gated && $0 ~ /^[[:space:]]*#\[path[[:space:]]*=/) {
            path = $0; sub(/^[^"]*"/, "", path); sub(/".*/, "", path)
        }
        if (gated && $0 ~ /(^|[][:space:]])mod[[:space:]]+[A-Za-z0-9_]+[[:space:]]*;/) {
            name = $0; sub(/.*(^|[][:space:]])mod[[:space:]]+/, "", name); sub(/[^A-Za-z0-9_].*/, "", name)
            if (path != "") print dir "/" path
            else { print modules "/" name ".rs"; print modules "/" name "/" }
            gated = 0; path = ""; next
        }
        if (!attribute && $0 !~ /^[[:space:]]*(\/\/.*)?$/) { gated = 0; path = "" }
    }
' | sort -u
