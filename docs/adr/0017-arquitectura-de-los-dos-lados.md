# La arquitectura de los dos lados: puertos en la ventana, capas en el backend

Dieciséis ADR y ninguno decía **dónde va el código que vas a escribir**. La palabra «puerto»
no aparecía en toda la documentación: vivía solo en comentarios de módulo, y la dirección de
las dependencias del backend no vivía en ningún sitio —tanto es así que dos ciclos,
`signing ↔ ffi` y `destination ↔ memory`, estuvieron ahí meses sin que nadie se enterara,
porque Rust no vigila los ciclos dentro de un crate—. Este ADR escribe lo que los dos lados ya
practican y le pone un nombre, para que la pregunta «¿dónde pongo esto?» tenga respuesta
deducible sin abrir un solo fichero.

## La ventana: puertos

La interfaz **no habla con Tauri**. Habla con **puertos**: interfaces declaradas en el módulo
de dominio al que pertenecen —`documents/picker.ts` declara `DocumentPicker`,
`signing/certificate.ts` el almacén de certificados, `i18n/preference.ts` el idioma
guardado—. La regla completa son cuatro piezas y no se separan:

- **El puerto vive con su dominio**, no en una carpeta de puertos. Quien lee `documents/`
  encuentra ahí el vocabulario de la bandeja y por dónde entra un documento, junto.
- **Un adaptador por puerto**, todos en `tauri.ts`, que es la otra cara de `commands/mod.rs`.
- **Un doble por puerto**, exportado al lado del puerto (`inMemoryDocumentPicker`,
  `inMemoryRecents`, `emptyRubricPicker`). Las pruebas de la ventana no conocen a Tauri, y por
  eso corren en Vitest sin backend.
- **El cableado en un solo sitio**: `main.tsx` elige qué implementa cada puerto y nadie más lo
  hace (ID-75). Sustituir un doble en memoria por su adaptador real es una línea, ahí.

Al añadir capacidad nueva el orden es: **el puerto en su módulo de dominio → `tauri.ts` →
`main.tsx`**. Esa frase, en seis líneas, es «La regla del puerto» del mapa de la interfaz.

## El backend: capas con una dirección

El equivalente en Rust no son puertos por sistema —Rust tiene `trait`, pero un `trait` por
dependencia sería ceremonia sin comprador—. Los que hay viven en el `ports.rs` de su contexto y
existen porque tienen **dos compradores reales**: el adaptador de producción y el doble en
memoria de las pruebas. Son los que tocan el mundo desde un caso de uso: el puente y su hilo
(`Bridge`, `IsolateHost`), los dos motores que presta (`FilterEngine`, `PolicyEngine`), el
token (`Token`), la carga de NSS (`NssHost`), y en la sede el códec, el transporte, los
almacenes de confianza y las ranuras de la CA local (`ProtocolCodec`, `Transport`,
`TrustStores`, `LocalCaSlots`). Un puerto habla solo en tipos de `domain/`; por eso la CA local
es dominio de `site/` aunque la fabrique `openssl`: es material puro que no toca el disco.

Un puerto se pasa como `&dyn` salvo cuando su método es genérico: `IsolateHost::run` recibe un
cierre que produce un `T`, no es compatible con `dyn`, y los casos de uso lo reciben como
`impl IsolateHost` (y `ErrandDesk` lo lleva como parámetro de tipo, igual que los dos motores).
Se descartó el envoltorio con `Box<dyn Any>` que lo habría hecho compatible con `dyn`: costaba
un `downcast` en cada uso para ahorrar un parámetro de tipo que ya existía para los motores.

Lo que sí es de este ADR es **una dirección**, la del ID-81, y va siempre hacia dentro:

```
commands/  →  app/  →  dominio e infraestructura
```

- **`commands/`** es el adaptador de Tauri y la capa más externa. Desempaqueta el `State`,
  llama a `app/` y traduce el resultado a lo que cruza a la ventana (ID-79). Si lo que estás
  escribiendo dentro de una orden no es una de esas tres cosas, está en el fichero equivocado.
- **`app/`** son los **casos de uso**, y es la interfaz por la que se prueba el backend (ID-77,
  TD-20). Reciben sus dependencias **explícitas** —`Environment` es la raíz de composición, y
  cada caso de uso toma por argumento las memorias, los almacenes y el isolate que necesita—,
  así que una prueba les pasa un `Environment` de andamio y no monta una aplicación de Tauri.
- **Dominio e infraestructura** —`signing/`, `destination/`, `rubric/`, `memory/`, `pkcs11/`,
  `ffi.rs`, `paths.rs`— **no nombran a `app/` ni a `commands/`**. Reciben valores ya decididos.

Entre módulos de dominio la dirección es la misma idea: **el que sabe menos no nombra al que
sabe más**. `ffi` puede importar `signing::SessionSeal` porque es infraestructura mirando al
dominio; `signing` no puede importar `ffi`, porque el ciclo trifásico es un caso de uso y vive
en `app/cycle.rs` (ID-82). `memory` puede importar `destination::DestinationFolder`;
`destination` no puede importar `memory`, porque desenvolver la configuración lo hace `app/`
(ID-83).

