#!/usr/bin/env bash
# Publica una versión desde main al día: changelog, bump, commit, etiqueta y push atómico.
set -euo pipefail

version="${1:?Uso: scripts/release.sh <version>}"
tag="v$version"
cd "$(dirname "$0")/.."

fail() { echo "$1" >&2; exit 1; }

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "La versión tiene que ser X.Y.Z: $version"
[ "$(git branch --show-current)" = main ] || fail "Se publica desde main."
[ -z "$(git status --porcelain)" ] || fail "El árbol tiene cambios sin comitear."

git fetch --quiet origin main
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || fail "main no coincide con origin/main."
if git rev-parse --quiet --verify "refs/tags/$tag" >/dev/null; then
    fail "La etiqueta $tag ya existe en local."
fi
if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null; then
    fail "La etiqueta $tag ya existe en origin."
fi

scripts/changelog-release.sh "$version"
scripts/bump-version.sh "$version"

git add -A
git commit --quiet -m "chore: version $version"
git tag "$tag"

echo
git show --stat --format='%h %s' HEAD
echo
read -r -p "¿Subir main y $tag a origin? [s/N] " answer
if [ "$answer" = s ] || [ "$answer" = S ]; then
    git push --atomic origin main "$tag"
else
    echo "Sin subir. Para publicar: git push --atomic origin main $tag"
    echo "Para deshacer: git tag -d $tag && git reset --hard origin/main"
fi
