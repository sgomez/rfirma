# 08. Operación `batch`

Este capítulo describe la operación `batch` del protocolo `afirma://` de
AutoFirma 1.9.2. Esta operación permite procesar múltiples solicitudes de firma
electrónica (o multifirma: cofirma o contrafirma) en una única transacción,
requiriendo del usuario una sola selección de certificado y, si aplica, una sola
introducción de PIN en el almacén o tarjeta criptográfica.

A diferencia del resto de operaciones unitarias de AutoFirma, la operación `batch`
presenta una dualidad arquitectónica esencial: soporta tanto la **firma por lotes
trifásica remota** (donde el cliente interactúa con servicios de prefirma y
postfirma remotos para que los documentos se firmen y custodien en el servidor),
como la **firma por lotes monofásica local** (`localBatchProcess=true`, introducida
para el formato JSON, donde AutoFirma firma cada documento íntegramente en la
estación de trabajo y devuelve el binario firmado de cada uno en Base64).

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. Visión general y ciclo de vida de `batch`

El procesado de lotes fue diseñado originalmente para permitir que aplicaciones
de gestión tramitasen expedientes con decenas o centenares de documentos en un
único paso sin requerir la intervención reiterada del usuario.

En AutoFirma 1.9.2 existen dos modalidades radicalmente distintas de ejecución:

1. **Lotes trifásicos remotos (XML o JSON):**
   AutoFirma actúa exclusivamente como componente de firma cliente dentro de un
   proceso trifásico. La aplicación web o el gestor documental define un lote cuyos
   documentos residen en el servidor (o en URLs externas). AutoFirma descarga la
   definición del lote, contacta con un servicio remoto de prefirma
   (`batchpresignerurl`) enviando el lote y la cadena de certificados, recibe un
   conjunto de operaciones trifásicas con los hashes a firmar (`TriphaseData`),
   ejecuta las operaciones criptográficas cliente con la clave privada del usuario
   (firmas PKCS#1), y remite los resultados a un servicio remoto de postfirma
   (`batchpostsignerurl`). El servidor remoto compone las firmas completas
   (CAdES, XAdES, PAdES) y las guarda mediante un mecanismo de almacenamiento de
   servidor (`SignSaver` en XML o `DocumentManager` en JSON). AutoFirma **nunca**
   recibe ni devuelve los documentos firmados resultantes; únicamente recibe y
   entrega al llamante un informe de resultados del lote (`signs`). El protocolo
   `afirma://` no contempla ninguna vía ni parámetro para retornar los binarios
   firmados de un lote remoto al cliente web, requiriendo necesariamente que el
   servidor disponga de componentes de custodia o persistencia para almacenarlos.

2. **Lotes monofásicos locales (`localBatchProcess=true`, sólo JSON):**
   Diseñado para escenarios donde el servidor no dispone de los servicios de prefirma
   ni postfirma o donde las firmas completas deben volver al navegador del usuario.
   El lote se suministra en formato JSON conteniendo los datos completos a firmar
   codificados en Base64 en el campo `datareference` de cada firma. AutoFirma procesa
   cada firma secuencialmente en local utilizando sus firmadores nativos, captura
   los errores individuales según la política configurada, y compone un JSON de
   resultado que incorpora en Base64 el documento firmado completo de cada elemento.

```
                           ┌────────────────────────────────────────┐
                           │ Invocación: afirma://batch?dat=...     │
                           └───────────────────┬────────────────────┘
                                               │
                                               ▼
                           ┌────────────────────────────────────────┐
                           │ ¿Parámetro fileid presente?            │
                           │  - Sí: descarga sobre de rtservlet     │
                           │  - No: usa parámetros de la URI        │
                           └───────────────────┬────────────────────┘
                                               │
                                               ▼
                           ┌────────────────────────────────────────┐
                           │ Selección de certificado (única vez):  │
                           │ AOKeyStoreDialog + resolución sticky   │
                           └───────────────────┬────────────────────┘
                                               │
                     ┌─────────────────────────┴─────────────────────────┐
                     │                                                   │
        [localBatchProcess=false]                             [localBatchProcess=true]
                     │                                                   │
                     ▼                                                   ▼
       ┌───────────────────────────┐                       ┌───────────────────────────┐
       │ 1. POST batchpresignerurl │                       │ Ejecución local de firmas │
       │    (envío lote + certs)   │                       │ (AOSigner / LocalSigner): │
       │                           │                       │                           │
       │ 2. TriphaseDataSigner     │                       │ Forzado headless=true     │
       │    (firmas PKCS#1 cliente)│                       │ Secuencial por documento  │
       │                           │                       │ Soporte stoponerror       │
       │ 3. POST batchpostsignerurl│                       │                           │
       │    (envío td2)            │                       │ Resultado contiene:       │
       │                           │                       │  - id, result             │
       │ Servidor persiste firmas  │                       │  - signature (Base64)     │
       │ Retorna XML/JSON log      │                       └─────────────┬─────────────┘
       └─────────────┬─────────────┘                                     │
                     │                                                   │
                     └─────────────────────────┬─────────────────────────┘
                                               │
                                               ▼
                           ┌────────────────────────────────────────┐
                           │ Conformación de respuesta:             │
                           │ ¿needcert=true? -> log | certEncoded   │
                           │ Cifrado simétrico (key) o Base64       │
                           └───────────────────┬────────────────────┘
                                               │
                                               ▼
                           ┌────────────────────────────────────────┐
                           │ Devolución al llamante:                │
                           │  - Servidor intermedio (stservlet)     │
                           │  - Socket / WebSocket                 │
                           └────────────────────────────────────────┘
```

### 1.1 Prefijos de URI reconocidos

El despachador `ProtocolInvocationLauncher.launch` captura la invocación de lotes en la
línea 293 de
`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncher.java`:

```java
else if (urlString.startsWith("afirma://batch?") || urlString.startsWith("afirma://batch/?")) {
```

Se admiten idénticamente las variantes con interrogación directa (`afirma://batch?`) y
con barra inclinada previa (`afirma://batch/?`).

### 1.2 Comparativa de modalidades de lote

| Característica | Lote trifásico remoto XML | Lote trifásico remoto JSON | Lote monofásico local JSON |
|---|---|---|---|
| Parámetro identificador | `jsonbatch=false` (o ausente) | `jsonbatch=true` | `jsonbatch=true` y `localBatchProcess=true` |
| Formato del lote | XML (`<signbatch>`) | JSON (`{"singlesigns": [...]}`) | JSON (`{"singlesigns": [...]}`) |
| URLs remotas requeridas | `batchpresignerurl` y `batchpostsignerurl` | `batchpresignerurl` y `batchpostsignerurl` | Ninguna |
| Dónde se generan las firmas finales | En el servidor de postfirma | En el servidor de postfirma | En la estación de trabajo (AutoFirma) |
| Custodia / guardado de firmas | Servidor (`SignSaver`) | Servidor (`DocumentManager`) | Devueltas al llamante en la respuesta |
| Campo `signature` en la respuesta | No existe | No existe | Sí, en Base64 por cada documento exitoso |
| Métodos involucrados | `BatchSigner.signXML` | `BatchSigner.signJSON` | `LocalBatchSigner.signLocalBatch` |
| Citas en código | `BatchSigner.java:212-293` | `BatchSigner.java:340-443` | `LocalBatchSigner.java:48-84` |

---

## 2. Parámetros de la invocación `batch`

El análisis, deserialización y validación de los parámetros de invocación se realiza en la
clase `UrlParametersForBatch`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersForBatch.java`),
que extiende la clase abstracta `UrlParameters`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java`).

