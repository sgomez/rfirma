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
rfirma en `tests/conformance_bench.rs`. El sujeto es **nuestro** canal; el cliente publicado es
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
y el carril lento las ejecuta con `--ignored` en `test-native` para no repetir los tests unitarios. Descartada una *feature* de cargo, que
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

## La prueba del ciclo completo tiene dueño

Es el **único sub-issue vertical** de #10: el último de la cadena, bloqueado por todos los
módulos, y su entregable es `tests/native_cycle.rs` más el paso de `pdfsig`. Necesita el puente
Java, la FFI, PKCS#11, el PDF y `pdfsig` a la vez, así que no pertenece a ningún módulo del
corte horizontal. Su cuerpo tiene que decir **por qué** es la excepción, o el siguiente agente
que lea el spec creerá que el corte se rompió por descuido.

## Herramental

**Rust**: `cargo clippy -D warnings` y `cargo fmt --check` dentro de `lint`; `cargo test`;
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
(`just crap-ffi`), ejecutando el ciclo de grada C bajo `llvm-cov` y comprobando con `cargo crap --path`
que ninguna función del adaptador FFI supera el umbral de 30 sin repetir la medición de toda la suite.

**`--allow` corrige una cobertura que se mide en otro carril; no perdona a un módulo por ser
difícil de probar.** La distinción es la que sostiene la puerta entera: el módulo FFI se oculta
en el carril rápido porque **sí está probado**, sólo que en la grada C, y el carril lento lo
vuelve a medir con `crap-ffi`. Un `--allow` concedido a un módulo que nadie mide en ningún
carril —«es entrada/salida», «es fontanería»— desactiva la puerta por precedente, y el segundo
entra solo. Si un módulo de entrada/salida no baja de 30, la conversación es **sobre el corte
del módulo**, no sobre el umbral: que la parte con lógica sea una capa aparte y probable es
exactamente la señal que la métrica existe para dar.

### Puerta absoluta, sin trinquete

Umbral fijo, **sin `--baseline` ni `--fail-regression`**. El trinquete exige versionar un JSON
que cambia en casi cada PR, y eso son conflictos de merge en un fichero generado. Su única
ventaja —amnistiar deuda existente— no aplica: hoy el repositorio tiene **cero líneas de Rust**.

> El argumento original apoyaba esto en el `execution: parallel` que entonces declaraba
> `developer-defaults.md`. Con `sequential` ese apoyo se debilita, pero la decisión se sostiene
> sola sobre el segundo motivo, que es el fuerte: no hay deuda que amnistiar.

### El riesgo de la herramienta, dicho en voz alta

`cargo-crap` tiene **cuatro meses** (primera versión 2026-04-27) y **un solo mantenedor**. Se
instala con `cargo binstall` a una **versión fijada** en el `justfile`. Si se abandona, la puerta
se quita en una línea y no arrastra nada: es una comprobación aparte, no un formato que impregne
el código.

## Un solo hook: formato, antes del push

`pre-push` con **lefthook**, y dentro **solo formato**: `cargo fmt --all -- --check`, el
formateador de biome y `ruff format --check`, filtrados por glob y en paralelo, de modo que una
push que no toca Rust no arranca cargo. Ni clippy, ni pruebas, ni nada que compile o dependa de
`build-ts`: la puerta se mide en segundos o no sobrevive. No comprueba nada que `just check` no
comprobara ya, y `just check` sigue siendo el único punto de entrada que promete
`docs/agents/code-host.md`.

El gestor va como dependencia de desarrollo de `rfirma-app`, a versión exacta por la misma razón
que `cargo-crap`, y lo instala el `prepare` de `package.json`: cualquier `pnpm install` —`just
deps` incluido— la deja puesta. `bootstrap.sh` no crece (ADR-0013). La configuración es
`lefthook.yml` en la raíz.

