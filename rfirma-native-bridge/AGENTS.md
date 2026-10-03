# Mapa del puente Java (GraalVM Native Image)

El puente es el preproceso y el postproceso del ciclo trifásico (ADR-0001) que
rfirma no reescribe, compilado con `native-image` a la biblioteca compartida
de ADR-0004.

## Dónde mirar

* **Qué es cada clase:** `just outline rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/`
  da el índice con la cabecera `//!` de cada fichero (ADR-0017). Un `.java`
  suelto no tiene esqueleto: sus tramos, con `fichero.java:A-B`.
* **Orden de lectura:** `NativeBridge.java` primero, que es la frontera con Rust
  y lista los `@CEntryPoint`; cada `*Bridge.java` es lo que esa frontera
  delega, y se abre solo el del formato que tocas.
* `src/main/resources/META-INF/native-image/`: las banderas de la imagen
  (`native-image.properties`) y los tipos que solo se alcanzan por reflexión
  (`reachability-metadata.json`).
* `testbench/`: el validador y el firmante de referencia de la grada C. Se
  abre por su `README.md`.

## Trampas al construir

* **Las dependencias son AutoFirma en el tag `v1.9.2`**, compiladas en su
  propio checkout con `mvn clean install -Dclienteafirma.version=1.9.2`. La
  propiedad no es opcional: el `pom.xml` de ese tag declara `1.9` con el
  proyecto en `1.9.2`, y sin ella los módulos se bajan `afirma-core:1.9` de
  Maven Central en vez de usar el reactor. `afirma-crypto-cades` no compila, y
  lo que sí compila lo hace contra la 1.9 en silencio. Lo automatiza
  `bootstrap.sh`. La 1.9.2 no está en Maven Central: si no está en `~/.m2`,
  hay que construirla.
* **Construye con la GraalVM CE de `GRAALVM_VERSION` de `versions.env`** (ADR-0004, ADR-0035). SDKMAN deja
  `21-graalce` por defecto, así que fija `GRAALVM_HOME` a la 25: la línea 21
  aborta en el `JNI_OnLoad` de `libawt.so` con cualquier firma visible. El
  `pom.xml` sigue en `release 21`: cambia el JDK que construye, no el lenguaje.
  Ver `docs/research/graalvm-libawt-shared.md`.
* **Local y CI construyen con la misma GraalVM**, la de `GRAALVM_VERSION` de `versions.env`: el
  `justfile` deriva de ella la ruta de SDKMAN y los workflows la instalan con
  `.github/actions/setup-runner` (ADR-0035). Dos compilaciones distintas de
  la 25 alcanzan clases distintas: con una `ApacheCanonicalizer` no queda
  alcanzable y las pruebas de grada C pasan; con otra sí queda, y revientan
  con un `MissingResourceException` de
  `com.sun.org.apache.xml.internal.security.resource.xmlsecurity`. Si un rojo
  del carril lento no se reproduce, comprueba antes que tu `GRAALVM_HOME` es
  la de `GRAALVM_VERSION` de `versions.env`.
* **En Windows, `just native` se lanza desde Git Bash** y toma GraalVM de
  `GRAALVM_HOME` o `JAVA_HOME` (ADR-0035). `native-image` enlaza con MSVC por
  su cuenta, sin abrir una consola de Visual Studio, y deja
  `librfirma_crypto.dll`, que la receta instala como `rfirma_crypto.dll`.
* **GraalVM CE 25.4.4 no compila `PdfTimestamper.initialize()`**: revienta en
  `[6/8]` con `failed guarantee: Must not have guards attached to exception
  object node`, también con `-H:+EnableFallbackCompilation` y con `-Ob`. Es
  el `Class.forName` de `TsaParams` dentro de un `try`. Con
  `NATIVE_IMAGE_OPTIONS=--initialize-at-build-time=es.gob.afirma.signers.tsp.pkcs7.TsaParams`
  compila (su inicializador solo pide un `Logger`). La bandera no está en
  `native-image.properties` porque cambiaría también la imagen de Linux, que
  se construye con la versión fijada y no tiene este fallo.
