# Mapa de `scripts/`

Los arneses que llaman las recetas del `justfile` (ADR-0013).

| Fichero | Qué es |
|---|---|
| `bootstrap.sh` | Instala en `~/.m2` las dependencias Java de AutoFirma que no están en Maven Central. |
| `outline.sh` | El esqueleto de un `.rs`, `.ts` o `.tsx`, para `just outline`. |
| `tools.sh` | Comprueba las herramientas del entorno con una tabla por plataforma, y falla nombrando la que falte o la que no esté en su versión fijada. |
| `install-tools.sh` | Instala las herramientas en la versión que fija `versions.env`, para `just install-tools`. |
| `pinned-version.sh` | Imprime una clave de `versions.env` para los arranques que no pasan por `just`. |
| `check-versions.sh` | La guarda de `versions.env`: ningún valor fijado escrito fuera de él y los pom con la misma AutoFirma. |
| `release.sh` | Publica una versión desde `main`: changelog, bump, commit, etiqueta y push atómico, para `just release`. |
| `changelog-release.sh` | Escribe en `CHANGELOG.md` la sección de una versión a partir de los títulos de PR desde la última etiqueta. |
| `bump-version.sh` | Sube la versión en los sitios del candado de `check-version.py`, para `just bump-version`. |
| `dev-handler.sh` | Registra o quita el binario de desarrollo como manejador de `afirma://`. |
| `isolated-store.sh` | Monta, para un cliente de la suite de conformidad y un almacén (`rsa`, `ec`, `token`, `token_apart`, `ed25519`, `several` o `expired`), su perfil de usar y tirar con su envoltorio y su raíz de confianza, sin lanzar el cliente. Lo llama la consola web de la suite al resolver el cliente, no una receta. |
| `check-glibc.sh` | Comprueba el suelo de glibc de la librería nativa. |
| `flatpak-sources.sh` | Regenera las fuentes de cargo vendorizadas del manifiesto flatpak y el sello de `Cargo.lock`. |
| `token-per-test.sh` | Envoltorio de nextest que da a cada proceso de prueba su propia copia del almacén de SoftHSM. Lo llama `.config/nextest.toml` de `rfirma-app/src-tauri`, no una receta. |
| `ci-lanes.sh` | Dice, a partir de los ficheros de un PR, qué carriles del CI tienen que correr. Lo llama el job `scope` de `ci.yml`, no una receta. |
| `packages-manifest.sh` | Escribe y lee el `paquetes.json` de una entrega: una fila por paquete con plataforma, fichero, formato, si es firmable y su aviso. Lo llama la receta `packages-manifest` y lo leen `build.yml`, `release.yml`, `publish.yml`, `preview.yml` y `packaging/check-digests.sh`. |
| `preview-comment.sh` | Compone el Markdown del comentario fijo de la preview de una PR a partir de los artefactos, el manifiesto y el contexto de la ejecución; no imprime nada si no hay nada que comentar. Lo llama `preview-comment.yml`. |
| `tests/outline_test.sh` | Prueba el esqueleto que produce `outline.sh` sobre los fixtures de `tests/fixtures/`. |
| `tests/ci_lanes_test.sh` | Prueba qué carriles enciende `ci-lanes.sh` para cada clase de fichero. |
| `tests/packages_manifest_test.sh` | Prueba el manifiesto de paquetes (entrega completa, candidata, extensión desconocida, firmable, plataformas) y la excepción de `check-digests.sh`. |
| `tests/preview_comment_test.sh` | Prueba el comentario de la preview: filas por artefacto, avisos del manifiesto, fallo, salida vacía y marcador en la primera línea. |
| `tests/check_versions_test.sh` | Prueba la guarda de `versions.env` sobre árboles temporales: literales, pom, formato y el fichero viejo de GraalVM. |
| `tests/check_workflows_test.sh` | Prueba la guarda de workflows sobre copias de `.github/`: secretos en `build.yml` y en las acciones que alcanza, y `save-cache` y `save-if` de la acción de preparación. |