Cada trabajo se salta solo, con un aviso y sin bloquear, si su herramienta no está: `cargo` y
`ruff` no viven en el `PATH` de una shell no interactiva, que es lo que es un hook de git. Cuando
falla, el mensaje nombra la receta que lo arregla —`just fmt`, y `fmt-rust`, `fmt-ts` y
`fmt-python`, que escriben, frente a las `lint-*`, que comprueban—.

`ruff format` es la única de las tres que el CI no ejecuta: la cadena de Python es `ruff check`,
que es otra cosa. La puerta local es aquí más estricta a propósito, acotada por su glob a
`packaging/**.py`.

### Considered Options

**Ningún hook**, que es lo que hubo hasta ahora: se temía que uno desconocido se esquivara con
`--no-verify` o explotara sin que nadie entendiera por qué. Lo primero se acepta —detrás está el
CI, que no se esquiva, y adelantarse a un fallo barato no exige ser infranqueable—; lo segundo lo
cierran el aviso y el mensaje de arriba.

## Dónde corre cada puerta: el agente no es el CI

`just check` es la puerta entera, y **su sitio es el CI**, que la reparte en tres runners
simultáneos y por tanto paga el carril más lento. En un portátil se pagan los tres sumados, y
un agente que la repite tras cada arreglo convierte el gasto dominante de una ronda en algo que
ya iba a correr solo. Medido en el equipo de desarrollo, con cachés calientes: `check-repo` 4 s,
`check-java` 4 s, `check-ts` 15 s, `check-rust` 46 s.

La escalera es de tres peldaños y la escribe `AGENTS.md`, que es donde un agente la lee:

1. **Cada rojo → verde**: sólo la prueba que se está tocando. Nunca una receta de `check`.
2. **Antes de commitear**: `just fmt` y **`just check-changed`**, una vez.
3. **Al revisar**: nada, si el CI está verde para ese head sha.

`check-changed` deduce los carriles de lo que la rama toca respecto a `origin/main`. `check-repo`
entra siempre —son cuatro segundos—, y el `justfile`, `.github/` y `bootstrap.sh` disparan las
tres cadenas, porque son justo los ficheros que pueden romper cualquiera. `docs/adr/` y los
`AGENTS.md` entran por el carril de Rust y no por descuido: sus guardas son pruebas de la grada A
y viven ahí, aunque lo que las rompe sea prosa.

**Esto no relaja nada.** La puerta que decide sigue siendo `just check` entera, corriendo en el
CI sobre el head sha; lo que cambia es que deje de correrse tres veces en el sitio donde más
cuesta y menos decide.

### El árbol de compilación se comparte entre worktrees, y el CI no se toca

Cada agente constructor trabaja en un worktree propio, así que cada uno recompilaba el árbol de
dependencias de Tauri desde cero: medidos, **73 s y entre 6,8 y 13 GB por worktree**. El
`justfile` apunta `CARGO_TARGET_DIR` a `.claude/worktrees/target` **cuando corre desde un
worktree**: el primer agente paga la compilación entera una vez y los demás entran en **11 s**.

**El checkout principal se queda fuera.** Cargo toma un cerrojo sobre el `target/` mientras
compila, y meterlo dentro haría que un `cargo` a mano esperase a que terminara el agente de
turno. Los agentes sí se serializan entre sí, y con `execution: sequential` eso no cuesta nada;
si algún día se vuelve a `parallel`, esta es la línea que hay que volver a mirar.

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

## Consequences

- La fila `lint` del ADR-0013 decía `eslint`; queda sustituida por Biome.
- Nada de esto se construye en este ticket: hoy no hay una línea de Rust ni de TypeScript que
  lintear. Llega con los sub-issues de #10, y `docs/agents/code-host.md` sigue describiendo lo
  que el CI comprueba **hoy** hasta entonces.
- `just check` crece por dentro (Biome, clippy, `fmt --check`, `cargo test --no-run` de la grada
  C, `cargo crap`); su nombre y su papel no cambian, que es el contrato del ADR-0013.
