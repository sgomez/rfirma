# Mapa del backend (Rust / Tauri)

Este índice **sustituye a explorar el árbol**. Localiza el módulo por su línea,
abre **solo** ese fichero, y solo el tramo que necesitas.

`tests/agents_map_is_complete.rs` comprueba que aquí está listado, por su ruta,
todo `.rs` versionado bajo `src/`. **Un módulo nuevo se añade a esta tabla en la
misma PR que lo crea**, o el PR sale en rojo.

## Presupuesto de lectura

- **Para situarte, `just outline <ruta>`; nunca `cat` de un módulo de más de 300
  líneas.** El esqueleto trae cada `fn`, `struct` y prueba con su número de línea
  y la primera línea de su `///`; desde ahí, `sed -n 'A,Bp'` para el tramo.
  `commands/guards.rs` entero son 14 840 caracteres (~4,2k tokens) y su esqueleto
  3149. La tabla de abajo da el tamaño de cada módulo antes de que lo abras.
- **Los tests van al final de cada módulo**, tras `#[cfg(test)]`. No los leas
  salvo que vayas a tocarlos. Para saber qué cubren sin leerlos:
  `awk '/#\[cfg\(test\)\]/,0' <fichero> | grep -n '    fn '` — los nombres son
  frases en inglés y dicen la invariante entera.
- El fichero más grande del backend es `ffi.rs`, con 993 líneas; detrás van
  `signing/placement.rs` (664), `app/documents.rs` (643), `app/signing.rs` (604)
  y `rubric/normalize.rs` (586). Ninguno de `commands/` pasa de 400 y así se
  quedan: cuando uno crece, lo que ha entrado casi siempre es una decisión, y
  una decisión va en `app/`.
- El primer bloque `//!` de cada módulo es su contrato. `head -40 <fichero>` casi
  siempre basta para decidir si es el fichero que buscas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `main.rs` | 8 | El binario. No hay nada dentro. |
| `lib.rs` | 122 | Registro de comandos y estados de Tauri. Empieza aquí para ver el cableado. |
| `isolate.rs` | 179 | El hilo dueño del isolate de GraalVM. |
| `ffi.rs` | 993 | La frontera FFI: cargar `librfirma_crypto.so` y volver sin fugas. |
| **`commands/`** | | El adaptador de Tauri: desempaqueta, llama a `app/` y traduce (ID-79). |
| `commands/mod.rs` | 319 | **Las catorce órdenes de Tauri**, y nada más que sus cuerpos. |
| `commands/views.rs` | 328 | Los tipos que cruzan a la ventana y las conversiones que los producen (ID-80). |
| `commands/failure.rs` | 181 | Cómo se le cuenta a la ventana que algo salió mal (ID-29). |
| `commands/orders.rs` | 138 | Lo que la ventana manda, ya deserializado. |
| `commands/guards.rs` | 394 | Las cuatro guardas que ven todas las órdenes a la vez (ID-85), y las pruebas del descubrimiento de tipos. Solo en pruebas. |
| **`app/`** | | Los casos de uso. Es la interfaz por la que se prueba (ID-77, TD-20). |
| `app/mod.rs` | 183 | El reparto, `Environment` —la raíz de composición— y la carpeta de destino elegida (ID-83). Léelo antes que sus hermanos. |
| `app/cycle.rs` | 432 | El ciclo trifásico: prefirma Java, firma Rust, postfirma Java. El único caso de uso que cruza la FFI (ID-82). |
| `app/certificates.rs` | 420 | Qué certificados hay, cuál eligió la ventana y cuál se recordó. |
| `app/signing.rs` | 604 | El recorrido de la firma en tres pasos y la sesión a medias. |
| `app/documents.rs` | 643 | Por dónde entra el documento y dónde cae el firmado. |
| `app/recents.rs` | 539 | La bandeja, del disco a la ventana: quién la lee, quién la escribe y el reparto del recuadro (ID-74, ID-75). |
| `app/configuration.rs` | 256 | Los ajustes, del disco a la ventana y de vuelta. |
| `app/fixtures.rs` | 74 | Los andamios que comparten las pruebas de `app/`. Solo en pruebas. |
| `paths.rs` | 536 | Las tres rutas de la memoria entre sesiones. Único sitio que conoce el sistema operativo (ADR-0010). |
| `dropped.rs` | 185 | Qué se decide al soltar ficheros en la ventana (ID-67, ID-68, ID-70). |
| **`memory/`** | | Lo que rFirma recuerda: seis memorias en dos mitades (ADR-0010). |
| `memory/mod.rs` | 427 | El reparto de las seis memorias. Léelo antes que sus hermanos. |
| `memory/state.rs` | 341 | El estado que la aplicación acumula por su cuenta (ID-31), y lo **global** de la firma visible (ID-74). |
| `memory/configuration.rs` | 154 | Lo que el usuario elige y la aplicación obedece. |
| `memory/recents.rs` | 482 | Los diez recientes, por ruta canónica, con la página y la posición del recuadro de cada uno (ID-74). |
| `memory/store.rs` | 463 | El fichero JSON versionado que soporta las dos memorias. |
| `memory/opened.rs` | 179 | Los documentos abiertos en esta sesión: del identificador opaco al fichero. |
| `memory/listed.rs` | 168 | Los certificados listados en esta sesión: del asa opaca a la referencia. |
| `memory/handles.rs` | 90 | Cómo se acuña un asa opaca (ID-61, ADR-0011). |
| `memory/error.rs` | 89 | Situaciones de la memoria (ADR-0009). |
| **`signing/`** | | Las reglas puras de la firma. |
| `signing/mod.rs` | 30 | El reparto. Qué se le pide al puente y qué se le exige de vuelta. **No importa `ffi`** (ID-82). |
| `signing/config.rs` | 321 | Los cinco ajustes de firma y ni uno más (ID-18). Aquí vive `SignatureBox`. |
| `signing/placement.rs` | 664 | Del recuadro arrastrado en el visor al `/Rect` del PDF (ID-21). |
| `signing/admissibility.rs` | 316 | Lo que no se puede firmar, decidido antes del PIN. |
| `signing/layer2_text.rs` | 369 | El texto del recuadro visible. |
| `signing/properties.rs` | 178 | Los `extraParams` en el formato del puente. |
| `signing/session_seal.rs` | 152 | El sello de sesión: una invariante entre prefirma y postfirma (ADR-0016). |
| `signing/language.rs` | 98 | Los seis idiomas (ADR-0009). |
| **`pkcs11/`** | | La única parte que habla con el token. |
| `pkcs11/mod.rs` | 572 | La capa PKCS#11. |
| `pkcs11/stores.rs` | 553 | Dónde se buscan los certificados. |
| `pkcs11/certificate.rs` | 411 | El certificado tal y como sale del token. |
| `pkcs11/error.rs` | 233 | Situaciones del token (ID-29, ADR-0009). |
| **`destination/`** | | Dónde cae el firmado y por dónde entra el original (ADR-0011). |
| `destination/mod.rs` | 353 | El reparto, y `DestinationFolder`. **No importa `memory`** (ID-83). |
| `destination/naming.rs` | 190 | Cómo se llama el firmado y qué pasa si el nombre existe. |
| `destination/portal.rs` | 207 | El documento tal y como entra por el portal (ID-37). |
| `destination/error.rs` | 114 | Situaciones del destino (ADR-0009). |
| **`rubric/`** | | De lo que aporta el usuario al JPEG que acepta el puente (ADR-0012). |
| `rubric/mod.rs` | 33 | El reparto. |
| `rubric/normalize.rs` | 586 | La normalización. |
| `rubric/store.rs` | 289 | Se copia, no se referencia (ID-33). |
| `rubric/error.rs` | 92 | Situaciones de la rúbrica (ADR-0009). |

