# Estructura del repositorio y cadena de compilación

Este repositorio es **políglota en la raíz**: un módulo Maven que produce la librería
nativa, una aplicación Tauri, y un empaquetado flatpak. La decisión de fondo es que
**la raíz no pertenece a ninguna de las tres cadenas de herramientas**: coordina, y
cada pieza lleva la suya dentro.

```
rfirma-native-bridge/   Maven -> GraalVM CE 25 -> librfirma_crypto.so (ADR-0004)
rfirma-app/             Tauri: src-tauri/ (Rust) + src/ (React)
packaging/flatpak/      manifiesto, generadores de fuentes y verificación
justfile                el único orquestador
```

`rfirma-app` no hereda el nombre de la promesa que arrastraba `AGENTS.md`: lo gana por
simetría con `rfirma-native-bridge`. Se descartó la disposición plana en la raíz
(`src/`, `src-tauri/`, `package.json` arriba, como
[tabularis](https://github.com/TabularisDB/tabularis)) porque allí JavaScript es la
única cadena de primer nivel y aquí no: con `package.json` y `pom.xml` de vecinos,
quien llega no puede saber qué manda.

## El frontend es React, no Svelte

`AGENTS.md`, `README.md` y `design-system.md` prometían **Svelte** desde el borrador
inicial. Se decide **React 19 + Vite + TypeScript**, con **pnpm** e **i18next**.

Para esta interfaz, escrita por una persona, Svelte sería la mejor elección. Sus dos
ventajas reales son concretas: el visor es interoperación **imperativa** sobre un
canvas de pdf.js, y el recuadro se arrastra a 60 fps. Pero las dos se pagan una vez y
caben en dos ficheros —cancelar la `RenderTask` en vuelo hay que escribirlo en
cualquier framework, porque lo exige el zoom, y el arrastre sale con una `ref` que
escribe `transform` y confirma el estado en `pointerup`—, mientras que el coste de
Svelte 5 con runas **se paga en cada sub-issue**: este código lo escriben agentes en
paralelo, con contexto limpio, y el modo de fallo característico es mezclar idioma de
Svelte 4 (`writable`, `$:`) dentro de un componente con runas, que a veces compila y
no reacciona. Equivocarse en React cuesta un repintado de más.

Los dos argumentos con los que suele decidirse esto **aquí no valen nada** y conviene
dejarlo escrito para que nadie los reabra: no hay tabla de datos, ni gráficas, ni
biblioteca de componentes, ni router —el ADR-0007 dejó un solo recorrido sin cambiar
de pantalla—, así que el ecosistema de React no cobra; y el tamaño del bundle, el
argumento de Svelte, no cuenta al lado de 36 MB de `.so`.

**Sin Tailwind**, que el `README.md` prometía: `design-system.md` ya es un sistema
completo en custom properties y clases `rf-*`, y añadir Tailwind sería tener dos
sistemas de color. El sistema de diseño es agnóstico de framework y **no se toca**.
De i18next se usa el mecanismo, no el olfateo: el idioma es una preferencia guardada
(ADR-0010), así que **no** entra `i18next-browser-languagedetector`.

## `just` orquesta; nadie llama a nadie por su cuenta

`build.rs` no invoca a Maven ni a `native-image` jamás; se queda en `tauri_build::build()`.
Un `cargo build` que dispare por sorpresa 1 m 22 s de `native-image` arruina el bucle de
realimentación que este repositorio decidió proteger en el
[issue #11](https://github.com/sgomez/rfirma/issues/11).

| receta | qué hace | carril |
| --- | --- | --- |
| `tools` | comprueba herramientas y nombra la que falte | rápido |
| `bootstrap` | `~/.m2` contra la etiqueta `v1.9.1` | rápido |
| `lint` | `-Xlint:all` + `biome` + `cargo clippy` + `cargo fmt --check` | rápido |
| `test` | `mvn test` + `vitest` + `cargo test` | rápido |
| `build` | puente Java + `tsc -b && vite build` + `cargo build` | rápido |
| `check` | `tools lint build test` | rápido |
| `native` | `librfirma_crypto.so` con GraalVM CE 25 | lento |
| `flatpak` | `flatpak-builder` sobre el manifiesto | lento |
| `flatpak-sources` | regenera `cargo-sources.json` y `node-sources.json` | a mano |
| `dev` | `RFIRMA_LIB_DIR` + `tauri dev` | ninguno |

> La casilla de `lint` decía `eslint`, escrito sin razonarlo y sin decir nada del formateador.
> El [ADR-0014](0014-gradas-de-prueba-y-puerta-de-calidad.md) lo sustituye por **Biome**, y es
> también quien decide **qué** se ejecuta dentro de `lint` y `test` y en qué carril cae cada
> prueba. Añade además la receta voluntaria `rapido` (solo `lint`).

**`check` es un contrato**: `docs/agents/code-host.md` promete que el CI ejecuta
exactamente `just check` y que un pase local significa lo mismo. Crece por dentro; su
nombre y su papel no. `tsc -b` va **dentro** de `build`, no en una receta aparte: un
`build` que compila TypeScript sin comprobar tipos miente sobre lo que ha comprobado.

`bootstrap.sh` **no crece**. Sigue resolviendo `~/.m2` y nada más. Instalar GraalVM,
`flatpak-builder` o el token de pruebas son cosas con `sudo`, SDKMAN o
`flatpak remote-add` que un script no debe hacer a espaldas de nadie ni dentro de un
runner; quien las comprueba es `just tools`, que ya falla nombrando lo que falta.

## La librería nativa en desarrollo

> Esta sección se escribió cuando la imagen eran **seis** ficheros. El
> [#36](https://github.com/sgomez/rfirma/issues/36) la dejó en **uno**,
> `librfirma_crypto.so`, al excluir `afirma-ui-utils` (ADR-0012), y el ADR-0004 quedó
> reescrito. Lo que sigue está corregido a esa cifra; el razonamiento no cambia.

Ruta canónica única: **`rfirma-native-bridge/target/lib/rfirma/`**. Hoy hay dos rivales
—`target/native`, que produce `just native`, y `target/ce25-noui`, al que apunta el
manifiesto flatpak—, o sea que **la receta no construye lo que se distribuye**. La receta
`native` pasa a producir la imagen buena, en la ruta canónica, y el manifiesto apunta ahí.

`dev` y `build` **comprueban** que la librería está y, si falta, **fallan nombrando
`just native`**; no lo encadenan. La comprobación de arranque que exige el ADR-0004 nombra
**las dos** rutas que miró: la relativa al ejecutable y `RFIRMA_LIB_DIR`.

Y la ruta de distribución **no es** el directorio de construcción: `native-image` sigue
emitiendo ahí los cinco auxiliares de AWT, así que un `install *.so` reintroduciría
`libawt.so` — y con él, el aborto del proceso ante un JPEG con perfil ICC que midió el #36.

En `tauri.conf.json`, **`bundle.active: false`**: el binario lo instala el manifiesto
flatpak, como ya hace la sonda. Los empaquetadores de Tauri (`.deb`, AppImage) están
descartados por el [#17](https://github.com/sgomez/rfirma/issues/17).

## Los metadatos de `native-image` se versionan

Van a `rfirma-native-bridge/src/main/resources/META-INF/native-image/`, donde
`native-image` los recoge del classpath sin bandera. Hoy viven solo en `target/`, así
que **desde un clon limpio la imagen que se distribuye no es reproducible**.

Regenerarlos con el agente de trazado es un acto **manual y explícito**, nunca parte de
la construcción: exigiría ejecutar el ciclo trifásico completo bajo la JVM dentro del
CI. Y el [#14](https://github.com/sgomez/rfirma/issues/14) midió que su contenido
depende de **qué formatos de imagen se declaren en construcción**, así que es contenido
revisable que tiene que verse en un diff.

## El flatpak se construye sin red

`cargo-sources.json` y `node-sources.json` se generan con `flatpak-cargo-generator.py`
y `flatpak-node-generator` —que soporta `pnpm-lock.yaml`, no solo npm y yarn—, se
**versionan** en `packaging/flatpak/`, y `--share=network` desaparece del manifiesto.
Se regeneran a mano con `just flatpak-sources` cuando cambia un fichero de bloqueo; el
CI **comprueba que están al día** en vez de regenerarlos, porque un fichero generado
dentro del CI es un fichero que nadie ha mirado.

Aquí se decía que quedaba **sin decidir** cómo entra la librería nativa en una
construcción apta para Flathub. Ya no aplica: el
[ADR-0015](0015-canal-de-distribucion-propio.md) deja la tienda fuera de v0.1, y con
canal propio el `type: dir` es válido. La regla de construir sin red **se mantiene**,
pero por la razón de la sección anterior y no por la de Flathub.

## La sonda se borra

`packaging/flatpak/probe/` ya contiene FFI con `libloading`, carga de la librería nativa,
ciclo trifásico y PKCS#11 con `cryptoki`. En cuanto exista el código real, eso son **dos
implementaciones de la misma frontera FFI**, y esa frontera es justo donde este proyecto
lleva tres hallazgos de fallo silencioso. Se borra en el mismo sub-issue que aporte el
FFI real, y el manifiesto pasa a empaquetar `rfirma-app`. `verifica.sh` sobrevive.

## Consequences

- El `README.md`, `AGENTS.md`, `design-system.md` y `docs/agents/prototyping.md` decían
  Svelte —y el `README.md`, además, Tailwind—. Eran promesas del borrador inicial, no
  decisiones; quedan corregidas con este ADR.
- `rfirma_development_spec.md` era borrador a auditar y **ya no existe**: lo borró el
  [#10](https://github.com/sgomez/rfirma/issues/10) al publicar el spec ejecutable
  [#46](https://github.com/sgomez/rfirma/issues/46), no este ADR.
- No hay workspace de Cargo en la raíz: hoy habría un solo miembro real. Se revisa si
  aparece un segundo crate.
