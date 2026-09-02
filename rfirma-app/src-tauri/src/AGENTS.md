# Mapa del backend (Rust / Tauri)

Este índice **sustituye a explorar el árbol**. Localiza el módulo por su línea,
abre **solo** ese fichero, y solo el tramo que necesitas.

`tests/agents_map_is_complete.rs` comprueba que aquí está listado, por su ruta,
todo `.rs` versionado bajo `src/`. **Un módulo nuevo se añade a esta tabla en la
misma PR que lo crea**, o el PR sale en rojo.

## Presupuesto de lectura

- **Nunca `cat` de un fichero de más de 300 líneas.** `grep -n '<símbolo>'` para
  situarte y `sed -n 'A,Bp'` para leer el tramo. La tabla de abajo da el tamaño
  de cada módulo antes de que lo abras.
- **Los tests van al final de cada módulo**, tras `#[cfg(test)]`. No los leas
  salvo que vayas a tocarlos. Para saber qué cubren sin leerlos:
  `awk '/#\[cfg\(test\)\]/,0' <fichero> | grep -n '    fn '` — los nombres son
  frases en inglés y dicen la invariante entera.
- El fichero más grande del backend es `ffi.rs`, con 993 líneas; detrás van
  `signing/placement.rs` (664), `app/documents.rs` (638), `rubric/normalize.rs`
  (586) y `pkcs11/mod.rs` (572). Ninguno de `commands/` pasa de 400 y así se
  quedan: cuando uno crece, lo que ha entrado casi siempre es una decisión, y
  una decisión va en `app/`.
- El primer bloque `//!` de cada módulo es su contrato. `head -40 <fichero>` casi
  siempre basta para decidir si es el fichero que buscas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `main.rs` | 8 | El binario. No hay nada dentro. |
| `lib.rs` | 119 | Registro de comandos y estados de Tauri. Empieza aquí para ver el cableado. |
| `isolate.rs` | 179 | El hilo dueño del isolate de GraalVM. |
| `ffi.rs` | 993 | La frontera FFI: cargar `librfirma_crypto.so` y volver sin fugas. |
| **`commands/`** | | El adaptador de Tauri: desempaqueta, llama a `app/` y traduce (ID-79). |
| `commands/mod.rs` | 252 | **Las once órdenes de Tauri**, y nada más que sus cuerpos. |
| `commands/views.rs` | 283 | Los tipos que cruzan a la ventana y las conversiones que los producen (ID-80). |
| `commands/failure.rs` | 180 | Cómo se le cuenta a la ventana que algo salió mal (ID-29). |
| `commands/orders.rs` | 138 | Lo que la ventana manda, ya deserializado. |
| `commands/guards.rs` | 333 | Las cuatro guardas que ven todas las órdenes a la vez (ID-85), y las pruebas del descubrimiento de tipos. Solo en pruebas. |
| **`app/`** | | Los casos de uso. Es la interfaz por la que se prueba (ID-77, TD-20). |
| `app/mod.rs` | 134 | El reparto y `Environment`, la raíz de composición. Léelo antes que sus hermanos. |
| `app/certificates.rs` | 420 | Qué certificados hay, cuál eligió la ventana y cuál se recordó. |
| `app/signing.rs` | 539 | El recorrido de la firma en tres pasos y la sesión a medias. |
| `app/documents.rs` | 638 | Por dónde entra el documento y dónde cae el firmado. |
| `app/configuration.rs` | 256 | Los ajustes, del disco a la ventana y de vuelta. |
| `app/fixtures.rs` | 74 | Los andamios que comparten las pruebas de `app/`. Solo en pruebas. |
| `paths.rs` | 536 | Las tres rutas de la memoria entre sesiones. Único sitio que conoce el sistema operativo (ADR-0010). |
| `dropped.rs` | 185 | Qué se decide al soltar ficheros en la ventana (ID-67, ID-68, ID-70). |
| **`memory/`** | | Lo que rFirma recuerda: seis memorias en dos mitades (ADR-0010). |
| `memory/mod.rs` | 406 | El reparto de las seis memorias. Léelo antes que sus hermanos. |
| `memory/state.rs` | 210 | El estado que la aplicación acumula por su cuenta (ID-31). |
| `memory/configuration.rs` | 195 | Lo que el usuario elige y la aplicación obedece. |
| `memory/recents.rs` | 406 | Los diez recientes, por ruta canónica. |
| `memory/store.rs` | 463 | El fichero JSON versionado que soporta las dos memorias. |
| `memory/opened.rs` | 154 | Los documentos abiertos en esta sesión: del identificador opaco al fichero. |
| `memory/listed.rs` | 168 | Los certificados listados en esta sesión: del asa opaca a la referencia. |
| `memory/handles.rs` | 90 | Cómo se acuña un asa opaca (ID-61, ADR-0011). |
| `memory/error.rs` | 89 | Situaciones de la memoria (ADR-0009). |
| **`signing/`** | | Las reglas puras de la firma. |
| `signing/mod.rs` | 26 | El reparto. Qué se le pide al puente y qué se le exige de vuelta. |
| `signing/cycle.rs` | 426 | El ciclo trifásico: prefirma Java, firma Rust, postfirma Java. |
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
| `destination/mod.rs` | 346 | El reparto. |
| `destination/naming.rs` | 190 | Cómo se llama el firmado y qué pasa si el nombre existe. |
| `destination/portal.rs` | 207 | El documento tal y como entra por el portal (ID-37). |
| `destination/error.rs` | 114 | Situaciones del destino (ADR-0009). |
| **`rubric/`** | | De lo que aporta el usuario al JPEG que acepta el puente (ADR-0012). |
| `rubric/mod.rs` | 33 | El reparto. |
| `rubric/normalize.rs` | 586 | La normalización. |
| `rubric/store.rs` | 289 | Se copia, no se referencia (ID-33). |
| `rubric/error.rs` | 92 | Situaciones de la rúbrica (ADR-0009). |

## Al añadir o cambiar una orden de Tauri

El cuerpo de la orden va en `commands/mod.rs` y **lo que decide, en `app/`**: si
lo que estás escribiendo dentro de la orden no es desempaquetar el `State` ni
traducir el resultado, está en el fichero equivocado (ID-79).

Las cuatro guardas de conjunto están juntas en `commands/guards.rs`, y solo dos
piden algo de ti:

- **La lista cerrada de órdenes** hay que renumerarla o renombrarla.
- **La lista de ficheros del módulo** (`SOURCES`) hay que ampliarla si creas un
  fichero nuevo dentro de `commands/`; una guarda propia se pone roja si se te
  olvida.
- La **guarda de rutas** ya no hay que tocarla: descubre sola todo tipo que
  derive `Serialize`, esté en el fichero de `commands/` que esté (ID-84).
- La del **hilo del portal** tampoco: un comando que llame a un `blocking_*` de
  un plugin necesita `#[tauri::command(async)]`, y ella lo vigila.

Y una prueba nueva no se escribe contra la orden, sino contra el caso de uso de
`app/` al que llama (TD-21).
