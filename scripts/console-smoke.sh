#!/usr/bin/env bash
# La prueba de humo del binario de consola de Windows (ADR-0041): `rfirma.com`, junto a un
# `rfirma.exe`, lanzado desde cmd, PowerShell 5.1, PowerShell 7 y Git Bash. Comprueba que el
# código de salida llega, que el error sale por stderr y que `rfirma` sin extensión resuelve a
# `rfirma.com` en cmd y en PowerShell.
#
# Uso: scripts/console-smoke.sh <carpeta con rfirma.exe y rfirma-console.exe>
set -euo pipefail

built="${1:?falta la carpeta con rfirma.exe y rfirma-console.exe}"
folder="$(mktemp -d)"
trap 'rm -rf "$folder"' EXIT
cp "$built/rfirma.exe" "$folder/rfirma.exe"
cp "$built/rfirma-console.exe" "$folder/rfirma.com"
windows_folder="$(cygpath -w "$folder")"
shells=(cmd powershell pwsh bash)
failures=0

fail() {
    echo "FALLO  $*" >&2
    failures=$((failures + 1))
}

# Lanza `rfirma.com <argumentos>` desde el shell dado y deja su salida en out, err y code.
launch() {
    local shell="$1"
    shift
    set +e
    case "$shell" in
        cmd) (cd "$folder" && MSYS_NO_PATHCONV=1 cmd /d /c ".\\rfirma.com $*") ;;
        powershell | pwsh)
            "$shell" -NoLogo -NoProfile -NonInteractive -Command \
                "Set-Location -LiteralPath '$windows_folder'; & .\\rfirma.com $*; exit \$LASTEXITCODE"
            ;;
        bash) (cd "$folder" && ./rfirma.com "$@") ;;
    esac >"$folder/out" 2>"$folder/err"
    code=$?
    set -e
}

for shell in "${shells[@]}"; do
    if ! command -v "$shell" >/dev/null; then
        fail "$shell: no esta en este equipo"
        continue
    fi
    launch "$shell" listaliases
    if [ "$code" -eq 0 ]; then
        echo "OK  $shell: listaliases termina con 0"
    else
        fail "$shell: listaliases termina con $code"
        cat "$folder/err" >&2
    fi

    launch "$shell" sign
    if [ "$code" -eq 0 ]; then
        fail "$shell: un sign mal formado termina con 0"
    elif ! grep -q 'rfirma: falta -i' "$folder/err"; then
        fail "$shell: un sign mal formado no dice por stderr que falta -i"
        cat "$folder/err" >&2
    elif [ -s "$folder/out" ]; then
        fail "$shell: un sign mal formado escribe en stdout"
        cat "$folder/out" >&2
    else
        echo "OK  $shell: un sign mal formado termina con $code y lo dice por stderr"
    fi
done

resolved_by_cmd="$( (PATH="$folder:$PATH" cmd //d //c "where rfirma" || true) | head -n 1 | tr -d '\r')"
case "$resolved_by_cmd" in
    *\\rfirma.com) echo "OK  cmd: rfirma resuelve a rfirma.com" ;;
    *) fail "cmd: rfirma resuelve a '$resolved_by_cmd'" ;;
esac

for shell in powershell pwsh; do
    command -v "$shell" >/dev/null || continue
    resolved="$("$shell" -NoLogo -NoProfile -NonInteractive -Command \
        "\$env:Path = '$windows_folder;' + \$env:Path; (Get-Command rfirma -CommandType Application | Select-Object -First 1).Source" |
        tr -d '\r')"
    case "$resolved" in
        *\\rfirma.com) echo "OK  $shell: rfirma resuelve a rfirma.com" ;;
        *) fail "$shell: rfirma resuelve a '$resolved'" ;;
    esac
done

if [ "$failures" -gt 0 ]; then
    echo "$failures comprobacion(es) del binario de consola fallaron" >&2
    exit 1
fi