La instanciación y carga de parámetros se coordina a través de
`ProtocolInvocationUriParserUtil.getParametersToBatch`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParserUtil.java:64-70`):

```java
public static UrlParametersForBatch getParametersToBatch(final Map<String, String> params,
        final boolean servicesRequired) throws ParameterException {
    final UrlParametersForBatch ret = new UrlParametersForBatch(servicesRequired);
    ret.setCommonParameters(params);
    ret.setBatchParameters(params);
    return ret;
}
```

### 2.1 Catálogo completo de parámetros

| Parámetro | Tipo / Formato | Obligatorio | Por defecto | Descripción | Cita en código |
|---|---|---|---|---|---|
| `dat` | String (Base64) | Condicional (si no hay `fileid`) | `null` | Definición del lote en XML o JSON codificada en Base64 (admite variante URL-safe con `-` y `_`). | `UrlParameters.java:34`, `UrlParametersForBatch.java:227` |
| `fileid` | String alfanumérico | Condicional (si no hay `dat`) | `null` | Identificador de fichero remoto en el servidor intermedio para recuperar el sobre de parámetros. Longitud máx. 20 caracteres. | `UrlParameters.java:59`, `UrlParametersForBatch.java:196-210` |
| `batchpresignerurl` | URL (HTTP/HTTPS) | Sí, si `!localBatchProcess` | `null` | URL del servlet de prefirma trifásica remota. Prohibido acceso a red local salvo configuración explícita. | `UrlParametersForBatch.java:243-259` |
| `batchpostsignerurl` | URL (HTTP/HTTPS) | Sí, si `!localBatchProcess` | `null` | URL del servlet de postfirma trifásica remota. Prohibido acceso a red local salvo configuración explícita. | `UrlParametersForBatch.java:238-254` |
| `jsonbatch` | Booleano (`true`/`false`) | No | `false` | Indica que la definición del lote proporcionada en `dat` está en formato JSON. Estrictamente sensible a minúsculas (`jsonbatch`). En caso contrario, se asume XML. | `UrlParametersForBatch.java:44, 330-332` |
| `localBatchProcess` | Booleano (`true`/`false`) | No | `false` | Indica si el proceso de lote se debe ejecutar de forma monofásica local sin contactar con servicios de pre/postfirma. Exclusivamente soportado en lotes JSON (`jsonbatch=true`). | `UrlParametersForBatch.java:47, 232-234` |
| `needcert` | Booleano (`true`/`false`) | No | `false` | Si se establece a `true`, la respuesta incluirá el certificado utilizado para firmar separado por `\|`. | `UrlParametersForBatch.java:41, 322-327` |
| `id` | String alfanumérico | Sí en servidor intermedio | `null` | Identificador de sesión para el intercambio con el servidor intermedio (`stservlet`). Longitud máx. 20 caracteres. | `UrlParametersForBatch.java:25, 196-211` |
| `key` | String (8 caracteres) | Opcional | `null` | Clave simétrica de 8 caracteres (DES) utilizada para descifrar la entrada y cifrar la respuesta. | `UrlParameters.java:53-56`, `ProtocolInvocationLauncherUtil.java` |
| `stservlet` | URL (HTTP/HTTPS) | Sí si `servicesRequired` e `id` presente | `null` | URL del servlet `StorageService` al que AutoFirma enviará el resultado del lote. | `UrlParameters.java:46`, `UrlParametersForBatch.java:263-283` |
| `rtservlet` | URL (HTTP/HTTPS) | Sí si se indica `fileid` | `null` | URL del servlet `RetrieveService` para descargar los parámetros asociados a `fileid`. | `UrlParameters.java:43`, `ProtocolInvocationLauncherUtil.java:47` |
| `properties` | String (Base64) | No | Vacío | Propiedades de configuración en formato `java.util.Properties` codificadas en Base64 (contiene filtros de certificados, opciones SSL, etc.). | `UrlParametersForBatch.java:31, 285-303` |
| `keystore` | String | No | `null` | Nombre del almacén de claves solicitado (ej. `Windows`, `Apple`, `PKCS12`). | `UrlParameters.java:63`, `UrlParametersForBatch.java:334` |
| `ksb64` | String (Base64) | No | `null` | Nombre del almacén codificado en Base64 (prioritario sobre `keystore`). | `UrlParameters.java:66`, `UrlParameters.java:220-224` |
| `sticky` | Booleano (`true`/`false`) | No | `false` | Si es `true`, almacena la clave privada seleccionada en memoria estática de la JVM para reutilizarla en subsiguientes firmas. | `UrlParametersForBatch.java:34, 306-311` |
| `resetsticky` | Booleano (`true`/`false`) | No | `false` | Si es `true`, invalida cualquier clave privada previamente fijada en memoria estática forzando nueva selección. | `UrlParametersForBatch.java:38, 314-319` |
| `ver` | String | No | `"0"` | Versión de protocolo de la operación (`ProtocolVersion`); solo se lee sin canal abierto. Ver [14 §2.3](14-versiones.md). | `UrlParametersForBatch.java:31, 214-219` |
| `mcv` | String | No | `null` | Versión mínima de la aplicación AutoFirma requerida (ej. `"1.9.2"`). Si la versión local es inferior, aborta con `SAF_41`. | `UrlParameters.java:73`, `ProtocolInvocationLauncherBatch.java:90-101` |
| `aw` | Booleano (`true`/`false`) | No | `false` | Solicita espera activa (*active waiting*) sobre `stservlet` mientras se procesa la firma. | `UrlParameters.java:70, 88`, `ProtocolInvocationLauncher.java:337-339` |
| `appname` | String | No | `null` | Nombre de la aplicación o dominio web que origina la invocación. | `UrlParametersForBatch.java:80, 221-223` |

### 2.2 Validación de identificadores y URLs

1. **Identificadores de sesión (`id` y `fileid`):**
   Ambos parámetros se validan en `UrlParametersForBatch.java:199-208`. La longitud
   no puede exceder de 20 caracteres (`MAX_ID_LENGTH`). Todos los caracteres deben
   ser estrictamente alfanuméricos en el rango `[a-z0-9]` (evaluados tras pasar a
   minúsculas con `Locale.ENGLISH`). Si contienen cualquier otro carácter (como guiones,
   puntos o símbolos), se lanza `ParameterException("El identificador de la firma debe ser alfanumerico.")`.

2. **Validación de URLs remotas:**
   Las URLs `batchpresignerurl`, `batchpostsignerurl` y `stservlet` se someten a
   `UrlParameters.validateURL(String)` (`UrlParameters.java:279-317`). Este método
   rechaza URLs mal formadas y bloquea direcciones IP o nombres de host pertenecientes
   a redes locales privadas o de bucle invertido (`localhost`, `127.0.0.1`, `10.*.*.*`,
   `192.168.*.*`, `172.16-31.*.*`), a menos que la propiedad de sistema
   `es.gob.afirma.allowLocalAccess` esté habilitada. El acceso local no permitido lanza
   `ParameterLocalAccessRequestedException`.

### 2.3 Sensibilidad a mayúsculas en `jsonbatch` e incompatibilidad con `localBatchProcess`

1. **Sensibilidad estricta a minúsculas en `jsonbatch`:**
   El analizador de la URI `ProtocolInvocationUriParser.parserUri` (`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParser.java:273-286`) descompone los parámetros y los almacena en un `HashMap<String, String>` sin transformar ni normalizar el tamaño de las letras de las claves. La clase `UrlParametersForBatch.java:44, 330-332` comprueba estrictamente la presencia de `PARAM_JSON_BATCH = "jsonbatch"`.
   El cliente web de referencia `autoscript.js` genera siempre `jsonbatch` íntegramente en minúsculas (`autoscript.js:2023, 2849, 4080`).
   Si un llamante construye una petición utilizando grafías en camelCase (como `jsonBatch=true`, tal como aparecía en el test unitario deshabilitado `afirma-simple/src/test/java/es/gob/afirma/test/simple/TestProtocolInvocationJSON.java:40`), la condición `params.containsKey("jsonbatch")` evalúa falso. AutoFirma degrada entonces al valor por defecto `jsonbatch=false` y canaliza la petición hacia el motor XML (`BatchSigner.signXML`). Al intentar extraer el algoritmo invocando `getAlgorithmForXML` sobre los datos JSON en Base64, el parser SAX/DOM de XML falla arrojando `IOException`, lo que se traduce hacia el llamante en el error `SAF_27` (`ERROR_BATCH_SIGNATURE`).

2. **Incompatibilidad de `localBatchProcess=true` con lotes XML:**
   La modalidad de procesado monofásico local (`localBatchProcess=true`) está implementada de manera exclusiva para lotes en formato JSON gestionados por `JSONBatchManager` y `LocalBatchSigner`.
   En `UrlParametersForBatch.java:236-260`, la presencia de `localBatchProcess=true` hace que el parser omita la obligatoriedad de `batchpresignerurl` y `batchpostsignerurl`. Sin embargo, el parser no comprueba que `jsonbatch` sea `true`.
   Si se invoca un lote XML con `localBatchProcess=true` y URLs ausentes, AutoFirma no aborta en la fase de análisis de la URI; en su lugar, despliega la interfaz gráfica modal `AOKeyStoreDialog` y solicita la selección de certificado y PIN a la persona usuaria. Únicamente al llegar a `signBatch` (`ProtocolInvocationLauncherBatch.java:400-422`), el flujo bifurca hacia `BatchSigner.signXML` con URLs nulas, arrojando `IllegalArgumentException` y reportando tardíamente `SAF_03` ([BUG-16](A1-bugs-autofirma.md#bug-16-incompatibilidad-de-localbatchprocess-con-lotes-xml-provoca-fallo-tardío-con-saf_03-tras-seleccionar-certificado-y-pin)).

---

## 3. Recuperación de parámetros por servidor intermedio (`fileid`)

Cuando la definición del lote excede el tamaño máximo admisible en una URI de sistema
operativo (o cuando el integrador web decide no transferir el lote directamente por URL),
la petición incluye el parámetro `fileid` en lugar de `dat`.

El flujo de recuperación en `ProtocolInvocationLauncher.java:307-333` opera como sigue:

1. **Descarga de datos:**
   Se invoca `ProtocolInvocationLauncherUtil.getDataFromRetrieveServlet(params)`. Este
   método realiza una petición HTTP GET a la URL indicada en `rtservlet`, pasando como
   parámetro `id=fileid`.
2. **Descifrado de datos:**
   Si la URI original contenía el parámetro `key`, el contenido recibido se descifra
   mediante algoritmo simétrico DES (en modo CBC con relleno PKCS#5) utilizando la
   clave de 8 bytes decodificada. Si falla el tamaño del bloque o el descifrado,
   se elevan `InvalidEncryptedDataLengthException` o `DecryptionException`, resultando
   en los códigos de error `SAF_16` o `SAF_15` respectivamente.
3. **Estructura del sobre descargado:**
   A pesar de que el comentario interno en el código (`ProtocolInvocationLauncher.java:304`) indica que se descargará «el JSON o XML de definicion de lote», el contenido recuperado del servidor intermedio **nunca es el documento de lote directamente**, sino un sobre estructurado con la colección completa de parámetros de la operación (`paramsMap`):
   - **Determinación del formato del sobre:** La elección del parser depende de `params.isJsonBatch()` evaluado sobre los parámetros presentes en la URI inicial de arranque (`ProtocolInvocationLauncher.java:323-328`). Si la URI de invocación no incluyó `jsonbatch=true` (lo habitual cuando el navegador genera la URL abreviada con `buildUrlWithoutData` en `autoscript.js:4413-4424`, que solo transfiere `fileid`, `rtservlet` y `key`), `params.isJsonBatch()` evalúa `false`. En tal caso, el sobre recuperado debe ser obligatoriamente XML, procesándose mediante `ProtocolInvocationUriParserUtil.parseXml(batchDefinition)` (`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParserUtil.java:99-136`), que parsea elementos `<batch><e k="..." v="..."/></batch>` (generados por `autoscript.js:4392-4401` vía `buildXML`).
   - Si la URI inicial incluyó expresamente `jsonbatch=true`, se procesa mediante `TriphaseDataParser.parseParamsListJson(batchDefinition)` (`afirma-crypto-batch-client/src/main/java/es/gob/afirma/signers/batch/client/TriphaseDataParser.java:156-177`), esperando la estructura `{"params": [{"k": "...", "v": "..."}, ...]}`.
4. **Instanciación final de parámetros:**
   Los parámetros recuperados del sobre reemplazan o completan a los de la URI inicial volviendo a llamar a `ProtocolInvocationUriParserUtil.getParametersToBatch(paramsMap, !bySocket)`. La definición concreta del lote (el documento XML o JSON de las firmas individuales) reside obligatoriamente dentro de la clave `dat` del sobre descargado (codificada en Base64), junto con las URLs `batchpresignerurl`, `batchpostsignerurl`, `stservlet` y demás opciones de la transacción.
5. **Espera activa (*Active Waiting*):**
   Si `params.isActiveWaiting()` es `true` y la invocación no es por socket local
   (`!bySocket`), se inicia un hilo en segundo plano mediante `requestWait(storageServletUrl, id)`
   (`ProtocolInvocationLauncher.java:338, 974-1002`). Este hilo realiza peticiones
   periódicas HTTP POST al servlet `StorageService` con el comando de espera para evitar
   que el servidor intermedio descarte la sesión por inactividad mientras el usuario
   interactúa con los diálogos de AutoFirma.

---

## 4. Selección de almacén, certificado y gestión de PIN

Una vez validados los parámetros, el control pasa a
`ProtocolInvocationLauncherBatch.processBatch` (`ProtocolInvocationLauncherBatch.java:72-226`).

### 4.1 Resolución del almacén de certificados

El almacén criptográfico (`AOKeyStore`) a interrogar se determina siguiendo una
jerarquía de prioridades estricta (`ProtocolInvocationLauncherBatch.java:103-128`):

1. **Almacén previo persistido (*sticky*):**
   Si `KeyStorePreferencesManager.getLastSelectedKeystore()` contiene un valor válido,
   se recupera dicho almacén vía `SimpleKeyStoreManager.getLastSelectedKeystore()`.
2. **Preferencia de almacén por defecto de la aplicación:**
   Si la preferencia general `PreferencesManager.PREFERENCE_USE_DEFAULT_STORE_IN_BROWSER_CALLS`
   está activa, se utiliza el almacén configurado por el usuario en las preferencias de
   AutoFirma (`PreferencesManager.PREFERENCE_KEYSTORE_DEFAULT_STORE`).
3. **Almacén solicitado en la URI:**
   Si se proporcionó el parámetro `ksb64` o `keystore`, se instancia mediante
   `SimpleKeyStoreManager.getKeyStore(options.getDefaultKeyStore())`.
4. **Almacén por defecto del sistema operativo:**
   En ausencia de los anteriores, se invoca
   `AOKeyStore.getDefaultKeyStoreTypeByOs(Platform.getOS())` (CAPI/Windows en Microsoft
   Windows, KeyChain en macOS, y NSS/PKCS#11 en GNU/Linux).

### 4.2 Diálogo de selección y certificados pegajosos (*sticky*)

Si los parámetros indican `sticky=true`, `resetsticky=false` y ya existe una entrada de
clave privada fijada en memoria estática (`ProtocolInvocationLauncher.getStickyKeyEntry() != null`),
se reutiliza dicha entrada directamente (`ProtocolInvocationLauncherBatch.java:232-236`)
sin mostrar ninguna interfaz gráfica ni pedir confirmación.

En caso contrario, se inicializa el manejador de filtros de certificado
`CertFilterManager` con los `extraParams` recibidos y se muestra el diálogo nativo modal
`AOKeyStoreDialog` (`273-285`):

```java
final AOKeyStoreDialog dialog = new AOKeyStoreDialog(
    ksm,
    null,
    true,
    true, // showExpiredCertificates
    true, // checkValidity
    filterManager.getFilters(),
    filterManager.isMandatoryCertificate(),
    libName
);
dialog.allowOpenExternalStores(filterManager.isExternalStoresOpeningAllowed());
dialog.show();
```

En macOS, antes de mostrar el diálogo se invoca `MacUtils.focusApplication()` (`266`)
para forzar que la ventana pase al primer plano. Si el usuario cancela la selección,
se lanza `AOCancelledOperationException`, que provoca el retorno del literal `"CANCEL"`
(`135-140`). Si no se encuentran certificados válidos que cumplan los filtros en el
almacén, se lanza `AOCertificatesNotFoundException`, traduciéndose en el error
`SAF_19` (`ERROR_NO_CERTIFICATES_KEYSTORE`).

### 4.3 Manejo inteligente de PIN erróneo (`PinException`)

Si durante la ejecución criptográfica de las firmas del lote el token criptográfico o
la tarjeta inteligente rechaza la clave privada por un error de PIN (`PinException`),
AutoFirma cuenta con un mecanismo de reintento automático (`ProtocolInvocationLauncherBatch.java:321-336`):

```java
catch (final PinException e) {
    List<CertificateFilter> filters;
    try {
        final byte[] certEncoded = pke.getCertificate().getEncoded();
        final CertificateFilter filter = new EncodedCertificateFilter(Base64.encode(certEncoded));
        filters = Collections.singletonList(filter);
    } catch (final Exception ex) {
        filters = null;
    }
    final CertFilterManager newFilterManager = new CertFilterManager(filters, filters != null, true);
    ProtocolInvocationLauncher.setStickyKeyEntry(null);
    return sign(options, aoks, useDefaultStore, newFilterManager);
}
```

El algoritmo captura la excepción, construye un filtro estricto
(`EncodedCertificateFilter`) fijando el certificado que el usuario ya había seleccionado,
descarta la clave pegajosa estática y reinvoca recursivamente el método `sign(...)`. Esto
obliga al almacén a recargar la sesión PKCS#11 y volver a solicitar el PIN al usuario
sin obligarle a buscar y elegir nuevamente su certificado en la lista.

---

## 5. Lotes trifásicos remotos XML (`signXML`)

Cuando `jsonbatch` es `false` (comportamiento por defecto histórico), la ejecución
se delega en `BatchSigner.signXML(...)`
(`afirma-crypto-batch-client/src/main/java/es/gob/afirma/signers/batch/client/BatchSigner.java:212-293`).

### 5.1 Estructura del XML de definición del lote

La definición del lote debe ser un documento XML conforme al esquema formal definido
en `afirma-server-triphase-signer/src/main/java/es/gob/afirma/signers/batch/doc-files/batch-scheme.html`.

```xml
<?xml version="1.0" encoding="UTF-8" ?>
<signbatch stoponerror="true" algorithm="SHA256withRSA" concurrenttimeout="30">
 <singlesign Id="doc-001">
  <datasource>https://sede.gob.es/docs/documento1.pdf</datasource>
  <format>PAdES</format>
  <suboperation>sign</suboperation>
  <extraparams>bW9kZT1pbXBsaWNpdA==</extraparams>
  <signsaver>
   <class>es.gob.afirma.signers.batch.SignSaverFile</class>
   <config>RmlsZU5hbWU9L3RtcC9maXJtYTEucGRm</config>
  </signsaver>
 </singlesign>
 <singlesign Id="doc-002">
  <datasource>SG9sYSBNdW5kbw==</datasource>
  <format>XAdES</format>
  <suboperation>sign</suboperation>
  <extraparams></extraparams>
  <signsaver>
   <class>es.gob.afirma.signers.batch.SignSaverFile</class>
   <config>RmlsZU5hbWU9L3RtcC9maXJtYTIueG1s</config>
  </signsaver>
 </singlesign>