## La regla de la dirección

Las dependencias van **hacia dentro**: `commands/` → `app/` → dominio e
infraestructura. Ningún módulo de dominio nombra a `app/` ni a `commands/`, y
entre hermanos el que sabe menos no nombra al que sabe más (`ffi` importa
`signing`, no al revés). Si vas a añadir capacidad nueva, el orden es: la regla
pura en su módulo de dominio → el caso de uso en `app/` → el cuerpo de la orden
en `commands/`. Lo vigila `tests/module_directions.rs` (ADR-0017).

## Al añadir o cambiar una orden de Tauri

El cuerpo de la orden va en `commands/mod.rs` y **lo que decide, en `app/`**: si
lo que estás escribiendo dentro de la orden no es desempaquetar el `State` ni
traducir el resultado, está en el fichero equivocado (ID-79).

Las cuatro guardas de conjunto están juntas en `commands/guards.rs`, y solo dos
piden algo de ti:

- **La lista cerrada de órdenes** hay que renumerarla. El nombre de la prueba
  **ya no lleva el número dentro** (TD-11): el conteo vive en la aserción de
  `the_list_of_commands_is_closed_and_this_is_how_long_it_is`, porque cambiar el
  número es la información y renombrar la prueba en cada sub-issue no dice nada.
- **La lista de ficheros del módulo** (`SOURCES`) hay que ampliarla si creas un
  fichero nuevo dentro de `commands/`; una guarda propia se pone roja si se te
  olvida.
- La **guarda de rutas** ya no hay que tocarla: descubre sola todo tipo que
  derive `Serialize`, esté en el fichero de `commands/` que esté (ID-84).
- La del **hilo del portal** tampoco: un comando que llame a un `blocking_*` de
  un plugin necesita `#[tauri::command(async)]`, y ella lo vigila.

Y una prueba nueva no se escribe contra la orden, sino contra el caso de uso de
`app/` al que llama (TD-21).

## Las pruebas que se leen a sí mismas

Cuatro ficheros vigilan invariantes leyendo el código **como texto**, no
ejecutándolo: `app/cycle.rs`, `app/signing.rs`, `commands/guards.rs` y
`tests/module_directions.rs`. Abren el `.rs` con `include_str!` y buscan cadenas
dentro. Antes de tocar uno de esos módulos, o las guardas mismas, dos cosas:

- **`production_half` corta por `"\nmod tests {"`**, así que todo lo que quede
  **después** del módulo de pruebas es invisible para la guarda. Si estás
  comprobando a mano que una guarda falla como debe, coloca el tipo o la
  llamada de mentira **antes** del `mod tests`: detrás pasa en verde sin que
  nadie lo mire, que es justo el fallo que estas pruebas existen para evitar.
- **Algunas aserciones cuentan apariciones literales** —
  `assert_eq!(cycle.matches("bridge.").count(), 2)` en `app/cycle.rs` es la que
  sostiene el ADR-0001: una tercera llamada al puente sería la clave privada
  cruzando a Java. Mover código entre módulos que llevan una de estas pruebas
  obliga a elegir entre reescribir sus aserciones o mudar el fichero entero.
  Decídelo al planificar el cambio, no al final.
