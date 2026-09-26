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
| `isolated-store.sh` | Monta, para un cliente de la suite de conformidad y un almacén (`rsa`, `ec`, `token`, `token_apart`, `ed25519`, `several` o `expired`), su perfil de usar y tirar con su envoltorio y su raíz de confianza, sin lanzar el cliente. Lo llama la consola web de la suite al resolver el cliente, no una receta. |
| `check-glibc.sh` | Comprueba el suelo de glibc de la librería nativa. |
| `flatpak-sources.sh` | Regenera las fuentes vendorizadas del manifiesto flatpak. |
| `check-native.sh` | Falla nombrando `just native` si la librería nativa no está construida. |
| `token-per-test.sh` | Envoltorio de nextest que da a cada proceso de prueba su propia copia del almacén de SoftHSM. Lo llama `.config/nextest.toml` de `rfirma-app/src-tauri`, no una receta. |
| `ci-lanes.sh` | Dice, a partir de los ficheros de un PR, qué carriles del CI tienen que correr. Lo llama el job `scope` de `ci.yml`, no una receta. |
| `clean-coverage.sh` | Borra el árbol instrumentado y los informes de cobertura sueltos. |
| `protocol-map.py` | Genera el mapa del protocolo de AutoFirma contra la etiqueta fijada. |
| `tests/outline_test.sh` | Prueba el esqueleto que produce `outline.sh` sobre los fixtures de `tests/fixtures/`. |
| `tests/ci_lanes_test.sh` | Prueba qué carriles enciende `ci-lanes.sh` para cada clase de fichero. |
