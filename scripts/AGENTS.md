# Mapa de `scripts/`

Los arneses que las recetas del `justfile` llaman por nombre, siguiendo el patrón de una receta reducida a una línea que invoca al script (ADR-0013).

| Fichero | Qué es |
|---|---|
| `bootstrap.sh` | Instala en `~/.m2` las dependencias Java de AutoFirma que no están en Maven Central. |
| `outline.sh` | El esqueleto de un `.rs`, `.ts` o `.tsx`, para `just outline`. |
| `tools.sh` | Comprueba las herramientas del entorno y falla nombrando la que falte. |
| `changelog-release.sh` | Reúne los fragmentos de `changelog.d/` en la sección de una versión de `CHANGELOG.md`. |
| `bump-version.sh` | Sube la versión en los sitios del candado de `check-version.py`, para `just bump-version`. |
| `dev-handler.sh` | Registra o quita el binario de desarrollo como manejador de `afirma://`. |
| `check-glibc.sh` | Comprueba el suelo de glibc de la librería nativa. |
| `flatpak-sources.sh` | Regenera las fuentes vendorizadas del manifiesto flatpak. |
| `check-native.sh` | Falla nombrando `just native` si la librería nativa no está construida. |
| `clean-coverage.sh` | Borra el árbol instrumentado y los informes de cobertura sueltos. |
| `protocol-map.py` | Genera el mapa del protocolo de AutoFirma contra la etiqueta fijada. |
| `tests/outline_test.sh` | Prueba el esqueleto que produce `outline.sh` sobre los fixtures de `tests/fixtures/`. |
