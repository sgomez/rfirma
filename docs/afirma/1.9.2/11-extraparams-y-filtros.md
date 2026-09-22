# 11. El diccionario `properties` y los filtros de certificado

Este capítulo describe en profundidad el parámetro `properties` del protocolo
`afirma://` de AutoFirma 1.9.2: su codificación y decodificación, el ciclo de vida
en la aplicación, el procesamiento y macro-expansión de políticas de firma mediante
`ExtraParamsProcessor`, los parámetros generales de comportamiento (`headless`,
`mandatoryCertSelection`, opciones de diálogo de ficheros, etc.) y la arquitectura
y sintaxis exhaustiva de los filtros de certificados gestionados por `CertFilterManager`
y el diálogo de selección `AOKeyStoreDialog`.

El catálogo de propiedades específicas por cada formato criptográfico de firma
(PAdES, CAdES, XAdES, FacturaE y ASiC) se detalla en el [capítulo 12](12-extraparams-por-formato.md).

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. Visión general y propósito de `properties`

En el protocolo `afirma://`, las operaciones que involucran selección de certificados
o tratamiento de datos criptográficos (`sign`, `cosign`, `countersign`, `signandsave`,
`selectcert`, `batch`) permiten recibir una colección arbitraria de parámetros de
configuración agrupados bajo el parámetro de URL `properties`.

Este parámetro cumple tres funciones esenciales:
1. **Configuración de la interfaz y ejecución**: define si el proceso se ejecuta de
   forma desatendida (`headless`), si se fuerza la selección de certificado cuando
   solo existe uno (`mandatoryCertSelection`), si se bloquea la apertura de almacenes
   externos (`disableopeningexternalstores`), o las rutas y extensiones por defecto
   en los diálogos de ficheros (`filename*`).
2. **Filtrado de certificados**: especifica criterios para cribar qué certificados del
   almacén del usuario se muestran en el diálogo de selección o se seleccionan de forma
   automática (por emisor, titular, huella digital, número de serie, usos de clave,
   política o presencia de seudónimo).
3. **Parámetros de firma y políticas criptográficas**: transmite opciones del formato
   de firma y permite la macro-expansión de políticas de firma oficiales (como la
   política de la Administración General del Estado vía `expPolicy=FirmaAGE`).

---

## 2. Codificación, transmisión y decodificación

### 2.1 En el cliente JavaScript (`autoscript.js`)

En el cliente JavaScript de referencia, los parámetros adicionales se configuran
habitualmente como una cadena multilínea de pares `clave=valor` separados por el
carácter de salto de línea `\n` (o `\r\n`).

Antes de incorporar el diccionario a la URL del protocolo `afirma://`, `autoscript.js`
lo codifica en **Base64 URL-safe** mediante la función auxiliar `Base64.encode(data, true)`:
* `afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js:1945` (en `sign` por servidor intermedio):
  `data.properties = createKeyValuePair("properties", extraParams != null ? Base64.encode(extraParams, true) : null, true);`
* `autoscript.js:2013` (en `selectcert`):
  `data.properties = createKeyValuePair("properties", certFilters != null ? Base64.encode(certFilters, true) : null, true);`
* `autoscript.js:2700, 2839` (en transporte por socket local y WebSocket).

La variante URL-safe de Base64 sustituye los caracteres conflictivos en URLs:
el signo `+` se transforma en `-`, y la barra `/` se transforma en `_`.

### 2.2 Decodificación en Java (`AOUtil.base642Properties`)

En la aplicación de escritorio AutoFirma, la recepción y parseo del parámetro `properties`
se centraliza en el método utilitario `AOUtil.base642Properties`:
(`afirma-core/src/main/java/es/gob/afirma/core/misc/AOUtil.java:485-495`).

```java
public static Properties base642Properties(final String base64) throws IOException {
    final Properties p = new Properties();
    if (base64 == null || base64.isEmpty()) {
        return p;
    }
    p.load(new InputStreamReader(
        new ByteArrayInputStream(Base64.decode(base64.replace('-', '+').replace('_', '/'))),
        DEFAULT_ENCODING)
    );
    return p;
}
```

1. **Reversión URL-Safe**: se reemplaza `-` por `+` y `_` por `/`.
2. **Decodificación Base64**: se procesan los octetos con `es.gob.afirma.core.misc.Base64.decode`.
3. **Carga en `java.util.Properties`**: se alimenta un `InputStreamReader` con la codificación
   `DEFAULT_ENCODING`, que en `AOUtil.java:52` está fijada como constante estática:
   `StandardCharsets.UTF_8`.
4. **Semántica del formato Properties**:
   * Las líneas en blanco o aquellas cuyo primer carácter no blanco sea `#` o `!` se consideran comentarios.
   * El delimitador entre clave y valor puede ser el signo igual (`=`), dos puntos (`:`) o espacios en blanco.
   * Admite secuencias de escape estándar de Java (como `\n`, `\t`, `\\` o escapes Unicode `\uXXXX`).

### 2.3 Variante de deserialización (`ExtraParamsProcessor.convertToProperties`)

El módulo `afirma-core` incluye asimismo el método `ExtraParamsProcessor.convertToProperties`:
(`afirma-core/src/main/java/es/gob/afirma/core/signers/ExtraParamsProcessor.java:59-76`).

```java
public static Properties convertToProperties(final String entries) {
    final Properties params = new Properties();
    if (entries == null) {
        return params;
    }
    try {
        params.load(new ByteArrayInputStream(entries.getBytes()));
    }
    catch (final Exception e) {
        LOGGER.warning("Se han encontrado entradas no validas en la configuracion de la operacion: " + e);
    }
    return params;
}
```

A diferencia de `AOUtil.base642Properties`, este método procesa una cadena de texto ya
descodificada y utiliza `entries.getBytes()` (el juego de caracteres por defecto de la
plataforma del sistema operativo anfitrión), lo que en entornos donde la codificación local
no sea UTF-8 provocaría la corrupción de caracteres no ASCII.

Sin embargo, en el protocolo de escritorio `afirma://`, este método **jamás se invoca**:
todas las invocaciones de protocolo (`UrlParametersToSign`, `UrlParametersToSignAndSave`,
`UrlParametersToSelectCert`, `UrlParametersForBatch`) procesan exclusivamente el parámetro
`properties` a través de `AOUtil.base642Properties`, el cual fija de forma inmutable
`StandardCharsets.UTF_8`. El método `ExtraParamsProcessor.convertToProperties` constituye
código legado utilizado exclusivamente por el applet web de navegador
(`afirma-ui-miniapplet` · `MiniAfirmaApplet.java:149, 322, 478, 1401, 1475`), el cual
recibía los parámetros como cadenas planas mediante LiveConnect. Por tanto, no existe riesgo
de corrupción de juego de caracteres en las invocaciones por protocolo en ninguna plataforma.

