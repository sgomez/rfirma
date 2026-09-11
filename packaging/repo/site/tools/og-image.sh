#!/usr/bin/env bash
# Rasteriza packaging/repo/site/public/og.png desde su plantilla.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
site="$here/.."
root="$site/../../.."
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cp "$here/og.html" "$work/"
cp "$root/rfirma-app/src/design-system/bundle/fonts/inter-latin.woff2" "$work/"
google-chrome --headless --disable-gpu --hide-scrollbars --allow-file-access-from-files \
    --force-device-scale-factor=1 --window-size=1200,630 \
    --screenshot="$work/og.png" "file://$work/og.html"
cp "$work/og.png" "$site/public/og.png"
