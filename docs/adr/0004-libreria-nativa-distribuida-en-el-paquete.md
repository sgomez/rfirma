# La librería criptográfica se distribuye como un fichero del paquete, y el paquete es un flatpak

La librería nativa se instala en `/app/lib/rfirma/` dentro de un **flatpak, el único
canal de distribución soportado**, y el backend de Rust la carga por una ruta
**relativa al ejecutable** (`../lib/rfirma` desde `/app/bin/`), sobreescribible con la
variable de entorno `RFIRMA_LIB_DIR` para desarrollar contra `target/` sin instalar
nada.

Es **un solo fichero**, `librfirma_crypto.so` (27,7 MB), y cubre los cuatro casos del
hito: sin rúbrica, rúbrica de texto y rúbrica de imagen. No hay auxiliares que
acompañarlo, ni metadatos de alcanzabilidad que generar, ni formatos de imagen que
declarar en tiempo de construcción.

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

## Considered Options

- **Incrustar la librería con `include_bytes!` y extraerla** a `~/.cache/rfirma/`.
  Conserva el binario suelto a cambio de mantener extracción, detección de fichero
  corrupto o desactualizado y arranques concurrentes, todo en el camino crítico del
  arranque. Con seis ficheros era además rehacer a mano el enlazado que un empaquetador
  ya hace bien; con uno solo el argumento se debilita, pero el resto del paquete
  (runtime, portales, PKCS#11, WebKitGTK) sigue exigiendo flatpak igual.
- **Renunciar a la rúbrica de imagen** para volver a un único `.so`. Ya no compra nada:
  la rúbrica de imagen cabe en el mismo fichero que los otros casos.
- **Reducir la superficie Java hasta que AWT desaparezca**. Descartado con datos cuando
  se creía que la dependencia era de `afirma-lib-itext` y por tanto transversal a
  cualquier operación sobre el PDF. **Resultó ser la opción ganadora**, por un camino
  que no se había visto: la dependencia transversal existe en el código de iText, pero
  el árbol de alcanzabilidad solo la alcanza a través de `ImageUtils`, y ese módulo es
  opcional por diseño del original.
- **Un `.deb`**, que fue la conclusión de la primera redacción de este ADR. Lo descarta
  el objetivo, no la mecánica: el hito quiere que la aplicación la use gente de
  cualquier distribución, y un `.deb` obliga a un `.rpm`, un PKGBUILD y una matriz de
  versiones de glibc y de WebKitGTK detrás. Medido en
  [Flatpak como único canal](../research/flatpak-canal-unico.md); decidido en el
  [issue #17](https://github.com/sgomez/rfirma/issues/17).
- **AppImage**. Descartado por el #17: no lleva glibc dentro, así que construido en
  26.04 no arranca en una Ubuntu más vieja, y su FUSE 2 ya no viene por omisión.

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
- El puente **exige recibir un JPEG ya normalizado**, sin perfil ICC. Sin `ImageUtils`
  nadie reencoda: un PNG llega a `new Jpeg(bytes)` y falla con «*no está codificada en
  JPEG*». Es el contrato del ADR-0012 convertido en un fallo temprano y ruidoso, y
  quien toque la normalización de Rust tiene que saber que el puente no le va a
  perdonar nada.
- La comprobación de arranque sigue existiendo, pero cambia de forma: ya no hay una
  lista de seis nombres que verificar, sino un fichero. Hace `dlopen` de
  `librfirma_crypto.so` y `dlsym` de los símbolos FFI esperados, y falla nombrando el
  fichero ausente y el directorio donde lo buscaba. El enlazado ocurre en tiempo de
  ejecución, así que un desajuste entre la librería instalada y las firmas FFI que Rust
  espera no lo detecta el compilador.
- **Los `.afm` de iText siguen haciendo falta** y ahora también para la rúbrica de
  imagen, porque el `layer2Text` que la acompaña usa Courier. Son 524.288 B dentro de la
  imagen, y se quedan.
- La librería se construye con **GraalVM CE 25** **en el anfitrión**, no dentro de
  `org.gnome.Sdk`: el suelo de glibc es 2.34 y el runtime da 2.42, ocho versiones de
  margen ([#23](https://github.com/sgomez/rfirma/issues/23)). El `pom.xml` sigue
  compilando a `release 21`: lo que cambia es el JDK que construye, no el lenguaje de
  destino.
- El runtime fija la glibc que se ejecuta, así que **la distribución del usuario deja
  de intervenir**. A cambio, todo lo que el arenero no expone hay que declararlo, y
  eso alcanza a cosas que fuera eran gratis: el módulo PKCS#11 lo empaqueta el propio
  flatpak, y los ficheros entran y salen por portales.
- El PDF con rúbrica de imagen **deja de ser idéntico bit a bit** al que ensambla
  AutoFirma en una JVM completa, porque el codificador JPEG pasa a ser el de Rust. Sigue
  siéndolo frente a una JVM con el mismo recorte de dependencias, que es contra lo que
  hay que comparar de aquí en adelante.
- No hay ejecutable portable ni instalación fuera de flatpak. Quien quiera correr
  rfirma sin instalar el paquete tiene que colocar `librfirma_crypto.so` en un
  directorio y apuntar `RFIRMA_LIB_DIR` ahí.
