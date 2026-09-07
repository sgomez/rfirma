# La arquitectura de los dos lados: puertos en la ventana, contextos con capas en el backend

Dieciséis ADR y ninguno decía **dónde va el código que vas a escribir**. La palabra «puerto»
no aparecía en toda la documentación, y la dirección de las dependencias del backend no vivía
en ningún sitio —tanto es así que dos ciclos, `signing ↔ ffi` y `destination ↔ memory`,
estuvieron ahí meses sin que nadie se enterara, porque Rust no vigila los ciclos dentro de un
crate—. Este ADR escribe lo que los dos lados practican y le pone un nombre, para que la
pregunta «¿dónde pongo esto?» tenga respuesta deducible **leyendo una ruta**, sin abrir un solo
fichero.

## La ventana: puertos

La interfaz **no habla con Tauri**. Habla con **puertos**: interfaces declaradas en el módulo
de dominio al que pertenecen —`documents/picker.ts` declara `DocumentPicker`,
`signing/certificate.ts` el almacén de certificados, `i18n/preference.ts` el idioma
guardado—. La regla completa son cuatro piezas y no se separan:

- **El puerto vive con su dominio**, no en una carpeta de puertos. Quien lee `documents/`
  encuentra ahí el vocabulario de la bandeja y por dónde entra un documento, junto.
- **Un adaptador por puerto**, todos en `tauri.ts`, que es la otra cara de las órdenes que
  cada contexto del backend publica en su `adapters/tauri.rs`.
- **Un doble por puerto**, exportado al lado del puerto (`inMemoryDocumentPicker`,
  `inMemoryRecents`, `emptyRubricPicker`). Las pruebas de la ventana no conocen a Tauri, y por
  eso corren en Vitest sin backend.
- **El cableado en un solo sitio**: `main.tsx` elige qué implementa cada puerto y nadie más lo
  hace. Sustituir un doble en memoria por su adaptador real es una línea, ahí.

Al añadir capacidad nueva el orden es: **el puerto en su módulo de dominio → `tauri.ts` →
`main.tsx`**. Esa frase, en seis líneas, es «La regla del puerto» del mapa de la interfaz.

## El backend: cinco contextos

El primer nivel de `src-tauri/src/` se parte por **lo que el producto hace**, no por mecanismo
ni por capa. Son los cinco contextos que ya nombra el glosario (`CONTEXT.md`), y cada uno es
una carpeta con su propio mapa:

| Contexto | Qué es |
|---|---|
| `site/` | El trámite de sede: el protocolo `afirma://`, el canal sobre el *loopback*, el material TLS, la confianza de la CA local y el arranque que decide si se abre la ventana principal o la del trámite. |
| `signing/` | La firma local: las reglas puras de admisibilidad y colocación, el ciclo trifásico, la sesión en curso, el puente FFI y el hilo del aislado, y la memoria entre sesiones. |
| `documents/` | Por dónde entra el documento y dónde cae: destino, soltados, abiertos, recientes, rúbrica y el documento en curso. |
| `identity/` | Quién firma: PKCS#11, los certificados que hay, los listados con su asa y el `.p12`. |
| `desktop/` | El escritorio de la persona: canal de distribución, manejadores de `afirma://`, invocación desde fuera, versión publicada y rutas de la máquina. |

Fuera de los cinco solo cuelgan de la raíz `lib.rs` (la composición), `main.rs`,
`crossing.rs` (el rasgo y el registro de lo que cruza a la ventana, con `Failure`, que todos
los contextos producen y ninguno posee) y `compile_fail.rs` (los cebos de compilación
negativa). Nada más: un fichero nuevo en la raíz es un contexto que no se ha decidido.

## Las cuatro capas, por nombre de carpeta

Dentro de cada contexto la capa **es la carpeta**, y por eso la regla se lee sin conocerla:

- **`domain/`**: reglas puras y tipos del contexto. No nombra nada del crate fuera de sí
  mismo. Un `LocalCa` es dominio de `site/` aunque lo fabrique `openssl`: es material que no
  toca el disco.
- **`ports.rs`**: los `trait` del contexto. Existen porque tienen **dos compradores reales**,
  el adaptador de producción y el doble en memoria de las pruebas, y hablan solo en tipos de
  `domain/`. Un puerto se pasa como `&dyn` salvo cuando su método es genérico: `IsolateHost::run`
  recibe un cierre que produce un `T`, no es compatible con `dyn`, y los casos de uso lo
  reciben como `impl IsolateHost` (y `ErrandDesk` lo lleva como parámetro de tipo, igual que
  los dos motores). Lo que un contexto necesita de otro dentro de un caso de uso entra
  también por un puerto propio —`Certificates`, `ScratchDocuments`, `SiteSigning` en
  `site/ports.rs`— que sirve un adaptador sobre la raíz del vecino.
