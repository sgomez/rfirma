#!/usr/bin/env bash
# Sin `set -e`: la salida es corta a proposito, pero si alguien la pasa por
# `head` el SIGPIPE mataria a awk y la receta fallaria con un 141 que no
# significa nada.
set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
status=0
file=""

resolve() {
    file="$1"
    [ -f "$file" ] || file="$root/$1"
    [ -f "$file" ]
}

skeleton() {
    local requested="$1" lang
    if ! resolve "$requested"; then
        echo "outline: no existe $requested" >&2
        status=1
        return
    fi
    case "$file" in
        *.rs)        lang=rust ;;
        *.ts|*.tsx)  lang=ts ;;
        *)
            echo "outline: solo .rs, .ts y .tsx tienen esqueleto ($requested). Para el resto, $requested:A-B o grep -n" >&2
            status=1
            return ;;
    esac
    awk -v lang="$lang" '
# Una linea de esqueleto: sin la sangria, sin la llave suelta del final.
function emit(n, s) {
    sub(/^[ \t]+/, "", s)
    sub(/[ \t]+$/, "", s)
    sub(/[ \t]+\{[ \t]*$/, " {", s)
    if (length(s) > 160) s = substr(s, 1, 157) "..."
    printf "%5d  %s\n", n, s
}
# La documentacion se acumula hasta el final de la PRIMERA FRASE y se
# imprime entera: cortarla por donde cayo el salto de linea entrega media
# frase, que cuesta lo mismo y no dice nada.
function adddoc(n, marker, text) {
    if (docbuf == "") { docbuf = marker " " text; docline = n; docdone = 0; return }
    if (docdone) return
    if (text == "") { docdone = 1; return }
    docbuf = docbuf " " text
}
function flushdoc(   t) {
    if (docbuf == "") return
    t = docbuf
    if (match(t, /\.[ ]/)) t = substr(t, 1, RSTART)
    emit(docline, t)
    docbuf = ""; docdone = 0
}
{ if (docbuf != "" && length(docbuf) > 200) docdone = 1 }

lang == "rust" && /^[ \t]*(\/\/\/|\/\/!)/ {
    line = $0; sub(/^[ \t]*/, "", line)
    text = line; sub(/^(\/\/\/|\/\/!)[ \t]?/, "", text)
    adddoc(FNR, substr(line, 1, 3), text)
    next
}
lang == "rust" && /^[ \t]*(pub |impl |fn |mod |const |static |type |struct |enum |macro_rules!)/ {
    flushdoc(); emit(FNR, $0); next
}
lang == "rust" && /^[ \t]*#\[(test|tauri::command|derive|cfg\(test\))/ {
    flushdoc(); emit(FNR, $0); next
}
lang == "rust" { flushdoc(); next }

lang == "ts" && /^[ \t]*\/\*\*/ {
    # Un bloque de UNA linea (`/** ... */`) se cierra aqui mismo: si se
    # entrara en modo bloque nunca se saldria y el codigo de debajo pasaria
    # por documentacion.
    if (/\*\//) {
        line = $0
        sub(/^[ \t]*\/\*\*[ \t]*/, "", line)
        sub(/[ \t]*\*\/.*$/, "", line)
        adddoc(FNR, "//", line)
        next
    }
    inblock = 1; next
}
lang == "ts" && inblock {
    if (/\*\//) { inblock = 0; next }
    line = $0; sub(/^[ \t]*\*[ \t]?/, "", line); sub(/[ \t]+$/, "", line)
    adddoc(FNR, "//", line)
    next
}
lang == "ts" && /^(export |function |class |interface |type |const |async |declare )/ {
    flushdoc(); emit(FNR, $0); next
}
lang == "ts" && /^[ \t]*(it|test|describe)\(/ { flushdoc(); emit(FNR, $0); next }
lang == "ts" && /^[ \t]+const [A-Za-z_$]+ = (async )?(\(|useCallback|function)/ {
    flushdoc(); emit(FNR, $0); next
}
lang == "ts" && /^[ \t]*$/ { flushdoc(); next }
END { flushdoc() }
' "$file"
    wc -lc < "$file" | awk -v name="$requested" '{
        printf "\n-- %s: %d lineas, %d caracteres (~%.1fk tokens si lo lees entero).\n", \
            name, $1, $2, $2 / 3500
        if ($1 < 120)
            printf "   Es corto: leelo entero si vas a tocarlo. --\n"
        else {
            printf "   Abre los tramos que necesites, de TODOS los ficheros en UNA SOLA LLAMADA:\n"
            printf "     just outline %s:A-B,C-D otro:E-F\n", name
            printf "   Un turno por tramo sale mas caro que leer el fichero entero. --\n"
        }
    }'
}

ranges() {
    local requested="$1" spec="$2" range from to first=1
    if ! resolve "$requested"; then
        echo "outline: no existe $requested" >&2
        status=1
        return
    fi
    if ! [[ "$spec" =~ ^[0-9]+-[0-9]+(,[0-9]+-[0-9]+)*$ ]]; then
        echo "outline: tramo mal formado en $requested:$spec (se espera A-B[,C-D...])" >&2
        status=1
        return
    fi
    for range in ${spec//,/ }; do
        from="${range%-*}"
        to="${range#*-}"
        if [ "$from" -lt 1 ] || [ "$from" -gt "$to" ]; then
            echo "outline: tramo invalido $range en $requested" >&2
            status=1
            continue
        fi
        [ "$first" = 1 ] || echo "   ..."
        first=0
        awk -v a="$from" -v b="$to" 'FNR >= a && FNR <= b { printf "%5d  %s\n", FNR, $0 }' "$file"
    done
}

if [ "$#" -eq 0 ]; then
    echo "outline: uso: outline ruta | ruta:A-B[,C-D...] ..." >&2
    exit 1
fi

labelled=0
if [ "$#" -gt 1 ] || [[ "$1" == *:* ]]; then labelled=1; fi
n=0
for arg in "$@"; do
    n=$((n + 1))
    if [ "$labelled" = 1 ]; then
        [ "$n" -gt 1 ] && echo
        echo "== $arg =="
    fi
    if [[ "$arg" == *:* ]]; then
        ranges "${arg%:*}" "${arg##*:}"
    else
        skeleton "$arg"
    fi
done
exit "$status"