</signbatch>
```

#### Elementos y atributos del XML:

1. **Elemento `<signbatch>`:**
   - `algorithm` (Obligatorio): Algoritmo de firma a utilizar en todo el lote
     (`SHA1withRSA`, `SHA256withRSA`, `SHA384withRSA`, `SHA512withRSA`). El analizador
     sintáctico `SignBatchXmlHandler.java:93-98` exige obligatoriamente su presencia;
     en caso contrario, la carga aborta con excepción.
   - `stoponerror` (Opcional, booleano): `true` o `false`. Si es `true`, el proceso
     se interrumpe al fallar cualquier firma individual del lote. Si no se indica,
     por defecto en el servidor es `true` (`SignBatchConfig.java:23`).
   - `concurrenttimeout` (Opcional, numérico): Tiempo máximo de espera en segundos
     para la ejecución concurrente en servidor (`SignBatchXmlHandler.java:104-111`).
   - `Id` (Opcional, alfanumérico): Identificador global del lote.

2. **Elemento `<singlesign>` (Repetible):**
   - Atributo `Id` (Obligatorio): Identificador único de la firma individual dentro del lote.
     **Sensibilidad estricta a mayúsculas:** El analizador SAX del servidor `SignBatchXmlHandler.java:40, 123-126` busca el atributo mediante `attributes.getValue(ATTR_ID)` donde `ATTR_ID = "Id"` con 'I' mayúscula. En XML los nombres de atributos distinguen entre mayúsculas y minúsculas, por lo que si una aplicación emisora genera `id="doc-001"` en minúsculas, la comprobación devuelve `null` y el servidor lanza fulminantemente `SAXException("No se ha indicar el atributo Id de una de las firmas")`. Esto detona un error HTTP 400 (`SAF_03`) o HTTP 500 (`SAF_27`) desde el servlet de prefirma. (Nótese el contraste con los lotes JSON, donde la clave es `id` en minúsculas, y con el propio log XML devuelto por el servidor, donde el atributo de salida se serializa en minúsculas: `<signresult id="..." .../>`).
   - `<datasource>`: Origen de los datos. Puede ser una URL accesible por el servidor
     (HTTP/HTTPS) o los datos directamente en Base64.
   - `<format>`: Formato de firma criptográfica (`XAdES`, `CAdES`, `PAdES`).
   - `<suboperation>`: Operación a realizar (`sign` para firma, `cosign` para cofirma).
   - `<extraparams>`: Propiedades específicas de formato codificadas en Base64
     (parámetros de política, producción, etc.).
   - `<signsaver>`: Configuración del guardado en servidor de la firma terminada:
     - `<class>`: Nombre completamente cualificado de la clase Java en servidor que
       implementa `SignSaver` (ej. `es.gob.afirma.signers.batch.SignSaverFile`).
     - `<config>`: Configuración propia de la clase `SignSaver` en formato `Properties`
       codificado en Base64.

### 5.2 Protocolo de red trifásico XML

El método `BatchSigner.signXML` ejecuta la secuencia siguiente:

1. **Paso 1: Solicitud de prefirma:**
   Se transforma el lote a Base64 URL-Safe (`replace("+", "-").replace("/", "_")`) y
   se codifica la cadena de certificados del firmante en Base64 separada por punto y
   coma (`;`). Se realiza una llamada HTTP POST a la URL de prefirma:
   ```
   POST {batchPresignerUrl}?xml={batchUrlSafe}&certs={certsChain}
   ```
   Se configura un `SSLErrorProcessor(extraParams)` para gestionar posibles certificados
   SSL de servidor no reconocidos, permitiendo al usuario aceptarlos si se requiere.
2. **Paso 2: Generación de firmas cliente (PKCS#1):**
   El servidor devuelve un XML con la sesión trifásica, que se deserializa en un objeto
   `TriphaseData` (`BatchSigner.java:265`).
   A continuación, AutoFirma ejecuta localmente la firma de los hashes trifásicos
   utilizando el motor `TriphaseDataSigner.doSign` (`268-275`):
   ```java
   final TriphaseData td2 = TriphaseDataSigner.doSign(
       new AOPkcs1Signer(),
       getAlgorithmForXML(batchB64),
       pk,
       certificates,
       td1,
       null // Sin ExtraParams para el PKCS#1 en lotes
   );
   ```
   El algoritmo de firma se extrae leyendo directamente el atributo `algorithm` del
   XML del lote mediante `getAlgorithmForXML(batchB64)` (`BatchSigner.java:463-497`).
3. **Paso 3: Solicitud de postfirma y guardado:**
   AutoFirma serializa el objeto resultante `td2` a texto XML, lo codifica en Base64
   y realiza una segunda llamada HTTP POST:
   ```
   POST {batchPostSignerUrl}?xml={batchUrlSafe}&certs={certsChain}&tridata={td2Base64}
   ```
   El servidor de postfirma (`BatchPostsigner`) ensambla las firmas definitivas,
   invoca a las clases `SignSaver` configuradas en cada documento y devuelve al cliente
   un XML con el registro de resultados.

### 5.3 Formato de respuesta del lote XML

El servidor de postfirma (`SignBatch.java:210-219`) genera el resultado en un XML
acorde al siguiente formato:

```xml
<?xml version="1.0" encoding="UTF-8" ?>
<signs>
 <signresult id="doc-001" result="DONE_AND_SAVED" description=""/>
 <signresult id="doc-002" result="DONE_AND_SAVED" description=""/>
