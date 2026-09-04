# Estructura del repositorio y cadena de compilación

Este repositorio es **políglota en la raíz**: un módulo Maven que produce la librería
nativa, una aplicación Tauri, y un empaquetado flatpak. La decisión de fondo es que
**la raíz no pertenece a ninguna de las tres cadenas de herramientas**: coordina, y
cada pieza lleva la suya dentro.

```
rfirma-native-bridge/   Maven -> GraalVM CE 25 -> librfirma_crypto.so (ADR-0004)
rfirma-app/             Tauri: src-tauri/ (Rust) + src/ (React)
packaging/flatpak/      manifiesto, generadores de fuentes y verificación
packaging/repo/         la imagen nginx y la landing de rfirma.sgomez.me (ADR-0015)
packaging/verifica-contenido.sh   la invariante del ADR-0012 sobre cualquier artefacto
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
| `check` | `tools check-repo check-java check-ts check-rust` | rápido |
| `native` | `librfirma_crypto.so` con GraalVM CE 25 | lento |
| `flatpak` | `flatpak-builder` sobre el manifiesto | lento |
| `bundle` | `.deb` y `.rpm` con el *bundler* de Tauri | lento |
| `check-glibc` | el suelo `GLIBC_2.34` sobre lo que se va a publicar | lento |
| `flatpak-sources` | regenera `cargo-sources.json` y `node-sources.json` | a mano |
| `dev` | `RFIRMA_LIB_DIR` + `tauri dev` | ninguno |

> La casilla de `lint` decía `eslint`, escrito sin razonarlo y sin decir nada del formateador.
> El [ADR-0014](0014-gradas-de-prueba-y-puerta-de-calidad.md) lo sustituye por **Biome**, y es
> también quien decide **qué** se ejecuta dentro de `lint` y `test` y en qué carril cae cada
> prueba. Añade además la receta voluntaria `rapido` (solo `lint`).

> **Enmienda: `check` es un carril por cadena, no `tools lint build test`.** La forma
> original encadenaba las tres cadenas en una sola cola, y en el CI eso es una pared:
> el tiempo de pared era la suma. `check` pasa a ser `tools` más `check-repo`,
> `check-java`, `check-ts` y `check-rust`, y el workflow le da **un job a cada
> carril**, así que corren a la vez y la espera es la de la cadena más lenta. Lo que
> se comprueba no cambia salvo en dos puntos, ambos medidos y ambos deliberados:
> `cargo build --release` se muda al carril lento —nadie ejecutaba ese binario en el
> rápido y era un árbol de dependencias entero— y el `cargo test` suelto desaparece
> porque `cargo llvm-cov`, que la puerta CRAP ya arrastra, **ejecuta la suite él
> mismo**. `lint`, `build` y `test` siguen existiendo como atajos locales.
>
> `just check` bajó de ~45 s a ~31 s en el equipo de desarrollo, y el carril rápido
> del CI de 4 min 18 s a la duración de su cadena más lenta.

**`check` es un contrato**: `docs/agents/code-host.md` promete que el CI ejecuta
exactamente lo que `just check` ejecuta —hoy repartido en un job por carril— y que un
pase local significa lo mismo. Crece por dentro, y puede repartirse; su nombre y su
papel no cambian. `tsc -b` va **dentro** de `build`, no en una receta aparte: un
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

## El *bundler* de Tauri está encendido, y produce `deb` y `rpm`

`bundle.active: true`, `targets: ["deb", "rpm"]`. Estuvo apagado mientras el flatpak fue el
canal único, y el [ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md) lo revisó:
son tres canales. **En el flatpak nada cambia** — ahí el binario lo sigue instalando el
manifiesto, y el bundler no interviene.

Lo que se configura, medido sobre los fuentes de `tauri-bundler` 2.9.4 en el
[#228](https://github.com/sgomez/rfirma/issues/228):

- **La librería nativa entra por `bundle.linux.<formato>.files`**, con la clave absoluta
  (`/usr/lib/rfirma/librfirma_crypto.so`) y el origen relativo a `src-tauri/`. **No** por
  `bundle.resources`, que llega al mismo sitio sólo por la casualidad de que `productName`
  sea `rfirma`: la ruta la decide el ADR-0004, no `resource_dir()`.
- **No se declara ninguna dependencia de tarjeta.** Hasta el
  [#256](https://github.com/sgomez/rfirma/issues/256) el trío `opensc-pkcs11`,
  `libpcsclite1` y `pcscd` (Debian y Ubuntu) o `opensc-libs`, `pcsc-lite-libs` y
  `pcsc-lite` (Fedora) viajaba como `recommends`. Se retiró junto con la fontanería de
  tarjeta del flatpak: la fontanería nunca se había publicado, tarjetas y DNIe no están
  soportados en la v0.4, y `CANDIDATE_MODULES` (`pkcs11/stores.rs`) ya no lleva las rutas
  de OpenSC del anfitrión, así que recomendar el paquete no serviría de nada. **rfirma
  firma sin OpenSC** —quien tenga el certificado en Firefox, en `~/.pki/nssdb` o en un
  `.p12` no necesita ni el módulo ni el demonio—.
  `libwebkit2gtk-4.1-0` y `libgtk-3-0` **no se declaran**: los inyecta el bundler solo.
- **`compression: zstd` nivel 19 en el `.rpm`.** El `.deb` es gzip-6 y no es configurable;
  con 27,7 MB de `.so` dentro no hay razón para dejar en su valor de fábrica el único ajuste
  que sí se puede tocar.
- **La identidad de escritorio diverge entre canales, y es impuesta.** El bundler nombra el
  lanzador `<productName>.desktop`, y `desktopTemplate` sustituye el contenido, no el nombre:
  el flatpak está obligado a `me.sgomez.rfirma.desktop` y los nativos a `rfirma.desktop`. Se
  usa **un `desktopTemplate` compartido por deb y rpm** con el mismo contenido que el del
  flatpak, para que sólo diverja el nombre del fichero. Instalar el nuestro por `files`
  dejaría **dos** entradas en el menú.
- **No se envía `metainfo` en los paquetes nativos**, hoy: el bundler no instala nada de
  AppStream, y el metainfo actual declara un `<launchable>` que en un `.deb` no existe. Un
  metainfo que apunta a un lanzador inexistente es peor que ninguno. Quien lo necesita es el
  repositorio del [ADR-0015](0015-canal-de-distribucion-propio.md), y es él quien lo recoge.

**La invariante del ADR-0012 sale del sandbox a un script propio.**
`packaging/verifica-contenido.sh <artefacto>` acepta un `.deb`, un `.rpm` o el `files/` de
una construcción de flatpak, y afirma exactamente un `.so` bajo el directorio de la librería
y `libawt.so` en ninguna parte. `verifica.sh` pasa a llamarlo en vez de arrancar el sandbox
entero para comprobarlo, y el CI lo llama sobre cada artefacto **antes de subirlo**: es una
puerta, no un informe.

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