### 2.4 Resiliencia ante errores de decodificación

En todos los parsers de parámetros de protocolo (`UrlParametersToSign.java:301-314`,
`UrlParametersToSignAndSave.java:291-308`, `UrlParametersToSelectCert.java:185-202`,
`UrlParametersForBatch.java:286-303`), si el parámetro `properties` está ausente,
es vacío o sufre una excepción de decodificación (`IOException`), AutoFirma **no aborta**
la operación con un error `SAF_03`: registra la incidencia en el log con severidad
`LOGGER.severe` y continúa la ejecución inicializando un objeto `Properties` vacío
(`setExtraParams(new Properties())`).

---

## 3. Ciclo de vida y etapas de procesamiento

El siguiente diagrama ilustra cómo fluye el diccionario `properties` desde que se
deserializa de la URL hasta que sus propiedades se aplican en los distintos módulos:

```
                  afirma://<op>?properties=<Base64URL>&...
                                     │
                                     ▼
                ┌─────────────────────────────────────────┐
                │ AOUtil.base642Properties                │
                │ -> Genera java.util.Properties (UTF-8)  │
                └────────────────────┬────────────────────┘
                                     │
                                     ▼
                ┌─────────────────────────────────────────┐
                │ ProtocolInvocationLauncherSign / Save   │
                │ options.getExtraParams().remove("profile")│ <-- Eliminación incondicional
                └────────────────────┬────────────────────┘
                                     │
                                     ▼
                ┌─────────────────────────────────────────┐
                │ ExtraParamsProcessor.expandProperties   │
                │ - Procesa expPolicy (FirmaAGE...)       │
                │ - Normaliza formatos y setea AdESPolicy │
                │ - Borra expPolicy al finalizar          │
                └────────────────────┬────────────────────┘
                                     │
                                     ▼
                ┌─────────────────────────────────────────┐
                │ CertFilterManager                       │
                │ - Lee headless, mandatoryCertSelection  │
                │ - Lee filter, filters, filters.1..N     │
                │ - Genera lista List<CertificateFilter>  │
                └────────────────────┬────────────────────┘
                                     │
                                     ▼
                ┌─────────────────────────────────────────┐
                │ AOKeyStoreDialog / KeyStoreUtilities    │
                │ - Filtra certificados con clave privada │
                │ - Aplica filtros disyuntivos (OR)       │
                │ - Oculta diálogo si headless/auto (n==1)│
                └────────────────────┬────────────────────┘
                                     │
                                     ▼
                ┌─────────────────────────────────────────┐
                │ AOSigner.sign / cosign / countersign    │
                │ Recibe las propiedades de formato       │
                └─────────────────────────────────────────┘
```

Etapas clave del ciclo de vida:

1. **Extracción en los tipos `UrlParametersTo*`**:
   Los parsers invocan `AOUtil.base642Properties` y almacenan el resultado en el atributo
   privado `extraParams`.
2. **Eliminación preventiva de `profile`**:
   Al despachar operaciones `sign` (`ProtocolInvocationLauncherSign.java:153`) o `signandsave`
   (`ProtocolInvocationLauncherSignAndSave.java:150`), AutoFirma elimina de forma explícita
   e incondicional la clave `profile` mediante `options.getExtraParams().remove("profile")`.
   Esto se implementó para evitar que integradores forzasen perfiles *baseline* no
   soportados por el flujo interactivo de escritorio.
3. **Macro-expansión (`expandExtraParams`)**:
   En `UrlParametersToSign.java:342-350` y `UrlParametersToSignAndSave.java:349-357`, se invoca
   `ExtraParamsProcessor.expandProperties(extraParams, data, format)`. Si la propiedad `expPolicy`
   está presente, se inyectan las propiedades completas de la política y se elimina `expPolicy`.
4. **Instanciación de `CertFilterManager`**:
   Tanto en firma como en `selectcert` y `batch`, se crea una instancia de `CertFilterManager`
   pasándole `extraParams`. De ahí se extraen los criterios de filtrado y las directivas
   de comportamiento del diálogo.
5. **Consumo de parámetros de interfaz y guardado**:
   Las clases lanzadoras leen propiedades de interacción como `checkSignatures`,
   `headless`, `target`, o las opciones de diálogo de ficheros `filename*`.
6. **Entrega a `AOSigner`**:
   Las propiedades depuradas se entregan como argumento `extraParams` a los métodos
   `sign`, `cosign` o `countersign` de la implementación correspondiente de `AOSigner`.
7. **Forzado de `headless` en lotes locales**:
   En `LocalBatchSigner.java:137`, al ejecutar cada firma individual de un lote local,
   se clonan las propiedades y se añade obligatoriamente `headless=true` para prevenir
   bloqueos por ventanas emergentes.
8. **Fijación en el cliente web tras selección previa**:
   En `autoscript.js:4675-4676`, cuando se ejecuta una firma con un certificado previamente
   obtenido, el cliente elimina los filtros anteriores y añade `filters=encodedcert:<b64Cert>`
   y `headless=true` en el diccionario `properties`.

---

## 4. Expansión de políticas de firma (`ExtraParamsProcessor`)

La clase `ExtraParamsProcessor` (`afirma-core/src/main/java/es/gob/afirma/core/signers/ExtraParamsProcessor.java`)
actúa como un preprocesador que expande alias de alto nivel en configuraciones
detalladas requeridas por las especificaciones AdES.

### 4.1 La macro-propiedad `expPolicy`

Permite configurar de forma unificada todos los metadatos de una política de firma
mediante una única clave en `properties`:
`expPolicy=<nombre_politica>`

El método `expandProperties(Properties params, byte[] signedData, String format)`
(`ExtraParamsProcessor.java:122-129`) clona las propiedades y llama a `expandPolicyKeys`:

```java
private static void expandPolicyKeys(final Properties p, final byte[] signedData, final String format)
        throws IncompatiblePolicyException
```