</signs>
```

Los posibles valores del atributo `result` corresponden a las constantes del enum
`ProcessResult.Result` (`afirma-server-triphase-signer/src/main/java/es/gob/afirma/signers/batch/ProcessResult.java:5-15`):
`DONE_AND_SAVED`, `DONE_BUT_NOT_SAVED_YET`, `DONE_BUT_SAVED_SKIPPED`,
`DONE_BUT_ERROR_SAVING`, `ERROR_PRE`, `ERROR_POST`, `SKIPPED`, `SAVE_ROLLBACKED`.

#### Esquema XML oficial frente a Javadoc desactualizado en `BatchSigner`

Existe una discrepancia documental en el repositorio oficial de AutoFirma:
* En el Javadoc del método cliente `BatchSigner.signXML` (`afirma-crypto-batch-client/src/main/java/es/gob/afirma/signers/batch/client/BatchSigner.java:181-207`), figura un esquema XML tentativo compuesto por nodos `<signs><sign id="..."><result>OK|KO|NP</result><reason>...</reason></sign></signs>`.
* Sin embargo, la implementación ejecutable del servidor de postfirma (`afirma-server-triphase-signer/src/main/java/es/gob/afirma/signers/batch/xml/SignBatch.java:204-219`) y el esquema XSD oficial (`afirma-server-triphase-signer/src/main/java/es/gob/afirma/signers/batch/doc-files/resultlog-scheme.html`) generan e imponen invariablemente la estructura `<signs><signresult id="..." result="..." description="..."/></signs>`.
* AutoFirma no analiza ni valida el contenido del XML retornado por el servidor de postfirma: `BatchSigner.java:291` se limita a transformar la respuesta HTTP en texto UTF-8 (`return new String(ret, DEFAULT_CHARSET)`). Por consiguiente, el formato de respuesta del protocolo en entornos reales es el generado por `SignBatch.java`. La especificación reflejada en el Javadoc de `BatchSigner` es un residuo documental que nunca fue implementado por el componente servidor de AutoFirma.

---

## 6. Lotes trifásicos remotos JSON (`signJSON`)

Cuando la invocación incluye `jsonbatch=true` y `localBatchProcess=false`, la
ejecución se canaliza por `BatchSigner.signJSON(...)`
(`afirma-crypto-batch-client/.../BatchSigner.java:340-443`).

### 6.1 Estructura del JSON de definición del lote

La estructura JSON requerida está especificada en
`afirma-crypto-batch-client/src/main/java/es/gob/afirma/signers/batch/client/doc-files/batch-scheme.html`:

```json
{
  "algorithm": "SHA256withRSA",
  "format": "PAdES",
  "suboperation": "sign",
  "concurrenttimeout": 30,
  "stoponerror": false,
  "extraparams": "bW9kZT1pbXBsaWNpdA==",
  "singlesigns": [
    {
      "id": "7725374e-728d-4a33-9db9-3a4efea4cead",
      "datareference": "QzovcHJ1ZWJhcy90ZXN0X2ZpY2hlcm8xLnBkZg==",
      "format": "XAdES",
      "suboperation": "sign",
      "extraparams": "Iw0KI1RodSBBdWcgMTM..."
    },
    {
      "id": "93d1531c-cd32-4c8e-8cc8-1f1cfe66f64a",
      "datareference": "https://sede.gob.es/doc2.pdf",
      "suboperation": "sign",
      "extraparams": "Iw0KI1RodSBBdWcgMTM..."
    }
  ]
}
```

Campos principales:
- `algorithm` (String, obligatorio a nivel raíz): Algoritmo de firma general para el lote.
- `format` (String): Formato por defecto para los documentos que no lo sobreescriban.
- `suboperation` (String): Suboperación por defecto (`sign`, `cosign`).
- `stoponerror` (Boolean): Si debe interrumpirse el lote ante el primer fallo.
- `singlesigns` (Array): Lista de objetos con la definición individual de cada documento.
  Cada entrada requiere `id` y `datareference` (URL o Base64), admitiendo sobreescrituras
  de `format`, `suboperation` y `extraparams`.

### 6.2 Protocolo de red trifásico JSON

1. **Llamada de prefirma:**
   POST contra `batchPreSignerUrl` pasando `json` (Base64 URL-safe) y `certs`:
   ```
   POST {batchPreSignerUrl}?json={batchUrlSafe}&certs={certsChain}
   ```
2. **Procesamiento de prefirma y control de errores parciales:**
   La respuesta se analiza mediante `JSONPreSignBatchParser.parseFromJSON(ret)`
   (`BatchSigner.java:392`). Dicho objeto devuelve tanto los datos trifásicos
   (`presignBatch.getTriphaseData()`) como una lista de posibles errores producidos
   en servidor (`presignBatch.getErrors()`).
   - Si no hay datos trifásicos ni errores: retorna un resultado vacío (`398-400`).
   - Si no hay datos trifásicos pero sí errores (`td == null`): significa que todas
     las firmas fallaron en la fase de prefirma. En tal caso, AutoFirma **no continúa**
     a las siguientes fases y devuelve directamente el log de errores (`402-406`).
   - Si hubo errores parciales: se actualiza el objeto `BatchInfo` con los fallos y
     se reempaqueta `batchUrlSafe` (`408-414`) para que la postfirma conozca qué firmas
     fueron descartadas.
3. **Firma cliente PKCS#1:**
   Se firma el objeto `TriphaseData` en local con la clave privada:
   ```java
   td = TriphaseDataSigner.doSign(
       new AOPkcs1Signer(),
       getAlgorithmForJSON(batchB64),
       pk,
       certificates,
       td,
       null
   );
   ```
4. **Llamada de postfirma:**
   POST a `batchPostSignerUrl` pasando el JSON del lote actualizado, la cadena de
   certificados y los datos trifásicos postfirmados en `tridata` serializados en JSON:
   ```
   POST {batchPostSignerUrl}?json={batchUrlSafe}&certs={certsChain}&tridata={td2JsonB64}
   ```
5. **Resultado JSON:**
   El servidor devuelve un JSON conforme al esquema
   `afirma-crypto-batch-client/.../doc-files/resultlog-scheme.html`:
   ```json
   {
     "signs": [
       {
         "id": "7725374e-728d-4a33-9db9-3a4efea4cead",
         "result": "DONE_AND_SAVED",
         "description": ""
       },
       {
         "id": "93d1531c-cd32-4c8e-8cc8-1f1cfe66f64a",
         "result": "DONE_AND_SAVED",
         "description": ""
       }
     ]
   }
   ```

---

## 7. Lotes monofásicos locales (`localBatchProcess=true`)

La modalidad local monofásica se activa cuando la invocación contiene simultáneamente
`jsonbatch=true` y `localBatchProcess=true` (`ProtocolInvocationLauncherBatch.java:400-405`).
Esta funcionalidad está implementada en las clases `JSONBatchManager` y `LocalBatchSigner`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/`).

