# Las cadenas devueltas por el puente FFI se reservan a mano y las libera Rust

GraalVM Native Image libera automáticamente la memoria de las cadenas C creadas
con `CTypeConversion.toCString(...)` al salir del bloque `try-with-resources`.
Devolver una de esas cadenas a Rust deja un puntero colgante: Rust lee memoria
ya liberada y, al liberarla él, provoca un *double-free*. Por eso el JSON que el
puente devuelve a Rust se reserva **manualmente** en el heap de C con
`UnmanagedMemory.malloc(bytes.length + 1)`, y **Rust es el responsable** de
llamar a `autofirma_free_string(thread, ptr)` una vez leído.

## Consequences

- `CTypeConversion.toCString(...)` sigue siendo correcto para cadenas que no
  sobreviven a la llamada; el `malloc` manual es obligatorio solo en los valores
  de retorno. Confundir ambos casos es un fallo silencioso: funciona en pruebas
  cortas y corrompe memoria bajo carga.
- Toda ruta de salida de Rust que consuma un puntero del puente —incluidos los
  caminos de error y los `?` tempranos— debe liberar. La disciplina se
  concentra en un único wrapper en `crypto.rs` en lugar de repetirse en cada
  punto de llamada.
- Este reparto no es simétrico ni evidente al leer solo uno de los dos lados:
  quien toque la firma de una función FFI debe revisar los dos.
