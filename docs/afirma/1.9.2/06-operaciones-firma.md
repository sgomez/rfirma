# 06. Operaciones de firma: `sign`, `cosign`, `countersign`

Este capítulo describe las operaciones criptográficas fundamentales del protocolo
`afirma://` de AutoFirma 1.9.2: la firma electrónica inicial (`sign`), la cofirma
o multifirma en paralelo (`cosign`) y la contrafirma o multifirma en serie o cascada
(`countersign`). Se detalla la gramática de invocación, el procesamiento de
parámetros, la selección del almacén y certificado del firmante, la validación
previa y confirmaciones interactivas, la gestión de firma visible en documentos
PDF, la ejecución criptográfica, los plugins de procesado, el formato binario y
textual de la respuesta y el catálogo completo de errores.

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. La tríada de operaciones y su ciclo de despacho

Las tres operaciones comparten exactamente el mismo canal de entrada y el mismo
motor de ejecución en el código Java (`ProtocolInvocationLauncherSign`), pero
representan semánticas criptográficas distintas sobre los datos:

* **`sign`:** Firma electrónica de datos originales. Si los datos ya contenían
  una firma previa, según el formato se añadirá una nueva firma envolvente o
  coexistente, o se reemplazará la estructura previa si el formato no admite
  múltiples firmas.
* **`cosign` (cofirma):** Firma en paralelo al mismo nivel jerárquico que las
  firmas previas ya existentes en el documento (`signer.cosign(...)`). Los
  datos de entrada deben ser una firma electrónica válida preexistente.
* **`countersign` (contrafirma):** Firma de las firmas previas existentes en el
  documento en cascada jerárquica (`signer.countersign(...)`), ya sea sobre
  las hojas del árbol de firmas (`leafs`) o sobre todo el árbol (`tree`).

### 1.1 Prefijos de URI reconocidos

El despachador principal `ProtocolInvocationLauncher.launch` captura las tres
operaciones en un único bloque condicional
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncher.java:643-646`):

```java
else if (urlString.startsWith("afirma://sign?")        || urlString.startsWith("afirma://sign/?") ||
         urlString.startsWith("afirma://cosign?")      || urlString.startsWith("afirma://cosign/?") ||
         urlString.startsWith("afirma://countersign?") || urlString.startsWith("afirma://countersign/?")
)
```

Cada una de las tres operaciones se admite tanto sin barra (`afirma://<op>?`)
como con barra antes del interrogante (`afirma://<op>/?`).

### 1.2 Determinación de la operación (`op` vs `cop`)

El protocolo presenta una dualidad histórica entre los identificadores `op` y
`cop` (*crypto operation*):

1. **En la URI directa (`afirma://<op>?`):**
   El método `ProtocolInvocationUriParser.parserUri` analiza inicialmente los parámetros
   de la query string (`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParser.java:273-294`),
   donde un llamante podría haber incluido `op=...`. Sin embargo, inmediatamente después
   (`299-304`), extrae la parte de host/ruta de la URI y sobreescribe de forma incondicional
   la clave `op` (`ProtocolConstants.OPERATION_PARAM`):
   ```java
   params.put(ProtocolConstants.OPERATION_PARAM, path.indexOf("/") == -1 ? path : path.substring(0, path.indexOf("/")));
   ```
   Por tanto, en una URI directa como `afirma://sign?op=cosign`, el path de la URI (`"sign"`)
   tiene precedencia absoluta y sobreescribe cualquier valor que viaje en la query string.
2. **En la configuración por XML (servidor intermedio):**
   Cuando los parámetros se descargan desde el servlet intermedio mediante `fileid`,
   el analizador XML `ProtocolInvocationUriParserUtil.parseXml`
   (`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParserUtil.java:99-129`)
   inspecciona en primer lugar el nombre del elemento raíz: si es `<sign>`, `<cosign>`
   o `<countersign>`, dicho nombre se fija como valor inicial de `op` (o `"SIGN"` si el
   nodo raíz se llama `<op>`). A continuación, el bucle que procesa las etiquetas hijas
   `<e k="..." v="..."/>` (`111-129`) asigna cada par clave-valor al mapa, por lo que una
   etiqueta explícita `<e k="op" v="..."/>` sobreescribe el nombre del elemento raíz.
   Dado que el despachador general `ProtocolInvocationLauncher.java:642-651` canaliza
   `sign`, `cosign` y `countersign` a través del mismo flujo de descarga y parseo de XML,
   la operación final ejecutada por `processSign` vendrá determinada enteramente por el
   valor de `op` resultante del XML, con independencia de si la URI de arranque fue
   `afirma://sign?fileid=...` o `afirma://cosign?fileid=...`.
3. **Conversión interna al enumerado `Operation`:**
   En `UrlParametersToSign.setSignParameters`
   (`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSign.java:246-248`),
   se lee `params.get("op")`. Luego, en `ProtocolInvocationLauncherSign.processSign`
   (`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherSign.java:158`),
   se convierte al enumerado interno mediante `Operation.getOperation(op)`
   (`afirma-simple-plugins/src/main/java/es/gob/afirma/standalone/plugins/SignOperation.java:67-78`):
   * `"SIGN"` (insensible a mayúsculas) → `Operation.SIGN`
   * `"COSIGN"` (insensible a mayúsculas) → `Operation.COSIGN`
   * `"COUNTERSIGN"` (insensible a mayúsculas) → `Operation.COUNTERSIGN`
   * Cualquier otro valor devuelve `null`.
4. **El rol del parámetro `cop`:**
   En las operaciones directas de este capítulo, el cliente JavaScript
   (`autoscript.js:1958, 2944`) envía `op=sign`, `op=cosign` o `op=countersign`.
   El parámetro `cop` no se utiliza en `UrlParametersToSign`; está reservado para
   `signandsave` (`UrlParametersToSignAndSave.java:29, 237`, donde `op` es
   siempre `"signandsave"` y `cop` indica si se firma, cofirma o contrafirma),
   y para las peticiones HTTP internas hacia los servidores de firma trifásica
   (`afirma-crypto-cadestri-client/.../ProtocolConstants.java:40`).

---

## 2. Parámetros de la invocación de firma

La clase que encapsula la configuración es `UrlParametersToSign`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSign.java`),
que hereda de la clase base abstracta `UrlParameters`.

```
                    ┌────────────────────────┐
                    │     UrlParameters      │
                    │  (parámetros comunes)  │
                    └───────────┬────────────┘
                                │ extends
                    ┌───────────┴────────────┐
                    │  UrlParametersToSign   │
                    │ (parámetros de firma)  │
                    └────────────────────────┘
