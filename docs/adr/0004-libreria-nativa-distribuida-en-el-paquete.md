# La librería criptográfica se distribuye como un fichero del paquete, y hay tres paquetes

La librería nativa se instala en el directorio de librerías del paquete y el backend de
Rust la carga por una ruta **relativa al ejecutable** (`../lib/rfirma`), sobreescribible
con la variable de entorno `RFIRMA_LIB_DIR` para desarrollar contra `target/` sin instalar
nada.

Los canales son **tres** —flatpak, `.deb` y `.rpm`—, y esa ruta relativa vale en los tres
sin una línea de código condicional:

| canal | ejecutable | librería |
| --- | --- | --- |
| flatpak | `/app/bin/rfirma` | `/app/lib/rfirma/` |
| `.deb`, `.rpm` | `/usr/bin/rfirma` | `/usr/lib/rfirma/` |

El `.deb` y el `.rpm` los produce el *bundler* de Tauri, que instala el binario en
`/usr/bin` en los dos formatos, así que `../lib/rfirma` **es** `/usr/lib/rfirma`. No hay
tercera ruta que añadir a `ffi.rs`, ni lista de rutas por canal: es el mismo mecanismo que
ya hacía funcionar `/app/bin` → `/app/lib/rfirma`. Medido sobre los fuentes de
`tauri-bundler` 2.9.4 en el [#228](https://github.com/sgomez/rfirma/issues/228).

Es **un solo fichero**, `librfirma_crypto.so` (27,7 MB), y cubre los cuatro casos: sin
rúbrica, rúbrica de texto y rúbrica de imagen. No hay auxiliares que acompañarlo, ni
metadatos de alcanzabilidad que generar, ni formatos de imagen que declarar en tiempo de
construcción.

La primera redacción de este ADR decía **seis ficheros**, y era cierta entonces: la
rúbrica de imagen pasaba por `es.gob.afirma.ui.utils.ImageUtils`, que reencoda con
`ImageIO` y arrastra `libawt.so`, `libawt_headless.so`, `libjavajpeg.so`, `libjava.so`
y `libjvm.so` al mismo directorio. El [ADR-0012](0012-normalizacion-de-la-rubrica-en-rust.md)
movió esa normalización a Rust y **excluyó `afirma-ui-utils` del `pom.xml` del puente**;
`PdfPreProcessor` lo invoca por reflexión dentro de un `catch (Throwable)`, así que la
llamada degrada sola. Con eso desaparece el subárbol de `javax.imageio` y, con él, los
cinco auxiliares. Medido en
[Excluir `afirma-ui-utils`](../research/exclusion-afirma-ui-utils.md) e
[issue #36](https://github.com/sgomez/rfirma/issues/36).

## Los tres canales llevan los mismos bytes

Una sola construcción de la `.so` por entrega, publicada como artefacto, y los tres
paquetes la consumen. GraalVM no es reproducible bit a bit: si el flatpak y el `.deb` de la
misma versión llevaran `.so` distintas, «reprodúcelo en el otro canal» dejaría de ser una
pregunta contestable, y esta frontera es justo donde este proyecto lleva tres hallazgos de
fallo silencioso. El `sha256` queda registrado en la Release
([ADR-0015](0015-canal-de-distribucion-propio.md)).

**Sólo `x86_64` en la v0.4**, y es un límite consciente, no un olvido: el bundler soporta
`arm64` y `riscv64` en los dos formatos. `arm64` pediría una imagen nativa de GraalVM
construida en un anfitrión `aarch64`, y arreglar la ruta multiarch clavada a
`x86_64-linux-gnu` en `CANDIDATE_MODULES` (`pkcs11/stores.rs`), que rompería un `.deb` de
`arm64` **en silencio**.

## Considered Options

- **Incrustar la librería con `include_bytes!` y extraerla** a `~/.cache/rfirma/`.
  Conserva el binario suelto a cambio de mantener extracción, detección de fichero
  corrupto o desactualizado y arranques concurrentes, todo en el camino crítico del
  arranque. Con seis ficheros era además rehacer a mano el enlazado que un empaquetador
  ya hace bien; con uno solo el argumento se debilita, pero sigue siendo trabajo
  permanente para no usar el mecanismo que los tres formatos ya traen.
- **Renunciar a la rúbrica de imagen** para volver a un único `.so`. Ya no compra nada:
  la rúbrica de imagen cabe en el mismo fichero que los otros casos.
- **Reducir la superficie Java hasta que AWT desaparezca**. Descartado con datos cuando
  se creía que la dependencia era de `afirma-lib-itext` y por tanto transversal a
  cualquier operación sobre el PDF. **Resultó ser la opción ganadora**, por un camino
  que no se había visto: la dependencia transversal existe en el código de iText, pero
  el árbol de alcanzabilidad solo la alcanza a través de `ImageUtils`, y ese módulo es
  opcional por diseño del original.
- **Flatpak como canal único**, que fue la decisión de la v0.1 y el título de este ADR
  hasta el hito v0.4. Se sostenía en que el hito quería que la aplicación la usara gente
  de cualquier distribución y un `.deb` obliga a un `.rpm` y a una matriz de glibc y de
  WebKitGTK detrás. Ese argumento **se cae en cuanto los formatos conviven en vez de
  competir**: el `.rpm` sale de la misma configuración que el `.deb`, las dependencias de
  WebKitGTK y GTK las inyecta el bundler solo, y el suelo de glibc lo hace verdad una
  puerta (`just check-glibc`) en vez del entorno de construcción. Lo que el flatpak
  resuelve —una glibc fija, independiente de la distribución— sigue valiendo, y por eso
  **el flatpak se queda**: no lo sustituye nadie, se le suman dos. Lo que ya no se
  sostiene es que sea el **único**: el sandbox le cuesta la ruta del original, el módulo
  PKCS#11 del anfitrión y el esquema `afirma://`, y esas tres cosas son el hito v0.4.
  Medido en [Flatpak como único canal](../research/flatpak-canal-unico.md); decidido
  primero en el [#17](https://github.com/sgomez/rfirma/issues/17) y revisado en el
  [#228](https://github.com/sgomez/rfirma/issues/228).
- **AppImage**. Fuera de la v0.4, sin cerrar la puerta: es «rfirma portátil» y cuesta poco
  una vez existe el `.deb`. Los dos peros del #17 siguen en pie —no lleva glibc dentro y su
  FUSE 2 ya no viene por omisión—, y se le suma uno que lo descarta para siempre como
  destino del hito siguiente: **un fichero suelto no registra `afirma://`, no instala la CA
  del [ADR-0005](0005-servidor-local-https-y-ca-en-los-almacenes-nss.md) y no asocia tipos
  de fichero**.
- **Snap**. Descartado, y conviene que la razón quede escrita bien, porque la que se dio de
  palabra era falsa. **No** se descarta por «no puede instalar la CA»: el
  [#243](https://github.com/sgomez/rfirma/issues/243) midió que un sandbox comparable —el
  del propio flatpak— **sí puede** escribir en los almacenes NSS de la persona. Se descarta
  por dos motivos que sí aguantan: su confinamiento estricto **reproduce el sandbox entero**
  (portales, sin ruta original, sin módulo PKCS#11 del anfitrión), o sea que no compra
  ninguna de las tres cosas por las que existe este hito; y distribuirlo de verdad exige la
  **Snap Store**, que es volver a meter a un tercero entre el usuario y una aplicación de
  firma electrónica, exactamente lo que descarta el
  [ADR-0015](0015-canal-de-distribucion-propio.md).

## Consequences

- **Se instala un fichero y solo uno.** No es una simplificación cosmética: si los
  auxiliares están en el directorio, un JPEG con perfil ICC incrustado que llegue al
  puente **aborta el proceso** (`rc=134`, `NoClassDefFoundError:
  java/awt/GraphicsEnvironment` desde el `JNI_OnLoad` de `libawt.so`), y como la
  librería se carga dentro del ejecutable de Tauri se lleva la aplicación entera. Con
  un solo fichero el mismo JPEG produce un `UnsatisfiedLinkError` que el
  `catch (Throwable)` del `@CEntryPoint` devuelve como error y se puede enseñar al
  usuario. **El recorte del empaquetado es lo que convierte un aborto en un error**, y
  por eso instalar los auxiliares «por si acaso» está prohibido.
- **La invariante anterior es del paquete, no del sandbox, y se comprueba en los tres.**
  `packaging/verifica-contenido.sh` acepta un `.deb`, un `.rpm` o el `files/` de una
  construcción de flatpak y afirma **exactamente un `.so` bajo el directorio de la
  librería, y `libawt.so` en ninguna parte**. Es una puerta del CI antes de subir cada
  artefacto, no un informe. Tenerla escrita una vez por formato es tenerla escrita mal.
- El puente **exige recibir un JPEG ya normalizado**, sin perfil ICC. Sin `ImageUtils`
  nadie reencoda: un PNG llega a `new Jpeg(bytes)` y falla con «*no está codificada en
  JPEG*». Es el contrato del ADR-0012 convertido en un fallo temprano y ruidoso, y
  quien toque la normalización de Rust tiene que saber que el puente no le va a
  perdonar nada.
- La comprobación de arranque sigue existiendo, y sigue nombrando **las dos rutas que
  miró**: la relativa al ejecutable y `RFIRMA_LIB_DIR`. Hace `dlopen` de
  `librfirma_crypto.so` y `dlsym` de los símbolos FFI esperados, y falla nombrando el
  fichero ausente y el directorio donde lo buscaba. El enlazado ocurre en tiempo de
  ejecución, así que un desajuste entre la librería instalada y las firmas FFI que Rust
  espera no lo detecta el compilador.
- **Los `.afm` de iText siguen haciendo falta** y ahora también para la rúbrica de
  imagen, porque el `layer2Text` que la acompaña usa Courier. Son 524.288 B dentro de la
  imagen, y se quedan.
- La librería se construye con **GraalVM CE 25** **en el anfitrión**, no dentro de
  `org.gnome.Sdk` ni de un contenedor de construcción: el suelo de glibc de la `.so`
  construida en un anfitrión de glibc 2.43 ya es **2.34**, medido en
  [`glibc-libreria-nativa.md`](../research/glibc-libreria-nativa.md)
  ([#23](https://github.com/sgomez/rfirma/issues/23)). El `pom.xml` sigue compilando a
  `release 21`: lo que cambia es el JDK que construye, no el lenguaje de destino.
- **La glibc es la diferencia real entre los canales, y la promesa la sostiene una puerta.**
  Bajo flatpak la aplicación corre contra la del runtime (2.42) y la distribución del
  usuario no interviene. En el `.deb` y el `.rpm` sí interviene: se promete `GLIBC_2.34` y
  lo comprueba `just check-glibc` sobre lo que se va a publicar, no el entorno donde se
  construyó. Una puerta que no puedes reproducir en tu equipo es una puerta que un día se
  salta con `continue-on-error`.
- **El sandbox es del flatpak, no de rfirma.** Todo lo que no expone hay que declararlo, y
  eso alcanza a cosas que fuera son gratis: los documentos entran y salen por portales, y
  el diálogo devuelve rutas de verdad ([ADR-0011](0011-destino-del-documento-firmado.md)).
  La fontanería de tarjeta —el cliente PC/SC y el módulo PKCS#11 de OpenSC— se retiró en
  el [#256](https://github.com/sgomez/rfirma/issues/256): nunca se había publicado, y
  tarjetas y DNIe no están soportados en la v0.4, ni en el flatpak ni en el `.deb`/`.rpm`.
  `CANDIDATE_MODULES` solo lleva las rutas de SoftHSM, el token de pruebas.
- **`--filesystem=home` queda cerrado por escrito**, y no por ancho: **no hace lo que
  hace falta**. El [#240](https://github.com/sgomez/rfirma/issues/240) midió que
  `GtkFileChooserNative` se enruta al portal en cuanto existe `/.flatpak-info`, sea cual
  sea el permiso, así que el diálogo sigue devolviendo `/run/user/…/doc/…`; y aun con
  `home`, la aplicación no puede averiguar la carpeta del original. Conviene que quede
  escrito por esta razón y no por la del sandbox: «lo rechazamos por seguridad» invita a
  reabrirlo, «no hace lo que creíamos» no.
- **Lo que el flatpak declara sobre los almacenes NSS no se decide aquí**, sino en el
  [ADR-0005](0005-servidor-local-https-y-ca-en-los-almacenes-nss.md), que es quien tiene
  el motivo delante. Este ADR trata de dónde vive la librería y qué formatos hay; meterle
  además el permiso de NSS sería una segunda decisión en un ADR que trata de otra cosa.
- El PDF con rúbrica de imagen **deja de ser idéntico bit a bit** al que ensambla
  AutoFirma en una JVM completa, porque el codificador JPEG pasa a ser el de Rust. Sigue
  siéndolo frente a una JVM con el mismo recorte de dependencias, que es contra lo que
  hay que comparar de aquí en adelante.
- **Instalar por dos canales son dos aplicaciones con memorias separadas**, y no se
  reconcilian: ni los recientes, ni la rúbrica, ni las preferencias se comparten. Es la
  conducta normal de Linux —el Firefox flatpak y el `.deb` tampoco comparten perfil— y no
  se corrige con código; se dice en el README
  ([#232](https://github.com/sgomez/rfirma/issues/232)).
- No hay ejecutable portable. Quien quiera correr rfirma sin instalar ningún paquete tiene
  que colocar `librfirma_crypto.so` en un directorio y apuntar `RFIRMA_LIB_DIR` ahí.
