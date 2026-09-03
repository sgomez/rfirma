# Cómo entran los seis `.so` en una construcción apta para Flathub

Investigación para el issue [#37](https://github.com/sgomez/rfirma/issues/37). El
[#22](https://github.com/sgomez/rfirma/issues/22) eligió Flathub como destino y dio por bueno un
módulo `type: dir` sin contrastarlo con las reglas de la tienda; el
[ADR-0013](../adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md) dejó el punto abierto
a propósito. Aquí se contrasta.

**No hay medición: no se ha construido nada.** Esto es lectura de reglas publicadas, del código del
linter oficial y de manifiestos vivos de la organización `flathub`. Todo lo afirmado lleva enlace;
lo que no he podido verificar está marcado como tal y agrupado al final.

Consultado el 31 de agosto de 2026.

## Respuesta corta

**Tal como está hoy, el manifiesto no entra: `type: dir` es un error duro del linter oficial de
Flathub**, no un aviso ni una recomendación
([`modules.py`](https://github.com/flathub/flatpak-builder-lint/blob/df46a15879db1091640941cae6f75045660c0309/flatpak_builder_lint/checks/modules.py#L65-L66)).

**Un archivo de release con `sha256` tampoco entra por la regla escrita.** La regla de construir
desde el código fuente alcanza explícitamente a «cualquier dependencia de ejecución incluida en el
manifiesto», que es exactamente lo que son los seis `.so`. Existe una cláusula de excepción, pero su
propio texto excluye «herramientas de nicho o configuraciones de construcción poco habituales» —una
descripción bastante exacta de GraalVM `native-image` sobre el árbol Maven de AutoFirma—. Hay
precedentes vivos que hacen justo eso (TuxGuitar, Ghidra), pero son anteriores a la política actual
y la propia política dice que no se aplica retroactivamente, así que **no demuestran que una
submission nueva pasaría**.

**Construir los seis dentro del sandbox es caro, no imposible.** No hay extensión de SDK de GraalVM
(comprobado: cero repositorios y cero coincidencias de código en la organización `flathub`), así que
GraalVM CE entraría como tarball prebuilt de 352 MB; y no hay generador oficial de Maven en
`flatpak-builder-tools`, así que las ~790 entradas de `~/.m2` habría que vendorizarlas con un guion
propio, sabiendo que las 36 de `es.gob.afirma` no están en Maven Central y hay que compilarlas desde
git dentro del sandbox.

**Y hay un obstáculo mayor que los seis `.so`, que el ticket no contemplaba: la política de IA
generativa de Flathub.** Prohíbe expresamente las aplicaciones «que contengan código, documentación
o cualquier otro contenido generado o asistido por IA», y prohíbe que el propio pull request de
submission lo abra o lo redacte un agente. Este repositorio lo escriben agentes. Bajo el texto
vigente, **rfirma no es admisible en Flathub**, independientemente de cómo entren los `.so`. Ver
la [sección 6](#6-el-obstáculo-que-el-ticket-no-vio-la-política-de-ia-generativa).

Para v0.1, el **bundle** sigue siendo el plan y cumple el criterio de terminado del
[#10](https://github.com/sgomez/rfirma/issues/10). Flathub se aplaza; el hito no.

## 1. El archivo de release con `sha256`

### Qué regla lo alcanza, y cuál no

Hay dos reglas que suenan parecidas y **dicen cosas distintas**. Conviene no confundirlas, porque el
ticket las mezcla.

La primera es sobre **el contenido del pull request**, no sobre las fuentes que el manifiesto se
descarga ([Required files](https://docs.flathub.org/docs/for-app-authors/requirements#required-files)):

> Under no circumstances should source code nor build artifacts be included in the submission.
> Flathub is not intended to host neither application source code nor binaries, including that of
> any dependencies.

Y, en el mismo sentido
([No network access during build](https://docs.flathub.org/docs/for-app-authors/requirements#no-network-access-during-build)):

> Binary or precompiled files must not be present in the submission pull request.

Un archivo alojado en una release de este repositorio y referido por URL **no está en el pull
request**, así que estas dos no lo prohíben.

La que sí lo alcanza es
[Building from source](https://docs.flathub.org/docs/for-app-authors/requirements#building-from-source):

> All source available submissions must be built entirely from source code. This requirement applies
> to the main application component defined in the manifest, as well as any runtime dependencies
> included in the manifest.

Los seis `.so` son una dependencia de ejecución del componente principal, incluida en el manifiesto.
Están dentro del alcance textual. **La regla escrita los prohíbe.**

### La excepción, y por qué apunta en nuestra contra

El párrafo siguiente de la misma sección:

> Exceptions may be granted to well-known vendors on a case-by-case basis, for example where the
> necessary tooling to perform an offline source build is not available. However, the use of niche
> tooling or uncommon build setups may not constitute grounds for an exception.

Leído contra nuestro caso, la excepción tiene una mitad a favor y otra en contra:

- **A favor:** «where the necessary tooling to perform an offline source build is not available» es
  literalmente nuestra situación —no existe extensión de SDK de GraalVM ni generador de fuentes
  Maven (secciones 2 y 3)—.
- **En contra:** «niche tooling or uncommon build setups may not constitute grounds» describe
  exactamente `native-image` sobre un árbol Maven de un cliente de firma del Gobierno de España. Y
  «well-known vendors» no es rfirma.

La misma sección añade un empujón que no es una regla pero sí una expectativa:

> When tooling for an offline source build is unavailable, application developers are generally
> encouraged (though not strictly required) to help develop, contribute, or upstream the necessary
> tooling for their app and for the benefit of the wider Flatpak ecosystem.

O sea: la respuesta esperada a «no hay generador de Maven» es **escribir el generador de Maven**, no
subir el binario.

### Las condiciones exactas que pregunta el ticket: no están escritas

El ticket pregunta por «etiqueta inmutable, reproducibilidad, que la receta viva en el repositorio».
**No he encontrado ninguna regla publicada de Flathub que enumere esas condiciones.** No están en
`requirements`, ni en `submission`, ni en la plantilla de pull request. Lo que sí está, y es lo único
mecánicamente exigido sobre inmutabilidad, vive en el código del linter
([`modules.py`](https://github.com/flathub/flatpak-builder-lint/blob/df46a15879db1091640941cae6f75045660c0309/flatpak_builder_lint/checks/modules.py)):

| Comprobación | Efecto |
|---|---|
| `sha1` en una fuente `archive`/`file` | error `source-sha1-deprecated` |
| `md5` en una fuente `archive`/`file` | error `source-md5-deprecated` |
| `tag` de git sin `commit` | error en el pipeline de submission nueva, aviso en el resto |
| `x-checker-data` con `commit-query` | error `checker-tracks-commits` |

Es decir: para un `archive`, la inmutabilidad se exige por **`sha256`** y nada más. No hay
comprobación de reproducibilidad, ni de que la receta esté publicada.

### Precedentes vivos que hacen exactamente esto

Tres, todos comprobados hoy en la organización `flathub`:

1. **[TuxGuitar](https://github.com/flathub/ar.com.tuxguitar.TuxGuitar/blob/master/ar.com.tuxguitar.TuxGuitar.yml)**
   es el caso idéntico al nuestro. No compila nada: descarga
   `tuxguitar-2.0.1-linux-swt-amd64.tar.gz` de la release de GitHub del propio upstream, con
   `sha256` y `only-arches: [x86_64]`, y lo descomprime en `${FLATPAK_DEST}`. Sigue vivo: actualizado
   a 2.0.1 en febrero de 2026 y al runtime 25.08 en octubre de 2025.
2. **[Ghidra](https://github.com/flathub/org.ghidra_sre.Ghidra/blob/master/org.ghidra_sre.Ghidra.yaml)**
   sí compila la aplicación desde git, pero mete **librerías compartidas precompiladas de terceros**:
   `z3-4.13.0-x64-glibc-2.31.zip` y su equivalente aarch64, de cuyo contenido copia `libz3*.so`
   directamente al árbol de dependencias. Un `.so` prebuilt bajado por URL con `sha256`.
3. **La propia extensión
   [openjdk25](https://github.com/flathub/org.freedesktop.Sdk.Extension.openjdk25/blob/branch/25.08/org.freedesktop.Sdk.Extension.openjdk25.yaml)**
   arranca de un JDK Temurin **binario** (`OpenJDK25U-jdk_x64_linux_hotspot_25_36.tar.gz`) como boot
   JDK, y empaqueta Maven 3.9.16 desde el tarball binario oficial de Apache.

### Por qué los precedentes no bastan

La propia política se blinda contra este argumento
([Inclusion policy](https://docs.flathub.org/docs/for-app-authors/requirements#inclusion-policy)):

> These policies may evolve over time, but new policies will not be retroactively applied to
> submissions predating the introduction of the policy.

Traducido: TuxGuitar y Ghidra pueden estar dentro por antigüedad. Que existan **no** prueba que una
submission nueva con la misma forma pase la revisión de hoy. Y la misma página avisa de que cada
caso se decide a mano: «Each application is evaluated on a case-by-case basis».

**Conclusión de la pregunta 1:** por la regla escrita, no. Por precedente, es la única vía con forma
de funcionar, pero depende de una excepción discrecional cuyo texto menciona expresamente el motivo
por el que nos la podrían negar. No he encontrado ningún precedente **posterior a la política** de
una aplicación que suba su propio artefacto binario de release; no lo he podido buscar de forma
exhaustiva.

## 2. Precedentes Java y GraalVM

### Java: la vía está pavimentada

Existen ocho extensiones de SDK de OpenJDK en la organización `flathub`, comprobado hoy:
`openjdk`, `openjdk8`, `openjdk9`, `openjdk10`, `openjdk11`, `openjdk17`, `openjdk21` y
[`openjdk25`](https://github.com/flathub/org.freedesktop.Sdk.Extension.openjdk25). El patrón es
uniforme y se ve entero en Ghidra:

```yaml
sdk-extensions:
  - org.freedesktop.Sdk.Extension.openjdk21
modules:
  - name: openjdk
    buildsystem: simple
    build-commands:
      - /usr/lib/sdk/openjdk21/installjdk.sh
```

Y la compilación real, dentro del sandbox, con las dependencias vendorizadas:

```yaml
      - source /usr/lib/sdk/openjdk21/enable.sh && gradle buildGhidra
```

Las dependencias de Gradle van en dos ficheros generados y versionados en el repositorio de la
submission: `gradle-dependencies.json` (91.553 bytes) y `gradle-fetched-dependencies.json`
(19.129 bytes), producidos por
[`flatpak-gradle-generator.py`](https://github.com/flatpak/flatpak-builder-tools/tree/master/gradle)
según su
[`generate-deps.sh`](https://github.com/flathub/org.ghidra_sre.Ghidra/blob/master/generate-deps.sh).
Es exactamente el mismo mecanismo que el ADR-0013 ya adoptó para cargo y pnpm.

Dato útil que no esperaba: **la extensión `openjdk25` trae Maven 3.9.16** dentro
(`$FLATPAK_DEST/maven/bin/mvn`, enlazado en `bin`). O sea que `mvn` **existe** dentro del sandbox de
construcción sin trabajo adicional. Lo que no existe es la forma de rellenar `~/.m2` sin red.

### GraalVM: no existe extensión. Dicho explícitamente.

Comprobado con cuatro búsquedas sobre la organización `flathub` de GitHub, todas con cero
resultados:

| Búsqueda | Resultados |
|---|---|
| repositorios con `graalvm` en el nombre | 0 |
| código con `graalvm` | 0 |
| código con `native-image` | 0 |
| código con `GRAALVM_HOME` | 0 |

**No hay ninguna extensión de SDK de GraalVM en Flathub, y no hay ninguna aplicación en Flathub que
use `native-image`.** Si rfirma se publicase construyendo dentro del sandbox, sería la primera. Esto
es una acotación de búsqueda por la API de código de GitHub, que solo indexa las ramas por omisión;
no es una prueba formal de inexistencia, pero cuatro ceros sobre una organización de este tamaño es
bastante concluyente.

## 3. Qué costaría construir los seis dentro del sandbox

Tres bloques, y ninguno es un muro; los tres son caros.

### 3.1 GraalVM CE 25

No hay extensión (sección 2), así que entra como fuente `archive`. El tarball oficial existe, es
público y publica su `sha256`
([`graal-25.3.4.1`](https://github.com/graalvm/graalvm-ce-builds/releases/tag/graal-25.3.4.1)):

| Asset | Tamaño |
|---|---|
| `graalvm-community-jdk-25i3-25.0.4.1_linux-x64_bin.tar.gz` | 351.842.204 B (352 MB) |

Es un binario precompilado, con el mismo estatus que el boot JDK de Temurin que usa la extensión
`openjdk25`: **herramienta de construcción**, no dependencia de ejecución, así que queda fuera del
alcance literal de la regla de la sección 1 —que habla de «main application component» y «runtime
dependencies»—. Esa lectura es mía y **no la he podido confirmar contra ninguna regla escrita ni
contra ningún precedente de una aplicación**; el precedente que sí existe es el de una extensión de
SDK, que es otra cosa.

Construir GraalVM CE desde fuente dentro del sandbox no lo he dimensionado: exige `mx`, un JDK de
arranque y un árbol de fuentes propio. Doy por hecho que está fuera de discusión, pero es una
suposición.

### 3.2 AutoFirma vendorizado

`bootstrap.sh` clona `ctt-gob-es/clienteafirma` en la etiqueta `v1.9.1` y ejecuta
`mvn clean install -DskipTests` sobre el cliente oficial **entero**. Dentro del sandbox eso se
traduce en una fuente `type: git` con `tag` **y** `commit` (el linter exige los dos en una submission
nueva, sección 1) y un módulo que compila el árbol completo.

Medido sobre `~/.m2/repository` de esta máquina, que es lo que ese `bootstrap.sh` produjo:

| Magnitud | Valor |
|---|---|
| Tamaño total de `~/.m2/repository` | **91 MB** |
| Ficheros `.jar` + `.pom` | **832** |
| Ficheros `.pom` | 549 |
| `.jar` de `es.gob.afirma` | 36 (9,2 MB) |
| `.pom` de `es.gob.afirma` | 39 |

Restando lo que produce el propio AutoFirma, quedan **~790 artefactos de terceros** que habría que
declarar uno a uno como fuentes `type: file` con `url` y `sha256`. Esa cifra es una **cota superior
holgada**: el repositorio local de esta máquina incluye plugins de Maven y dependencias de otros
proyectos, y no he separado unos de otros.

### 3.3 No hay generador oficial de Maven

Comprobado hoy en
[`flatpak/flatpak-builder-tools`](https://github.com/flatpak/flatpak-builder-tools): hay
generadores para `cargo`, `cpan`, `deno`, `dotnet`, `dub`, `go-get`, `go-modules`, **`gradle`**,
`node`, `npm`, `opam`, `pip`, `poetry`, `rubygems`, `spm` y `yarn`. **No hay `maven`.**

- El PR [#253](https://github.com/flatpak/flatpak-builder-tools/pull/253), «Add maven generator
  script», se **cerró sin fusionar** el 9 de mayo de 2025.
- El issue [#487](https://github.com/flatpak/flatpak-builder-tools/issues/487), «script for
  vendoring maven dependencies from pom.xml», sigue **abierto**.

Lo que hay es artesanía. El hilo
[flatpak-builder#58](https://github.com/flatpak/flatpak-builder/issues/58) recoge la receta que usan
las aplicaciones Maven que sí están en Flathub: construir una vez con red fuera del pipeline,
recorrer el `~/.m2` resultante y emitir un `type: file` por artefacto con su `sha256`. Con un aviso
explícito en el propio hilo:

> **There is a caveat,** though: This will not work with dependencies that _are not_ in
> `repo.maven.apache.org`, which means you'll have to adjust them manually

Que es justo nuestro caso: los 36 `.jar` de `es.gob.afirma` no están en Central; salen de compilar
el clon. Habría que compilarlos dentro del sandbox (3.2) e instalarlos en el `~/.m2` local antes de
compilar el puente.

Escala del fichero resultante, con dos referencias reales:

| Aplicación | Fichero de dependencias | Tamaño |
|---|---|---|
| [`es.estoes.wallpaperDownloader`](https://github.com/flathub/es.estoes.wallpaperDownloader) | `maven-dependencies.yaml` | 129.799 B |
| [`org.ghidra_sre.Ghidra`](https://github.com/flathub/org.ghidra_sre.Ghidra) | `gradle-dependencies.json` | 91.553 B |

### 3.4 Veredicto de coste

**Caro, no imposible.** Ninguna de las tres piezas es un muro:

- GraalVM entra como tarball de 352 MB con `sha256` (con la salvedad de la sección 3.1).
- AutoFirma entra como `type: git` con `tag` + `commit`.
- Las ~790 dependencias entran como fichero generado y versionado, con un guion propio de unas
  quince líneas, del mismo tipo que los `cargo-sources.json` y `node-sources.json` que el ADR-0013 ya
  decidió.

El precio: un fichero generado de ~130 KB que hay que regenerar en cada subida de versión de
AutoFirma, un guion que hay que escribir y mantener porque no existe upstream, y un carril lento que
pasa de «copiar 35 MB» a «compilar el cliente oficial de AutoFirma entero más una imagen nativa».

Y una **precondición dura** que ya estaba escrita: el ADR-0013 exige versionar los metadatos de
`native-image`, porque «desde un clon limpio la imagen que se distribuye no es reproducible». Si el
sandbox construye la imagen, eso deja de ser higiene y pasa a ser requisito de que el módulo
compile.

## 4. Qué invalida `type: dir`, y qué no

### Lo que sí

**El linter oficial lo rechaza como error duro cuando corre en el pipeline de Flathub.** Del código,
no de la documentación
([`modules.py`](https://github.com/flathub/flatpak-builder-lint/blob/df46a15879db1091640941cae6f75045660c0309/flatpak_builder_lint/checks/modules.py#L65-L66)):

```python
if source_type == "dir" and config.is_flathub_pipeline():
    self.errors.add(f"module-{module_name}-source-dir-not-allowed")
```

Y la documentación de flatpak-builder lo dice sin rodeos
([Module Sources](https://docs.flatpak.org/en/latest/module-sources.html)):

> When submitting an application to software stores like Flathub, `dir` should be avoided
> altogether.

La misma página añade un coste que nos afecta **hoy**, con Flathub o sin él:

> These don't support any caching, so it will be rebuilt each time the application is being built.

O sea que cada `just flatpak` recopia los 35 MB y rehace el módulo `libreria-nativa` entero. No es
grave, pero es medible y explica parte del tiempo del carril lento.

### Lo que no

El ticket sospechaba de `flatpak-builder --sandbox`. **No es eso.** Del
[Flatpak Builder Command Reference](https://docs.flatpak.org/en/latest/flatpak-builder-command-reference.html):

> `--sandbox`: Disable the possibility to specify build-args that are passed to flatpak build. This
> means the build process can't break out of its sandbox, and is useful when building less trusted
> software.

`--sandbox` no toca las fuentes: lo que mata es `build-args`. Lo que rompería en nuestro manifiesto
es el `--share=network` del módulo `sonda`, que ya está condenado por otra vía —el linter lo marca
como error `module-<n>-build-network-access` en el pipeline de construcción de Flathub, en el mismo
fichero— y que el ADR-0013 ya decidió quitar.

`--disable-download` tampoco:

> Don't download any sources. This only works if some version of all sources are downloaded already.
> [...] However, the build will fail if some source is not locally available.

Un `type: dir` **es** local, así que sobrevive intacto a `--disable-download`. Es más: hoy es lo
único del manifiesto que ya está garantizado sin red.

`--show-deps` («List all the (local) files that the manifest depends on») tampoco es un problema:
enumera precisamente los ficheros locales, incluidos los del `type: dir`.

### La ruta `../../`, que nadie prohíbe

La documentación dice que `path` «should be the path of the local directory relative to the manifest
root path». Nuestro `path: ../../rfirma-native-bridge/target/ce25-awt` **sale** del directorio del
manifiesto. Busqué en el código de `flatpak/flatpak-builder` una comprobación que lo impida —cadenas
como «outside of the manifest» o «Local file path»— y **no encontré ninguna**. Funciona hoy, y lo
que lo rechaza es el linter de Flathub por ser `dir`, no flatpak-builder por la ruta.

**Conclusión de la pregunta 4:** mientras el destino sea el bundle o la construcción en casa,
`type: dir` es perfectamente legal y solo cuesta la ausencia de caché. Lo único que invalida es la
publicación en Flathub.

## 5. Plan B: qué es un bundle y qué pierde

`flatpak build-bundle` empaqueta en un fichero una aplicación **que ya está en un repositorio local**
([Single-file bundles](https://docs.flatpak.org/en/latest/single-file-bundles.html),
[referencia](https://docs.flatpak.org/en/latest/flatpak-command-reference.html)):

```
flatpak build-bundle [OPTION...] LOCATION FILENAME NAME [BRANCH]
```

> Unless `--oci` is used, the format of the bundle file is that of an ostree static delta (against
> an empty base) with some flatpak specific metadata for the application icons and appdata.

Se instala directamente con `flatpak install fichero.flatpak`. Lo que pierde frente a Flathub:

- **Actualizaciones.** Un bundle es una foto fija. La única forma de que el usuario reciba la
  siguiente versión es `--repo-url=URL`, que «Installing the bundle will automatically configure a
  remote for this URL» —es decir, obliga a **alojar un repositorio propio**, que es exactamente la
  fricción que el #22 rechazó al elegir Flathub sobre un repositorio propio—.
- **Las dependencias no van dentro.** La documentación avisa: «single-file bundles don't include
  dependencies or AppStream data». `org.gnome.Platform` no viaja en el bundle; hay que apuntar
  `--runtime-repo=` al `.flatpakrepo` de Flathub y el usuario acaba necesitando el remoto de Flathub
  igualmente. Ironía útil: el plan B depende de Flathub para el runtime aunque no publique en él.
- **Firma y confianza.** El bundle admite `--gpg-keys=FILE` y hay `flatpak build-sign`, pero la clave
  es nuestra y no la avala nadie. En Flathub la firma y el alojamiento los pone la tienda. Un bundle
  que se descarga de una release de GitHub tiene, para el usuario, el modelo de confianza de un
  `.exe`.
- **Descubribilidad.** Sin AppStream en el bundle no hay ficha, ni búsqueda, ni «Instalar» desde el
  centro de software.

Para una aplicación de firma electrónica, el punto de la confianza no es menor y conviene que el
spec lo diga en voz alta.

## 6. El obstáculo que el ticket no vio: la política de IA generativa

Esto no lo pedía el issue y es, con diferencia, el hallazgo más caro. De
[Generative AI policy](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy),
citado literal:

> This policy applies to both the application being submitted to Flathub and the Flathub submission
> itself, including the manifest, metadata, patches, build scripts, and pull request.

> Submission pull requests must not be generated, opened, or automated using AI tools or agents.
> Review comments, reply, descriptions also must not be be LLM-generated

> Applications containing AI-generated or AI-assisted code, documentation, or any other content are
> not allowed.

> Repeatedly violating these policies may result in a permanent ban from future submissions and
> activities.

Y la única puerta:

> Exceptions may be granted for mature, well-maintained projects.

La página de [Submission](https://docs.flathub.org/docs/for-app-authors/submission) lo repite y le
pone dientes: «Pull requests that [...] contain excessive AI-generated content may be closed without
a review». La
[plantilla de PR](https://github.com/flathub/flathub/blob/master/.github/pull_request_template.md)
obliga a marcar «I have read and followed all the Submission requirements [...] and I agree to
them».

rfirma se escribe con agentes: el código, los ADR, este mismo fichero. Bajo el texto vigente eso lo
sitúa fuera, y la excepción disponible —«mature, well-maintained projects»— tampoco encaja con un
proyecto en v0.1, que además choca con
[Insufficient development history](https://docs.flathub.org/docs/for-app-authors/requirements#insufficient-development-history):
«Applications that have only existed for a very short period of time will generally not be accepted».

No es una interpretación torcida ni una lectura pesimista: es lo que dice el texto. **Si Flathub
sigue siendo el destino declarado, esto hay que decidirlo antes que la forma del módulo
`libreria-nativa`**, porque decide si la pregunta del #37 llega a importar.

## Qué cambia

**En el manifiesto** (`packaging/flatpak/me.sgomez.rfirma.yml`):

- El módulo `libreria-nativa` con `type: dir` **se queda tal cual mientras el destino sea el bundle**.
  Es legal, funciona y su único coste es que no cachea. No hay que tocarlo por precaución.
- Si algún día se apunta a Flathub, `type: dir` desaparece obligatoriamente y hay dos formas, ninguna
  gratis: (a) `type: archive` a un asset de una release etiquetada de este repositorio, con `sha256`,
  sabiendo que necesita una excepción discrecional; (b) módulo que compila —GraalVM como `archive`,
  AutoFirma como `git` con `tag` y `commit`, y un `maven-sources.json` generado.
- Añadir un `flathub.json` con `only-arches: [x86_64]`: Flathub construye x86_64 **y aarch64** por
  omisión ([flathub.json](https://docs.flathub.org/docs/for-app-authors/requirements#flathubjson)) y
  hoy no tenemos imagen nativa aarch64.
- El `--share=network` del módulo `sonda` es error del linter, no solo mala práctica. Ya decidido en
  el ADR-0013; esto lo confirma con la línea de código que lo rechaza.

**En el carril lento del CI:** nada obligatorio hoy. Si se va por la vía (a), el CI tendría que
publicar los seis `.so` como asset de una release etiquetada y dejar el `sha256` a la vista; y los
metadatos de `native-image` versionados (ADR-0013) pasan de higiene a precondición.

**En los ADR:**

- El **ADR-0004** puede anotar que Flathub no es hoy un canal disponible tal como está el manifiesto,
  y que el motivo no es técnico.
- El **ADR-0013** puede **cerrar el punto que dejó abierto**: para v0.1 **no** se construye la
  librería dentro del sandbox. Es caro (sección 3) y no resuelve el bloqueo real (sección 6).
- Merece un ADR propio la pregunta que abre la sección 6: si Flathub sigue siendo el destino
  declarado del #17 y del #22, o si el canal pasa a ser el bundle más, quizá, un repositorio propio.

## Trampas

1. **La política de IA generativa manda sobre todo lo demás.** Optimizar la forma del módulo
   `libreria-nativa` sin resolver esto es trabajo que puede no llegar a servir para nada.
2. **`--sandbox` no era el culpable.** La suposición del ticket es falsa: `--sandbox` desactiva
   `build-args`, no las fuentes. Lo que rechaza `type: dir` es el linter de Flathub, y lo que
   `--sandbox` rompería es el `--share=network` de la sonda.
3. **Precedente vivo no es permiso.** TuxGuitar y Ghidra hacen hoy lo que la regla escrita prohíbe,
   y la política dice explícitamente que no se aplica retroactivamente. Citarlos en una submission
   nueva no es un argumento.
4. **El linter exige `commit` junto a `tag`** en el pipeline de submission nueva. Una fuente git de
   `clienteafirma` con solo `tag: v1.9.1` es error, no aviso.
5. **`sha1` y `md5` son error.** Solo `sha256` (o `sha512`, que usa la extensión openjdk25).
6. **`type: dir` no cachea.** Cada construcción del flatpak rehace el módulo y recopia los 35 MB. Es
   el precio que ya estamos pagando en el carril lento sin haberlo escrito en ningún sitio.
7. **La receta artesanal de Maven falla en silencio con lo que no está en Central**, y 36 `.jar`
   nuestros no lo están. Un `maven-sources.json` generado con el `find | sha256sum` del hilo #58 sale
   incompleto y el fallo aparece tarde, dentro de la construcción sin red.
8. **El runtime debe ser el último al enviar** («must be the latest version at that time of
   submission»). Hoy `org.gnome.Platform//50` cumple; en una submission futura hay que revisarlo.

## Lo que no he podido verificar

- **Si un reviewer aceptaría GraalVM CE como tarball binario de herramienta de construcción.** Mi
  lectura de que una herramienta de construcción cae fuera de «main application component» y
  «runtime dependencies» es una inferencia del texto, no una regla escrita. No he encontrado ninguna
  aplicación en Flathub que baje un toolchain binario de 352 MB; el único precedente cercano es una
  extensión de SDK, que es otra categoría.
- **Si `native-image` funciona dentro del sandbox de construcción de flatpak.** No lo he probado. Ni
  aquí ni en ningún issue anterior de este repositorio. Necesita el toolchain de C —que el SDK trae—
  y bastante memoria; ninguna de las dos cosas está medida contra el sandbox.
- **Los límites de tiempo, memoria y disco de los trabajadores de construcción de Flathub.** No los
  he encontrado publicados. Es la incógnita que decide si la sección 3 es «caro» o «imposible en la
  práctica».
- **Un precedente posterior a la política actual** de una aplicación que suba su propio artefacto
  binario de release. Busqué en `flathub/flathub` los pull requests que mencionan «built entirely
  from source» (cuatro, ninguno concluyente) y no encontré ninguno que resuelva esto. La búsqueda no
  fue exhaustiva.
- **El texto de la wiki de `flathub/flathub`** que menciona el encargo: no encontré ninguna wiki
  activa; la documentación de revisión vive hoy en `docs.flathub.org`, que es lo que he citado.

## Fuentes

Reglas de Flathub:

- [Requirements](https://docs.flathub.org/docs/for-app-authors/requirements), secciones
  [Inclusion policy](https://docs.flathub.org/docs/for-app-authors/requirements#inclusion-policy),
  [Generative AI policy](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy),
  [Insufficient development history](https://docs.flathub.org/docs/for-app-authors/requirements#insufficient-development-history),
  [No network access during build](https://docs.flathub.org/docs/for-app-authors/requirements#no-network-access-during-build),
  [Building from source](https://docs.flathub.org/docs/for-app-authors/requirements#building-from-source),
  [Required files](https://docs.flathub.org/docs/for-app-authors/requirements#required-files),
  [Dependency manifest](https://docs.flathub.org/docs/for-app-authors/requirements#dependency-manifest),
  [flathub.json](https://docs.flathub.org/docs/for-app-authors/requirements#flathubjson)
- [Submission](https://docs.flathub.org/docs/for-app-authors/submission)
- [Plantilla del pull request de submission](https://github.com/flathub/flathub/blob/master/.github/pull_request_template.md)
- [`flatpak-builder-lint`, `checks/modules.py`, commit `df46a15`](https://github.com/flathub/flatpak-builder-lint/blob/df46a15879db1091640941cae6f75045660c0309/flatpak_builder_lint/checks/modules.py)

Documentación de flatpak:

- [Module Sources](https://docs.flatpak.org/en/latest/module-sources.html)
- [Flatpak Builder Command Reference](https://docs.flatpak.org/en/latest/flatpak-builder-command-reference.html)
- [Single-file bundles](https://docs.flatpak.org/en/latest/single-file-bundles.html)
- [Flatpak Command Reference](https://docs.flatpak.org/en/latest/flatpak-command-reference.html)

Manifiestos y herramientas:

- [`flathub/org.ghidra_sre.Ghidra`](https://github.com/flathub/org.ghidra_sre.Ghidra)
- [`flathub/ar.com.tuxguitar.TuxGuitar`](https://github.com/flathub/ar.com.tuxguitar.TuxGuitar)
- [`flathub/org.freedesktop.Sdk.Extension.openjdk25`](https://github.com/flathub/org.freedesktop.Sdk.Extension.openjdk25)
- [`flathub/es.estoes.wallpaperDownloader`](https://github.com/flathub/es.estoes.wallpaperDownloader)
- [`flatpak/flatpak-builder-tools`](https://github.com/flatpak/flatpak-builder-tools),
  [PR #253 cerrado sin fusionar](https://github.com/flatpak/flatpak-builder-tools/pull/253),
  [issue #487 abierto](https://github.com/flatpak/flatpak-builder-tools/issues/487)
- [`flatpak/flatpak-builder` issue #58](https://github.com/flatpak/flatpak-builder/issues/58)
- [GraalVM CE, release `graal-25.3.4.1`](https://github.com/graalvm/graalvm-ce-builds/releases/tag/graal-25.3.4.1)

## Reproducir las cifras locales

```bash
find ~/.m2/repository \( -iname '*.jar' -o -iname '*.pom' \) | wc -l   # 832
find ~/.m2/repository/es/gob/afirma -name '*.jar' | wc -l              # 36
du -sh ~/.m2/repository                                                # 91M
```