No hay excepciones escritas. Un caso de uso devuelve **dominio** —su enumerado de situación,
sus tipos— y quien lo traduce a lo que cruza a la ventana es el adaptador de Tauri de su
contexto, en `adapters/views.rs` y `adapters/failures.rs`. Cada situación se traduce **una sola
vez**: del mismo `match` salen la vista que recibe la ventana (`Failure`) y el código de cable
que recibe la sede (`SafCode`), así que una variante nueva sin decidir vista y código no
compila. Lo que `app/` tampoco puede nombrar es un **cuerpo de orden**: eso sería el caso de
uso llamando a su propio adaptador, y la guarda lo rechaza.

## La regla se vigila, no se recuerda

Una regla que solo está escrita dura hasta el primer `use` cómodo, y la prueba está en los dos
ciclos. `src-tauri/tests/module_directions.rs` es una guarda de **grada A** (ADR-0014) que lee
las líneas `use crate::` de los ficheros versionados de `src/` y falla ante cualquier arista
que apunte contra esta dirección. Cuando falla nombra la arista sobrante, dice hacia dónde
debería ir, y dice **que no se relaje la regla ni se le añada una excepción, sino que se mueva
la decisión** —casi siempre a `app/`—.

Mira solo la mitad de producción de cada módulo: un `use` dentro de `#[cfg(test)]` no
participa en el grafo que se compila y no cierra ningún ciclo. Y mira **líneas `use`**, no
caminos absolutos escritos en medio de una expresión, que es un agujero conocido y estrecho.

### Sin `cargo-pup`

Se evaluó `cargo-pup` como forma declarativa de expresar esta regla. Encaja bien: sus reglas de
módulo dicen justo esto y se ejecutan como prueba de integración. Se descarta porque exige un
**toolchain nightly fijado a una fecha** con `rustc-dev`, `rust-src` y `llvm-tools-preview`, y
este repositorio compila en estable y no tiene fichero de toolchain. Meter un nightly fijado
para vigilar una regla de cuatro líneas cambia una deuda barata por una cara: cada
actualización del toolchain pasa a ser un trabajo, y el CI gana una segunda cadena que
mantener. Se **reconsiderará si `cargo-pup` llega a funcionar en estable**; mientras tanto la
guarda se escribe a mano, que además da el mensaje de fallo en castellano y con el ID de la
decisión dentro.

### Sin tipos de frontera en los casos de uso

La primera versión de esta regla tenía una excepción escrita: `app/` podía nombrar los tipos
de frontera de `commands/` —las vistas y `Failure`— porque el caso de uso los producía. Se
descarta porque obligaba a traducir cada situación **dos veces**, en dos tablas que nadie
mantenía a la par: la de la ventana en `commands/failure.rs` y la del cable en la frontera
de sede; y porque una excepción a tres caminos es la puerta por la que entra la cuarta. Con
la traducción en el adaptador de cada contexto, la regla se lee de la ruta y no necesita
excepción (#440).

## Lo que este ADR **no** decide

Para que no se le atribuyan decisiones que no toma:

- **No decide cómo se prueba nada.** Las gradas de prueba y la puerta de calidad son el
  ADR-0014, y siguen siendo suyas.
- **No introduce `trait` de puerto por sistema en el backend**, ni inyección de dependencias, ni
  un contenedor. Las dependencias de un caso de uso son argumentos de función y, salvo los
  puertos con doble en memoria, tipos concretos. Cuál se declara y dónde lo dice cada spec, no
  este ADR; lo que este ADR añade es que **el caso de uso no nombra a ningún adaptador**, y la
  guarda de dirección lo vigila en todo el crate (RD-03 del #408).
- **No dice cuántos módulos hay ni cómo se llaman.** Eso lo dice el mapa
  (`src-tauri/src/AGENTS.md`), que se actualiza en la misma PR que crea un módulo.
- **No dice dónde vive un tipo de salida.** Eso es el ID-80: en `commands/views.rs`.
- **No decide nada sobre la frontera FFI ni sobre la memoria** —son el ADR-0003 y el ADR-0010—;
  solo dice desde qué lado se las nombra.

## Consecuencias

Un agente con el contexto limpio puede decidir dónde va lo que va a escribir leyendo seis
líneas del mapa del backend, sin abrir un fichero. Y si se equivoca, el PR sale en rojo con el
nombre de la arista y la dirección correcta en el mensaje, en vez de compilar y quedarse.

El coste es que la guarda hay que mantenerla: una carpeta nueva de dominio no necesita nada
—cae en la capa por defecto—, pero renombrar `app/` o `commands/` deja la guarda ciega. Por eso
tiene una prueba hermana que exige que **cada capa tenga módulos**, y otra que exige que las
direcciones permitidas —`ffi → signing`, `memory → destination`— sigan existiendo: una guarda
que no encuentra nada que vigilar es una guarda que miente en verde.
