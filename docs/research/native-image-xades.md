# XAdES en GraalVM Native Image: xmlsec, JAXP y la alcanzabilidad que hace falta

Spike del issue [#534](https://github.com/sgomez/rfirma/issues/534), sub-issue de
[#468](https://github.com/sgomez/rfirma/issues/468) (protocolo `afirma://`, XAdES 3/3).
**Registra hechos, no cambia el puente de producción**: `rfirma-native-bridge/pom.xml` y su
`src/main` no se han tocado. Contesta antes de que exista ningún ticket de implementación de
XAdES si `native-image` compila y ejecuta una prefirma/postfirma `XAdESTriPhasePreProcessor`, y
con qué configuración de alcanzabilidad y qué coste en tamaño.

Entorno: GraalVM CE 25.3.4.1 (JDK 25.0.4.1, Substrate VM), Maven 3.9.12, Linux x86_64.
Dependencias de `~/.m2` (ADR-0002): `afirma-server-triphase-signer-core:1.9.2`,
`org.apache.santuario:xmlsec:3.0.5`. Banco: `rfirma-native-bridge/testbench/xades-spike/`, un
módulo Maven **independiente** del reactor, con su propio `pom.xml`.

## Veredicto

**Sí en las cinco preguntas.** `native-image` compila una prefirma y una postfirma XAdES
Enveloping sin más configuración de alcanzabilidad que la que capturó el agente de trazas; el
`.so` de producción (hoy sin XAdES) crecería **un 54,0 %**; la exclusión de `afirma-ui-utils`
(ADR-0012) no está amenazada porque XAdES nunca la arrastra; y el `xalan` que excluye
`afirma-crypto-core-xml/pom.xml` en el original tampoco aparece en ningún classpath medido aquí.
La pregunta 5 (`PK1_DECODED` con ECDSA) tiene una respuesta de código, no de medición: no obliga a
nada en la frontera, porque el que decodifica el PKCS#1 es el propio preprocesador, no el llamante.

## 1. `native-image` compila y ejecuta el ciclo completo

`xades-spike/pom.xml` depende de `afirma-server-triphase-signer-core` (con la misma exclusión de
`afirma-ui-utils` que `rfirma-native-bridge/pom.xml`, ADR-0012) y declara `xmlsec:3.0.5`
explícito — ver la sección 4, es la línea que no es opcional. `Main.java` hace, en un solo
proceso: `XAdESTriPhasePreProcessor.preProcessPreSign` → firma PK1 con `java.security.Signature`
(esto sustituye a Rust, ADR-0001: aquí no hay separación de fase 2 porque el banco no cruza FFI,
solo mide compilación) → `preProcessPostSign`.

Compilado como **ejecutable** (`build-native.sh`, sin `@CEntryPoint`) y ejecutado bajo
`env -i PATH=/usr/bin:/bin` (sin `JAVA_HOME`, sin GraalVM en el `PATH`, sin `HOME`):

```
PRESIGN_NS=177063127
POSTSIGN_NS=3332407
PRESIGN OK (1460 bytes) -> PRE
POSTSIGN OK (5385 bytes) -> xades-out-envi.xml
```

Con un certificado ECDSA P-256, mismo binario:

```
PRESIGN_NS=11823779
POSTSIGN_NS=2811861
PRESIGN OK (1462 bytes) -> PRE
POSTSIGN OK (4155 bytes) -> xades-out-envi-ec.xml
```

Las dos firmas se validan criptográficamente con un segundo control independiente
(`Validate.java`, `javax.xml.crypto.dsig` — **no** Apache Santuario, para no validar con la misma
librería que firmó):

```
CORE_VALIDITY=true   # RSA
CORE_VALIDITY=true   # ECDSA
```

(El único fallo al validar fue de la propia API de JAXP, no de la firma: sin
`Element.setIdAttribute("Id", true)` la resolución de la referencia `#Signature-…-SignedProperties`
falla — JDK-8134575 — porque el DOM no trae DTD. `Validate.java` lo marca a mano antes de
validar.)

`XAdESTriPhaseSignerServerSide.preSign` y `XAdESTriPhasePreProcessor.preProcessPostSign`
(`Op.SIGN`) compilan y corren, entonces, sin ninguna adaptación de código: la única entrada nueva
sobre el patrón que ya usan `CadesBridge`/`PadesBridge` es la propiedad `PK1_DECODED` (sección 5).

## 2. Configuración de alcanzabilidad: 118 tipos de reflexión, 21 recursos, cero serialización

Capturada con `-agentlib:native-image-agent=config-merge-dir=…` (`trace.sh`) ejercitando **las dos
ramas de clave** —RSA y ECDSA, cada una prefirma+postfirma— sobre la misma sesión. GraalVM 25
emite un único `reachability-metadata.json` (23.002 B) con dos secciones; no hay tercera: **no
hace falta `serialization-config`**.

`reflection`: 118 tipos. Nada de AWT, `javax.imageio` ni `xalan` — el catálogo completo son
proveedores JCE/JCA (`sun.security.*`, SpongyCastle vía `afirma-keystores-*`, `XMLDSigRI`),
`KeyStoreSpi`/`PKCS12KeyStore` (del `KeyStore.load` del propio banco, no del preprocesador) y los
tipos de certificado X.509 estándar.

`resources`: 21 entradas, y estas sí son la lista que un `pom.xml`/`build-native.sh` de
producción necesitaría (`-H:ConfigurationFileDirectories` o el `resource-config.json`
equivalente):

```
META-INF/services/javax.xml.parsers.DocumentBuilderFactory
META-INF/services/javax.xml.parsers.SAXParserFactory
META-INF/services/java.net.spi.URLStreamHandlerProvider
META-INF/services/java.nio.charset.spi.CharsetProvider
META-INF/services/java.time.zone.ZoneRulesProvider
META-INF/services/java.util.spi.ResourceBundleControlProvider
org/apache/xml/security/resource/xmlsecurity(_en|_en_US)?.properties
org/slf4j/impl/StaticLoggerBinder.class
resources/mimetypes_oids.properties
com/sun/org/apache/xml/internal/serializer/{Encodings,XMLEntities*}.properties   (módulo java.xml)
sun/util/logging/resources/logging_en*.properties                                (módulo java.logging)
jdk/internal/icu/impl/data/icudt76b/nfkc.nrm                                     (módulo java.base)
```

Ningún `resource-config.json` de esquemas XSD explícito: xmlsec 3.0.5 no carga los esquemas XAdES
desde el classpath en este flujo (Enveloping, sin validación de esquema en la prefirma/postfirma).
Si un ticket posterior activa validación de esquema, esa lista crecerá y habrá que volver a
trazar.

## 3. Cuánto crece el `.so`: +54,0 % (28.772.448 → 44.304.480 B)

Medido con el **mismo** `-H:Name=librfirma_crypto`, el **mismo** `--shared`, la **misma**
ausencia de banderas de arquitectura, y comparando dos compilaciones desde limpio:

| Imagen | Classpath | Ficheros | `.so` principal |
|---|---|---|---|
| **Hoy** (`rfirma-native-bridge/testbench/build-native.sh`) | `pom.xml` actual: CAdES + PAdES | 6 (los 5 auxiliares de AWT de siempre, ADR-0004) | **28.772.448 B** (27,44 MiB) |
| **+ XAdES** (mismo classpath, más `afirma-server-triphase-signer-core` ya lo trae, más `xmlsec:3.0.5` explícito, y los dos `@CEntryPoint`: los siete de `NativeBridge` más uno del spike) | CAdES + PAdES + XAdES | 6 (los mismos 5 auxiliares; XAdES no añade ninguno) | **44.304.480 B** (42,25 MiB) |

Crecimiento: **+15.532.032 B, +54,0 %**. Los 5 auxiliares de AWT (`libawt.so`,
`libawt_headless.so`, `libawt_xawt.so`, `libjava.so`, `libjvm.so`) no cambian de tamaño ni de
cuenta: siguen siendo cosa de PAdES, no de XAdES (sección 4).

Aislado, sin CAdES/PAdES en el classpath, el árbol de XAdES + xmlsec + JAXP por sí solo compila a
**39.389.280 B** (37,56 MiB) en modo `--shared` y **39.454.976 B** (37,63 MiB) como ejecutable —
la diferencia entre los dos es solo la cabecera de isolate de `--shared`, no el contenido. Es más
grande en solitario que el `.so` de producción **completo** (CAdES+PAdES, 28,77 MiB): el peso no
es XAdES en sí, es traer xmlsec y su árbol de proveedores JCE al analizador de alcanzabilidad.

Residente al primer uso (ejecutable, ciclo prefirma+postfirma completo, `/usr/bin/time -v`):
**50.972 KiB** (49,8 MiB) de *maximum resident set size*. No hay una cifra equivalente publicada
para el `.so` de hoy con la que compararla directamente: quien la necesite, que mida con el mismo
`/usr/bin/time -v` sobre `run-native.sh`.

## 4. La exclusión de `afirma-ui-utils` no está amenazada, y `xalan` tampoco aparece

Ni `rfirma-native-bridge/target/cp.txt` (hoy) ni `xades-spike/target/cp.txt` contienen
`afirma-ui-utils`, `xalan` (el artefacto `xalan:xalan` que excluye
`afirma-crypto-core-xml/pom.xml` en el original), `xerces` ni `xml-apis`. XAdES no depende de
`afirma-ui-utils` — esa dependencia solo la trae `afirma-crypto-pdf`, y solo en tiempo de
ejecución, por reflexión (ver `exclusion-afirma-ui-utils.md`) — así que la pregunta del issue
("¿sobrevive la exclusión?") tiene una respuesta más fuerte que "sí": **la exclusión no tiene
nada que ver con XAdES**, y los `libawt.so` que sí aparecen en la imagen combinada de la sección 3
son el efecto ya conocido de PAdES, sin ninguna interacción nueva.

Lo que sí aparece en el árbol de código es `java.xml/com.sun.org.apache.xalan.internal.xsltc.compiler`
(654 KiB) — es el compilador XSLT **interno del JDK** (módulo `java.xml`, paquete con `internal`
en el nombre), sin relación con el artefacto Maven `xalan:xalan` que se excluye. Confirmado por
ausencia en el classpath, no solo por el nombre del paquete.

## 5. `PK1_DECODED` con ECDSA: no obliga a nada en la frontera

Leído en `XAdESTriPhasePreProcessor.java` (`~/…/clienteafirma`, tag `v1.9.2`), no medido con
trazas: cuando `AOSignConstants.isDSAorECDSASignatureAlgorithm(algorithm)`, `preProcessPreSign`
añade `PK1_DECODED=true` al `TriSign` de la sesión. `preProcessPostSign` lo lee de vuelta y, si
está a `true`, decodifica el PKCS#1 recibido con `Pkcs1Utils.decodeSignature` **antes** de
insertarlo en el XML — el preprocesador decodifica, no lo pide decodificado.

Verificado en este banco: `PK1_DECODED=true` con el certificado EC P-256, `PK1_DECODED=null`
(ausente) con RSA, y las dos sesiones completan la postfirma con una firma que valida (sección 1).
Rust no necesita saber qué es `PK1_DECODED` ni tratarlo distinto: es una propiedad más del
`TriSign`, y viaja igual que `TIME` en `CadesBridge` — opaca para el llamante, de ida y vuelta sin
tocar. El bridge que implemente XAdES no necesita ninguna rama nueva para ECDSA en la frontera FFI
por esta causa; Rust sigue devolviendo el PKCS#1 tal cual lo da PKCS#11/CNG/Keychain (ASN.1 DER),
igual que hoy hace para CAdES y PAdES.

## Recomendación

- **Añadir `org.apache.santuario:xmlsec:3.0.5` explícito** al `pom.xml` que implemente XAdES: no
  es opcional, `afirma-server-triphase-signer-core:1.9.2` lo excluye de su propia dependencia
  transitiva en `afirma-crypto-xades` (visto en su `.pom` de `~/.m2`), así que sin esta línea el
  fallo es un `NoClassDefFoundError` en tiempo de ejecución, no un error de compilación.
- **La configuración de alcanzabilidad de la sección 2 va al `pom.xml`/`build-native.sh`** tal
  cual, capturada con el agente sobre los JUnit que traiga el ticket de implementación —
  ejercitando RSA y ECDSA, las dos ramas cambian la sección `reflection`.
- **Ningún recorte a los tickets de XAdES**: el flujo compila, corre y valida sin adaptar código,
  sin tocar la exclusión de `afirma-ui-utils`, y sin ninguna rama nueva en la frontera FFI para
  ECDSA. El único coste es tamaño (+54 %, sección 3), que es una decisión de empaquetado
  (`packaging/flatpak/`), no de esta medición.
- **No determinado, fuera del alcance de esta nota**: XAdES Detached/Enveloped/Externally
  Detached (solo se midió Enveloping, que es el que cita el issue), cofirma/contrafirma,
  validación de esquema XSD explícita (que ampliaría la sección 2), y el mismo ciclo cruzando la
  frontera FFI real con Rust firmando fuera del isolate — esto último es del ticket de
  implementación, no de un spike.

## Cómo reproducir

```bash
export GRAALVM_HOME=$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce
cd rfirma-native-bridge/testbench/xades-spike
JAVA_HOME=$GRAALVM_HOME mvn -q package -DskipTests

# fixtures: reutiliza target/fixtures/ (make-fixtures.sh) y genera el par EC
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 \
  -keyout ../../../target/fixtures/ec-key.pem -out ../../../target/fixtures/ec-cert.pem \
  -days 3650 -nodes -subj "/CN=Prueba rfirma EC/O=rfirma/C=ES"
openssl pkcs12 -export -inkey ../../../target/fixtures/ec-key.pem \
  -in ../../../target/fixtures/ec-cert.pem -out ../../../target/fixtures/ec.p12 \
  -name ec -passout pass:1234
openssl pkcs12 -export -inkey ../../../target/fixtures/key.pem \
  -in ../../../target/fixtures/cert.pem -out ../../../target/fixtures/rsa.p12 \
  -name rsa -passout pass:1234

./trace.sh                      # metadatos de alcanzabilidad (RSA + EC)
./build-native.sh                # ejecutable, para las secciones 1 y 5
./build-native-shared.sh         # --shared aislado, para la sección 3
./build-native-combined.sh       # --shared con el classpath de produccion + XAdES, seccion 3

./run-jvm.sh   target/fixtures/rsa.p12 rsa 1234 SHA256withRSA in.xml out-jvm.xml
env -i PATH=/usr/bin:/bin ./target/native/xades-spike \
  target/fixtures/rsa.p12 rsa 1234 SHA256withRSA in.xml out-native.xml
./validate.sh out-native.xml target/fixtures/rsa.p12 rsa 1234
```
