#!/usr/bin/env bash
# PROTOTIPO #9 — sirve el visor en http://localhost:8099/
cd "$(dirname "$0")" && exec python3 -m http.server 8099