En esta modalidad **no intervienen servlets de prefirma ni postfirma**: no se envían
peticiones de red a ningún servidor intermedio de firma y la operación se resuelve
íntegramente en la memoria de AutoFirma.

### 7.1 Modelo de datos de entrada

El JSON de entrada se deserializa en `JSONBatchManager.parseBatchConfig(byte[])`
(`JSONBatchManager.java:44-161`):

1. **Parámetros globales del lote:**
   - `stoponerror`: Booleano (por defecto `false`).
   - `suboperation`: Suboperación global (`sign` por defecto, `cosign`, `countersign`).
   - `format`: Formato global obligatorio (lanza `ParameterException` si no existe).
   - `algorithm`: Algoritmo obligatorio (ej. `SHA256withRSA`).
   - `extraparams`: Parámetros adicionales globales en Base64. Se decodifican y
     transforman en `Properties` reemplazando literales `\n` por saltos de línea reales
     (`JSONBatchManager.java:169-182`).
2. **Lista de firmas (`singlesigns`):**
   Cada elemento del array debe contener:
   - `id`: Identificador obligatorio del documento.
   - `datareference`: Cadena en Base64 que contiene **los datos binarios directos del
     documento a firmar**. A diferencia de los lotes remotos (donde el servidor de prefirma descarga URLs o
     recupera documentos mediante su `DocumentManager`), en el procesado local `JSONBatchManager.java:126` invoca
     directamente `Base64.decode(jsonSingleSign.getString(ELEM_DATAREFERENCE))`. Por tanto, **no se admiten URLs
     ni referencias remotas**: si un llamante suministra una URL HTTP/HTTPS en `datareference`, la decodificación
     Base64 falla arrojando una excepción que `parseBatchConfig` (`JSONBatchManager.java:68`) captura y relanza como
     `ParameterException`, traduciéndose en el error `SAF_20` (`ERROR_LOCAL_BATCH_SIGN`) en
     `ProtocolInvocationLauncherBatch.java:386` (o, si la cadena contuviese caracteres decodificables como Base64,
     fallaría posteriormente durante la firma del formato con `SAF_28`, `SAF_29` o `SAF_30`).
   - `format`, `suboperation`, `extraparams`: Opcionales. Si se omiten, heredan los
     valores globales del lote.