* **`native-image` emite seis ficheros y se distribuye uno.** Los cinco
  auxiliares de AWT en `target/native/` (en macOS no sale ninguno, solo las
  cabeceras) son normales, no un fallo; instalarlos
  «por si acaso» convierte un JPEG con perfil ICC en un aborto del proceso
  (ADR-0004). La exclusión de `afirma-ui-utils` del `pom.xml` es lo que deja
  `javax.imageio` sin métodos alcanzables: no la quites.
* **El `WARNING` de `ClassNotFoundException: es.gob.afirma.ui.utils.ImageUtils`**
  en una firma visible con rúbrica es la exclusión haciendo su trabajo.
* **La prefirma XAdES con ECDSA necesita a SpongyCastle dentro de la imagen.**
  El original genera una clave de curva elíptica de mentira con
  `KeyPairGenerator.getInstance("ECDSA")`, que solo sirve ese proveedor, y sus
  clases se alcanzan por nombre: sin las entradas
  `org.spongycastle.jcajce.provider.asymmetric...` del `reachability-metadata.json`
  la prefirma muere con `NoSuchAlgorithmException: ECDSA KeyPairGenerator not
  available`. En RSA no pasa porque la clave falsa la da el proveedor del JDK.
* **FacturaE no tiene módulo propio**: `AOFacturaESigner` vive en
  `afirma-crypto-xades`. No añadas `afirma-crypto-facturae` al `pom.xml`: no
  existe en la 1.9.2.
* **`TriphaseData.getTriSigns(id)` devuelve COPIAS** (`new TriSign(ts)`), así que
  escribir el `PK1` en lo que devuelve no toca la sesión: la firma sale
  incompleta sin que nadie lo diga. Para mutar, la lista viva de
  `getTriSigns()`.
* **El puente exige un JPEG ya normalizado y sin perfil ICC**: la
  normalización es de Rust (ADR-0012). Un PNG que llegue aquí falla con «no
  está codificada en JPEG», y eso es lo correcto.

## Trampas al validar

* **`AcroFields.getSignatureNames()` de iText devuelve las firmas de la
  revisión más nueva a la más vieja.** La comprobación de PDF Shadow Attack de
  `ValidatePdfSignature` mira la primera de esa lista, así que cofirmar nunca
  la dispara: solo salta cuando alguien añade una revisión incremental que no
  es una firma. Leerla al revés da una guarda que parece correcta y deja pasar
  justo el ataque que persigue.
* **`ValidatePdfSignature.validateSign()` no es el validador entero.** Es la
  pieza por firma que llama `validate(byte[], Properties)`; lo que esta añade
  alrededor —el documento certificado que no admite más firmas y la regla de
  `SignValider#checkLongStandingValiditySign`— no ocurre si llamas solo a
  `validateSign`. Dos consecuencias: con `checkCert=true` puede devolver
  varios veredictos para una misma firma (`CERTIFICATE_EXPIRED` junto a
  `SIGN_PROFILE_NOT_CHECKED` en una longeva caducada), y quedarse con el
  primer KO dice «caducado» donde el original dice «sin comprobar del todo»;
  y nunca produce `UNKOWN_SIGNATURE_FORMAT`, así que un `/SubFilter` ajeno
  pero íntegro cae en el mismo `SIGN_PROFILE_NOT_CHECKED` que la longeva y
  solo el perfil `"PDF"` de `SignatureFormatDetectorPadesCades` los separa.
* **`checkPdfShadowAttack` no se llama: rasteriza con AWT** (ADR-0004).
  `PreviousSignaturesBridge` lo sustituye comparando la última revisión
  firmada con la actual, página a página: el flujo de contenido distinto es
  `modifiedAfterLastSignature`, y una anotación nueva o movida (referencia y
  `/Rect`) que se solapa estrictamente con otra visible es
  `contentAddedOnTop`. Mirar solo los solapamientos de la revisión actual
  da el hallazgo en cuanto llega el `/DSS` de un perfil LT sobre un
  formulario que ya se solapaba al firmar; el original no lo da, porque
  compara las dos imágenes.
