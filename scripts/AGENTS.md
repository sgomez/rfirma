# Mapa de `scripts/`

Los arneses que llaman las recetas del `justfile` (ADR-0013).

| Fichero | Qué es |
|---|---|
| `bootstrap.sh` | Instala en `~/.m2` las dependencias Java de AutoFirma que no están en Maven Central. |
| `structural-guards.sh` | Las pruebas de grada A del backend que solo leen el árbol (tamaño, mapas, citas de ADR, capas), ejecutadas sin compilar la crate. Lo llaman el commit y el pre-push por `just structural-guards`. |
| `outline.sh` | El esqueleto de ficheros `.rs`, `.ts` o `.tsx`, los tramos numerados de cualquier fichero y el índice de los módulos `.rs`, `.ts`, `.tsx` y `.java` de un directorio por su cabecera `//!`, para `just outline`. |
| `tools.sh` | Comprueba las herramientas del entorno con una tabla por plataforma, y falla nombrando la que falte o la que no esté en su versión fijada. |
| `install-tools.sh` | Instala las herramientas en la versión que fija `versions.env`, para `just install-tools`. |
| `pinned-version.sh` | Imprime una clave de `versions.env` para los arranques que no pasan por `just`. |
| `check-versions.sh` | La guarda de `versions.env`: ningún valor fijado escrito fuera de él y los pom con la misma AutoFirma. |
| `app_version.py` | La versión de la aplicación con tres verbos: `bump` la sube en todos sus sitios, `check` comprueba que cuadran y `changelog` imprime la sección de una versión. No es la guarda de las herramientas fijadas. |
| `dev-handler.sh` | Registra o quita el binario de desarrollo como manejador de `afirma://`. |
| `isolated-store.sh` | Monta, para un cliente de la suite de conformidad y un almacén (`rsa`, `ec`, `token`, `token_apart`, `ed25519`, `several` o `expired`), su perfil de usar y tirar con su envoltorio y su raíz de confianza, sin lanzar el cliente. Lo llama la consola web de la suite al resolver el cliente, no una receta. |
| `check-glibc.sh` | Comprueba el suelo de glibc de la librería nativa. |
| `flatpak-sources.sh` | Regenera las fuentes de cargo vendorizadas del manifiesto flatpak y el sello de `Cargo.lock`. |
| `design_sync_selection.py` | Deriva de los títulos y del `parameters.designSync` de las historias la entrada de design-sync, el `titleMap`, los `overrides`, la tabla de catálogo de `conventions.md` (entre marcadores) (`just design-sync-selection`, antes de cada `/design-sync`; el CI no la verifica). Falla nombrando la historia con capa desconocida, título fuera de convención o componente sin mapear. |
| `token-per-test.sh` | Ejecuta el comando que recibe con una copia privada del almacén de SoftHSM (directorio temporal con su `SOFTHSM2_CONF`, borrado al salir aunque falle), copiada bajo el cerrojo que toma `testdata/softhsm/certs.sh`. Lo llama la receta `coverage` y, como envoltorio de nextest, `.config/nextest.toml` de `rfirma-app/src-tauri`. |
| `ci-lanes.sh` | Dice, a partir de los ficheros de un PR o de un push a `main`, qué carriles del CI tienen que correr, los de Linux y los de Windows y macOS. Lo llama el job `scope` de `ci.yml`, no una receta. |
| `platform-files.sh` | Lista los `.rs` de `rfirma-app/src-tauri` con un `cfg` de plataforma y los módulos que ese `cfg` declara, para `ci-lanes.sh`. Lo llama el job `scope` de `ci.yml`. |
| `main-moved.sh` | Dice, a partir de las ejecuciones de un workflow, si `main` se ha movido desde la anterior. Lo llaman las puertas de `nightly.yml` y `warm-release-cache.yml`. |
| `packages-manifest.sh` | Escribe y lee el `paquetes.json` de una entrega: una fila por paquete con plataforma, fichero, formato, si es firmable y su aviso; también genera las notas del borrador de la Release. Lo llama la receta `packages-manifest` y lo leen `release.yml`, `preview.yml`, `packaging/verify-packages.sh` y `packaging/repo/`. |
| `preview-comment.sh` | Compone el Markdown del comentario fijo de la preview de una PR a partir de los artefactos, el manifiesto y el contexto de la ejecución; no imprime nada si no hay nada que comentar. Lo llama `preview-comment.yml`. |
| `tests/outline_test.sh` | Prueba el esqueleto, los tramos y el índice que produce `outline.sh` sobre los fixtures de `tests/fixtures/`. |
| `tests/ci_lanes_test.sh` | Prueba qué carriles enciende `ci-lanes.sh` para cada clase de fichero, con la lista de plataforma, en un push y con etiquetas. |
| `tests/platform_files_test.sh` | Prueba `platform-files.sh` sobre un árbol temporal y, contra el repositorio, que el adaptador CNG cuenta como de Windows. |
| `tests/main_moved_test.sh` | Prueba `main-moved.sh` con respuestas de la API hechas a mano: primera ejecución, sin cambios, canceladas, otro título y API caída. |
| `tests/packages_manifest_test.sh` | Prueba el manifiesto de paquetes (entrega completa, candidata, extensión desconocida, firmable, plataformas). |
| `tests/preview_comment_test.sh` | Prueba el comentario de la preview: filas por artefacto, avisos del manifiesto, fallo, salida vacía y marcador en la primera línea. |
| `tests/check_versions_test.sh` | Prueba la guarda de `versions.env` sobre árboles temporales: literales, pom, formato y el fichero viejo de GraalVM. |
| `tests/test_app_version.py` | Prueba los tres verbos de `app_version.py` sobre un árbol temporal con su propio git. |
| `tests/test_design_sync_selection.py` | Prueba la selección sobre árboles de historias temporales: capas publicables, pantallas, errores de título y componentes de `testing/`. |
| `tests/check_workflows_test.sh` | Prueba la guarda de workflows sobre copias de `.github/`: rompe cada invariante de una forma y espera el mensaje. |
