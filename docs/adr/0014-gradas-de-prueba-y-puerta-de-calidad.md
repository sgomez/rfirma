# Gradas de prueba y puerta de calidad

El [ADR-0013](0013-estructura-del-repositorio-y-cadena-de-compilacion.md) fijó qué recetas
existen; este fija **qué se ejecuta dentro de ellas y en qué carril**, que era lo que
[#11](https://github.com/sgomez/rfirma/issues/11) dejó a propósito en la niebla por no haber
código que probar. Con [#10](https://github.com/sgomez/rfirma/issues/10) aparece, y su corte es
**horizontal por módulo**: sin esta decisión cada sub-issue se inventa su propio criterio de
terminado.

> Este ADR decía «horizontal por módulo **y en paralelo**». La entrega pasó a
> `execution: sequential` al publicar el [#46](https://github.com/sgomez/rfirma/issues/46),
> porque el repositorio arranca de cero y los sub-issues se pisarían en los ficheros de
> cimientos, no en su módulo. El corte horizontal no cambia; la razón está en
> `docs/agents/developer-defaults.md`.

## Las cuatro gradas

Una prueba se clasifica por **lo que necesita para ejecutarse**, no por lo que significa. Cada
sub-issue de #10 declara la grada de sus pruebas, y la grada decide el carril:

| Grada | Necesita | Ejemplos | Carril |
| --- | --- | --- | --- |
| **A** | nada | sello de sesión, coordenadas del [#9](https://github.com/sgomez/rfirma/issues/9), composición del `layer2Text` y su máscara, `paths.rs` | rápido |
| **B** | SoftHSM (`apt-get install softhsm2`, segundos) | firma `CKM_SHA256_RSA_PKCS`, listado de certificados, mapeo de `CKR_*` | rápido |
| **C** | los seis `.so` (GraalVM, `just native`) | ciclo trifásico completo, PDF válido, rúbrica visible | lento |
| **D** | **un tercero vivo mientras la prueba corre** | OCSP en vivo contra el certificado revocado del kit | ni PR ni carril lento: cron |

La grada que sostiene la decisión es la **B**. Sin ella, «probar de verdad» y «tardar cuatro
minutos» serían lo mismo, y la mitad del código de riesgo —PKCS#11— caería al carril que nadie
espera. SoftHSM es de software y se instala en segundos: no hay razón para tratarlo como caro.

La **D** la heredamos de #11 y su carril no se toca: atar cada PR a la disponibilidad de
`sede.fnmt.gob.es` con `merge: auto` significa que un corte ajeno bloquea la entrega. Va a un
job programado que abre una issue si falla.

### La D es quién es el sujeto, no si en algún momento hubo red

La columna «Necesita» de la D decía **«red»**, a secas, y eso es demasiado ancho: convierte en
grada D cualquier prueba que en algún momento haya tocado un servidor ajeno, aunque el ajeno ya
no esté delante cuando la prueba corre. Con esa lectura, **el banco de conformidad acabaría en
el cron** —donde nadie lo mira, y donde no puede ser una puerta— sólo porque su accesorio se
descarga.

Lo que hace grada D a una prueba es que **el sujeto de la medición sea un tercero vivo**: que lo
que se comprueba es cómo responde `sede.fnmt.gob.es` **hoy**, y que si está caído no hay
veredicto. Un accesorio que se descarga en el paso de preparación, **a etiqueta fijada, con su
`sha256` comprobado y cacheado**, no es eso: es un fichero en disco, exactamente igual que los
`.p12` del kit de la FNMT, y la prueba que lo usa cae en la grada que le toque por lo demás que
necesite.

El caso que obliga a escribirlo es el **banco de conformidad** (TD-55): el `autoscript.js`
publicado en el tag `v1.9.2` de `clienteafirma`, corriendo bajo Node contra el canal `wss://` de
rfirma en `tests/conformance_*.rs`. El sujeto es **nuestro** canal; el cliente publicado es
el instrumento, y es un fichero de 219 KB con su huella. **Es grada B, en el carril rápido**, y
es el único sitio donde se mide que hablamos el mismo dialecto que el cliente que ejecutan las
sedes de verdad. Lo baja `just autoscript` —el pin, URL y `sha256`, vive en el `justfile`— y el
CI lo hace en un paso propio, con caché por ese `sha256`.

Con la mitad que impide que se apague solo: **si el accesorio falta, el CI falla; sólo se salta
en local**. `just autoscript` sale con 1 si la descarga falla o si el `sha256` no cuadra, y la
propia prueba mira la variable `CI` y falla en vez de saltarse. Un `skip` silencioso convertiría
el banco en decoración el primer día que la descarga dejase de funcionar.

### Abrir un socket de escucha en el *loopback* es grada A

La columna «Necesita» habla de **lo que hay que instalar**, y para escuchar en `127.0.0.1` no
hay que instalar nada: es el sistema operativo. Así que el servidor local del protocolo
`afirma://` —levantarlo, completar el saludo TLS, aceptar o rechazar el `Origin` y la dirección
de origen— **es grada A y carril rápido**, aunque monte un servidor de verdad, y aunque por eso
viva en `tests/` en vez de en un `#[cfg(test)]`. La **D** sigue siendo red *ajena*: lo que
depende de un tercero que puede estar caído.

Con una regla que no es opcional: **el puerto se le pide al sistema (`:0`), nunca se fija**.
`cargo test` corre los ficheros de prueba en paralelo, y dos pruebas con un puerto fijo se pisan
de forma intermitente —el fallo más caro que existe, porque se atribuye a cualquier otra cosa—.
Como efecto secundario, pedir el puerto al sistema ejercita gratis lo que la sede hace de
verdad: ofrecer tres puertos y quedarse con el que abra.

### Las pruebas de la grada C se compilan siempre y se ejecutan solo en el lento

Se marcan con **`#[ignore]`**, en un fichero que lo dice por su nombre (`tests/native_cycle.rs`),
y el carril lento ejecuta solo esas con `--run-ignored only` en `test-native` para no repetir los tests unitarios. Descartada una *feature* de cargo, que
además las sacaría de la compilación.

`#[ignore]` tiene un punto ciego —una prueba que deja de compilar contra la frontera FFI se
salta en silencio— y se compensa con una regla: el carril **rápido** las compila
(`cargo test --no-run`) aunque no las ejecute. Así un error de tipos contra la FFI cae en 48 s y
solo el coste de *ejecutar* se paga en los tres minutos.

## Qué prueba de verdad que la firma vale

**`pdfsig` de poppler es la puerta automática**, en la grada C, con la trampa que midió el
[#14](https://github.com/sgomez/rfirma/issues/14): la rúbrica **se comprueba rasterizando**,
porque `pdftotext` no la ve y da un falso negativo.

**El validador oficial es una puerta manual de release**, no de CI: lo ejecuta una persona una
vez por etiqueta `v*`. VALIDe es red, web y sin API estable, así que no cabe en ningún carril, y
el destino del mapa promete precisamente eso —«que un validador oficial acepte la firma»—. Sin
escribirlo, un check en verde se lee como una promesa que el CI no puede demostrar.

### La segunda puerta manual: el navegador de verdad

Desde la v0.5 hay una segunda comprobación que no corre sola, y por el mismo motivo que la
primera: **necesita un navegador o una sede de verdad**, y meter cualquiera de los dos en el CI
sería comprobar sobre todo cosas del navegador. Vive en `docs/pruebas-manuales-protocolo.md` y
se ejecuta **por etiqueta `v*`**, junto al validador oficial.

Su criterio de entrada está escrito **como prohibición**, que es lo único que impide que una
lista manual se llene sola de lo que costaba automatizar:

> Entra en la lista **sólo** lo que necesita un navegador de verdad o una sede de verdad, y por
> tanto no lo puede tener el CI. Si una comprobación cabe en A, B o C, va ahí, **aunque sea
> incómoda**. Añadir una fila obliga a justificar en la propia fila por qué no cabe en ninguna
> de las tres.

La lista lleva además su **condición de salida** —qué la haría innecesaria— dentro del propio
fichero, no aquí: el ADR fija que la puerta existe y con qué criterio se llena; qué la sustituiría
es estado de la lista, y ahí lo lee quien la ejecuta.

### La suite de conformidad no es una grada ni una puerta

La **suite de conformidad** (`just conformance`, glosario en `CONTEXT-MAP.md`) mide si un cliente
instalado, AutoFirma o rFirma, cumple el protocolo, y queda fuera de las cuatro gradas, de las dos
puertas manuales de arriba y del CI: ni ella ni sus pruebas bloquean ningún PR ni ninguna
etiqueta, y se lanza en local cuando alguien lo decide.

## La prueba del ciclo completo tiene dueño

Es el **único sub-issue vertical** de #10: el último de la cadena, bloqueado por todos los
módulos, y su entregable es `tests/native_cycle.rs` más el paso de `pdfsig`. Necesita el puente
Java, la FFI, PKCS#11, el PDF y `pdfsig` a la vez, así que no pertenece a ningún módulo del
corte horizontal. Su cuerpo tiene que decir **por qué** es la excepción, o el siguiente agente
que lea el spec creerá que el corte se rompió por descuido.

## Herramental

**Rust**: `cargo clippy -D warnings` y `cargo fmt --check` dentro de `lint-rust`; `cargo test`;
`cargo llvm-cov` para la cobertura.

**TypeScript: Biome**, no `eslint` + `prettier`. El ADR-0013 escribió `eslint` en una casilla de
tabla sin razonarlo y no dijo nada del formateador; **queda corregido aquí**. Biome es un
binario que formatea y lintea en milisegundos, frente a `typescript-eslint` con su cadena de
paquetes más un `prettier` aparte. El argumento habitual a favor de `eslint` —el ecosistema de
plugins— aquí no cobra: no hay router, ni tabla de datos, ni biblioteca de componentes
(ADR-0007, ADR-0013). `vitest` sigue siendo el ejecutor.

**Java**: `-Xlint:all`, como decidió #11. No cambia.

## La métrica CRAP: solo en Rust

`cargo crap --lcov lcov.info --fail-above --threshold 30` (el umbral de Savoia; `--fail-above`
es un interruptor sin valor), alimentado por
`cargo llvm-cov --lcov`. En **Java no entra** —lo único en Maven Central es un plugin de Hudson
de 2010— y el puente es código que reenvía. En **TypeScript tampoco**: `crap4ts` existe pero
lleva desde junio parado en `2.0.0-rc.5`, y sobre todo la complejidad ciclomática de un
componente React es JSX condicional, que no es lo que la métrica mide. El código de riesgo de
este proyecto está todo en Rust por decisión del ADR-0010 y del ADR-0013.

### Dónde se calcula, y por qué no es obvio

`cargo-crap` puntúa por omisión con `--missing pessimistic`: **una función sin datos de
cobertura cuenta como 0 %**. La cobertura del carril rápido incluye las gradas A y B pero no la
C, así que sin más cuidado los peores CRAP del repositorio serían el módulo FFI y la sesión
trifásica — es decir, el código que **sí** está probado, solo que en el otro carril. Un umbral
así se desactiva en una semana, o enseña a los agentes a no escribir código ahí.

La puerta vive en el **carril rápido**, que es donde un agente la lee, con
**`--allow` sobre la ruta del módulo FFI**: `--allow` analiza el fichero y oculta sus funciones,
que es exactamente el matiz que hace falta. El **carril lento mide ese módulo de forma dirigida**
en la misma pasada que da el veredicto de la grada C (`just test-native`): ejecuta sus pruebas bajo
`cargo llvm-cov nextest --run-ignored only` y comprueba con `cargo crap --path` que ninguna función
del adaptador FFI supera el umbral de 30, sin repetir la medición de toda la suite. Descartada la
pasada aparte de antes (`cargo test --ignored` y luego `crap-ffi` bajo `llvm-cov`): compilaba la
suite dos veces y ejecutaba los binarios de prueba uno detrás de otro, y era el camino crítico del CI.

**`--allow` corrige una cobertura que se mide en otro carril; no perdona a un módulo por ser
difícil de probar.** La distinción es la que sostiene la puerta entera: el módulo FFI se oculta
en el carril rápido porque **sí está probado**, sólo que en la grada C, y el carril lento lo
vuelve a medir en `test-native`. Un `--allow` concedido a un módulo que nadie mide en ningún
carril —«es entrada/salida», «es fontanería»— desactiva la puerta por precedente, y el segundo
entra solo. Si un módulo de entrada/salida no baja de 30, la conversación es **sobre el corte
del módulo**, no sobre el umbral: que la parte con lógica sea una capa aparte y probable es
exactamente la señal que la métrica existe para dar.

El adaptador CNG de Windows (`windows_allow`) es el mismo caso: Linux no lo compila, así que el
carril rápido lo oculta, y el carril de Windows lo mide con `just test-windows` (ADR-0035).

### Puerta absoluta, sin trinquete

Umbral fijo, **sin `--baseline` ni `--fail-regression`**. El trinquete exige versionar un JSON
que cambia en casi cada PR, y eso son conflictos de merge en un fichero generado. Su única
ventaja —amnistiar deuda existente— no aplica: hoy el repositorio tiene **cero líneas de Rust**.

> El argumento original apoyaba esto en el `execution: parallel` que entonces declaraba
> `developer-defaults.md`. Con `sequential` ese apoyo se debilita, pero la decisión se sostiene
> sola sobre el segundo motivo, que es el fuerte: no hay deuda que amnistiar.

### El riesgo de la herramienta, dicho en voz alta

`cargo-crap` tiene **cuatro meses** (primera versión 2026-04-27) y **un solo mantenedor**. Se
instala con `cargo binstall` a una **versión fijada** en `versions.env`. Si se abandona, la puerta
se quita en una línea y no arrastra nada: es una comprobación aparte, no un formato que impregne
el código.

## El tamaño de fichero se congela con un baseline, no con un umbral suelto

Un umbral de líneas sin memoria no frena el crecimiento: cada PR añade unas
decenas de líneas a un fichero que ya se pasaba, y ninguna PR concreta cruza
la raya sola. Umbrales: **500** líneas en producción y **600** en tests, con
dos guardas según la cadena.

En Rust, `tests/files_stay_small.rs` (grada A) recorre
`rfirma-app/src-tauri/src` y `rfirma-app/src-tauri/tests` y compara lo medido
—líneas no vacías— contra `files_stay_small.baseline`, un fichero versionado
de `ruta → líneas`; es test todo fichero bajo un directorio `tests/` o llamado
`tests.rs`.

En TypeScript, la regla `style/noExcessiveLinesPerFile` de biome sobre los
`.ts` y `.tsx` de `rfirma-app/src`, con `skipBlankLines` y 600 para los
`.test.ts(x)`. Biome no cuenta las líneas de comentario, así que la regla es
más permisiva que la de Rust: se acepta a cambio de que salte en el pre-push.
Reescribir la guarda de Rust como script propio para los `.ts` conservaba la
cuenta exacta, pero duplicaba en el repositorio lo que biome ya trae. La de
Rust también salta en el pre-push, por la vía de las guardas estructurales
descrita más abajo. Fuera
de ella, lo que biome ya excluye: `rfirma-app/src/i18n/locales/*` (datos, no
código) y los ficheros generados.
Sin baseline: un fichero que tenga que pasarse lo declara en su cabecera con
`biome-ignore-all`, a la vista de quien lo abra.

La guarda de Rust falla en los cinco casos que hacen del baseline una lista
exacta, no un suelo: un fichero listado que crece, uno nuevo por encima del umbral que no
está en la lista, uno que baja sin que su línea se actualice, uno listado que
ya no existe, y uno listado que ha quedado por debajo del umbral. El mensaje
de fallo dice qué hacer: partir un fichero de tests por comportamiento en
`tests/<comportamiento>.rs` sacando los helpers a `tests/support.rs`, o
separar la responsabilidad que sobra en uno de producción. Esta decisión
**no** parte los ficheros que ya se pasan hoy: la guarda los congela con su
tamaño actual, y cada uno se parte por su responsabilidad real en su propia
PR. Con ella se acaba también el `tests.rs` único por módulo en los módulos
nuevos.

Acompaña a `too_many_lines` de clippy, con `too-many-lines-threshold = 80` en
`clippy.toml` y `too_many_lines = "warn"` en `[lints.clippy]` de `Cargo.toml`
—no está en el grupo por omisión de clippy, a diferencia de
`too_many_arguments`, que ya estaba activo y no se toca—. Las funciones que ya
pasan de 80 líneas llevan `#[expect(clippy::too_many_lines)]`: `expect` falla
en cuanto la función deja de pasarse, así que hace de trinquete sin fichero
aparte.

## El suelo de cobertura y la cobertura del diff

CRAP puntúa por función; nada impedía que la cobertura global bajara sin que
ninguna función concreta lo delatara. El [#852](https://github.com/sgomez/rfirma/issues/852)
añade dos puertas más, las dos en Rust y las dos alimentadas por el mismo
`lcov.info` de `cargo llvm-cov` que ya produce `coverage`:

- **Un suelo global que no baja**: `--fail-under-lines` dentro de la propia
  receta `coverage`, con el porcentaje en `coverage_floor` del `justfile` —
  fijado igual que `CRAP_VERSION`, no una media móvil. Sube **a mano, en su
  propia PR**, cuando la medida real lo supere en un punto entero; nunca lo
  sube un commit del CI, por la misma razón que el trinquete de CRAP se
  descartó más arriba: un fichero que cambia solo no tiene revisor.
- **Cobertura del diff**: `diff-cover` sobre ese mismo `lcov.info`, comparando
  contra `origin/main` con `--diff-range-notation=..` para no depender de un
  merge-base que un `git fetch --depth=1` no puede calcular. Quedan fuera
  `**/adapters/tauri.rs` y `main.rs` (pegamento de Tauri, cobertura cero por
  diseño) y el módulo FFI (`ffi_allow`, el mismo que `--allow` oculta de
  CRAP): ese se mide en el carril lento, no en este `lcov`.

**La cobertura del diff pide red — `origin/main` como ref local — y por eso
no vive dentro de `check-rust`.** La receta `diff-coverage` es `[group('ci')]`,
no parte de la cadena `check`: `check-rust` tiene que seguir corriendo sin red
en un portátil, la misma invariante que ya protegía al banco de conformidad
más arriba. El `git fetch` va en `ci.yml`, no en el `justfile`.

Ni una ni otra tocan Java ni TypeScript, por la misma razón que CRAP: el
código de riesgo está en Rust.

## Mutation testing y duplicación: informan, no bloquean

El [#852](https://github.com/sgomez/rfirma/issues/852) añade dos arneses más, y a
propósito ninguno de los dos entra en `just check` ni en el CI de las PR: los dos
son caros de correr en cada `push` y ninguno tiene todavía un histórico que diga
qué veredicto es ruido.

**`just mutants`** (`cargo-mutants`, grupo `release`) mide si las pruebas
detectarían un cambio real en el código, no solo si lo cubren: muta una línea —
invierte una comparación, cambia el resultado de una función— y falla si ninguna
prueba lo nota. Correr sobre todo el árbol es demasiado lento para el día a día,
así que el alcance es `--in-diff` contra el último tag `v*`
(`git describe --tags --abbrev=0 --match 'v*'`), con el diff generado en
`rfirma-app/src-tauri` mismo (`git diff --relative`, para que las rutas del diff
casen con las que `cargo-mutants` espera relativas al paquete) y los mismos
adaptadores ocultos que `crap --allow`: `adapters/tauri.rs`, `main.rs` y el módulo
FFI (`ffi_allow`). Se lanza **a mano, antes de publicar una versión**: el
veredicto es leer la lista de mutantes que sobreviven y decidir, no un exit code
que bloquee. Pasará a `release.yml` cuando haya histórico de tiempos y de
resultados que diga que merece la pena automatizarlo.

**`just duplication`** (`jscpd`, grupo `dev`) informa de código duplicado en los
ficheros de tests, en Rust y en TypeScript — el mismo sumidero que motivó la
guarda de tamaño de más arriba tiende también a repetir el mismo `arrange` en vez
de extraerlo a un helper. Solo informa: sin `--threshold` ni `--exit-code`, así
que nunca sale en rojo. No entra en el CI hasta ver cuánto ruido da en la
práctica.

## Un solo hook: lo que el CI tumba a menudo y se ve en local, antes del push

`pre-push` con **lefthook**, encadenado (`piped`: el primer trabajo que falla para los demás), y dentro cuatro trabajos. Los dos primeros corren siempre. El primero, **formato y lint de biome**: `just
fmt-check`, que comprueba `cargo fmt` en la app y en la suite de conformidad, `biome check` —el
formateador, el orden de imports y el linter— y `ruff format --check` sobre todo el Python del
repositorio. El lint de biome entra porque no compila ni depende de `build-ts` y tarda menos de
un segundo en todo el árbol; `biome format` solo, sin él, dejaba pasar al CI imports sin ordenar
y ficheros por encima del umbral de tamaño. La llaman el hook y `check-repo`, así que el
CI y el push comprueban lo mismo.

El segundo, **las guardas estructurales del backend**: `just structural-guards` compila con
`rustc --test`, sueltas y en paralelo, las pruebas de grada A que solo leen el árbol como texto
—el tamaño de los ficheros, los mapas `AGENTS.md`, las citas de ADR, los módulos de test en
línea, la dirección entre capas y los condicionales de sistema operativo— y las ejecuta. No
compila la crate ni sus dependencias: cada guarda es un fichero que solo usa `std`, y el conjunto
tarda menos de un segundo. Son las mismas pruebas que corre `cargo test` en el carril de Rust,
así que el umbral, el baseline y la regla tienen un único sitio, el fichero de la guarda. Existe
porque una guarda de estas que saltaba en el CI costaba una vuelta entera de más de diez minutos
por algo que se ve leyendo el árbol.

Los otros dos corren solo si el push toca su cadena, con el `glob` de lefthook sobre los ficheros
del push. El tercero, si toca `rfirma-app/src/`: `just build-ts lint-i18n knip` —tipos, claves de
i18n y exports sin uso—, unos quince segundos. El cuarto, si toca `rfirma-app/src-tauri/`: `just
check-rust`, lo mismo que el carril de Rust del CI —clippy, `cargo-machete` y la puerta CRAP—, unos
dos minutos con el `target/` compartido caliente cuando hay que recompilar la crate instrumentada. Entran por lo que dicen los rojos: de los diez que
dio el CI entre la llegada de las guardas al hook y el 3 de octubre de 2026, cuatro fueron CRAP
—siempre un adaptador nuevo sin pruebas— y dos clippy; otros cuatro solo se ven en Windows, y
esos no los adelanta un hook en Linux. Cada uno costaba una vuelta entera del CI, con su arreglo y
su revisión, de diez a quince minutos de reloj: más que los dos minutos que el hook suma a cada push
de Rust. La suite y la cobertura del diff se quedan fuera: no están entre los rojos frecuentes, y la
suite es lo más caro del carril. `just check` sigue siendo el único punto de entrada que promete
`docs/agents/code-host.md`.

El gestor va como dependencia de desarrollo de `rfirma-app`, a versión exacta por la misma razón
que `cargo-crap`, y lo instala el `prepare` de `package.json`: cualquier `pnpm install` —`just
deps` incluido— la deja puesta. `bootstrap.sh` no crece (ADR-0013). La configuración es
`lefthook.yml` en la raíz, que antepone `~/.cargo/bin` y `~/.local/bin` al `PATH` porque un hook
de git es una shell no interactiva. Cuando falla, el mensaje nombra `just fmt`, que escribe.

Descartado para las guardas: `cargo test --test <guarda>`, que compila antes la crate entera
con sus dependencias, y un script de shell que reimplementara cada guarda, que duplicaba umbrales
y reglas en dos sitios que se desincronizan.

Descartado: un trabajo por cadena, filtrado por glob y en paralelo, que se saltaba la cadena sin
herramienta. Eran tres copias de lo que comprueba el CI, con un `ruff format` acotado a
`packaging/`, y un aviso que deja pasar el formato sin comprobar se ignora.

### Considered Options

**Solo formato y guardas, en segundos**, que fue la regla hasta octubre de 2026: «la puerta se mide en
segundos o no sobrevive». Se abandonó cuando los rojos de CRAP y de clippy pasaron a ser la mayoría de
los que se ven en Linux: el hook rápido ya no adelantaba casi ninguno.

**CRAP solo cuando el agente sospecha**, como paso de la escalera en `AGENTS.md`: barato, pero depende
de que quien empuja vea venir el rojo, y el de `chosen_on_tty` llegó al CI con la sospecha ya escrita.

**Ningún hook**, que es lo que hubo hasta ahora: se temía que uno desconocido se esquivara con
`--no-verify` o explotara sin que nadie entendiera por qué. Lo primero se acepta —detrás está el
CI, que no se esquiva, y adelantarse a un fallo barato no exige ser infranqueable—; lo segundo lo
cierran el aviso y el mensaje de arriba.

## Dónde corre cada puerta: el agente no es el CI

`just check` es la puerta entera, y **su sitio es el CI**, que la reparte en runners
simultáneos (Java, TypeScript, Rust y la landing, esta con carril propio por rutas) y por tanto
paga el carril más lento. En un portátil se pagan sumados, así
que **no hay puerta local que la sustituya**: en local corren la prueba concreta que se está tocando y,
en el pre-push, lo que dice la sección del hook. Medido en el equipo de desarrollo, con
cachés calientes: `check-repo` 4 s, `check-java` 4 s, `check-ts` 15 s, `check-rust` 46 s.

La escalera es de tres peldaños y la escribe `AGENTS.md`, que es donde un agente la lee:

1. **Cada rojo → verde**: sólo la prueba que se está tocando. Nunca una receta de `check`.
2. **Antes de commitear**: `just fmt`, y nada más.
3. **Al revisar**: nada, si el CI está verde para ese head sha.

**Esto no relaja nada.** La puerta que decide sigue siendo `just check` entera, corriendo en el
CI sobre el head sha; lo que cambia es que deje de correrse en el sitio donde más cuesta y menos
decide.

### Considered Options

**`check-changed`**, una receta que deducía de lo que la rama tocaba respecto a `origin/main`
qué carriles hacían falta y los corría en local antes de commitear. Nació de un rojo de formato
descubierto ya en una PR; ese caso concreto lo evita el hook de pre-push de más arriba, y el
resto de lo que atrapaba —lint, tipos, pruebas— lo atrapa igual el CI, en paralelo y sin ocupar
el portátil. Medido tocando solo el `justfile`, que dispara sus tres cadenas: **3 min 35 s de
reloj y 542 s de CPU de usuario** por ejecución, porque arrastra `crap` (el árbol instrumentado
de `cargo llvm-cov`), `check-landing` (construye la landing) y la cadena de Maven. Se retira: el
coste por ejecución superaba con creces lo que adelantaba.

### El árbol de compilación se comparte entre worktrees, y el CI no se toca

Cada agente constructor trabaja en un worktree propio, así que cada uno recompilaba el árbol de
dependencias de Tauri desde cero: medidos, **73 s y entre 6,8 y 13 GB por worktree**. El
`justfile` apunta `CARGO_TARGET_DIR` a `.claude/worktrees/target` **cuando corre desde un
worktree**: el primer agente paga la compilación entera una vez y los demás entran en **11 s**.

**El checkout principal se queda fuera.** Cargo toma un cerrojo sobre el `target/` mientras
compila, y meterlo dentro haría que un `cargo` a mano esperase a que terminara el agente de
turno. Los agentes sí se serializan entre sí, y con `execution: sequential` eso no cuesta nada;
si algún día se vuelve a `parallel`, esta es la línea que hay que volver a mirar.

**El árbol instrumentado lleva su `CACHEDIR.TAG`.** `cargo llvm-cov` limpia el paquete y los
volcados antes de cada pasada, y Cargo se niega a limpiar un directorio sin esa marca. Sin limpiar,
la pasada mezcla los binarios y los `.profraw` de otros worktrees, y la cobertura total se hunde
por debajo del suelo. Lo pone `just llvm-cov-tag`, del que dependen las recetas que instrumentan.

### Considered Options

**`sccache`**, que era la opción obvia y resultó no serlo: entre dos `target/` distintos
acierta el **0 %**. Medido en dos compilaciones seguidas del mismo código en árboles vírgenes:
74 s la primera, **73 s la segunda**, con la caché escrita (826 compilaciones, 759 MiB de un
máximo de 10 GiB, así que no era desalojo). Su clave depende de las rutas de los `--extern`,
que es justo lo que cambia entre worktrees. Cachea de verdad cuando la ruta se repite —el caso
del CI—, no el nuestro.

**`sccache` también en el CI** (`mozilla-actions/sccache-action`): descartado por lo mismo que
lo anterior no aplica allí, y con la medición delante. El job «Cadena Rust» de una PR tarda
148 s, de los cuales 71 s son `just check-rust` y 77 s la preparación —28 s de restaurar la
caché de `Swatinem/rust-cache`, 24 s de apt, 12 s de la toolchain—. Es decir: `target/` **ya
llega construido**, y lo que queda no es compilar sino ejecutar la suite y medir cobertura. Una
segunda capa de caché no tiene ahí casi nada que ahorrar, añade sus propios pasos de restaurar y
guardar, y sobre todo **compite por los 10 GB de cuota de cachés de Actions con la que sí está
funcionando**: el efecto neto más probable es que `rust-cache` empiece a desalojarse y falle más
a menudo, que es exactamente el caso caro. Se reconsidera si `rust-cache` deja de acertar.

## La bomba de relojería del kit FNMT

`testdata/fnmt/` con los tres `.p12`, sus contraseñas publicadas y sus huellas al lado, más la
excepción para el escáner de secretos que avisó #11. `active-rsa.p12` **caduca el 2028-10-30**, y
las dos mitades de la guardia viven en sitios distintos a propósito:

- la **prueba dura** (grada A, carril rápido) falla al caducar nombrando el fichero, la fecha y
  el enlace a STCERES;
- el **aviso a 90 días** va al **cron semanal**, que abre una issue.

Avisar en el carril rápido rompería todos los PRs a la vez, un día cualquiera de 2028, con
`merge: auto` puesto. El cron es el único sitio donde avisar no bloquea a nadie. Sin congelar el
reloj: escondería fallos reales de cadena.

## Cobertura del frontend: el mismo suelo que en Rust, en TypeScript

`@vitest/coverage-v8` mide `rfirma-app/src`, y `coverage.thresholds` en `vite.config.ts` es la
puerta: **90 % de líneas**, la medida inicial redondeada hacia abajo, con `autoUpdate: false` —la
misma política manual del suelo de Rust, nunca un commit del CI subiéndolo solo. Corre dentro de
`just test-ts`, que ya era parte de `check-ts` (Cadena TypeScript, carril rápido): el coste
añadido es la instrumentación de la propia suite de Vitest, segundos, no un job nuevo.

`coverage.include: ["src/**/*.{ts,tsx}"]` está puesto a propósito, y no es el comportamiento por
omisión de Vitest —que **sólo mide los ficheros que algún test llega a importar**—: sin `include`,
un fichero nuevo sin ninguna prueba no cuenta en absoluto, ni suma ni resta, y el suelo deja de
detectar exactamente el caso que existe para atrapar. Con `include`, un fichero de `src/` que
ningún test toca puntúa 0 % de líneas y arrastra la media, igual que en Rust un `cargo llvm-cov`
mide el binario entero y no sólo lo que una prueba ejecuta.

## `cargo-machete` y `knip`: dependencias y exports sin uso

Dos arneses de gradas A, uno por cadena, y los dos bloquean el CI:

| Arnés | Qué mide | Dónde vive | Job (carril rápido) |
| --- | --- | --- | --- |
| `cargo-machete` | Dependencias de `Cargo.toml` que no usa nadie | receta `machete`, dentro de `check-rust` | Cadena Rust |
| `knip` | Dependencias y *exports* de TypeScript sin uso | `pnpm exec knip`, receta `knip`, dentro de `check-ts` | Cadena TypeScript |

Ninguno de los dos compila ni instrumenta nada: `cargo-machete` analiza el árbol de sintaxis de
los fuentes sin invocar a `rustc`, y `knip` recorre los módulos que alcanza desde sus puntos de
entrada. El coste que añaden al CI son segundos.

`cargo-machete` va a **versión fijada** (`MACHETE_VERSION` en `versions.env`), la misma razón
que `cargo-crap`: un solo mantenedor no debe poder poner en rojo un PR que no lo ha tocado.

`knip.json` es la configuración mínima, y cada entrada existe por un falso positivo real, no por
gusto:

- **`entry`** añade `src/sede/main.tsx` y `i18next.config.ts`. `knip` ya detecta `src/main.tsx`
  desde `index.html` con su complemento de Vite, pero `sede.html` es un **segundo** punto de
  entrada (ID-335) que ese complemento no ve: sin declararlo, `sede/main.tsx` entero —y todo lo
  que sólo él usa, como `tauriSiteErrands`— sale como código muerto. `i18next.config.ts` no lo
  importa nada del árbol de `src/`: es la configuración que invoca `i18next-cli` desde fuera
  (`just lint-i18n`), y con él como entrada la dependencia de desarrollo `i18next-cli` deja de
  salir como no usada, porque su único `import` está ahí.
- **`ignore`** cubre `src/design-system/bundle/**`: el bundle normativo del sistema de diseño
  (ADR del #80), copiado en línea desde el proyecto de diseño y sellado por `bundle.lock`, no es
  código de la aplicación —es un IIFE que se serviría por `<script>` aparte, y hoy no lo sirve
  nadie—, así que no es dead code que podar, es un artefacto que otra puerta ya vigila
  (`check-bundle.sh`).

`cargo-machete` no encontró nada que limpiar al escribir esta puerta. `knip` sí: diecisiete
*exports* y otros tantos tipos exportados que ningún fichero fuera del suyo importaba —el
`export` sobraba, no el código—, incluida `cancelSigning` en `tauri.ts`, cuyo propio comentario ya
decía que no debía exportarse suelta. Tres de esos *exports* —`CloseIcon`, `noDocumentDrops` y el
tipo `ErrorSituationWithHelp`— resultaron, al quitarles el `export`, código muerto de verdad
(`tsc --noUnusedLocals` los denunció en cuanto dejaron de tener una salida): no sólo sobraba la
palabra, sobraba la declaración entera, y se borraron.

## Las versiones fijadas, en un solo fichero

Una herramienta que decide un veredicto va a versión fijada: `cargo-crap` y `cargo-machete` por
su único mantenedor, `ruff` porque sin `ruff.toml` sus reglas por defecto son las de la versión
instalada, `diff-cover` y `cargo-nextest` por la misma razón. **Rust** también: con `stable`, cada
versión nueva traía lints de clippy que ponían en rojo todas las PR e invalidaba de golpe la
caché de compilación, sin que nadie hubiera tocado el repositorio. Junto a ellas se fijan `just`, la
GraalVM CE (ADR-0035), la etiqueta de AutoFirma de la que salen las dependencias Java (ADR-0002)
y el sha256 del `autoscript.js` del banco de conformidad.

Todas se escriben **una sola vez**, en `versions.env` en la raíz: una línea `CLAVE=valor` por
versión, sin comillas, sin `export` y sin comentarios, para que el mismo fichero valga como
dotenv y como entorno de Actions. Subir una versión es cambiar una línea. Quién lo lee:

- **El `justfile`**, como dotenv con `dotenv-override`: una variable vieja del entorno de quien
  lo lanza no gana al fichero. Las recetas reciben las claves en su entorno.
- **El CI**: la acción local `load-versions` lo vuelca al entorno del job; `setup-runner`
  la llama antes de instalar, así que todo job que pasa por ella las tiene.
- **Maven**, que recibe `-Dautofirma.version` en la línea de órdenes. Los dos pom conservan su
  valor por defecto para el uso sin `just`.
- **Los arranques que no pasan por `just`** —`bootstrap.sh` y los guiones del testbench, que
  lanzan las pruebas de grada C— lo leen con `scripts/pinned-version.sh`.

`just install-tools [herramienta…]` instala las herramientas en esas versiones, en local y en el
CI, y `just tools` falla si una instalada no coincide con la fijada. Las claves de caché usan el
valor de la versión que les afecta —`AUTOFIRMA_VERSION` en la de `~/.m2`, `AUTOSCRIPT_SHA256` en
la del banco—, nunca el resumen del fichero entero: subir una herramienta no invalida la caché de
Maven. `scripts/check-versions.sh`, dentro de `check-repo`, falla si un valor fijado aparece
escrito en el `justfile`, en `.github/`, en `lefthook.yml` o en un guion de `scripts/` o del
testbench, y si el valor por defecto de un pom no coincide con el fichero.

`cargo-llvm-cov` y `cargo-mutants` quedan sin fijar: producen informes, no veredictos.

### Considered Options

- **`.tool-versions` con mise (o asdf)**: resuelve la instalación local, pero no cubre lo que no
  es una herramienta —la etiqueta de AutoFirma, el sha del `autoscript.js`—, su formato no es
  `CLAVE=valor` y en el CI obliga a instalar mise para leerlo. Añade una herramienta para
  sustituir una lectura de fichero.
- **Renovate**: automatiza las subidas, pero no decide dónde vive la versión, que es el problema;
  y una subida de `ruff` o de `cargo-crap` que cambia un veredicto debe llegar en una PR propia,
  a mano. Dependabot ya propone las acciones.
- **`taiki-e/install-action`**: instala binarios fijados en el CI, pero la versión seguiría escrita
  en el workflow, aparte de la local, y en local no sirve: `cargo binstall` y `pipx` sí.
- **Cada versión en su sitio, con un comentario «igual en ci.yml»** (lo anterior): el comentario
  no lo comprueba nadie, y dos copias de una versión se desincronizan sin que falle nada.
- **Un fichero por versión**, como fue `.graalvm-version`: un solo fichero, con su guarda, cubre
  las demás sin inventar un formato para cada una.

## Consequences

- La casilla de linting del ADR-0013 decía `eslint`; queda sustituida por Biome.
- Nada de esto se construye en este ticket: hoy no hay una línea de Rust ni de TypeScript que
  lintear. Llega con los sub-issues de #10, y `docs/agents/code-host.md` sigue describiendo lo
  que el CI comprueba **hoy** hasta entonces.
- `just check` crece por dentro (Biome, clippy, `fmt --check`, `cargo test --no-run` de la grada
  C, `cargo crap`); su nombre y su papel no cambian, que es el contrato del ADR-0013.
- `coverage` deja de ser solo informativa: falla si la cobertura de líneas baja de
  `coverage_floor`. `diff-coverage`, que sí pide red, corre aparte en el CI y no entra en
  `check-rust` ni en `just check`.