### 7.2 Motor de ejecución local: `LocalBatchSigner.signLocalBatch`

La ejecución itera secuencialmente sobre la lista de documentos en
`LocalBatchSigner.signLocalBatch` (`LocalBatchSigner.java:48-84`):

```java
for (final SingleSignOperation singleConfig : batchConfig.getSigns()) {
    if (errorOcurred && batchConfig.isStopOnError()) {
        results.add(new LocalSingleBatchResult(singleConfig.getDocId())); // Estado SKIPPED
    } else {
        try {
            final byte[] signature = processSign(singleConfig, pke);
            results.add(new LocalSingleBatchResult(singleConfig.getDocId(), signature)); // DONE_AND_SAVED
        }
        catch (final SocketOperationException e) {
            errorOcurred = true;
            if (batchConfig.isStopOnError()) {
                for (final LocalSingleBatchResult singleResult : results) {
                    singleResult.setResult(SKIPPED_SIGN);
                    singleResult.setSignature(null);
                }
            }
            results.add(new LocalSingleBatchResult(singleConfig.getDocId(), e.getMessage())); // ERROR_PRE
        }
    }
}
```

#### Particularidades de la firma local por documento (`processSign`):

- **Composición del algoritmo:** Se compone el algoritmo criptográfico combinando el
  algoritmo de resumen solicitado y el tipo de clave privada del certificado:
  `AOSignConstants.composeSignatureAlgorithmName(algorithm, keyType)` (`LocalBatchSigner.java:96`).
- **Resolución de firmadores:** Se obtiene el `AOSigner` correspondiente al formato
  (`AOSignerFactory.getSigner(format)`). Si el formato indicado es `"AUTO"`, se analiza
  el binario mediante `ProtocolInvocationLauncherUtil.identifyFormatFromData(data, ...)`
  (`121`).
- **Forzado de modo headless:** AutoFirma fuerza obligatoriamente la propiedad
  `headless=true` en los `extraParams` de cada firma individual (`137`):
  ```java
  extraParamsCopy.setProperty(EXTRAPARAM_HEADLESS, Boolean.TRUE.toString());
  ```
  Esto garantiza que ningún firmador despliegue ventanas de diálogo, interfaces gráficas
  o solicitudes de confirmación durante la ejecución de los elementos del lote.