```

### 2.1 Tabla de parámetros aceptados

`UrlParametersToSign.KNOWN_PARAMETERS` (`UrlParametersToSign.java:52-57`) define la
lista cerrada de parámetros reconocidos por el analizador para esta operación:

| Parámetro | Requerido | Tipo | Origen | Descripción |
|---|---|---|---|---|
| `op` | Sí | Texto | Sintaxis URI / XML | Operación solicitada: `sign`, `cosign` o `countersign`. |
| `format` | Sí* | Texto | `UrlParametersToSign:28` | Formato de la firma (p. ej. `CAdES`, `PAdES`, `XAdES`, `auto`). |
| `algorithm` | Sí* | Texto | `UrlParametersToSign:31` | Algoritmo criptográfico de resumen o firma (p. ej. `SHA256`). |
| `dat` | Condicional | Base64 URL-safe | `UrlParameters:34` | Datos a firmar o firma previa a multifirmar. |
| `gzip` | No | Booleano | `UrlParameters:40` | `true` si los datos en `dat` vienen comprimidos en GZIP. |
| `fileid` | Condicional | Alfanumérico | `UrlParameters:59` | Identificador para descargar datos/configuración de `rtservlet`. |
| `rtservlet` | Condicional | URL HTTP/HTTPS | `UrlParameters:43` | Servlet de descarga remota de datos (`RetrieveService`). |
| `stservlet` | Condicional | URL HTTP/HTTPS | `UrlParameters:46` | Servlet de subida remota del resultado (`StorageService`). |
| `id` | Condicional | Alfanumérico | `UrlParametersToSign:34` | Identificador de sesión para el guardado en `stservlet` (máx. 20 caracteres). |
| `key` | No | 8 caracteres ASCII | `UrlParameters:56` | Clave DES para cifrado simétrico en servidor intermedio. |
| `properties` | No | Base64 | `UrlParameters:31` | Parámetros adicionales de configuración en formato `.properties`. |
| `keystore` | No | Texto | `UrlParameters:63` | Nombre del almacén de claves en texto plano (obsoleto). |
| `ksb64` | No | Base64 | `UrlParameters:66` | Nombre del almacén de claves codificado en Base64. |
| `sticky` | No | Booleano | `UrlParametersToSign:42` | `true` para fijar el certificado en memoria para siguientes llamadas. |
| `resetsticky` | No | Booleano | `UrlParametersToSign:46` | `true` para ignorar y descartar un certificado fijado previamente. |
| `ver` | No | Entero | `UrlParametersToSign:38` | Versión mínima requerida del protocolo (por defecto `0`). |
| `mcv` | No | Versión (X.Y.Z) | `UrlParameters:73` | Versión mínima requerida de la aplicación AutoFirma. |
| `aw` | No | Booleano | `UrlParameters:70` | `true` para activar el hilo de espera activa hacia `stservlet`. |
| `appname` | No | Texto | `UrlParameters:76` | Nombre o dominio web de la aplicación que invoca AutoFirma. |

*\* Nota:* Si se indica `fileid`, los parámetros `format`, `algorithm`, `properties`
y `dat` no se exigen en la URI inicial, ya que se descargarán dentro del XML desde el
servidor intermedio (`UrlParametersToSign.java:251-253`).

### 2.2 `format`: Formatos de firma y el pseudomodo `auto`

El parámetro `format` es obligatorio en la invocación directa
(`UrlParametersToSign.java:279-281`). Si no se recibe, se lanza
`ParameterException("No se ha recibido el formato de firma")`, que deviene en el
error `SAF_03`.

#### A) Catálogo de formatos reconocidos

La factoría `AOSignerFactory`
(`afirma-core/src/main/java/es/gob/afirma/core/signers/AOSignerFactory.java:51-81`)
mantiene la correspondencia entre identificadores de formato y sus clases
manejadoras (`AOSigner`):

| Formato (`format`) | Clase manejadora | Identificación de firmas previa |
|---|---|---|
| `CAdES` | `es.gob.afirma.signers.cades.AOCAdESSigner` | Sí |
| `CAdEStri` | `es.gob.afirma.signers.cadestri.client.AOCAdESTriPhaseSigner` | No |
| `CAdES-ASiC-S` | `es.gob.afirma.signers.cades.asic.AOCAdESASiCSSigner` | Sí |
| `CAdES-ASiC-S-tri` | `es.gob.afirma.signers.cadestri.client.asic.AOCAdESASiCSTriPhaseSigner` | No |
| `CMS/PKCS#7` | `es.gob.afirma.signers.cms.AOCMSSigner` | Sí |
| `FacturaE`, `Factura-E` | `es.gob.afirma.signers.xades.AOFacturaESigner` | Sí |
| `FacturaEtri` | `es.gob.afirma.signers.xadestri.client.AOFacturaETriPhaseSigner` | No |
| `XAdES` | `es.gob.afirma.signers.xades.AOXAdESSigner` | Sí |
| `XAdES Detached` | `es.gob.afirma.signers.xades.AOXAdESSigner` | No |
| `XAdES Enveloped` | `es.gob.afirma.signers.xades.AOXAdESSigner` | No |
| `XAdES Enveloping` | `es.gob.afirma.signers.xades.AOXAdESSigner` | No |
| `XAdEStri` | `es.gob.afirma.signers.xadestri.client.AOXAdESTriPhaseSigner` | No |
| `XAdES-ASiC-S` | `es.gob.afirma.signers.xades.asic.AOXAdESASiCSSigner` | Sí |
| `XAdES-ASiC-S-tri` | `es.gob.afirma.signers.xadestri.client.asic.AOXAdESASiCSTriPhaseSigner` | No |
| `XMLDSig` | `es.gob.afirma.signers.xmldsig.AOXMLDSigSigner` | Sí |
| `XMLDSig Detached` | `es.gob.afirma.signers.xmldsig.AOXMLDSigSigner` | No |
| `XMLDSig Enveloped` | `es.gob.afirma.signers.xmldsig.AOXMLDSigSigner` | No |
| `XMLDSig Enveloping` | `es.gob.afirma.signers.xmldsig.AOXMLDSigSigner` | No |
| `Adobe PDF`, `PAdES` | `es.gob.afirma.signers.pades.AOPDFSigner` | Sí |
| `Adobe PDF TriPhase`, `PAdEStri` | `es.gob.afirma.signers.padestri.client.AOPDFTriPhaseSigner` | No |
| `ODF (Open Document Format)`, `ODF` | `es.gob.afirma.signers.odf.AOODFSigner` | Sí |
| `OOXML (Office Open XML)`, `OOXML` | `es.gob.afirma.signers.ooxml.AOOOXMLSigner` | Sí |
| `NONE`, `PKCS1`, `PKCS#1` | `es.gob.afirma.core.signers.AOPkcs1Signer` | No |
| `NONEtri` | `es.gob.afirma.core.signers.AOPkcs1TriPhaseSigner` | No |

Si se indica un formato no contemplado en la tabla y que no sea `"auto"`,
`AOSignerFactory.getSigner(format)` devuelve `null`, lo que genera inmediatamente
el error `SAF_06` (`ERROR_UNSUPPORTED_FORMAT`)
(`ProtocolInvocationLauncherSign.java:266-272`).

#### B) Resolución del formato `auto`

