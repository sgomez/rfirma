#!/usr/bin/env bash
# PROTOTIPO #9 — firma y comprueba todos los casos de salidas/.
cd "$(dirname "$0")"
fallos=0; ok=0
for p in salidas/*.properties; do
  case "$p" in *sonda-*) continue;; esac
  ./firmar.sh "$p" >/dev/null 2>&1 || { echo "FALLO AL FIRMAR $p"; fallos=$((fallos+1)); continue; }
  if .venv-proto/bin/python comprobar.py "$p"; then ok=$((ok+1)); else fallos=$((fallos+1)); fi
  echo
done
echo "== $ok coinciden, $fallos fallan"
exit $((fallos > 0))