- **Suboperaciones soportadas:**
  - `SIGN`: Invoca `signer.sign(...)`.
  - `COSIGN`: Invoca `signer.cosign(...)`.
  - `COUNTERSIGN`: Invoca `signer.countersign(...)`. La diana de contrafirma se extrae
    de la propiedad `target` de `extraparams` con `CounterSignTarget.getTarget`
    (`CounterSignTarget.TREE` si el valor es `tree`, `CounterSignTarget.LEAFS` en otro
    caso; `LocalBatchSigner.java:165-166`). Si falta, `getTarget` lanza
    `IllegalArgumentException` y el documento vuelve como `ERROR_PRE`
    ([BUG-30](A1-bugs-autofirma.md#bug-30-la-contrafirma-en-un-lote-local-sin-target-falla-con-el-objetivo-de-la-contrafirma-no-puede-ser-nulo)).

#### Gestión del retroceso ante errores (`stoponerror`):

Si se produce una excepción durante la firma de un documento y `stopOnError` está activo:
1. Todas las firmas que se habían completado previamente en el bucle **se invalidan**:
   su estado cambia a `"SKIPPED"` y su firma binaria se borra (`singleResult.setSignature(null)`).
2. El documento causante del fallo se registra con estado `"ERROR_PRE"` y en
   `description` se almacena el código de error o mensaje de la excepción (`76`).
3. Todos los documentos restantes del lote se agregan automáticamente con estado `"SKIPPED"`
   sin intentar su firma (`56`).

### 7.3 Formato del resultado JSON local

El JSON final devuelto se ensambla mediante `JSONBatchManager.buildBatchResultJson`
(`JSONBatchManager.java:184-206`):

```json
{
  "signs": [
    {
      "id": "doc-001",
      "result": "DONE_AND_SAVED",
      "signature": "MIAGCSqGSIb3DQEHAqCAMIACAQExCzAJBgUrDgMCGgUAMAsGCSqGSIb3DQEHAQAA..."
    },
    {
      "id": "doc-002",
      "result": "ERROR_PRE",
      "description": "SAF_28"
    },
    {
      "id": "doc-003",
      "result": "SKIPPED"
    }
  ]
}
```

Obsérvese la diferencia fundamental respecto a los lotes remotos: en la firma local
exitosa (`DONE_AND_SAVED`), el JSON incluye obligatoriamente el campo `"signature"` con
el contenido completo del documento firmado en Base64.

---

## 8. Formato de la respuesta del protocolo

Una vez concluido el lote (ya sea remoto o local), AutoFirma compone la carga útil
de respuesta en `ProtocolInvocationLauncherBatch.java:148-226`.

### 8.1 Composición del resultado y soporte de `needcert`

```
┌──────────────────────────────────────┬───┬──────────────────────────────────────┐
│  Resultado del lote (XML o JSON)     │ | │  Certificado firmante en Base64      │
│  (Base64 o Cifrado con key DES)      │   │  (Base64 o Cifrado con key DES)      │
└──────────────────────────────────────┴───┴──────────────────────────────────────┘
                                        ▲
                                        │ Separador presente sólo si needcert=true
```

1. **Resultado base:** La cadena devuelta por el motor de lote (`batchResult`) se convierte
   a bytes en codificación UTF-8.
2. **Inclusión del certificado firmante (`needcert=true`):**
   Si `options.isCertNeeded()` es verdadero, se obtiene el certificado de la clave privada
   seleccionada codificado en DER (`pke.getCertificate().getEncoded()`) (`154`). Si falla
   la codificación, la ejecución aborta con `SAF_18` (`ERROR_DECODING_CERTIFICATE`).
3. **Cifrado simétrico o Base64:**
   - **Si se proporcionó `key`:** Tanto el resultado del lote como el certificado se cifran
     de forma independiente con el algoritmo DES (modo CBC) utilizando la clave simétrica
     (`CypherDataManager.cipherData(..., desKey)`). Si falla el cifrado, se aborta con
     `SAF_12` (`ERROR_ENCRIPTING_DATA`). La respuesta final es:
     ```
     <resultado_lote_cifrado>[|<certificado_cifrado>]
     ```
   - **Si no se proporcionó `key`:** Se emite una advertencia en el log (`186-188`) y los
     datos se codifican en Base64 estándar:
     ```
     Base64(resultado_lote)[|Base64(certificado)]
     ```

### 8.2 Devolución según el canal de transporte

- **Servidor intermedio:**
  Si `options.getStorageServletUrl() != null`, AutoFirma interrumpe el hilo de espera activa
  (`waitingThread.interrupt()`), adquiere el semáforo de red
  `IntermediateServerUtil.getUniqueSemaphoreInstance()` para evitar colisiones de envío,
  y realiza un HTTP POST a `stservlet` con los datos cifrados/codificados y el `id` de
  sesión (`ProtocolInvocationLauncherBatch.java:196-218`). Si la subida falla, se muestra
  y devuelve el error `SAF_11` (`ERROR_SENDING_RESULT`).
- **Socket / WebSocket local:**
  Si la invocación provino de un socket local (`bySocket == true`), `processBatch` retorna
  directamente la cadena construida (`225`), la cual es transmitida al navegador por el
  servidor de sockets.

### 8.3 Procesamiento en el cliente JavaScript (`autoscript.js`)

El cliente JavaScript de AutoFirma (`autoscript.js:4503-4545`) procesa la respuesta:
1. Detecta la presencia del delimitador `|`. Si existe, separa el primer bloque
   (resultado del lote) del segundo bloque (certificado del firmante).
2. Si se utilizó `cipherKey`, descifra cada fragmento individualmente mediante `decipher(...)`;
   en caso contrario, convierte desde Base64 URL-safe.
3. Si la opción `stickySignatory` estaba activa en el navegador, guarda el certificado
   descifrado en la variable global `stickyCertificate` (`4529-4533`).
4. Intenta parsear el resultado con `AfirmaUtils.parseJSONData(result)` (`4540`). Si el lote
   fue en formato JSON, la función entregará un objeto estructurado JavaScript; si fue
   XML, la excepción es capturada silenciosamente y `result` se mantiene como cadena de texto.
5. Invoca el callback de éxito de la aplicación: `successCallback(result, certificate)` (`4545`).

---

## 9. Catálogo de errores de la operación `batch`

La siguiente tabla recoge todos los códigos de error específicos o aplicables al flujo de
la operación `batch`, con su código de protocolo (`SAF_nn`), la constante en código,
el mensaje que visualiza el usuario en la interfaz gráfica modal y la causa exacta:

| Código | Constante en código | Mensaje modal mostrado al usuario | Condición de disparo | Cita en código |
|---|---|---|---|---|
| `CANCEL` | `RESULT_CANCEL` | *(Ninguno: operación cancelada por el usuario)* | El usuario pulsa «Cancelar» o cierra el diálogo de selección de certificados `AOKeyStoreDialog`. | `ProtocolInvocationLauncherBatch.java:56, 135-140` |
| `SAF_03` | `ERROR_PARAMS` | Parámetros incorrectos | Se lanza por `ParameterException`: falta `dat` sin `fileid`, falta `idSession`, identificador no alfanumérico o mayor de 20 caracteres, falta `batchpresignerurl` o `batchpostsignerurl` en lote remoto, formato/algoritmo no declarado en lote local JSON, o el servidor de lotes devuelve HTTP 400. | `ProtocolInvocationLauncher.java:356-367`, `UrlParametersForBatch.java:194-336`, `ProtocolInvocationLauncherBatch.java:362` |
| `SAF_04` | `ERROR_UNSUPPORTED_FORMAT` / `ERROR_NO_SIGN_DATA` | Formato de firma no soportado | Formato no reconocido por `AOSignerFactory` en firma local, o datos vacíos en firma individual. | `LocalBatchSigner.java:113, 226` |
| `SAF_08` | `ERROR_CANNOT_ACCESS_KEYSTORE` | No se ha podido acceder al almacén de certificados | Excepción al instanciar el almacén (`AOKeyStoreManagerFactory.getAOKeyStoreManager`) o error general al mostrar el diálogo. | `ProtocolInvocationLauncherBatch.java:259, 312` |
| `SAF_09` | `ERROR_SIGNATURE_FAILED` | Error en la firma | Error genérico no tipado durante la ejecución criptográfica de una firma individual en lote local. | `LocalBatchSigner.java:256, 261` |
| `SAF_10` | `ERROR_SIGN_WITHOUT_DATA` | Se ha intentado realizar una firma sin datos | El documento individual a firmar contiene un array de bytes vacío. | `LocalBatchSigner.java:221` |
| `SAF_11` | `ERROR_SENDING_RESULT` | Error enviando los datos al servidor | Fallo de conexión de red al subir el resultado a `stservlet`. | `ProtocolInvocationLauncherBatch.java:210` |
| `SAF_12` | `ERROR_ENCRIPTING_DATA` | Error al cifrar los datos | Fallo durante el cifrado simétrico DES con `key` del resultado o del certificado. | `ProtocolInvocationLauncherBatch.java:177` |
| `SAF_13` | `ERROR_LOCAL_ACCESS_BLOCKED` | Acceso a recurso local denegado | La URL de `stservlet` apunta a una dirección IP de bucle invertido o red local privada sin permiso. Un pre/postsigner local en el lote da `SAF_03` ([BUG-28](A1-bugs-autofirma.md#bug-28-un-servlet-del-lote-en-el-loopback-se-rechaza-con-saf_03-en-lugar-de-saf_13)). | `UrlParameters.java:286-315` |
| `SAF_14` | `ERROR_UNSUPPORTED_OPERATION` | Operación no soportada | La suboperación de un documento en lote local no es `sign`, `cosign` ni `countersign`. | `LocalBatchSigner.java:175` |
| `SAF_15` | `ERROR_DECRYPTING_DATA` | Error al descifrar los datos obtenidos | Fallo al descifrar con `key` el sobre de parámetros descargado de `rtservlet`. | `ProtocolInvocationLauncher.java:318-322` |
| `SAF_16` | `ERROR_RECOVERING_DATA` | No se pudieron recuperar los datos del servidor | Fallo HTTP o longitud anómala al descargar el sobre de parámetros desde `rtservlet`. | `ProtocolInvocationLauncher.java:311-316` |
| `SAF_17` | `ERROR_UNKNOWN_SIGNER` | Firma no reconocida | En lote local con formato `"AUTO"`, el analizador no pudo deducir el formato del fichero. | `LocalBatchSigner.java:125` |
| `SAF_18` | `ERROR_DECODING_CERTIFICATE` | Error al decodificar el certificado | Fallo al invocar `getEncoded()` en el certificado firmante cuando `needcert=true`. | `ProtocolInvocationLauncherBatch.java:157` |
| `SAF_19` | `ERROR_NO_CERTIFICATES_KEYSTORE` | No se encontró ningún certificado válido en el almacén | El almacén está vacío o ningún certificado supera los filtros de `CertFilterManager`. | `ProtocolInvocationLauncherBatch.java:305` |
| `SAF_20` | `ERROR_LOCAL_BATCH_SIGN` | Error en el proceso del lote de firmas | Excepción general no controlada durante el proceso de firma de lote local (`localBatchProcess=true`). | `ProtocolInvocationLauncherBatch.java:386` |
| `SAF_21` | `ERROR_UNSUPPORTED_PROCEDURE` | Procedimiento no soportado | La versión de protocolo solicitada en la llamada supera `MAX_PROTOCOL_VERSION_SUPPORTED`. | `ProtocolInvocationLauncherBatch.java:81` |
| `SAF_26` | `ERROR_CONTACT_BATCH_SERVICE` | Error al contactar con el servicio de firma de lotes | El servidor de prefirma o postfirma de lotes responde con un error HTTP en el rango 4xx (distinto de 400). | `ProtocolInvocationLauncherBatch.java:367` |
| `SAF_27` | `ERROR_BATCH_SIGNATURE` | Error durante la firma del lote | El servidor de pre/postfirma devuelve un error HTTP 5xx, una respuesta ininteligible, no acepta la conexión ([BUG-29](A1-bugs-autofirma.md#bug-29-un-servicio-de-lotes-inalcanzable-se-reporta-como-saf_27-el-servicio-informó-de-un-error-en-lugar-de-saf_26)) o lanza `AOException` durante el lote remoto. | `ProtocolInvocationLauncherBatch.java:371, 380, 387` |
| `SAF_28` | `ERROR_INVALID_PDF` | El documento proporcionado no es un PDF válido | Documento corrupto o no PDF en firma PAdES local. | `LocalBatchSigner.java:196` |
| `SAF_29` | `ERROR_INVALID_XML` | El documento proporcionado no es un XML válido | Documento con sintaxis XML errónea en firma XAdES local. | `LocalBatchSigner.java:201` |
| `SAF_30` | `ERROR_INVALID_DATA` | Los datos proporcionados no son válidos | Error de estructura de fichero en firma local (`AOFormatFileException`). | `LocalBatchSigner.java:206` |
| `SAF_32` | `ERROR_FACE_ALREADY_SIGNED` | La factura ya contiene una firma | Se intentó firmar una FacturaE que ya estaba firmada y no se admite refirma. | `LocalBatchSigner.java:216` |
| `SAF_33` | `ERROR_PDF_WRONG_PASSWORD` | El PDF está protegido con contraseña | El PDF a firmar localmente en PAdES requiere clave de apertura o permisos. | `LocalBatchSigner.java:241` |
| `SAF_34` | `ERROR_PDF_UNREG_SIGN` | El PDF contiene firmas no registradas | El documento PDF presenta firmas incrementales o modificaciones no permitidas. | `LocalBatchSigner.java:231` |
| `SAF_35` | `ERROR_PDF_CERTIFIED` | El PDF está certificado | El documento PDF contiene una firma de certificación que prohíbe nuevas firmas. | `LocalBatchSigner.java:236` |
| `SAF_38` | `ERROR_INVALID_FACTURAE` | La factura no es válida | Contenido no conforme al esquema FacturaE en firma local. | `LocalBatchSigner.java:211` |
| `SAF_39` | `ERROR_INVALID_SIGNATURE` | La firma de entrada no es válida | Firma corrupta al realizar una operación local de cofirma o contrafirma. | `LocalBatchSigner.java:251` |
| `SAF_40` | `ERROR_RECOVER_SERVER_DOCUMENT` | Error al recuperar el documento del servidor | Fallo al resolver un documento en firma trifásica local (`AOTriphaseException`). | `LocalBatchSigner.java:191` |
| `SAF_41` | `ERROR_MINIMUM_VERSION_NON_SATISTIED` | Versión del aplicativo no suficiente | La versión de AutoFirma instalada es inferior a la exigida en el parámetro `mcv`. | `ProtocolInvocationLauncherBatch.java:94` |
| `SAF_51` | `ERROR_INCOMPATIBLE_KEY_TYPE` | Tipo de clave no compatible | La clave privada del certificado seleccionado no admite el algoritmo indicado (ej. clave EC con algoritmo RSA). | `LocalBatchSigner.java:100` |
| `SAF_52` | `ERROR_LOCKED_KEYSTORE` | El almacén de certificados está bloqueado | El almacén o token criptográfico se encuentra bloqueado por exceso de intentos de PIN. | `ProtocolInvocationLauncherBatch.java:357` |

