# Mapa del puente Java (GraalVM Native Image)

El puente es lo que AutoFirma hace en Java y rfirma no reescribe: preproceso y
postproceso del ciclo trifásico (ADR-0001), compilado a `librfirma_crypto.so`
con `native-image` (ADR-0004). Lo que decide y firma vive en Rust.

| Fichero | Qué es |
|---|---|
| `pom.xml` | Las dependencias de AutoFirma, consumidas desde `~/.m2` (ADR-0002), y la exclusión de `afirma-ui-utils` (ADR-0012). |
| `src/main/java/.../NativeBridge.java` | Los `@CEntryPoint`: la frontera con Rust y la reserva manual de las cadenas devueltas (ADR-0003). |
| `src/main/java/.../PadesBridge.java` | Preproceso y postproceso PAdES, incluida la firma visible. |
| `src/main/java/.../CadesBridge.java` | Preproceso y postproceso CAdES: firma, cofirma y contrafirma, y el contenedor ASiC-S, que entra por aquí con su propio procesador. |
| `src/main/java/.../XadesBridge.java` | Preproceso y postproceso XAdES: firma en las variantes Enveloping, Detached, Enveloped y ASiC-S, cofirma y contrafirma con `target=tree\|leafs`, y la factura electrónica, que entra por aquí con su propio procesador. |
| `src/main/java/.../FilterBridge.java` | Los filtros de certificado que pide la sede. |
| `src/main/java/.../ExtraParamsBridge.java` | La traducción de `extraParams` de AutoFirma. |
| `src/main/java/.../SessionStamp.java` | El sello de sesión (ADR-0016). |
| `src/main/java/.../SessionStampMismatchException.java` | El fallo con el que una postfirma rechaza un sello que no es el de su prefirma. |
| `src/main/resources/META-INF/native-image/` | Los metadatos con los que se construye la imagen: las banderas de `native-image.properties` y los tipos que solo se alcanzan por reflexión, en `reachability-metadata.json`. No es un `resource-config.json` generado dentro del build. |
| `testbench/` | El banco de la grada C y los guiones de medición de los `research/`. Se abre por su `README.md`. |

## Trampas al construir

* **Las dependencias son AutoFirma en el tag `v1.9.2`**, compiladas en su
  propio checkout con `mvn clean install -Dclienteafirma.version=1.9.2`. La
  propiedad no es opcional: el `pom.xml` de ese tag declara `1.9` con el
  proyecto en `1.9.2`, y sin ella los módulos se bajan `afirma-core:1.9` de
  Maven Central en vez de usar el reactor. `afirma-crypto-cades` no compila, y
  lo que sí compila lo hace contra la 1.9 en silencio. Lo automatiza
  `bootstrap.sh`. La 1.9.2 no está en Maven Central: si no está en `~/.m2`,
  hay que construirla.
* **Construye con GraalVM 25** (`25.3.4+1.r25-graalce`, ADR-0004). SDKMAN deja
  `21-graalce` por defecto, así que fija `GRAALVM_HOME` a la 25: la línea 21
  aborta en el `JNI_OnLoad` de `libawt.so` con cualquier firma visible. El
  `pom.xml` sigue en `release 21`: cambia el JDK que construye, no el lenguaje.
  Ver `docs/research/graalvm-libawt-shared.md`.
* **Un `just native` verde en local no dice que el carril lento del CI vaya a
  serlo**, porque no es el mismo JDK: aquí SDKMAN tiene una compilación
  concreta de la 25 y `ci.yml` pide `java-version: '25'`, que `setup-graalvm`
  resuelve a la última CE 25 publicada el día que corre. Dos compilaciones
  distintas de la 25 alcanzan clases distintas: con la de este equipo
  `ApacheCanonicalizer` no queda alcanzable y las pruebas de grada C pasan;
  con la del CI sí queda, y revientan con un `MissingResourceException` de
  `com.sun.org.apache.xml.internal.security.resource.xmlsecurity`. El tamaño
  del `.so` delata cuál has construido (41,5 MB frente a 48,4 MB). Antes de
  dar por transitorio un rojo del carril lento que no reproduces, baja el JDK
  exacto de ese run (`graalvm-ce-builds`) y reconstruye con él.
* **`native-image` emite seis ficheros y se distribuye uno.** Los cinco
  auxiliares de AWT en `target/native/` son normales, no un fallo; instalarlos
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