- **`application/`**: los casos de uso, y la interfaz por la que se prueba el backend. Reciben
  sus dependencias explícitas —dominio y `&dyn Puerto`— y devuelven **dominio**: su enumerado
  de situación, sus tipos. Su hermano `application/tests.rs` guarda los dobles de los puertos
  del contexto que comparten las gradas A de todos.
- **`adapters/`**: todo lo que toca el mundo, **incluidas las órdenes y las vistas de Tauri**
  (`tauri*.rs`, `views*.rs`, `orders*.rs`) y la traducción de cada situación a lo que cruza
  (`failures.rs`). Cada situación se traduce **una sola vez**: del mismo `match` salen la
  vista que recibe la ventana (`Failure`) y el código de cable que recibe la sede
  (`SafCode`), así que una variante nueva sin decidir vista y código no compila.

**El mundo entra solo por puertos.** `domain/`, `ports.rs` y `application/` no nombran
`std::fs`, `std::env`, `std::process`, `libloading`, `tauri` ni `tauri_plugin_*`: el disco, el
entorno del proceso y los tipos de la ventana llegan como hechos ya resueltos o tras un
`&dyn Puerto` cuyo adaptador vive en `adapters/`. El dominio no llega ni a eso: recibe el
hecho —una ruta ya canónica, un `FolderFact`, el instante de modificación— y decide sobre él;
quien pregunta al disco es el caso de uso. Un puerto tampoco habla en infraestructura: si su
firma necesitaba una `libloading::Library`, es que su comprador era otro adaptador y el rasgo
pertenece a `adapters/`. Esto lo vigila la misma guarda de dirección, con una regla textual,
porque un `std::fs::read` no se ve en el grafo de `use crate::`.

Cada contexto tiene su **raíz de composición** en `<contexto>/mod.rs`: el `struct` que
junta sus adaptadores, su estado de proceso y sus puertos ya instanciados, y la fachada con
la que los adaptadores de otro contexto le piden lo que necesitan. La raíz global es `lib.rs`:
construye las cinco en orden de dependencia sobre la misma memoria, las registra con cinco
`manage()`, y es **el único sitio que junta contextos**.

## La regla de la dirección, aplicada por rutas

```
domain/   →  nada del crate
ports.rs  →  domain/ (propio y ajeno), ports.rs propio
application/  →  domain/ y ports.rs propios, domain/ de otros
adapters/ y mod.rs  →  lo que quiera del propio contexto; de otro, su domain/, sus adapters/ y su raíz
```

Y una sola prohibición cruzada: **un contexto no importa `application/` de otro**. Los casos
de uso de un vecino llegan ya decididos desde la raíz de composición, o por un puerto propio.
Lo que cuelga de la raíz del crate no pertenece a ningún contexto y cualquiera lo nombra.

No hay excepciones escritas ni lista de deuda. Lo vigila `src-tauri/tests/module_directions.rs`,
una guarda de **grada A** (ADR-0014) que lee las líneas `use crate::` de los ficheros
versionados de `src/`, deduce contexto y capa de la ruta de cada uno, y falla ante cualquier
arista que apunte contra la tabla, y ante cualquier línea de `domain/`, `ports.rs` o
`application/` que nombre el mundo. Cuando falla nombra la arista y dice **adónde mover la
decisión** —casi siempre a `application/`, o a un puerto—, y se prueba a sí misma con árboles
sintéticos: cada combinación prohibida de capa y contexto la pone en rojo. Mira solo los
ficheros de producción —un `use` en un `tests.rs` no participa en el grafo que se compila— y
solo líneas `use`, no caminos absolutos escritos en medio de una expresión, que es un agujero
conocido y estrecho.

Las invariantes que no son de dirección también se sostienen sin leer texto donde el sistema
de tipos alcanza: la postfirma solo acepta una `SealedPreSignature`, que solo sale de una
`PreSignature` con la firma del token y el sello intacto; el sello de firmado de la bandeja
exige un `CompletedCycle`, que solo devuelve la postfirma; `SafCode` no se construye desde
una cadena; y un tipo cruza a la ventana solo si es un `WindowCrossing`, que solo escribe
`crossing!`. Cada una tiene su cebo `compile_fail` en `compile_fail.rs`. De ese mismo registro
salen el contrato ventana-backend (`just contract`) y la guarda de rutas del ADR-0011: un tipo
de cruce nuevo aparece por declararse, y uno escrito a mano no aparece.

