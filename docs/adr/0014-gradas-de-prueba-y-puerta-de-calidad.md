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
| **D** | red | OCSP en vivo contra el certificado revocado del kit | ni PR ni carril lento: cron |

La grada que sostiene la decisión es la **B**. Sin ella, «probar de verdad» y «tardar cuatro
minutos» serían lo mismo, y la mitad del código de riesgo —PKCS#11— caería al carril que nadie
espera. SoftHSM es de software y se instala en segundos: no hay razón para tratarlo como caro.

La **D** la heredamos de #11 y no se toca: atar cada PR a la disponibilidad de
`sede.fnmt.gob.es` con `merge: auto` significa que un corte ajeno bloquea la entrega. Va a un
job programado que abre una issue si falla.

### Las pruebas de la grada C se compilan siempre y se ejecutan solo en el lento

Se marcan con **`#[ignore]`**, en un fichero que lo dice por su nombre (`tests/ciclo_nativo.rs`),
y el carril lento las ejecuta con `--include-ignored`. Descartada una *feature* de cargo, que
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

## La prueba del ciclo completo tiene dueño

Es el **único sub-issue vertical** de #10: el último de la cadena, bloqueado por todos los
módulos, y su entregable es `tests/ciclo_nativo.rs` más el paso de `pdfsig`. Necesita el puente
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

`cargo crap --lcov lcov.info --fail-above` (umbral **30**, el de Savoia), alimentado por
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
que es exactamente el matiz que hace falta. El **carril lento repite la medición sin esa
exclusión**, y ahí ese módulo da la cara con la cobertura de la grada C incluida.

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

## Sin hook de pre-commit

**Ninguno**, y `just check` sigue siendo el único punto de entrada que promete
`docs/agents/code-host.md`. El [#33](https://github.com/sgomez/rfirma/issues/33) suponía que
`bootstrap.sh` instalaría el gestor de hooks; el ADR-0013, escrito después, decidió que
**`bootstrap.sh` no crece**. Y en un repositorio movido por agentes un hook desconocido se
esquiva con `--no-verify` o explota sin que nadie entienda por qué.

Se añade en su lugar una receta **`just rapido`** —solo `lint`, sin `build` ni `test`— para el
bucle corto de quien quiera formatear antes de commitear. Voluntaria, visible en `just --list`, y
sin tocar el contrato de `check`.

## La bomba de relojería del kit FNMT

`testdata/fnmt/` con los tres `.p12`, sus contraseñas publicadas y sus huellas al lado, más la
excepción para el escáner de secretos que avisó #11. `activo-rsa.p12` **caduca el 2028-10-30**, y
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