Si `format` equivale a `AOSignConstants.SIGN_FORMAT_AUTO` (`"auto"`, insensible a
mayúsculas), el firmador se deja en `null` provisionalmente y se resuelve una vez
disponibles los datos a firmar (`ProtocolInvocationLauncherSign.java:381-390`),
llamando a `ProtocolInvocationLauncherUtil.identifyFormatFromData(data, cryptoOperation)`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherUtil.java:150-180`).

Existe una asimetría técnica fundamental en la resolución automática según la operación:

* **Si la operación es `SIGN` (`153-166`):**
  Los datos de entrada representan el documento original que se pretende firmar por
  primera vez. Se aplican heurísticas de análisis de contenido binario (`DataAnalizerUtil`):
  1. Si `DataAnalizerUtil.isPDF(data)` → `PAdES`.
  2. Si no, si `DataAnalizerUtil.isFacturae(data)` → `FacturaE`.
  3. Si no, si `DataAnalizerUtil.isXML(data)` → `XAdES`.
  4. En cualquier otro caso → `CAdES` (formato comodín por defecto para cualquier fichero binario o textual arbitrario).
* **Si la operación es `COSIGN` o `COUNTERSIGN` (`167-177`):**
  Como se solicita una multifirma, los datos de entrada **deben constituir obligatoriamente
  una firma electrónica preexistente**. No se analiza la extensión ni el tipo MIME del
  documento, sino que se invoca `AOSignerFactory.getSigner(data)`
  (`afirma-core/src/main/java/es/gob/afirma/core/signers/AOSignerFactory.java:92-118`),
  el cual recorre secuencialmente la matriz de firmadores registrados con capacidad de
  identificación (`SIGNERS_CLASSES`, evaluando `signer.isSign(data)`) en este orden estricto:
  1. `CAdES` (`AOCAdESSigner`)
  2. `CAdES-ASiC-S` (`AOCAdESASiCSSigner`)
  3. `CMS/PKCS#7` (`AOCMSSigner`)
  4. `FacturaE` (`AOFacturaESigner`)
  5. `XAdES` (`AOXAdESSigner`)
  6. `XAdES-ASiC-S` (`AOXAdESASiCSSigner`)
  7. `XMLDSig` (`AOXMLDSigSigner`)
  8. `PAdES` (`AOPDFSigner`)
  9. `ODF` (`AOODFSigner`)
  10. `OOXML` (`AOOOXMLSigner`)

  Una vez hallado el primer firmador cuya llamada a `isSign(data)` devuelve `true`, se
  recupera su nombre mediante `AOSignerFactory.getSignFormat(signer)`. Si ninguno lo
  reconoce (por ejemplo, si se introducen datos binarios o XML planos que no son firmas),
  `getSigner` devuelve `null` y la operación se aborta de inmediato con el error fatal
  `SAF_17` (`ERROR_UNKNOWN_SIGNER`).

  **Consecuencias de compatibilidad:**
  - Un documento plano sin firma previa remitido con `format=auto` a `sign` se aceptará
    y firmará (como XAdES si es XML o CAdES si es binario/texto), mientras que el mismo
    fichero enviado a `cosign` o `countersign` fallará con `SAF_17`.
  - Dado que `CAdES` precede a `CMS` en la matriz de búsqueda y las firmas CAdES derivan
    de CMS/PKCS#7, una firma CMS estándar será identificada y clasificada como `CAdES`.
  - Los contenedores ASiC (`.asics`), clasificados como `CAdES` en `sign` (al no ser PDF
    ni XML), serán correctamente identificados como `CAdES-ASiC-S` o `XAdES-ASiC-S` en
    multifirma por sus respectivos reconocedores específicos.

### 2.3 `algorithm`: Algoritmos de firma y composición dinámica

El parámetro `algorithm` debe coincidir exactamente con uno de los valores
admitidos en la lista blanca estricta `SUPPORTED_SIGNATURE_ALGORITHMS`
(`UrlParametersToSign.java:60-74, 287-293`):

```
SHA1, SHA256, SHA384, SHA512,
SHA1withRSA, SHA256withRSA, SHA384withRSA, SHA512withRSA,
SHA1withECDSA, SHA256withECDSA, SHA384withECDSA, SHA512withECDSA
```

Cualquier otro algoritmo produce `ParameterException("Algoritmo de firma no soportado: " + algo)`.

#### Composición con la clave del certificado (`composeSignatureAlgorithmName`)

Aunque la aplicación web indique un algoritmo genérico como `SHA256` o uno
específico como `SHA256withRSA`, el algoritmo final de firma se recompone
dinámicamente tras conocer el tipo de clave privada del certificado seleccionado
(`ProtocolInvocationLauncherSign.java:632-640` y
`afirma-core/src/main/java/es/gob/afirma/core/signers/AOSignConstants.java:365-390`):

1. Se obtiene el algoritmo de la clave privada: `keyType = pke.getPrivateKey().getAlgorithm()`.
2. Se extrae el algoritmo de resumen eliminando guiones (`SHA256`, `SHA512`, etc.).
3. Se añade el sufijo correspondiente al tipo de clave:
   * Si `keyType` es `"RSA"` → añade `withRSA`.
   * Si `keyType` es `"DSA"` → añade `withDSA`.
   * Si `keyType` comienza por `"EC"` → añade `withECDSA`.
   * Si es otro tipo → lanza `IllegalArgumentException`, que deviene en el error
     `SAF_51` (`ERROR_INCOMPATIBLE_KEY_TYPE`).

Esto garantiza que si la web solicita `SHA256withRSA`, pero el usuario elige un
certificado en curva elíptica (`EC`), AutoFirma transmuta automáticamente el
algoritmo a `SHA256withECDSA` sin fallar.

### 2.4 `dat` y selección interactiva de fichero

Los datos pueden suministrarse directamente en la URI mediante el parámetro `dat` en
Base64 URL-safe, con o sin compresión GZIP (`gzip=true`). Por motivos de seguridad,
se prohíbe terminantemente el uso de URIs con esquema local: si `dat` comienza por
`file:/`, se produce una `ParameterException` inmediata
(`UrlParameters.java:300-304`).

#### Selección interactiva cuando `dat` es nulo (`needRequestData`)

Si no se proporciona `dat` ni `fileid` (o la descarga remota no devolvió datos),
AutoFirma evalúa si es obligatorio solicitar el fichero al usuario
(`ProtocolInvocationLauncherSign.java:301-308`):

* Si el firmador implementa `OptionalDataInterface`
  (`afirma-core/src/main/java/es/gob/afirma/core/signers/OptionalDataInterface.java`),
  se consulta `signer.needData(cryptoOperation, extraParams)`. Actualmente sólo
  `AOXAdESSigner` y `AOXAdESTriPhaseSigner` implementan esta interfaz para permitir
  firmas de tipo *Detached* con referencia externa sin aportar los datos binarios.
* En cualquier otro caso, `needRequestData` se marca a `true`.

Cuando `needRequestData` es `true`, AutoFirma despliega un diálogo nativo de
selección de fichero (`AOUIFactory.getLoadFiles`, `327-377`):

1. **Título del diálogo:**
   * En `SIGN`: etiqueta `"ProtocolLauncher.25"` («Seleccione el fichero de datos a firmar»).
   * En `COSIGN`/`COUNTERSIGN`: etiqueta `"ProtocolLauncher.26"` («Seleccione el fichero de firma»).
2. **Personalización mediante `properties`:**
   El diálogo se parametriza mediante las claves de `AfirmaExtraParams`
   (`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/AfirmaExtraParams.java`):
   * `filenameExts`: Extensiones permitidas separadas por comas (ej. `"pdf,txt"`).
   * `filenameDescription`: Texto explicativo del filtro de extensiones.
   * `filenameCurrentDir`: Directorio inicial de navegación.
   * `filenameActualName`: Nombre de fichero propuesto por defecto.
