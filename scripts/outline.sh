#!/usr/bin/env bash
# Sin `set -e`: la salida es corta a proposito, pero si alguien la pasa por
# `head` el SIGPIPE mataria a awk y la receta fallaria con un 141 que no
# significa nada.
set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
requested="$1"
file="$requested"
[ -f "$file" ] || file="$root/$requested"
if [ ! -f "$file" ]; then
    echo "outline: no existe $requested" >&2
    exit 1
fi
case "$file" in
    *.rs)        lang=rust ;;
    *.ts|*.tsx)  lang=ts ;;
    *)
        echo "outline: solo .rs, .ts y .tsx. Para el resto, grep -n" >&2
        exit 1 ;;
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
        printf "   Abre los tramos que necesites, TODOS EN UNA SOLA LLAMADA:\n"
        printf "     sed -n %cA,Bp;C,Dp%c %s\n", 39, 39, name
        printf "   Un turno por tramo sale mas caro que leer el fichero entero. --\n"
    }
}'
# Los dos caminos de error de arriba ya han salido con 1. Aqui solo queda el
# 141 de un SIGPIPE si alguien encadena un `head`, y eso no es un fallo.
exit 0
