# La librería criptográfica se incrusta en el binario y se extrae en tiempo de ejecución

El objetivo del proyecto es una aplicación **portable**: un único ejecutable que
el usuario pueda lanzar sin instalar dependencias ni configurar rutas. Por eso
la librería nativa generada por GraalVM (`.so` / `.dll` / `.dylib`) **no** se
enlaza en tiempo de compilación mediante `build.rs` ni se localiza vía
`LD_LIBRARY_PATH`: el backend de Rust la incrusta con `include_bytes!`, y al
arrancar la extrae —si no está ya— al directorio de caché del usuario (p. ej.
`~/.cache/rfirma/libautofirma_crypto.so`) y la carga dinámicamente con
`libloading`.

## Consequences

- El binario resultante es grande. Es el precio explícito de la portabilidad.
- El enlazado ocurre en tiempo de ejecución, así que un desajuste entre la
  librería incrustada y las firmas FFI que Rust espera no lo detecta el
  compilador: se manifiesta como un fallo al cargar el símbolo. La carga debe
  fallar de forma ruidosa y temprana, en el arranque, no en la primera firma.
- La extracción debe tolerar un fichero previo corrupto o de otra versión, y
  varias instancias arrancando a la vez.
