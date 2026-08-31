# La librería criptográfica se distribuye como ficheros del paquete, no incrustada en el binario

GraalVM Native Image **no** produce un artefacto autosuficiente para nuestro caso de
uso. La firma con **rúbrica de imagen** necesita seis ficheros —`librfirma_crypto.so`
más `libawt.so`, `libawt_headless.so`, `libjavajpeg.so`, `libjava.so` y `libjvm.so`—
que además tienen que convivir en el mismo directorio: los cinco auxiliares llevan
`RPATH`/`RUNPATH` a `$ORIGIN` y dos de ellos son stubs que enlazan de vuelta contra
`librfirma_crypto.so`. Con eso, la promesa de «binario único portable» que motivaba
incrustar la librería con `include_bytes!` y extraerla a `~/.cache/rfirma/` deja de
tener sentido: extraer seis ficheros a un directorio de caché y resolver el enlazado
entre ellos es rehacer a mano, con menos garantías, lo que el gestor de paquetes ya
hace bien.

Por eso el **`.deb` es el único canal de distribución soportado**. Los seis ficheros
se instalan juntos en `/usr/lib/rfirma/`, directorio privado del paquete, y el backend
de Rust carga `librfirma_crypto.so` por **ruta absoluta** con `libloading`. La ruta es
una constante de tiempo de compilación, sobreescribible con la variable de entorno
`RFIRMA_LIB_DIR` para desarrollar contra `target/` sin instalar el paquete.

## Considered Options

- **Incrustar los seis y extraerlos** a `~/.cache/rfirma/`. Conserva el binario suelto
  a cambio de mantener extracción, detección de fichero corrupto o desactualizado y
  arranques concurrentes, todo en el camino crítico del arranque.
- **Renunciar a la rúbrica de imagen** para volver a un único `.so` de 35,4 MB. Los
  cinco auxiliares pesan 1,2 MB en total: el coste de la rúbrica de imagen no es el
  tamaño, es pasar de un fichero a seis. Renunciar a una capacidad del hito para
  ahorrar 1,2 MB no sale a cuenta.
- **Reducir la superficie Java hasta que AWT desaparezca**. Descartado con datos: la
  dependencia de AWT es de `afirma-lib-itext`, transversal a cualquier operación sobre
  el PDF, y no se elimina renunciando a la firma visible.

## Consequences

- No hay ejecutable portable. Quien quiera correr rfirma sin instalar el paquete tiene
  que colocar los seis ficheros en un directorio y apuntar `RFIRMA_LIB_DIR` ahí.
- **Los seis ficheros van juntos o no van.** Instalar de menos no produce un fallo
  claro: sin `libawt.so`, la firma con rúbrica de imagen no aborta, degrada a
  «*Se ha proporcionado una imagen de rúbrica que no está codificada en JPEG*», un
  error que miente sobre la causa. Por eso el arranque comprueba que los seis ficheros
  existen por nombre, hace `dlopen` del principal y `dlsym` de los símbolos FFI
  esperados, y falla nombrando el fichero ausente y el directorio donde lo buscaba.
- El enlazado sigue ocurriendo en tiempo de ejecución, así que un desajuste entre la
  librería instalada y las firmas FFI que Rust espera no lo detecta el compilador. La
  comprobación de arranque es lo que lo convierte en un fallo ruidoso y temprano.
- La librería se construye con **GraalVM CE 25**. La línea 21 aborta dentro del
  `JNI_OnLoad` de `libawt.so` en cualquier variante de firma visible. El `pom.xml`
  sigue compilando a `release 21`: lo que cambia es el JDK que construye, no el
  lenguaje de destino.
