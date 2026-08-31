# La rúbrica la normaliza Rust, no Java

AutoFirma normaliza la imagen de la rúbrica dentro del puente:
`PdfSessionManager` llama a `PdfPreProcessor.getImage`, que invoca **por reflexión**
`es.gob.afirma.ui.utils.ImageUtils.normalizeImageToPdf` y termina en
`new Jpeg(bytes)`. Ese método hace dos cosas y nada más: **aplana el canal alfa y
reencoda a JPEG**.

**rFirma normaliza en Rust y excluye `afirma-ui-utils` del `pom.xml` del puente.**
Al no estar la clase, `Class.forName` falla, el `catch (Throwable)` que ya hay en
`getImage` lo recoge, y el JPEG que le damos pasa tal cual a `new Jpeg(...)`.
`javax.imageio.ImageIO` no se toca nunca.

## Por qué

La llamada es reflexiva **a propósito**: `afirma-crypto-pdf` declara
`afirma-ui-utils` con `<scope>runtime</scope>` para que el módulo pueda no estar.
Y ese módulo es **un solo fichero de 90 líneas sin una referencia a Swing** — es
literalmente `ImageUtils`.

Lo que cuesta conservarlo, todo medido en el
[#14](https://github.com/sgomez/rfirma/issues/14): **seis ficheros y 36,6 MB** en
vez de uno y 35,4 MB, **+655 KB** de metadatos de AWT en la imagen, y declarar los
formatos de imagen admitidos **en el comando de `native-image`** — porque la traza
de una rúbrica PNG no cubre una JPEG. Es decir: la lista de formatos quedaba
congelada en tiempo de construcción. Al normalizar en Rust deja de estarlo y pasa
a ser una capacidad de una biblioteca en tiempo de ejecución.

Y el modo de fallo de conservarlo es malo: con los seis `.so` pero **sin** los
metadatos, la prefirma no degrada, **muere con `rc=134`** en el `JNI_OnLoad` de
`libawt.so`.

**Lo que se pierde**: el PDF deja de ser **idéntico bit a bit** al que ensambla
AutoFirma en JVM, porque el encoder JPEG pasa a ser el nuestro. Eso era una
herramienta de verificación de la investigación (#13, #14, #23), no un requisito
del producto, pero quien escriba las pruebas de firma debe saber que ese criterio
ya no vale para el caso de rúbrica de imagen.

## Qué se decide de la rúbrica

**Formatos: PNG y JPEG, y nada más.** No es un límite técnico —el crate de Rust da
más casi gratis— sino de superficie: cada formato extra es un decodificador más
sobre un fichero que elige el usuario, y ni un TIFF ni un WebP aparecen en la vida
real de una firma escaneada.

**La transparencia no se puede ofrecer con ningún formato.** El PDF lleva siempre
un JPEG, y el JPEG no tiene alfa. Un PNG recortado acaba con **fondo blanco**
—el mismo que produce el original, ahí por accidente: en `removeAlphaChannel` la
línea `g.setColor(...)` está comentada y `Graphics2D` arranca en blanco—. No se
avisa con un cartel: **la miniatura del panel de firma enseña el resultado real**,
sobre blanco, y el usuario ve lo que va a salir antes de firmar.

**El JPEG se emite sin perfil ICC**, JFIF baseline pelado. No es cosmético:
`com.aowagie.text.Jpeg` parsea el segmento APP2 y construye un
`java.awt.color.ICC_Profile` si lo encuentra, y `liblcms.so` es justo uno de los
`.so` que #14 descartó por no cargarse nunca. Un perfil sRGB incrustado llevaría a
AWT por la puerta de atrás, en una imagen que ya no tiene ni metadatos ni
auxiliares.

**Constantes**: calidad 90, lado mayor máximo 1000 px, tope de fichero de entrada
10 MB, fondo del aplanado blanco.

**Se normaliza al elegir, y se guarda solo el JPEG** en el directorio que fija el
[ADR-0010](0010-memoria-entre-sesiones.md). Así la miniatura es el fichero que se
firma sin esfuerzo, un fichero que no vale falla **con el diálogo del usuario aún
abierto** y no al firmar, y la ruta de firma se queda en leer bytes y codificar en
Base64. Se paga que subir algún día el tope de los 1000 px obligue a volver a
cargar la rúbrica.

**El reescalado es silencioso**: es la operación que el usuario habría pedido. Los
tres fallos que sí se cuentan, clasificados como manda el
[ADR-0009](0009-catalogo-de-cadenas-propio-y-seis-idiomas.md), son *no es una
imagen PNG o JPEG*, *la imagen está dañada* y *la imagen es demasiado grande*. El
selector del portal filtra por **tipo MIME** (`image/png`, `image/jpeg`) y no por
extensión, porque la extensión miente.
