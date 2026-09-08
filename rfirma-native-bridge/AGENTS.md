# Mapa del puente Java (GraalVM Native Image)

El puente es lo que AutoFirma hace en Java y rfirma no reescribe: preproceso y
postproceso del ciclo trifásico (ADR-0001), compilado a `librfirma_crypto.so`
con `native-image` (ADR-0004). Lo que decide y firma vive en Rust.

| Fichero | Qué es |
|---|---|
| `pom.xml` | Las dependencias de AutoFirma, consumidas desde `~/.m2` (ADR-0002), y la exclusión de `afirma-ui-utils` (ADR-0012). |
| `src/main/java/.../NativeBridge.java` | Los `@CEntryPoint`: la frontera con Rust y la reserva manual de las cadenas devueltas (ADR-0003). |
| `src/main/java/.../PadesBridge.java` | Preproceso y postproceso PAdES, incluida la firma visible. |
| `src/main/java/.../CadesBridge.java` | Preproceso y postproceso CAdES: firma, cofirma y contrafirma. |
| `src/main/java/.../FilterBridge.java` | Los filtros de certificado que pide la sede. |
| `src/main/java/.../ExtraParamsBridge.java` | La traducción de `extraParams` de AutoFirma. |
| `src/main/java/.../SessionStamp.java` | El sello de sesión (ADR-0016). |
| `src/main/java/.../SessionStampMismatchException.java` | El fallo con el que una postfirma rechaza un sello que no es el de su prefirma. |
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
* **`native-image` emite seis ficheros y se distribuye uno.** Los cinco
  auxiliares de AWT en `target/native/` son normales, no un fallo; instalarlos
  «por si acaso» convierte un JPEG con perfil ICC en un aborto del proceso
  (ADR-0004). La exclusión de `afirma-ui-utils` del `pom.xml` es lo que deja
  `javax.imageio` sin métodos alcanzables: no la quites.
* **El `WARNING` de `ClassNotFoundException: es.gob.afirma.ui.utils.ImageUtils`**
  en una firma visible con rúbrica es la exclusión haciendo su trabajo.
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