3. **Cancelación:**
   Si el usuario pulsa cancelar en la selección de fichero, se captura
   `AOCancelledOperationException` y se aborta devolviendo `"CANCEL"` (`RESULT_CANCEL`,
   `354-357`).
4. **Metadatos resultantes:**
   El nombre simple del fichero seleccionado se guarda en la variable `inputFilename`
   (`360`), la cual se incluirá más adelante en el resultado devuelto a la web.

### 2.5 `properties`: Parámetros adicionales y expansión en dos tiempos

El parámetro `properties` transporta una colección de propiedades Java (`.properties`)
codificadas en Base64 (`UrlParametersToSign.java:298-314`). Estas propiedades se
procesan en dos etapas:

1. **Borrado incondicional de `profile`:**
   Nada más arrancar `processSign`, el código ejecuta de forma imperativa:
   ```java
   //TODO: Deshacer cuando se permita la generacion de firmas baseline
   options.getExtraParams().remove("profile");
   ```
   (`ProtocolInvocationLauncherSign.java:152-153`). Esta misma eliminación se repite
   en `ProtocolInvocationLauncherSignAndSave.java:149-150` y en los procesadores del
   servidor trifásico para lotes (`JSONSingleSignPreProcessor.java:99`, `SingleSignPreProcessor.java:87`).
   En consecuencia, en AutoFirma 1.9.2 **es estrictamente imposible generar perfiles
   *baseline*** (como `PAdES-Baseline-B`, `PAdES-Baseline-T`, `CAdES-Baseline-B`, etc.)
   a través del protocolo `afirma://`. La propiedad se descarta de forma transparente y
   silenciosa, sin elevar ningún error de parámetros (`SAF_03`) ni advertencia; los motores
   criptográficos generan invariablemente firmas tradicionales en formato CMS/CAdES,
   PAdES o XAdES avanzado/básico.
2. **Expansión semántica (`ExtraParamsProcessor.expandProperties`):**
   Antes de invocar al firmador, las propiedades se someten a expansión
   (`ProtocolInvocationLauncherSign.java:488-496` y
   `afirma-core/src/main/java/es/gob/afirma/core/signers/ExtraParamsProcessor.java`).
   Aquí se resuelven macro-políticas como `expPolicy=AGE` (traduciéndola a los
   OIDs, huellas digitales y URLs oficiales de la Administración General del
   Estado) y se contrastan incompatibilidades de formato. Si la política es
   incompatible con el formato o los datos, se lanza `IncompatiblePolicyException`,
   que deviene en el error `SAF_23` (`ERROR_INVALID_POLICY`).

### 2.6 `target`: Ámbito de la contrafirma

En las operaciones de contrafirma (`Operation.COUNTERSIGN`), el parámetro
adicional `target` dentro de `properties` define qué elementos del árbol de firmas
se van a contrafirmar (`ProtocolInvocationLauncherSign.java:723-724`):

```java
signature = signer.countersign(
        data,
        algorithm,
        "tree".equalsIgnoreCase(extraParams.getProperty(AfirmaExtraParams.TARGET)) ?
                CounterSignTarget.TREE : CounterSignTarget.LEAFS,
        null, // Targets
        pke.getPrivateKey(),
        pke.getCertificateChain(),
        extraParams
);
```

Reglas del protocolo respecto al objetivo de contrafirma:
* **Evaluación binaria estricta:** A diferencia de la API Java que dispone del validador
  `CounterSignTarget.getTarget(name)` con control estricto de sintaxis, la capa de protocolo
  bypassea dicho método mediante una comprobación ternaria directa.
* **Activación de `TREE`:** Solo si `target="tree"` (insensible a mayúsculas) se activa
  la contrafirma de todo el árbol (`CounterSignTarget.TREE`).
* **Degradación silenciosa a `LEAFS`:** Cualquier otro valor —incluyendo la omisión del
  parámetro, valores nulos o erratas tipográficas del cliente web (como `"all"`, `"lefs"`
  o `"leaf"`)— se resuelve por defecto como `CounterSignTarget.LEAFS` sin emitir ninguna
  advertencia ni arrojar error de parámetros (`SAF_03`).
* **Inaccesibilidad de nodos específicos:** Debido a que el parámetro `targets` (lista de
  firmantes o identificadores de nodo) se entrega fijado en `null`, las variantes `SIGNERS`
  y `NODES` definidas en `CounterSignTarget.java` no son invocables a través del protocolo
  `afirma://`.

---

## 3. Selección de almacén, certificado y gestión de PIN

El proceso de obtención de la clave privada (`PrivateKeyEntry`) sigue una
jerarquía rigurosa antes de mostrar cualquier diálogo a la persona usuaria.

### 3.1 Resolución del almacén de claves (`AOKeyStore`)

El almacén a utilizar se resuelve en `ProtocolInvocationLauncherSign.sign`
(`274-298`) siguiendo este orden de prioridad decreciente:

```
┌───────────────────────────────────────────────────────────┐
│ 1. ¿Hay almacén previo en sesión?                        │
│    KeyStorePreferencesManager.getLastSelectedKeystore()   │
└─────────────────────────────┬─────────────────────────────┘
                              │ No
┌─────────────────────────────▼─────────────────────────────┐
│ 2. ¿Preferencia usar almacén por defecto en navegador?    │
│    PreferencesManager.PREFERENCE_KEYSTORE_DEFAULT_STORE   │
└─────────────────────────────┬─────────────────────────────┘
                              │ No
┌─────────────────────────────▼─────────────────────────────┐
│ 3. ¿Almacén indicado en la invocación?                    │
│    options.getDefaultKeyStore() (ksb64 / keystore)        │
└─────────────────────────────┬─────────────────────────────┘
                              │ No
┌─────────────────────────────▼─────────────────────────────┐
│ 4. Almacén por defecto del Sistema Operativo              │
│    AOKeyStore.getDefaultKeyStoreTypeByOs(Platform.getOS())│
└───────────────────────────────────────────────────────────┘
```

1. **Último almacén seleccionado:** Si en la misma ejecución ya se eligió un
   almacén previamente, se reutiliza (`280-282`).
2. **Preferencia local:** Si en la configuración de AutoFirma está marcado
   `useDefaultStoreInBrowserCalls`, se usa dicho almacén predeterminado (`284-289`).
3. **Parámetro `ksb64` / `keystore`:** Almacén solicitado en la URI (`290-293`).
4. **Defecto del SO:** Windows (`WINDOWS`), macOS (`APPLE`), Linux (`MOZ_UNI`).

### 3.2 Persistencia en sesión: `sticky` y `resetsticky`

Para evitar que el usuario deba seleccionar su certificado repetidas veces en un
trámite con múltiples firmas consecutivas:

* Si `sticky=true`, `resetsticky=false`, `ProtocolInvocationLauncher.getStickyKeyEntry() != null`
  y no hay una clave prefijada por parámetro: se reutiliza directamente la clave
  privada en memoria (`pke = ProtocolInvocationLauncher.getStickyKeyEntry()`, `518-522`),
  omitiendo la apertura del almacén y el diálogo de selección.