1. Si `p` no contiene `expPolicy`, retorna inmediatamente sin alterar las propiedades.
2. Si contiene `expPolicy`, obtiene el valor `policyName`.
3. Comprueba si la política está soportada mediante `isSupportedPolicy(policyName)`
   (`ExtraParamsProcessor.java:217-220`):
   * `FirmaAGE` (`AdESPolicyPropertiesManager.POLICY_ID_AGE`): Política de firma de la
     Administración General del Estado v1.9.
   * `FirmaAGE18` (`AdESPolicyPropertiesManager.POLICY_ID_AGE_1_8`): Política de firma
     de la AGE v1.8.
   * Si no coincide con ninguno de estos dos valores literales, elimina `expPolicy` de `p`
     y lanza una excepción `IncompatiblePolicyException`:
     `"No se soporta la expansion de atributos para la politica: " + policyName`.
   * **Incompatibilidad de `FirmaAGE19` ([BUG-20](A1-bugs-autofirma.md#bug-20-incompatibilidad-entre-policyproperties-y-extraparamsprocessor-impide-el-uso-del-identificador-oficial-firmaage19)):**
     Aunque el recurso interno `afirma-core/src/main/resources/policy.properties:11-17`
     define explícitamente los parámetros para `FirmaAGE19` indicando en comentarios que
     se utilice mediante `"expPolicy=FirmaAGE19"`, la constante no existe en `AdESPolicyPropertiesManager`
     ni se contempló en `isSupportedPolicy`. Por ello, enviar `expPolicy=FirmaAGE19` arroja
     `IncompatiblePolicyException`, provocando que `ProtocolInvocationLauncherSign.java:492-496`
     o `SignAndSave.java:484-488` capturen el error y emitan `SocketOperationException(ERROR_INVALID_POLICY)`
     con código de error `SAF_23`.
4. Normaliza el nombre del formato mediante `normalizeFormat(format)` (`ExtraParamsProcessor.java:185-208`):
   * `CAdES` o `CAdES-Tri` -> `"CAdES"`.
   * `XAdES`, `XAdES-Tri` o cadenas que comiencen por `"XAdES "` -> `"XAdES"`.
   * `PAdES`, `PAdES-Tri`, `PDF` o `PDF-Tri` -> `"PAdES"`.
   * Los formatos ASiC (`CAdES-ASiC`, `XAdES-ASiC`) **no son normalizados** y se rechazan.
5. Aplica los atributos específicos del formato (ver §4.2).
6. Finalmente, **elimina incondicionalmente** `expPolicy` de las propiedades (`p.remove("expPolicy")`).

### 4.2 Lógica de expansión por formato

#### A. Firmas CAdES (`setCAdESPolicyAGEAttributes`, líneas 230-252)
* **Modo de empaquetado (`mode`)**: la política de la AGE establece que la firma debe
  ser implícita (*attached*) siempre que el tamaño del documento sea razonable.
  Si `mode` no está definido en `params` y se han proporcionado `signedData`:
  * Si `signedData.length < 1048576` (1 MB) -> fija `mode=implicit`.
  * Si `signedData.length >= 1048576` (1 MB) -> fija `mode=explicit`.
  * Si el integrador ya había definido `mode`, se respeta su valor.
* **Perfil de firma (`profile`)**: si el integrador había indicado `profile=baseline`,
  se emite una advertencia de log y se sobreescribe forzosamente a `profile=advanced`,
  dado que la política 1.9 de la AGE no admite perfiles *baseline*.
* Llama a `AdESPolicyPropertiesManager.setProperties(params, policyName, "CAdES")`.

#### B. Firmas XAdES (`setXAdESPolicyAGEAttributes`, líneas 261-282)
* **Formato (`format`)**: la firma conforme a la política de la AGE debe ser obligatoriamente
  *Detached* o *Enveloped*. Si el parámetro `format` en `params` no coincide con
  `XAdES Detached` ni con `XAdES Enveloped` (ignorando mayúsculas):
  * Se emite una advertencia en el log.
  * Se fuerza incondicionalmente a `format=XAdES Detached`.
* **Perfil de firma (`profile`)**: si venía como `baseline`, se ignora con advertencia
  y se fija a `profile=advanced`.
* Llama a `AdESPolicyPropertiesManager.setProperties(params, policyName, "XAdES")`.

#### C. Firmas PAdES (`setPAdESPolicyAGEAttributes`, líneas 293-306)
* **Compatibilidad de versión**: la firma PAdES solo está soportada en la política de la
  AGE a partir de la versión 1.9 (`policyName.equals("FirmaAGE")`). Si se solicita con
  `FirmaAGE18`, la condición de despacho en línea 165 no se cumple y lanza
  `IncompatiblePolicyException: El formato de firma PAdES no esta soportado por la politica FirmaAGE18`.
* **Subfiltro de firma (`signatureSubFilter`)**: exige estrictamente el valor
  `"ETSI.CAdES.detached"`.
  * Si `params` incluye `signatureSubFilter` con cualquier otro valor, lanza
    `IncompatiblePolicyException("En PAdES con politica firma AGE debe usarse siempre el filtro 'ETSI.CAdES.detached'")`.
  * Si no estaba fijado, establece `signatureSubFilter=ETSI.CAdES.detached`.
* Llama a `AdESPolicyPropertiesManager.setProperties(params, policyName, "PAdES")`.

### 4.3 Propiedades inyectadas desde `AdESPolicyPropertiesManager`

El gestor `AdESPolicyPropertiesManager` (`afirma-core/src/main/java/es/gob/afirma/core/signers/AdESPolicyPropertiesManager.java`)
carga los metadatos desde el fichero `policy.properties` del classpath:
(`afirma-core/src/main/resources/policy.properties`).

Inyecta en el objeto `Properties` las siguientes claves:

| Clave inyectada | Constante | Origen en `policy.properties` | Valor típico para `FirmaAGE` |
|---|---|---|---|
| `policyIdentifier` | `PROPERTY_POLICY_IDENTIFIER` | `<ID>.policyIdentifier` | `urn:oid:2.16.724.1.3.1.1.2.1.9` |
| `policyQualifier` | `PROPERTY_POLICY_QUALIFIER` | `<ID>.policyQualifier` | `https://sede.administracion.gob.es/politica_de_firma_anexo_1.pdf` |
| `policyIdentifierHashAlgorithm` | `PROPERTY_POLICY_HASH_ALGORITHM` | `<ID>.policyIdentifierHashAlgorithm` | `http://www.w3.org/2000/09/xmldsig#sha1` |
| `policyIdentifierHash` | `PROPERTY_POLICY_HASH` | `<ID>.policyIdentifierHash.<FORMATO>` | `G7roucf600+f03r/o0bAOQ6WAs0=` |
| `policyDescription` | `PROPERTY_POLICY_DESCRIPTION` | `<ID>.policyDescription` | *(Opcional, vacío en AGE)* |

* **Prevalencia**: según `AdESPolicyPropertiesManager.setProperty` (líneas 176-182), si
  alguna de estas propiedades ya existía previamente en `params`, se sobreescribe
  con el valor de la política y se emite un aviso en el log:
  `"La siguiente propiedad se ignora en favor del valor derivado de la politica establecida: " + property`.

### 4.4 Utilidades adicionales de `ExtraParamsProcessor`

* **`loadByteArrayFromExtraParams`** (`ExtraParamsProcessor.java:351-375`):
  Recupera un binario configurado dentro de una propiedad (por ejemplo rúbricas gráficas,
  marcas de agua o imágenes de firma en PDF):
  * Parámetros: `(Properties extraParams, String paramName, boolean secureMode)`.
  * Si `secureMode == true` (modo seguro por defecto): el valor **debe** ser una cadena
    Base64 válida; se decodifica con `Base64.decode(value)`.
  * Si `secureMode == false`: si la longitud del valor es menor de 255 caracteres y no es
    Base64, intenta interpretarlo como una URI o ruta del sistema de ficheros local
    (`AOUtil.createURI(value)` y `AOUtil.loadFile(uri)`).
* **`configAutoFormat`** (`ExtraParamsProcessor.java:323-335`):
  Cuando se solicita una firma o multifirma con formato `"AUTO"` sobre un documento PDF,
  invoca por introspección el método `configureRespectfulProperties(byte[] data, Properties params)`
  de `AOPDFSigner` para que la nueva firma respete las propiedades y revisiones de las
  firmas PDF existentes.

---

## 5. Parámetros generales de comportamiento y ejecución

Las siguientes propiedades controlan la ejecución de la aplicación, la interacción con
el usuario y los diálogos del sistema:

| Clave | Clase y constante | Tipo | Valor por defecto | Descripción y efectos |
|---|---|---|---|---|
| `headless` | `AfirmaExtraParams.HEADLESS`, `CertFilterManager.HEADLESS_PROPERTY` | Booleano | `false` | Ejecución desatendida. En `AOKeyStoreDialog:726-729`, omite el diálogo de selección de certificados **únicamente si queda un solo certificado candidato**. En `ProtocolInvocationLauncherSign:432,793`, omite diálogos modales de confirmación o preguntas sobre firmas previas. En lotes locales se fuerza a `true`. |
| `mandatoryCertSelection` | `CertFilterManager.MANDATORY_CERT_SELECTION_PROPERTY` | Booleano | `true` | Si se define explícitamente como `false`, AutoFirma activa la selección automática sin mostrar el diálogo cuando hay un único certificado disponible (`isMandatoryCertificate` devuelve `true`). |
| `checkSignatures` | `AfirmaExtraParams.CHECK_SIGNATURES` | Booleano | `false` | En co-firmas y contra-firmas (`ProtocolInvocationLauncherSign:410`, `SignAndSave:402`), valida la integridad y vigencia de las firmas preexistentes antes de añadir la nueva. |
| `target` | `AfirmaExtraParams.TARGET` | Cadena | `"leafs"` | Ámbito de una contra-firma: `"leafs"` (contra-firma solo las firmas que son hojas del árbol de firmas) o `"tree"` (contra-firma todos los nodos del árbol). |
| `mode` | `AfirmaExtraParams.MODE` | Cadena | `"implicit"` | Modo de empaquetado de datos en CAdES: `"implicit"` (datos embebidos) o `"explicit"` (datos separados / detached). |
| `filenameExts` | `AfirmaExtraParams.LOAD_FILE_EXTS` | Cadena | Ninguno | Extensiones de fichero permitidas en el diálogo de carga de fichero (separadas por coma o punto y coma, ej. `"pdf,txt"`). |
| `filenameDescription` | `AfirmaExtraParams.LOAD_FILE_DESCRIPTION` | Cadena | `"Todos los archivos"` | Descripción textual del filtro de ficheros en el diálogo de carga. |
| `filenameCurrentDir` | `AfirmaExtraParams.LOAD_FILE_CURRENT_DIR` | Cadena | Directorio de usuario | Ruta del directorio inicial donde se posiciona el diálogo de carga de ficheros. |
| `filenameActualName` | `AfirmaExtraParams.LOAD_FILE_FILENAME` | Cadena | Ninguno | Nombre de fichero propuesto por defecto en el diálogo de carga. |
| `filenameSaveExts` | `AfirmaExtraParams.SAVE_FILE_EXTS` | Cadena | Ninguno | Extensiones permitidas en el diálogo de guardado de fichero en `signandsave`. |
| `filenameSaveDescription` | `AfirmaExtraParams.SAVE_FILE_DESCRIPTION` | Cadena | `"ProtocolLauncher.30"` | Descripción textual del filtro en el diálogo de guardado de `signandsave`. |
| `filenameSaveCurrentDir` | `AfirmaExtraParams.SAVE_FILE_CURRENT_DIR` | Cadena | Directorio de usuario | Directorio inicial en el diálogo de guardado de `signandsave`. |

---

## 6. Arquitectura y lógica de filtrado de certificados

La gestión de filtros de certificados se implementa en el módulo `afirma-keystores-filters`
a través de la clase `CertFilterManager`
(`afirma-keystores-filters/src/main/java/es/gob/afirma/keystores/filters/CertFilterManager.java`).

### 6.1 Cómo se declaran los filtros en `properties`

`CertFilterManager.getFilterValues` (`líneas 167-183`) busca las definiciones de filtrado
en el objeto `Properties` siguiendo un orden de precedencia excluyente:

```java
private static List<String> getFilterValues(final Properties config) {
    final List<String> filterValues = new ArrayList<>();
    if (config.containsKey("filter")) {
        filterValues.add(config.getProperty("filter"));
    }
    else if (config.containsKey("filters")) {
        filterValues.add(config.getProperty("filters"));
    }
    else if (config.containsKey("filters.1")) {
        int i = 1;
        while (config.containsKey("filters." + i)) {
            filterValues.add(config.getProperty("filters." + i));
            i++;
        }
    }
    return filterValues;
}
```

* **Prioridad estricta**:
  1. Si existe la clave `filter`, se toma su valor y se ignoran por completo `filters` y `filters.N`.
  2. Si no existe `filter` pero existe `filters`, se toma su valor y se ignoran `filters.N`.
  3. Si no existen `filter` ni `filters`, se recorre la secuencia correlativa `filters.1`, `filters.2`, `filters.3`, ... hasta encontrar un hueco.

**Discrepancia con el MCF.** El ejemplo de selección de certificado del manual
declara `filters.0` y `filters.1` (MCF §6.6, pág. 74). El código empieza en
`filters.1` y no consulta `filters.0`, así que en ese ejemplo solo se aplica el
segundo filtro; el propio manual numera desde 1 en su apartado de filtros (MCF
§7.3, págs. 86-93). La clave `filter`, en singular, no aparece en el manual.

### 6.2 Semántica lógica: Conjunción (AND) vs. Disyunción (OR)

El modelo de filtrado combina conjunción y disyunción a dos niveles:

1. **Nivel interno (AND — Conjunción)**:
   Dentro de una misma cadena de filtrado (ya sea `filter`, `filters` o un `filters.N`),
   se pueden encadenar múltiples criterios separándolos por punto y coma (`;`):
   `filter=criterio1;criterio2;criterio3`
   En `parseFilter` (`CertFilterManager.java:185-269`), los criterios se separan por `;`
   y se envuelven en una instancia de `MultipleCertificateFilter` (`afirma-core-keystores/.../MultipleCertificateFilter.java`).
   Un certificado debe satisfacer **todos y cada uno** de los criterios del grupo para
   ser aceptado.

2. **Nivel externo (OR — Disyunción)**:
   La declaración indexada `filters.1`, `filters.2`, ..., `filters.N` modela una **disyunción**:
   En `KeyStoreUtilities.getAliasesByFriendlyName` (`afirma-core-keystores/.../KeyStoreUtilities.java:267-276`):
   ```java
   for (final CertificateFilter cf : certFilters) {
       final String[] certAliases = aliassesByFriendlyName.keySet().toArray(...);
       for (final String filteredAlias : cf.matches(certAliases, ksm)) {
           filteredAliases.put(filteredAlias, aliassesByFriendlyName.get(filteredAlias));
           aliassesByFriendlyName.remove(filteredAlias);
       }
   }
   ```
   Cada grupo de filtros `filters.i` se evalúa sobre los alias que aún no han sido aceptados.
   Si un certificado cumple los requisitos de `filters.1`, se añade a la lista de admitidos;
   si no, se evalúa contra `filters.2`, y así sucesivamente.

### 6.3 Filtro por defecto de caducidad (ETSI TS 119 102-1)

En `CertFilterManager.java:128-135`:
```java
// Siguiendo los criterios de la ETSI TS 119 102-1, un usuario no deberia firmar nunca con
// un certificado caducado, asi que, si no se definio ningun tipo de filtrado, se agregara
// un filtro omitiendo estos certificados. Si se agregaron filtros, se considerara que es
// el integrador estara definiendo sus preferencias concretas de filtrado
if (this.filters.isEmpty()) {
    this.filters.add(new ExpiredCertificateFilter(false));
}
```
* **Comportamiento por omisión**: Si el integrador **no define ningún filtro** (la lista `this.filters`
  permanece vacía tras analizar `filter`, `filters` y `filters.N`), AutoFirma aplica de oficio la
  recomendación de la norma ETSI TS 119 102-1 e inyecta automáticamente `ExpiredCertificateFilter(false)`,
  ocultando del diálogo todos los certificados expirados (`cert.checkValidity()`).
* **Desactivación implícita ante filtros específicos**: Si la aplicación web proporciona **cualquier**
  criterio de filtrado (como por ejemplo un emisor `issuer.contains:FNMT`, un NIF `subject.rfc2254:...`
  o un uso de clave), la condición `this.filters.isEmpty()` evalúa a `false` y el filtro de caducidad
  **no se añade**. Por diseño, el código asume que el integrador toma el control completo de los criterios
  de aceptación. En consecuencia, aquellos certificados caducados que cumplan las condiciones explícitas
  especificadas por la sede **aparecerán en el diálogo de selección** y podrán ser elegidos por la persona
  usuaria, a menos que el integrador concatene expresamente el subfiltro `;nonexpired:true`.

### 6.4 Bloqueo de almacenes externos

Si en cualquier parte de la cadena de filtros aparece el token:
`disableopeningexternalstores`
(`CertFilterManager.java:197-199`), no se añade ningún objeto de filtrado de certificados,
pero se conmuta el indicador booleano `this.allowExternalStores = false`.
Al construir el diálogo `AOKeyStoreDialog` (`ProtocolInvocationLauncherSelectCert.java:185`,
`ProtocolInvocationLauncherSign.java:655`), se invoca:
`dialog.allowOpenExternalStores(filterManager.isExternalStoresOpeningAllowed());`
Esto oculta o deshabilita los botones de la interfaz gráfica que permiten al usuario
abrir almacenes de certificados externos o ficheros PKCS#12/software.

### 6.5 Requisito obligatorio de clave privada

Independientemente de los filtros solicitados por la aplicación web, el motor de
AutoFirma (`KeyStoreUtilities.java:246-263`) exige de forma estricta que todo certificado
candidato tenga asociada una clave privada accesible (`ksm.isKeyEntry(alias) == true`).
Los certificados públicos huérfanos o los certificados de CA importados en el almacén
se descartan de manera silenciosa en el paso previo a la aplicación de los filtros.

---

## 7. Catálogo exhaustivo de filtros de certificados

A continuación se detallan todos los filtros implementados en `CertFilterManager.parseFilter`:

| Prefijo de filtro | Clase Java | Parámetro recibido | Descripción |
|---|---|---|---|
| `dnie:` | `SignatureDNIeFilter` | Ninguno (ignora lo posterior) | Exige que sea el certificado de firma del DNIe. |
| `authcert:` | `AuthCertificateFilter` | Ninguno | Muestra todos los certificados excepto el de firma del DNIe. |
| `signingcert:` | `SigningCertificateFilter` | Ninguno | Muestra todos los certificados excepto el de autenticación del DNIe. |
| `nonexpired:` | `ExpiredCertificateFilter` | Opcional booleano | Filtra certificados según su fecha de expiración. |
| `ssl:<sn>` | `SSLFilter` | Número de serie en hex | Filtra por número de serie. Si es el de autenticación del DNIe, busca la pareja de firma. |
| `qualified:<sn>` | `QualifiedCertificatesFilter` | Número de serie en hex | Filtra por número de serie buscando el certificado de firma emparejado. |
| `sscd:` | `SscdFilter` | Ninguno | Exige declaración de dispositivo cualificado seguro (QcSSCD). |
| `subject.rfc2254:<expr>` | `RFC2254CertificateFilter` | Expresión LDAP | Filtra el titular (*Subject*) mediante sintaxis RFC 2254. |
| `issuer.rfc2254:<expr>` | `RFC2254CertificateFilter` | Expresión LDAP | Filtra el emisor (*Issuer*) mediante sintaxis RFC 2254. |
| `issuer.rfc2254.recurse:<expr>` | `RFC2254CertificateFilter` | Expresión LDAP | Filtra recursivamente en toda la cadena de certificación de emisores. |
| `subject.contains:<txt>` | `TextContainedCertificateFilter` | Texto a buscar | Busca subcadena en el DN del titular (insensible a mayúsculas). |
| `issuer.contains:<txt>` | `TextContainedCertificateFilter` | Texto a buscar | Busca subcadena en el DN del emisor (insensible a mayúsculas). |
| `thumbprint:<alg>:<hash>` | `ThumbPrintCertificateFilter` | Algoritmo y huella hex | Compara la huella digital (SHA-1, SHA-256, etc.) del certificado. |
| `policyid:<oid1,oid2>` | `PolicyIdFilter` | OIDs separados por coma | Exige que todas las políticas del certificado estén en la lista de OIDs. |
| `pseudonym:` / `pseudonym:<modo>` | `PseudonymFilter` | `"only"` o `"andothers"` | Filtra certificados con extensión de seudónimo. |
| `encodedcert:<b64>` | `EncodedCertificateFilter` | Certificado en Base64 | Exige correspondencia binaria exacta con el certificado Base64 dado. |
| `keyusage.<uso>:<val>` | `KeyUsageFilter` | Valor booleano o `"null"` | Filtra por bits específicos de la extensión *KeyUsage*. |

---

### 7.1 Filtros específicos de DNI electrónico

#### A. `dnie:` (`SignatureDNIeFilter.java`)
* **Ubicación**: `afirma-keystores-filters/src/main/java/es/gob/afirma/keystores/filters/SignatureDNIeFilter.java`.
* **Criterio**: comprueba conjuntamente dos condiciones:
  1. Emisor RFC 2254: `(&(cn=AC DNIE *)(ou=DNIE)(o=DIRECCION GENERAL DE LA POLICIA)(c=ES))`.
  2. Uso de clave: `KeyUsageFilter.SIGN_CERT_USAGE` (requiere el bit `nonRepudiation = true`).
* **Particularidad**: en `CertFilterManager.java:194`, la condición es `filter.toLowerCase().startsWith("dnie:")`. Todo texto que siga a los dos puntos es ignorado.

#### B. `authcert:` (`AuthCertificateFilter.java`)
* **Ubicación**: `afirma-keystores-filters/.../AuthCertificateFilter.java:21-38`.
* **Comportamiento real y diseño intencionado**: el Javadoc de la clase documenta expresamente su propósito:
  > *«Filtro que muestra todos los certificados salvo el de firma del DNIe. Esto no se realiza mediante KeyUsage, se muestran todos los certificados con clave privada disponibles en el almacén y se retiran los de firma del DNIe.»*
* **Evaluación técnica**: su método `matches(cert)` implementa literalmente:
  ```java
  return !this.signatureDnieCertFilter.matches(cert);
  ```
  **No comprueba que el certificado posea capacidades de autenticación ni examina sus extensiones de uso de clave**.
  Se limita a excluir el certificado de firma del DNIe (`SignatureDNIeFilter`), permitiendo el paso de cualquier
  otro certificado presente en el almacén (persona física, persona jurídica, empleado público o certificados sin
  uso específico). Si una aplicación web requiere autenticación estricta con verificación de usos de clave, debe
  utilizar explícitamente `keyusage.digitalsignature:true`.

#### C. `signingcert:` (`SigningCertificateFilter.java`)
* **Ubicación**: `afirma-keystores-filters/.../SigningCertificateFilter.java:23-40`.
* **Comportamiento real y diseño intencionado**: el Javadoc de la clase describe su motivación de diseño:
  > *«Filtro que muestra únicamente los certificados preparados para firma. Esto no se realiza mediante KeyUsage, sino que se muestran todos los certificados con clave privada disponibles en el almacén y retirar aquellos certificados que se conoce que no son específicos para firma, como el certificado de autenticación del DNIe, por ejemplo.»*
* **Evaluación técnica**: su método `matches(cert)` implementa literalmente:
  ```java
  return !this.authenticationDnieCertFilter.matches(cert);
  ```
  **No comprueba que el certificado posea capacidades de firma ni examina su KeyUsage**. Excluye de forma
  exclusiva el certificado de autenticación del DNIe (`AuthenticationDNIeFilter`), dejando pasar cualquier otro
  certificado del almacén del usuario, con independencia de que carezca del bit de no repudio (`nonRepudiation`).
  Para imponer un filtrado riguroso de certificados cualificados para firma electrónica según el estándar X.509,
  el integrador debe declarar obligatoriamente `keyusage.nonrepudiation:true`.

---

### 7.2 Filtro de vigencia temporal (`nonexpired:`)

* **Ubicación**: `ExpiredCertificateFilter.java:21-52`.
* **Sintaxis**:
  * `nonexpired:` (sin valor) -> equivale a `nonexpired:true`.
  * `nonexpired:true` -> `showExpired = false` (solo certificados vigentes; oculta caducados).
  * `nonexpired:false` -> `showExpired = true` (permite certificados caducados).
* **Evaluación**: invoca `cert.checkValidity(new Date())`. Si lanza `CertificateExpiredException`
  o `CertificateNotYetValidException`, el certificado se rechaza cuando `showExpired == false`.

---

### 7.3 Filtros por número de serie y emparejamiento

#### A. `ssl:<serialNumberHex>` (`SSLFilter.java`)
* **Ubicación**: `afirma-keystores-filters/.../SSLFilter.java:26-150`.
* **Propósito**: diseñado para trámites web donde la persona usuaria se ha autenticado
  previamente en el servidor web mediante mTLS (SSL cliente) con un certificado y,
  posteriormente, la aplicación web solicita una firma enviando el número de serie
  observado en la conexión TLS.
* **Normalización del número de serie**:
  `prepareSerialNumber` elimina espacios, caracteres `#` y ceros (`'0'`) iniciales a la izquierda.
* **Búsqueda de pareja DNIe**:
  Si el certificado que coincide con el número de serie es el certificado de
  autenticación del DNIe, `SSLFilter` busca automáticamente en el almacén el
  certificado de **firma** emparejado (`getAssociatedCertAlias`, líneas 99-115),
  comprobando que:
  1. Sea un certificado de firma de DNIe (`isSignatureDnieCert(tempCert)`).
  2. Tenga el mismo número de serie de sujeto en el DN (`FilterUtils.getSubjectSN`).
  3. Tenga la misma fecha de caducidad en formato `yyyy-MM-dd`.

#### B. `qualified:<serialNumberHex>` (`QualifiedCertificatesFilter.java`)
* **Ubicación**: `afirma-keystores-filters/.../QualifiedCertificatesFilter.java:22-115`.
* **Semántica y discrepancia nominal**: a pesar del prefijo `qualified:`, **no comprueba la cualificación jurídica eIDAS**
  del certificado ni inspecciona declaraciones de cualificación (para verificar que un certificado reside en un
  dispositivo cualificado seguro debe emplearse el filtro `sscd:`, ver §7.4). El filtro opera exclusivamente como
  un localizador por número de serie con resolución automática de pareja de firma.
* **Lógica de evaluación en `matches(aliases, ksm)`**:
  1. Localiza el certificado cuyo número de serie hexadecimal coincide con el parámetro (tras normalizar espacios y ceros a la izquierda).
  2. Si dicho certificado ya posee uso de firma (`isSignatureCert(cert)`, que comprueba `nonRepudiation` salvo la regla de compatibilidad ACCV-CA2), se añade su alias a la lista de admitidos.
  3. Si no posee uso de firma, invoca `searchQualifiedSignatureCertificate` para buscar en el almacén un certificado pareja que cumpla conjuntamente:
     - Mismo emisor (`cert.getIssuerDN()`).
     - Mismo número de serie de titular (`FilterUtils.getSubjectSN(cert)`, atributo `2.5.4.5` / `serialNumber` del DN).
     - Misma fecha de caducidad formateada como `yyyy-MM-dd`.
     - `isSignatureCert(cert2) == true`.
  4. Si localiza la pareja de firma, devuelve el alias de dicho segundo certificado.
  5. Si no localiza ninguna pareja válida en el almacén: devuelve el alias del certificado original del número de serie, **salvo** si se trata del certificado de autenticación del DNIe (`!new AuthenticationDNIeFilter().matches(cert)`), en cuyo caso se descarta para impedir que el usuario firme inadvertidamente con la clave de autenticación.

---

### 7.4 Filtro por dispositivo cualificado seguro (`sscd:`)

* **Ubicación**: `afirma-keystores-filters/.../rfc/SscdFilter.java:31-69`.
* **Criterio**: inspecciona la extensión de declaraciones de cualificación del certificado
  `qcStatements` (OID `1.3.6.1.5.5.7.1.3`) según el estándar ETSI TS 101 862 / RFC 3739.
* **Evaluación**: parsea la secuencia ASN.1 y comprueba si alguna de las declaraciones
  contiene el identificador `id-etsi-qcs-QcSSCD` (OID `0.4.0.1862.1.4`), que certifica
  que la clave privada reside en un dispositivo cualificado de creación de firma (tarjeta
  criptográfica o token hardware seguro).

---

### 7.5 Filtros LDAP / RFC 2254 (`subject.rfc2254:`, `issuer.rfc2254:`)

* **Ubicación**: `afirma-keystores-filters/.../rfc/RFC2254CertificateFilter.java:28-150`.
* **Sintaxis**:
  * `subject.rfc2254:<expresion_ldap>`: evalúa el Distinguished Name del titular.
  * `issuer.rfc2254:<expresion_ldap>`: evalúa el DN del emisor.
  * `issuer.rfc2254.recurse:<expresion_ldap>`: aplica la expresión sobre el emisor
    y, recursivamente, sobre todos los certificados intermedios y raíz de la cadena
    de certificación (`ksm.getCertificateChain(alias)`).
* **Mecanismo de evaluación**:
  1. Convierte el nombre X.500 del certificado en un `javax.naming.ldap.LdapName`.
  2. Extrae cada RDN (`Rdn.getType()`, `Rdn.getValue()`) y los vuelca en un objeto
     `BasicAttributes`.
  3. Ejecuta el evaluador LDAP `SearchFilter` (`afirma-keystores-filters/.../rfc/SearchFilter.java`).
* **Capacidades de la expresión**:
  * Operadores lógicos: `&` (AND), `|` (OR), `!` (NOT).
  * Comparadores: `=` (igualdad / comodín), `~=` (aproximación), `<=` (menor o igual), `>=` (mayor o igual).
  * Comodines: asterisco `*` para prefijos, sufijos o subcadenas (ej. `(cn=*DNIE*)`, `(O=*POLICIA*)`).
  * Atributos comunes: `cn`, `sn`, `c`, `l`, `st`, `o`, `ou`, `title`, `serialnumber`, `mail`.
* **Ejemplo práctico**:
  `filter=subject.rfc2254:(&(cn=*Perez*)(c=ES)(!(ou=Pruebas)))`
* **Comportamiento ante errores y política de apertura (*fail-open*)**:
  En `RFC2254CertificateFilter.java:132-138` (conversión a `LdapName`) y líneas `170-176` (evaluación con `SearchFilter`),
  cualquier excepción originada por sintaxis LDAP malformada (paréntesis desbalanceados, operadores ilegales o atributos no reconocidos)
  o por imposibilidad de parsear el DN del certificado es interceptada:
  ```java
  catch (final Exception e) {
      LOGGER.log(
          Level.WARNING,
          "No ha sido posible filtrar el certificado (filtro: '" + f + "', nombre: '" + name + "'), no se eliminara del listado: " + e,
          e
      );
      return true;
  }
  ```
  El diseño de AutoFirma opta conscientemente por no eliminar el certificado de la lista ante errores de parseo (`return true`).
  Como consecuencia directa, si la sede electrónica comete un error de sintaxis en su expresión RFC 2254, el filtro
  falla en modo abierto (*fail-open*): **no descarta ningún certificado** y todos los certificados disponibles con clave
  privada en el almacén del usuario se muestran en el diálogo de selección.

---

### 7.6 Filtros de texto contenido (`subject.contains:`, `issuer.contains:`)

* **Ubicación**: `afirma-keystores-filters/.../TextContainedCertificateFilter.java:19-58`.
* **Sintaxis**:
  * `subject.contains:<texto>`
  * `issuer.contains:<texto>`
* **Evaluación**: obtiene la cadena completa del `X500Principal` en minúsculas y comprueba
  mediante `String.contains(fragmento.toLowerCase())` si la subcadena está presente.
  No requiere conocer la estructura de atributos LDAP ni escapar comas.

---

### 7.7 Filtro por huella digital (`thumbprint:`)

* **Ubicación**: `afirma-keystores-filters/.../ThumbPrintCertificateFilter.java:25-68`.
* **Sintaxis**:
  `thumbprint:<algoritmo>:<hash_hexadecimal>`
* **Parámetros**:
  * `<algoritmo>`: algoritmo de *digest* compatible con Java Cryptography Architecture
    (típicamente `SHA-1`, `SHA-256`, `SHA-384`, `SHA-512` o `MD5`).
  * `<hash_hexadecimal>`: huella del certificado en hexadecimal. Se eliminan los espacios
    en blanco automáticamente.
* **Evaluación**: calcula `MessageDigest.getInstance(algoritmo).digest(cert.getEncoded())`
  y compara el resultado en hexadecimal ignorando mayúsculas/minúsculas.
* **Particularidad de sintaxis**: en `CertFilterManager.java:240-243`, el valor se divide
  haciendo `split(":")`. Si no contiene exactamente dos fragmentos tras `thumbprint:`,
  el filtro se descarta silenciosamente.

---

### 7.8 Filtro por identificador de política (`policyid:`)

* **Ubicación**: `afirma-keystores-filters/.../PolicyIdFilter.java:24-90`; `CertFilterManager.java:245-250`.
* **Sintaxis**:
  `policyid:<oid1>,<oid2>,...,<oidN>`
* **Mecanismo de parseo**: en `CertFilterManager.java:248`, el parámetro que sigue a `policyid:` se divide
  por comas mediante `.split(",")` y se entrega como `List<String>` al constructor `PolicyIdFilter(allowedOids)`.
* **Diseño e intención declarada en Javadoc**:
  > *«Filtro de certificados por identificador de política de certificación. Si un certificado tiene varias políticas declaradas, todas deben estar dentro de la lista de políticas aceptadas.»*
* **Evaluación técnica**:
  1. Extrae la extensión X.509 `2.5.29.32` (`CertificatePolicies`) del certificado (`getCertificatePolicyIds`).
  2. Parsea la secuencia ASN.1 con SpongyCastle (`CertificatePolicies`, `PolicyInformation`) y extrae todos los OIDs presentes en la lista `actualPolicies`.
  3. Ejecuta la validación de inclusión estricta:
     ```java
     for (final String oid : actualPolicies) {
         if (!this.allowedOids.contains(oid)) {
             return false;
         }
     }
     return true;
     ```
* **Consecuencia funcional**: la comprobación exige que el conjunto de políticas del certificado sea un
  **subconjunto estricto** de las políticas permitidas (`actualPolicies ⊆ allowedOids`). No evalúa intersección
  (es decir, no comprueba si el certificado contiene *al menos una* de las políticas requeridas). Si un certificado
  incorpora múltiples OIDs de política en su extensión `2.5.29.32` (situación común en prestadores cualificados
  como FNMT o Camerfirma, que declaran simultáneamente el OID general de la jerarquía y el OID específico de perfil
  de empleado público o persona física), enviar un único OID provocará el descarte sistemático del certificado.
  Para que un certificado multiconfiguración supere el filtro, la sede electrónica debe listar **todos** los OIDs
  declarados en el certificado dentro del parámetro `policyid:`.

---

### 7.9 Filtro de seudónimo (`pseudonym:`)

* **Ubicación**: `afirma-keystores-filters/.../PseudonymFilter.java:30-151`.
* **Detección**: `AOUtil.isPseudonymCert` (`afirma-core/.../AOUtil.java:315-319`) comprueba
  si el titular contiene el RDN `2.5.4.65` (atributo `pseudonym`).
* **Sintaxis y modos**:
  * `pseudonym:` o `pseudonym:andothers`: muestra los certificados de seudónimo y aquellos
    certificados normales que **no** tengan un certificado de seudónimo equivalente en el
    almacén. Se consideran equivalentes si comparten emisor, mismo `KeyUsage` y su fecha
    de expiración difiere en menos de 60 segundos (`isPseudonymFor`, líneas 142-150).
  * `pseudonym:only`: muestra única y exclusivamente certificados que posean el RDN
    de seudónimo.

---

### 7.10 Filtro por certificado codificado (`encodedcert:`)

* **Ubicación**: `afirma-keystores-filters/.../EncodedCertificateFilter.java:23-48`.
* **Sintaxis**:
  `encodedcert:<certificado_x509_en_base64>`
* **Evaluación**: compara la codificación DER del certificado (`Base64.encode(cert.getEncoded())`)
  contra la cadena Base64 proporcionada.
* **Uso primordial**: utilizado por `autoscript.js` para preseleccionar de forma determinista
  el certificado del usuario tras haber realizado una primera consulta o cuando se opera
  con certificados seleccionados previamente.

---

### 7.11 Filtros por usos de clave (`keyusage.*`)

* **Ubicación**: `afirma-keystores-filters/.../rfc/KeyUsageFilter.java:18-64` y
  `KeyUsagesPattern.java:23-64`.
* **Sintaxis**:
  `keyusage.<nombre_uso>:<true|false|null>`
* **Catálogo de usos y posiciones de bit (RFC 5280 §4.2.1.3)**:

| Nombre en protocolo | Bit | Constante Java | Descripción |
|---|---|---|---|
| `keyusage.digitalsignature` | 0 | `digitalSignature` | Verificación de firmas digitales distintas de no repudio. |
| `keyusage.nonrepudiation` | 1 | `nonRepudiation` / `contentCommitment` | Firma de compromiso / no repudio legal. |
| `keyusage.keyencipherment` | 2 | `keyEncipherment` | Cifrado de claves simétricas de sesión. |
| `keyusage.dataencipherment` | 3 | `dataEncipherment` | Cifrado directo de datos de usuario. |
| `keyusage.keyagreement` | 4 | `keyAgreement` | Intercambio de claves (Diffie-Hellman). |
| `keyusage.keycertsign` | 5 | `keyCertSign` | Verificación de firmas de certificados (CAs). |
| `keyusage.crlsign` | 6 | `cRLSign` | Verificación de listas de revocación CRL. |
| `keyusage.encipheronly` | 7 | `encipherOnly` | Solo cifrado con `keyAgreement`. |
| `keyusage.decipheronly` | 8 | `decipherOnly` | Solo descifrado con `keyAgreement`. |

* **Valores admitidos**:
  * `true`: el bit debe estar activo en el certificado.
  * `false`: el bit debe estar inactivo.
  * `null` (o `"null"`): el bit no se evalúa (comodín).
* **Agrupación en `CertFilterManager`**:
  En `CertFilterManager.java:190-192`, los filtros separados por `;` se ordenan
  alfabéticamente (`Arrays.sort`). Esto garantiza que todas las declaraciones
  `keyusage.*` queden consecutivas. El método `generateKeyUsageFiltersPattern` (líneas 277-295)
  las fusiona en una única máscara `Boolean[9]` y genera un único objeto `KeyUsageFilter`.
* **Regla especial para ACCV-CA2**:
  La clase `KeyUsagesPattern` define una excepción para los certificados emitidos por
  `C=ES, O=Generalitat Valenciana, OU=PKIGVA, CN=ACCV-CA2`. Debido a que esta CA emitió
  históricamente certificados de firma cualificados con únicamente el bit `digitalSignature`
  activo (omitiendo `nonRepudiation`), AutoFirma ajusta el patrón para permitir su uso
  como certificados de firma.

---

## 8. Ejemplos de uso combinados

### Ejemplo 1: Firma CAdES con política AGE v1.9 y filtro de certificado no caducado
```properties
expPolicy=FirmaAGE
filter=nonexpired:true;keyusage.nonrepudiation:true
```
* **Expansión**: `expPolicy=FirmaAGE` inyecta el OID `urn:oid:2.16.724.1.3.1.1.2.1.9`,
  el hash de la política CAdES y la URL del PDF, forzando `profile=advanced`. Si el
  documento es menor de 1 MB, fija `mode=implicit`.
* **Filtrado**: exige que el certificado esté vigente y contenga el bit `nonRepudiation`.

### Ejemplo 2: Filtrado por DNI específico y DNIe con selección desatendida
```properties
headless=true
filter=subject.rfc2254:(serialnumber=99999999R);dnie:
```
* Exige que el certificado sea de firma del DNIe y pertenezca al titular con NIF `99999999R`.
* Al ser `headless=true`, si solo existe un DNIe conectado con ese NIF, la firma se
  realiza automáticamente sin mostrar diálogo Swing de selección.

### Ejemplo 3: Disyunción externa con certificados alternativos
```properties
filters.1=dnie:
filters.2=issuer.contains:FNMT;keyusage.nonrepudiation:true
```
* Se evalúa `filters.1`: si el usuario tiene el DNIe insertado, se admiten sus certificados.
* Se evalúa `filters.2`: sobre los certificados restantes, se admiten aquellos emitidos
  por la FNMT que tengan uso de no repudio.