## Considered Options

### Crates separados en un workspace

Un crate por contexto haría que el compilador aplicara la regla de dirección sin guarda. Se
descarta ahora por lo que toca fuera del código: `packaging/flatpak/cargo-sources.json` se
reproduce a mano por cada cambio del `Cargo.lock` y un workspace lo multiplica; el `justfile`
y el CI apuntan a un solo `src-tauri`; `tauri::generate_context!()` y el `bundler` esperan un
crate de aplicación; y el `outline` y los mapas están medidos sobre un árbol. La forma por
contextos con capas visibles deja la puerta abierta —cada carpeta ya tiene la forma de un
crate— y se reabre cuando el coste del empaquetado baje.

### Un `trait` de puerto por dependencia

Se descarta por ceremonia sin comprador: un puerto existe cuando tiene un adaptador de
producción y un doble de pruebas, y los que hay son exactamente esos. Las demás dependencias
de un caso de uso son argumentos de función con tipos concretos de dominio.

### Un envoltorio `Box<dyn Any>` para hacer `IsolateHost` compatible con `dyn`

Costaba un `downcast` en cada uso para ahorrar un parámetro de tipo que ya existía para los
dos motores. Se queda `impl IsolateHost`.

### Los casos de uso nombran los tipos de frontera del adaptador

La primera versión de la regla tenía esa excepción escrita: `app/` podía nombrar las vistas y
`Failure` porque el caso de uso los producía. Se descartó porque obligaba a traducir cada
situación **dos veces**, en dos tablas que nadie mantenía a la par —la de la ventana y la del
cable de sede—, y porque una excepción a tres caminos es la puerta por la que entra la cuarta.
Con la traducción en el `adapters/failures.rs` de cada contexto, la regla se lee de la ruta y
no necesita excepción.

### Una lista de deuda mientras el árbol se movía

Durante el reagrupamiento la guarda toleró una lista de aristas `application → adapters` que
solo podía menguar. Cumplió su función y se borró con la última arista: una lista que puede
crecer es una regla que no existe.

### `cargo-pup`

Encaja bien: sus reglas de módulo dicen justo esto y se ejecutan como prueba de integración.
Se descarta porque exige un **toolchain nightly fijado a una fecha** con `rustc-dev`,
`rust-src` y `llvm-tools-preview`, y este repositorio compila en estable y no tiene fichero de
toolchain. Meter un nightly fijado para vigilar una regla de cinco líneas cambia una deuda
barata por una cara. Se **reconsiderará si `cargo-pup` llega a funcionar en estable**;
mientras tanto la guarda se escribe a mano, que además da el mensaje de fallo en castellano.

## Lo que este ADR **no** decide

- **No decide cómo se prueba nada.** Las gradas de prueba y la puerta de calidad son el
  ADR-0014.
- **No introduce inyección de dependencias ni un contenedor.** Las dependencias de un caso
  de uso son argumentos; las raíces de composición son `struct` que se construyen a mano.
- **No dice cuántos módulos hay ni cómo se llaman.** Eso lo dicen los mapas (el raíz,
  `src-tauri/src/AGENTS.md`, y uno por contexto), que se actualizan en la misma PR que crea
  un módulo, y una guarda exige que todo `.rs` esté en alguno.
- **No decide nada sobre la frontera FFI ni sobre la memoria** —son el ADR-0003 y el
  ADR-0010—; solo dice desde qué capa se las nombra.

## Consecuencias

Un agente con el contexto limpio decide dónde va lo que va a escribir leyendo la ruta del
fichero de al lado. Si se equivoca, el PR sale en rojo con el nombre de la arista y la capa
correcta en el mensaje, en vez de compilar y quedarse.

El coste es que la guarda deduce las capas de cuatro nombres de carpeta: renombrar `domain/`
o `adapters/` la dejaría ciega. Por eso reconoce un contexto solo si tiene al menos una capa
dentro, y tiene pruebas hermanas que exigen que cada capa tenga módulos y que cada
combinación prohibida siga poniéndose roja: una guarda que no encuentra nada que vigilar es
una guarda que miente en verde.