* Tras completar la selección de un nuevo certificado, si `sticky=true`, se almacena
  en la variable estática (`ProtocolInvocationLauncher.setStickyKeyEntry(pke)`, `643`).
  Si `sticky=false`, se fuerza la limpieza de la clave estática asignando `null`.
* Si el almacén resulta estar bloqueado (`LockedKeyStoreException`), la clave
  persistente se invalida de inmediato asignando `null` (`654`).

### 3.3 Filtrado y selección de certificado (`CertFilterManager`)

Cuando no hay un certificado prefijado, se instancia el gestor de almacenes
(`AOKeyStoreManager`) y se configura el diálogo `AOKeyStoreDialog`
(`ProtocolInvocationLauncherSign.java:574-629`).

#### A) Extracción de filtros

`CertFilterManager` (`afirma-keystores-filters/.../CertFilterManager.java:117-136`)
extrae las condiciones especificadas en `properties` bajo las claves `filter` o
`filters`:

* Admite expresiones RFC 2254 sobre emisor y sujeto (`subject.rfc2254:`, `issuer.rfc2254:`).
* Búsqueda por subcadena (`subject.contains:`, `issuer.contains:`).
* Filtrado por huella digital SHA-1 (`thumbprint:`).
* Filtrado por KeyUsage (`keyusage.digitalsignature:`, `keyusage.nonrepudiation:`, etc.).
* Filtrado por origen: DNIe (`dnie:`), certificados cualificados (`qualified:`), SSCD (`sscd:`).
* **Filtro por defecto:** Siguiendo la directriz ETSI TS 119 102-1, si no se declara
  ningún filtro explícito, AutoFirma inyecta automáticamente un filtro que excluye los
  certificados caducados (`ExpiredCertificateFilter(false)`, `CertFilterManager.java:133-135`).

#### B) Autoselección si solo hay un candidato (`mandatoryCertificate` vs `mandatoryCertSelection`)

La propiedad `mandatoryCertSelection` en `properties` controla si la selección
interactiva manual por parte del usuario en el diálogo de certificados es preceptiva:

* **Por defecto o `mandatoryCertSelection=true`:**
  La selección manual por el usuario es obligatoria; el diálogo `AOKeyStoreDialog` se
  muestra **siempre**, incluso cuando en el almacén existe un único certificado que supera
  los filtros configurados.
* **Modo `mandatoryCertSelection=false`:**
  La selección interactiva no es obligatoria. Si tras aplicar los filtros de certificado
  queda **exactamente un único candidato válido**, AutoFirma omite por completo el diálogo
  gráfico y autoselecciona dicho certificado de manera transparente.
* **Modo desatendido (`headless=true`):**
  AutoFirma asume internamente que no debe mostrar interfaz gráfica, activando la misma
  vía de autoselección unitaria.

**Razón de la aparente inversión lógica en el código:**
En `CertFilterManager.java:145-154`, el método auxiliar se denomina `isMandatoryCertificate`:
```java
final boolean omitSelection = propertyFilters != null
        && propertyFilters.containsKey(MANDATORY_CERT_SELECTION_PROPERTY)
        && Boolean.FALSE.toString().equalsIgnoreCase(
                propertyFilters.getProperty(MANDATORY_CERT_SELECTION_PROPERTY));

return headless || omitSelection;
```
El nombre de este método proviene del parámetro booleano del constructor de
`AOKeyStoreDialog` (`mandatoryCertificate`, `afirma-core-keystores/.../AOKeyStoreDialog.java:136, 726`),
cuyo significado histórico en el diálogo era «¿debe forzarse la autoselección si el
certificado es el único disponible?»:
```java
if (this.mandatoryCertificate && namedCertificates != null && namedCertificates.length == 1) {
    this.selectedAlias = namedCertificates[0].getAlias();
    return this.selectedAlias;
}
```
Por tanto, cuando la sede web declara `mandatoryCertSelection=false` («no es obligatoria
la selección manual»), `omitSelection` evalúa a `true`, `this.mandatoryCertificate` se fija
en `true` y el diálogo gráfico se omite si hay un solo certificado candidato. Si tras el
filtrado existen 0 o más de un certificado, el diálogo se abre normalmente (o falla con
`HeadlessException` si opera en modo *headless*).

#### C) Cancelación o ausencia de certificados

* Si el usuario cierra o cancela el diálogo de selección de certificados, se captura
  `AOCancelledOperationException` y se devuelve `"CANCEL"` (`RESULT_CANCEL`,
  `ProtocolInvocationLauncherSign.java:615-618`).
* Si el almacén no contiene ningún certificado que supere los filtros, se lanza
  `AOCertificatesNotFoundException`, devolviendo el error `SAF_19`
  (`ERROR_NO_CERTIFICATES_KEYSTORE`, `619-623`).
