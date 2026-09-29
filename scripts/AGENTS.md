# Mapa de `scripts/`

Los arneses que las recetas del `justfile` llaman por nombre, siguiendo el patrón de una receta reducida a una línea que invoca al script (ADR-0013).

| Fichero | Qué es |
|---|---|
| `bootstrap.sh` | Instala en `~/.m2` las dependencias Java de AutoFirma que no están en Maven Central. |
| `outline.sh` | El esqueleto de un `.rs`, `.ts` o `.tsx`, para `just outline`. |
| `tools.sh` | Comprueba las herramientas del entorno y falla nombrando la que falte. |
| `release.sh` | Publica una versión desde `main`: changelog, bump, commit, etiqueta y push atómico, para `just release`. |
| `changelog-release.sh` | Escribe en `CHANGELOG.md` la sección de una versión a partir de los títulos de PR desde la última etiqueta. |
| `bump-version.sh` | Sube la versión en los sitios del candado de `check-version.py`, para `just bump-version`. |
| `dev-handler.sh` | Registra o quita el binario de desarrollo como manejador de `afirma://`. |
| `isolated-store.sh` | Monta, para un cliente de la suite de conformidad y un almacén (`rsa`, `ec`, `token`, `token_apart`, `ed25519`, `several` o `expired`), su perfil de usar y tirar con su envoltorio y su raíz de confianza, sin lanzar el cliente. Lo llama la consola web de la suite al resolver el cliente, no una receta. |
| `check-glibc.sh` | Comprueba el suelo de glibc de la librería nativa. |
| `flatpak-sources.sh` | Regenera las fuentes vendorizadas del manifiesto flatpak. |
| `check-native.sh` | Falla nombrando `just native` si la librería nativa no está construida. |
| `bundle-windows.sh` | Construye el instalador NSIS de Windows con el runtime de Visual C++ al lado, para `just bundle-windows`. |
| `pre-push-fmt.sh` | Comprueba el formato de una cadena para el `pre-push` de `lefthook.yml`, no una receta. |
| `token-per-test.sh` | Envoltorio de nextest que da a cada proceso de prueba su propia copia del almacén de SoftHSM. Lo llama `.config/nextest.toml` de `rfirma-app/src-tauri`, no una receta. |
| `ci-lanes.sh` | Dice, a partir de los ficheros de un PR, qué carriles del CI tienen que correr. Lo llama el job `scope` de `ci.yml`, no una receta. |
| `clean-coverage.sh` | Borra el árbol instrumentado y los informes de cobertura sueltos. |
| `protocol-map.py` | Genera el mapa del protocolo de AutoFirma contra la etiqueta fijada. |
| `packages-manifest.sh` | Escribe y lee el `paquetes.json` de una entrega: una fila por paquete con plataforma, fichero, formato, si es firmable y su aviso. Lo llama la receta `packages-manifest` y lo leen `build.yml`, `release.yml`, `publish.yml`, `preview.yml` y `packaging/check-digests.sh`. |
| `preview-comment.sh` | Compone el Markdown del comentario fijo de la preview de una PR a partir de los artefactos, el manifiesto y el contexto de la ejecución; no imprime nada si no hay nada que comentar. Lo llama `preview-comment.yml`. |
| `tests/outline_test.sh` | Prueba el esqueleto que produce `outline.sh` sobre los fixtures de `tests/fixtures/`. |
| `tests/ci_lanes_test.sh` | Prueba qué carriles enciende `ci-lanes.sh` para cada clase de fichero. |
| `tests/packages_manifest_test.sh` | Prueba el manifiesto de paquetes (entrega completa, candidata, extensión desconocida, firmable, plataformas) y la excepción de `check-digests.sh`. |
| `tests/preview_comment_test.sh` | Prueba el comentario de la preview: filas por artefacto, avisos del manifiesto, fallo, salida vacía y marcador en la primera línea. |