* Si falla la comunicación con el almacén subyacente (token desconectado, fallo
  PKCS#11), se produce `SAF_08` (`ERROR_CANNOT_ACCESS_KEYSTORE`, `624-628`).

### 3.4 Manejo de fallos de PIN (`PinException`) y reintento guiado

Si durante la firma el token criptográfico rechaza el PIN introducido
(`PinException`, `659-676`):

1. AutoFirma captura la excepción sin abortar el trámite.
2. Extrae el certificado X.509 que se intentó usar y genera un filtro unitario
   exacto mediante `EncodedCertificateFilter(Base64.encode(certEncoded))`.
3. Vuelve a invocar de forma recursiva a `selectCertAndSign(...)` forzando la
   selección exclusiva de ese mismo certificado. Esto fuerza al controlador del
   token a solicitar de nuevo el PIN al usuario sin requerir que vuelva a buscar
   el certificado en la lista.

Si el token se bloquea por agotar intentos (`LockedKeyStoreException`), se
invalida cualquier clave en `stickyKeyEntry` y se eleva el error `SAF_52`
(`ERROR_LOCKED_KEYSTORE`, `650-657`).

---

## 4. Validación previa y confirmaciones interactivas

AutoFirma contempla un mecanismo de inspección del documento antes de estampar la
firma y un sistema de avisos de seguridad que pueden requerir la confirmación de la
persona usuaria.

### 4.1 Comprobación previa de firmas (`checkSignatures`)

Si en `properties` se incluye `checkSignatures=true`
(`ProtocolInvocationLauncherSign.java:409-483`), AutoFirma comprueba el estado del
documento antes de proceder:

1. Obtiene el validador correspondiente al formato mediante `SignValiderFactory.getSignValider(signer)`.
2. Fija la validación en modo relajado (`validator.setRelaxed(true)`), lo que
   permite advertir problemas sin abortar ciegamente.
3. **Protección PSA en PDF:** Si se trata de un PDF, `configurePdfSignature` (`866-871`)
   comprueba `allowPdfShadowAttack`: si no es `true` y no se especificó
   `pagesToCheckPSA`, analiza por defecto las últimas 10 páginas (`pagesToCheckPSA="10"`).
4. Si la validez resulta `KO`, la operación aborta con `SAF_39`
   (`ERROR_INVALID_SIGNATURE`), salvo que la operación sea `SIGN` y el error sea
   únicamente `NO_SIGN` (lo cual es normal, pues se va a firmar por primera vez).

### 4.2 Excepciones de confirmación interactiva (`RuntimeConfigNeededException`)

Tanto durante la validación previa como durante la firma real (`executeSign`,
`789-837`), el motor criptográfico puede emitir advertencias de seguridad mediante
subclases de `RuntimeConfigNeededException`:

| Excepción | Parámetro que la desbloquea | Causa y diálogo mostrado |
|---|---|---|
| `SuspectedPSAException` | `allowPdfShadowAttack` | Sospecha de ataque *PDF Shadow Attack*. Aviso de posible alteración del contenido. |
| `PdfHasUnregisteredSignaturesException` | `allowCosigningUnregisteredSignatures` | El PDF contiene firmas no registradas en AcroFields que podrían invalidarse. |
| `PdfIsCertifiedException` | `allowSigningCertifiedPdfs` | El documento PDF está certificado; firmarlo podría anular la certificación. |
| `PdfFormModifiedException` | `allowSigningFormModifiedPdfs` | Se detectan modificaciones en campos de formulario tras la firma previa. |
| `SigningLTSException` | `allowSigningLtsSignatures` | Multifirma sobre una firma con sello de tiempo a largo plazo (LTS). |
| `AGEPolicyIncompatibilityException` | `avoidAGEPolicyIncompatibilities` | La firma no cumple la política AGE. Al confirmar, `prepareOperationWithConfirmation` purga las claves de política (`policyIdentifier`, etc.) y permite firmar sin ella. |
| `PdfIsPasswordProtectedException` | `userPassword` / `ownerPassword` | El PDF requiere contraseña de apertura o edición. Requiere entrada de contraseña. |

#### Comportamiento del diálogo interactivo

* **Tipo `RequestType.CONFIRM` (`798-810`):** Muestra un diálogo `JOptionPane.YES_NO_OPTION`
  con el texto del aviso (`SimpleAfirmaMessages.getString(e.getRequestorText())`).
  * Si el usuario pulsa **SÍ:** Se inyecta la propiedad en `extraParams` (`extraParams.setProperty(e.getParam(), "true")`)
    o se ejecuta `prepareOperationWithConfirmation(extraParams)` si es una excepción
    personalizada, y se reintenta la firma recursivamente.
  * Si el usuario pulsa **NO:** Se aborta la operación devolviendo `"CANCEL"`.
* **Tipo `RequestType.PASSWORD` (`812-829`):** Solicita la contraseña mediante
  `AOUIFactory.getPassword(...)`. Si se introduce, se asigna al parámetro y se reintenta;
  si se cancela, se devuelve `"CANCEL"`.
* **Modo Desatendido (`headless=true`):**
  Si en `properties` se fijó `headless=true`, AutoFirma tiene prohibido mostrar
  ventanas. Si salta cualquier `RuntimeConfigNeededException`, no se puede solicitar
  confirmación y se lanza inmediatamente el error `SAF_50` (`ERROR_CONFIRMATION_NEEDED`,
  `793-796`).

---

## 5. Firma visible en PDF y posicionamiento de rúbrica

Cuando se firma un documento en formato PDF/PAdES, AutoFirma permite posicionar
visualmente un sello o estampa gráfica de firma.

### 5.1 Criterios de activación (`isRubricPositionRequired`)

El método `isRubricPositionRequired` (`ProtocolInvocationLauncherSign.java:914-936`)
determina si se debe mostrar el diálogo interactivo de posicionamiento:

1. El formato debe ser de la familia PAdES (`Adobe PDF`, `Adobe PDF TriPhase`,
   `PAdES`, `PAdEStri`, `adbe.pkcs7.detached`, `ETSI.CAdES.detached`).
2. Debe existir en `properties` la clave `visibleSignature` (`PdfExtraParams.VISIBLE_SIGNATURE`).
3. El valor de `visibleSignature` debe ser estrictamente `"want"` o `"optional"`
   (insensible a mayúsculas). Si tiene cualquier otro valor, se ignora.

### 5.2 El diálogo `SignPdfDialog` y propiedades resultantes

Si se cumplen los criterios, se abre la ventana modal `SignPdfDialog` (`945-953`),
permitiendo al usuario dibujar un recuadro sobre la página del documento o elegir
el aspecto visual si `visibleAppearance=custom` (`990-999`).

Al aceptar el diálogo, `SignPdfListener.updateOptions` (`1027-1074`) vuelca en
`extraParams` las siguientes propiedades:

* **Coordenadas del área:**
  `signaturePositionOnPageLowerLeftX`, `signaturePositionOnPageLowerLeftY`,
  `signaturePositionOnPageUpperRightX`, `signaturePositionOnPageUpperRightY`.
* **Página:** `signaturePage` o `signaturePages`.
* **Tipografía y estilo (si aplica):**
  `layer2Text`, `layer2FontFamily`, `layer2FontSize`, `layer2FontStyle`,
  `layer2FontColor`, `signatureRotation`, `signatureRubricImage`.

### 5.3 Semántica de `visibleSignature`: `want` vs `optional`

La diferencia entre ambos modos se evalúa en `checkShowRubricDialogIsCanceled` (`960-980`):

* **Modo `want` (firma visible obligatoria):**
  Si el usuario cierra o cancela la ventana de posicionamiento sin haber definido el
  área gráfica, se lanza `AOCancelledOperationException("Incluir la firma visible en el documento es obligatoria.")`.
  Esta excepción se traduce en `VisibleSignatureMandatoryException` (`509-510`) y
  finalmente se detiene el proceso con el error fatal `SAF_43`
  (`ERROR_VISIBLE_SIGNATURE`, `181-183`).
* **Modo `optional` (firma visible opcional):**
  Si el usuario cancela la ventana de posicionamiento, la operación **no falla**:
  no se inyectan coordenadas y el proceso continúa normalmente generando una firma
  PDF invisible estándar.

> **Defecto de estado en sesiones continuas ([BUG-13](A1-bugs-autofirma.md#bug-13-fuga-de-estado-y-asignación-cruzada-en-showrubriciscanceled-entre-operaciones-de-firma)):**
> El estado de cancelación del diálogo de rúbrica se registra en la variable estática mutable
> `ProtocolInvocationLauncherSign.showRubricIsCanceled` (`108, 1014`), la cual **nunca se
> restablece a `false`**. Además, el listener de la operación hermana `signandsave` modifica por
> error este mismo campo estático de `Sign` (`ProtocolInvocationLauncherSignAndSave.java:1040`).
> Como consecuencia, en sesiones abiertas de Socket local o WebSocket, cancelar una rúbrica deja
> la bandera activada indefinidamente para futuras llamadas dentro del mismo proceso de la JVM.

---

## 6. Ejecución de la firma y casos especiales

La ejecución material de la firma se centraliza en el método `executeSign`
(`ProtocolInvocationLauncherSign.java:694-864`).

### 6.1 Despacho de la operación criptográfica

Según el enumerado `cryptoOperation`, se invoca el método correspondiente del firmador:

```java
switch (cryptoOperation) {
    case SIGN:
        signature = signer.sign(data, algorithm, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        break;
    case COSIGN:
        signature = signer.cosign(data, algorithm, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        break;
    case COUNTERSIGN:
        signature = signer.countersign(data, algorithm,
                "tree".equalsIgnoreCase(extraParams.getProperty(AfirmaExtraParams.TARGET)) ?
                        CounterSignTarget.TREE : CounterSignTarget.LEAFS,
                null, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        break;
    default:
        throw new SocketOperationException(ERROR_UNSUPPORTED_OPERATION);
}
```

La rama `COSIGN` invoca la sobrecarga de **una sola ranura de datos**
`cosign(sign, algorithm, key, certChain, extraParams)`
(`ProtocolInvocationLauncherSign.java:710-717`), y no la sobrecarga de dos ranuras
`cosign(data, sign, ...)` que sí declara la interfaz
(`afirma-core/src/main/java/es/gob/afirma/core/signers/AOCoSigner.java:36`). El
protocolo `afirma://` transporta un único parámetro `dat`, que en `cosign` contiene
la firma previa, de modo que **no existe forma de aportar los datos originales a una
cofirma**: la limitación es del protocolo, no del cliente que lo invoca.

La consecuencia se observa en CAdES. Al no recibir los datos, `CAdESCoSigner` busca
la huella digital que necesita en dos sitios por este orden
(`afirma-crypto-cades-multi/src/main/java/es/gob/afirma/signers/multi/cades/CAdESCoSigner.java:191-265`):

1. El contenido encapsulado de la firma previa, si esta es **implícita** (*attached*):
   lo extrae y calcula su huella con el algoritmo solicitado (`194-205`).
2. El atributo firmado `messageDigest` de alguno de los firmantes previos, **siempre
   que su algoritmo de huella coincida con el pedido** (`249-264`).

Si la firma previa es **explícita** (*detached*) y el algoritmo solicitado no coincide
con el de ningún firmante anterior, ambas vías fallan y se lanza
`ContainsNoDataException` (`268-270`), que el lanzador traduce a `SAF_44`
(`ProtocolInvocationLauncherSign.java:779-783`). Es decir: una firma CAdES explícita
solo se puede cofirmar reutilizando el algoritmo de huella de la firma original. El
cliente JavaScript de referencia deja constancia expresa de esta limitación en un
comentario de su propio código (`autoscript.js:471-473`), descrita en el
capítulo [16](16-cliente-javascript.md#93-la-cofirma-desde-la-api-pública-el-parámetro-datab64-que-nunca-viaja).

---

### 6.2 Parche heredado: Firmas XAdES explícitas

El código mantiene una compatibilidad transitoria con firmas XAdES en modo explícito
(`isXadesExplicitConfigurated`, `394-406` y `886-894`):
Si `cryptoOperation == Operation.SIGN`, el formato comienza por `"xades"` (sin ser
trifásico) y en `properties` se especificó `mode=explicit` sin activar `useManifest`:

1. AutoFirma calcula el resumen SHA-1 de los datos: `data = MessageDigest.getInstance("SHA1").digest(data)`.
2. Fuerza la propiedad `mimeType="hash/sha1"`.
3. Realiza una firma XAdES estándar sobre el hash resultante en lugar de los datos íntegros.

---

## 7. Formato de la respuesta y procesadores de datos

Una vez generada la firma, el resultado se procesa a través de la infraestructura de
procesadores de datos (`SignDataProcessor`).

### 7.1 Arquitectura de procesadores y plugins

En `ProtocolInvocationLauncherSign.selectProcessor` (`212-247`), AutoFirma comprueba
si hay algún plugin cargado con el permiso `Permission.INLINE_PROCESS` cuyo disparador
concuerde con la operación. Si no hay plugins aplicables, se utiliza el procesador
nativo estándar: `NativeSignDataProcessor`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/NativeSignDataProcessor.java`).

### 7.2 Estructura del resultado devuelto

`NativeSignDataProcessor.postProcess` (`54-104`) ensambla los componentes del
resultado separándolos mediante el carácter barra vertical (`|`, `RESULT_SEPARATOR`, `23`):

```
┌─────────────────────────┬───┬────────────────────────┬───┬─────────────────────────┐
│     Certificado X.509   │ | │    Firma Electrónica   │ | │  Metadatos extra JSON   │
│    (Base64 URL-safe)    │   │   (Base64 URL-safe)    │   │  (opcional, proto v>=3) │
└─────────────────────────┴───┴────────────────────────┴───┴─────────────────────────┘
```

1. **Certificado del firmante:**
   Se obtiene el certificado hoja: `certEncoded = pke.getCertificateChain()[0].getEncoded()`.
   Si se produce un error en la codificación binaria, se eleva `SAF_18`
   (`ERROR_DECODING_CERTIFICATE`).
2. **Firma electrónica:**
   El array de bytes resultante de `executeSign`.
3. **Metadatos adicionales (`extraData`):**
   Si el usuario seleccionó un fichero del disco (`inputFilename != null`), se
   construye un JSON con la propiedad `filename` mediante `buildExtraDataResult` (`112-126`):
   ```json
   {"filename": "documento.pdf"}
   ```
   **Condición de versión:** Estos metadatos adicionales **solo se añaden si la versión del protocolo solicitada es mayor o igual a 3** (`getProtocolVersion() >= 3`, `77, 97`). En versiones de protocolo 0, 1 y 2, el resultado se trunca al par certificado-firma.

### 7.3 Codificación y cifrado

* **Sin clave de cifrado (`key` ausente):**
  Cada parte se codifica individualmente en Base64 URL-safe (`Base64.encode(bytes, true)`,
  usando `-` y `_`, `89-100`).
  ```
  <Certificado_B64_URLSafe>|<Firma_B64_URLSafe>[|<JSON_B64_URLSafe>]
  ```
* **Con clave de cifrado (`key` presente en la URI):**
  Si se indicó una clave simétrica de 8 caracteres (`NativeDataCipher`, `68-84`), cada
  componente se cifra mediante DES/ECB con relleno manual antes de concatenarse
  (ver capítulo 03). Cada bloque toma la forma `PADDING.BASE64_URL_SAFE`:
  ```
  <Cipher_Cert>|<Cipher_Sign>[|<Cipher_JSON>]
  ```
  Si ocurre un fallo durante el cifrado, se lanza `SAF_12` (`ERROR_ENCRIPTING_DATA`).

### 7.4 Entrega del resultado según el transporte

* **En transporte por socket local o WebSocket:**
  El método `ProtocolInvocationLauncherSign.processSign` devuelve el `StringBuilder`
  directamente a `ProtocolInvocationLauncher.launch` (`724`), que a su vez lo entrega al
  hilo del socket (`CommandProcessorThread`). Este calcula el número de fragmentos y
  comienza a enviarlo al navegador (ver capítulo 04).
* **En transporte por servidor intermedio:**
  `ProtocolInvocationLauncher.launch` invoca a
  `sendDataToServer(dataToSend.toString(), params.getStorageServletUrl().toString(), params.getId())`
  (`721`), subiendo el resultado mediante una petición HTTP POST al `StorageService`
  asociado al `id` de la transacción.

---

## 8. Catálogo completo de errores de la operación de firma

Cuando se produce un error controlado o una cancelación, se lanza una
`SocketOperationException`. En el transporte por socket, el código se envía
directamente en la respuesta HTTP; en servidor intermedio, el mensaje se codifica en
URL (`URLEncoder.encode`) y se envía al servlet `stservlet` antes de retornar
(`ProtocolInvocationLauncher.java:697-715`).

| Código | Constante en ErrorManager | Causa en el flujo de firma | Mensaje o texto asociado |
|---|---|---|---|
| `CANCEL` | `RESULT_CANCEL` | El usuario cancela voluntariamente el diálogo de certificados, el diálogo de selección de fichero o un aviso de seguridad. | `CANCEL` |
| `SAF_00` | `ERROR_CANNOT_READ_DATA` / `ERROR_SIGNATURE_FAILED` | Fallo de lectura del fichero seleccionado en disco, o excepción genérica inesperada en el firmador (`AOException`). | «No se han podido leer los datos a firmar» / «Error realizando la firma electrónica» |
| `SAF_01` | `ERROR_NULL_URI` | Las opciones o la URI de entrada son nulas. | «La URL recibida es nula» |
| `SAF_02` | `ERROR_UNSUPPORTED_PROTOCOL` | El esquema no comienza por `afirma://` estrictamente en minúsculas. | «Protocolo no soportado» |
| `SAF_03` | `ERROR_PARAMS` | Parámetros obligatorios ausentes (formato, algoritmo no soportado, etc.). | «Error en los parámetros de entrada» |
| `SAF_04` | `ERROR_UNSUPPORTED_OPERATION` | Operación desconocida o no soportada. | «Operación no soportada. Compruebe que dispone de la última versión de Autofirma.» |
| `SAF_06` | `ERROR_UNSUPPORTED_FORMAT` | El formato de firma indicado no tiene firmador asociado en `AOSignerFactory`. | «Formato de firma no soportado» |
| `SAF_08` | `ERROR_CANNOT_ACCESS_KEYSTORE` | Error al acceder o instanciar el almacén de certificados (`AOKeyStoreManager`). | «Error accediendo al almacén de claves y certificados» |
| `SAF_11` | `ERROR_SENDING_RESULT` | Error al subir los datos al servidor intermedio. | «Error en el envio del resultado de la operación.» |
| `SAF_12` | `ERROR_ENCRIPTING_DATA` | Fallo criptográfico al cifrar el resultado con la clave `key`. | «Error en el cifrado de los datos a enviar» |
| `SAF_13` | `ERROR_LOCAL_ACCESS_BLOCKED` | Se intentó acceder a `localhost` o `127.0.0.1` en servlets remotos. | «Se ha pedido acceso a una dirección local, pero por seguridad se ha bloqueado el acceso» |
| `SAF_14` | `ERROR_OBSOLETE_APP` | Se requiere una versión más moderna de AutoFirma para los parámetros. | «La aplicación está obsoleta y no puede procesarse la petición.» |
| `SAF_15` | `ERROR_DECRYPTING_DATA` | Error al descifrar los datos descargados de `rtservlet`. | «Error en el descifrado de los datos» |
| `SAF_16` | `ERROR_RECOVERING_DATA` | Error de conexión al recuperar datos de `rtservlet`. | «Error al recuperar los datos del servidor intermedio» |
| `SAF_17` | `ERROR_UNKNOWN_SIGNER` | No se pudo identificar el formato de los datos cuando `format="auto"`. | «Los datos proporcionados no son una firma electrónica reconocida» |
| `SAF_18` | `ERROR_DECODING_CERTIFICATE` | Error al extraer la codificación binaria X.509 del certificado del firmante. | «Error al descodificar el certificado de firma» |
| `SAF_19` | `ERROR_NO_CERTIFICATES_KEYSTORE` | Ningún certificado del almacén supera los filtros aplicados. | «No hay ningun certificado válido en su almacén.» |
| `SAF_21` | `ERROR_UNSUPPORTED_PROCEDURE` | Versión del protocolo superior a la máxima soportada por la aplicación. | «La versión de Autofirma instalada no es compatible con este trámite.» |
| `SAF_23` | `ERROR_INVALID_POLICY` | La política de firma configurada es incompatible con los datos o formato. | «Se ha establecido una política de firma no válida o parámetros no compatibles con ella.» |
| `SAF_28` | `ERROR_INVALID_PDF` | El fichero recibido no es un PDF válido o está corrupto. | «El fichero no es un PDF o es un PDF no soportado.» |
| `SAF_29` | `ERROR_INVALID_XML` | Firma XAdES Enveloped solicitada sobre datos que no son XML válido. | «Las firmas XAdES Enveloped solo pueden realizarse sobre datos XML.» |
| `SAF_30` | `ERROR_INVALID_DATA` | El formato de los datos no encaja con el formato de firma seleccionado. | «El formato de los datos a firmar no es adecuado para el tipo de firma seleccionado.» |
| `SAF_31` | `ERROR_NO_SIGN_DATA` | Los datos introducidos no corresponden a un objeto de firma en multifirma. | «Los datos introducidos no se corresponden con un objeto de firma.» |
| `SAF_32` | `ERROR_FACE_ALREADY_SIGNED` | La FacturaE ya contiene una firma y no admite firmas adicionales. | «La factura ya tiene una firma electrónica y no admite firmas adicionales.» |
| `SAF_38` | `ERROR_INVALID_FACTURAE` | Los datos que se intentan firmar no constituyen una FacturaE válida. | «El archivo que intenta firmar no es una factura electrónica reconocida.» |
| `SAF_39` | `ERROR_INVALID_SIGNATURE` | La firma previa en `cosign` o `countersign` no es válida o está corrompida. | «La firma de entrada no es válida.» |
| `SAF_40` | `ERROR_RECOVER_SERVER_DOCUMENT` | Error al recuperar el documento original desde el servidor trifásico. | «Error al recuperar el documento» |
| `SAF_41` | `ERROR_MINIMUM_VERSION_NON_SATISTIED` | La versión de AutoFirma es inferior a la requerida por el parámetro `mcv`. | «El uso de este trámite web requiere una versión más reciente de Autofirma.» |
| `SAF_42` | `ERROR_POSTPROCESSING_DATA` | Error inesperado durante el postprocesado de los resultados (plugin). | «Error al postprocesar una firma, probablemente debido a un plugin.» |
| `SAF_43` | `ERROR_VISIBLE_SIGNATURE` | Cancelación o fallo al estampar una firma visible PDF requerida (`want`). | «Error durante la firma visible del PDF.» |
| `SAF_44` | `ERROR_SIGN_WITHOUT_DATA` | La firma no contiene datos y no es compatible con la configuración. | «La firma no contiene los datos y no se compatible con la configuración seleccionada» |
| `SAF_50` | `ERROR_CONFIRMATION_NEEDED` | Se requiere interacción del usuario (aviso de seguridad/contraseña) pero se ejecuta en modo `headless`. | «La operación puede generar firmas no validas, por lo que no se puede continuar sin confirmacion de usuario.» |
| `SAF_51` | `ERROR_INCOMPATIBLE_KEY_TYPE` | El tipo de clave del certificado no es compatible con el algoritmo solicitado. | «El tipo de clave del certificado no está soportado.» |
| `SAF_52` | `ERROR_LOCKED_KEYSTORE` | El almacén o tarjeta criptográfica está bloqueado por agotar intentos de PIN. | «El almacén de claves esta bloqueado. Siga las instrucciones del proveedor.» |

