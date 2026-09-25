# 09. Operación `selectcert`

Este capítulo describe la operación `selectcert` del protocolo `afirma://` de
AutoFirma 1.9.2. Esta operación permite a una aplicación web solicitar la
selección interactiva (o automática por filtrado) de un certificado electrónico
del usuario entre los disponibles en sus almacenes locales o dispositivos criptográficos
(DNIe, tarjetas inteligentes, tokens PKCS#11, almacenes del sistema operativo o ficheros
PKCS#12), obteniendo de vuelta el certificado seleccionado (en formato X.509 DER codificado
en Base64 y opcionalmente cifrado con clave simétrica) sin realizar ninguna operación
criptográfica de firma electrónica. Asimismo, se documenta la integración de esta operación
con el mecanismo de certificado fijado (*sticky signature*), el procesamiento de filtros,
el comportamiento en los tres transportes disponibles y su tratamiento en el cliente
JavaScript de referencia (`autoscript.js`).

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. Visión general y ciclo de vida de `selectcert`

El propósito principal de `selectcert` es permitir que la aplicación web invocadora obtenga
la identidad digital de la persona usuaria antes o al margen de un proceso de firma
(por ejemplo, para autenticación web previa, comprobación de autorizaciones o registro),
o bien para fijar de forma anticipada el certificado que se utilizará en operaciones
posteriores mediante el mecanismo de clave pegajosa (*sticky*).

A diferencia de las operaciones de firma (`sign`, `cosign`, `countersign`, `signandsave`,
`batch`), `selectcert` **no genera ninguna firma electrónica** ni requiere datos de entrada
a firmar. Sin embargo, AutoFirma exige de forma estricta que la entrada elegida en el almacén
contenga una clave privada asociada (`PrivateKeyEntry`), por lo que no permite seleccionar
certificados públicos huérfanos o meros certificados de confianza del sistema.

### 1.1 Diagrama del ciclo de vida

```
           ┌─────────────────────────────────────────────────────────────┐
           │ Invocación: afirma://selectcert?id=...&key=...              │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 1. ¿Configuración remota?                                   │
           │    - Si hay fileid: descarga XML desde rtservlet y parsea   │
           │    - Si no: procesa parámetros de la URL directamente       │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 2. ¿Reutilización de certificado fijado (sticky)?           │
           │    - Si sticky && !resetSticky && stickyKeyEntry != null:   │
           │      reutiliza la PrivateKeyEntry en memoria                │
           │    - En caso contrario: resuelve almacén y filtros          │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 3. Selección interactiva o automática (AOKeyStoreDialog)    │
           │    - Aplica filtros de CertFilterManager (caducidad, etc.)  │
           │    - Si mandatoryCertificate && 1 cert: selección auto      │
           │    - Si no: muestra diálogo modal Swing al usuario          │
           │    - Usuario cancela: devuelve "CANCEL"                     │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 4. Extracción de certificado y actualización sticky         │
           │    - Obtiene certEncoded: pke.getCertificateChain()[0]      │
           │    - Si sticky: almacena pke en stickyKeyEntry              │
           │    - Si !sticky: borra stickyKeyEntry (pasa a null)         │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 5. Codificación y cifrado opcional                          │
           │    - Si hay key (8 bytes): cifra DES CypherDataManager      │
           │      formato: <padding>.<Base64_URL_Safe>                   │
           │    - Si no hay key: Base64 URL-Safe de certEncoded          │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 6. Devolución de la respuesta al llamante                   │
           │    - Servidor intermedio: POST a stservlet + interrumpe aw  │
           │    - Socket local: retorno directo por SSLSocket            │
           │    - WebSocket: mensaje directo a la sesión activa          │
           └─────────────────────────────────────────────────────────────┘
```

### 1.2 Prefijos de URI reconocidos

El despachador central `ProtocolInvocationLauncher.launch` intercepta la operación en la
línea 370 de
`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncher.java`:

```java
else if (urlString.startsWith("afirma://selectcert?") || urlString.startsWith("afirma://selectcert/?")) {
```

Al igual que en las demás operaciones del protocolo, se admiten exactamente dos variantes
de prefijo:
1. `afirma://selectcert?` (forma estándar generada por `autoscript.js`).
2. `afirma://selectcert/?` (variante con barra inclinada previa al signo de interrogación).

### 1.3 Diferencias estructurales frente a `sign`

| Característica | Operación `sign` / `signandsave` | Operación `selectcert` | Cita en código |
|---|---|---|---|
| Clase despachadora | `ProtocolInvocationLauncherSign` / `ProtocolInvocationLauncherSignAndSave` | `ProtocolInvocationLauncherSelectCert` | `ProtocolInvocationLauncher.java:415, 582, 690` |
| Clase de parámetros | `UrlParametersToSign` / `UrlParametersToSignAndSave` | `UrlParametersToSelectCert` | `ProtocolInvocationUriParserUtil.java:171-177` |
| Datos a firmar (`dat`) | Obligatorios (o solicitados mediante diálogo en disco) | Inutilizados en la operación; si se envían erróneamente, `setCommonParameters` los descarga en memoria pero `processSelectCert` los ignora | `UrlParametersToSelectCert.java:69`, `UrlParameters.java:306-314` |
| Formato y algoritmo (`format`, `algorithm`) | Obligatorios para la generación de firma | Inexistentes; no se leen ni se interpretan | `UrlParametersToSelectCert.java:121-221` |
| Expansión de propiedades (`ExtraParamsProcessor`) | Se invoca `expandProperties()` para políticas y formatos | **No se invoca**: las propiedades se parsean como `Properties` crudo; no aplican políticas de firma al carecer de formato y datos | `UrlParametersToSelectCert.java:191` |
| Diálogo de firma visible (PDF) | Se activa si el formato es PAdES y se solicita rúbrica | Inexistente: no aplica | `ProtocolInvocationLauncherSelectCert.java:134-227` |
| Comprobación de clave privada | Requiere clave privada para firmar | **Requiere clave privada** (`checkPrivateKeys=true`), solicitando PIN en tarjetas criptográficas para validar la entrada y permitir su almacenamiento en `sticky` | `ProtocolInvocationLauncherSelectCert.java:178, 193` |
| Formato de la respuesta | Compuesto: `certEncoded \| signature [ \| extraData ]` | Simple: únicamente el certificado (cifrado o codificado) | `ProtocolInvocationLauncherSelectCert.java:233, 248, 262` |

---

## 2. Parámetros de la invocación `selectcert`

El análisis de parámetros se realiza en dos fases sucesivas dentro de
`ProtocolInvocationUriParserUtil.getParametersToSelectCert`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/ProtocolInvocationUriParserUtil.java:171-177`):
primero se invocan los métodos comunes de la clase base `UrlParameters.setCommonParameters`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java:253-321`), y
posteriormente los específicos en `UrlParametersToSelectCert.setSelectCertParameters`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSelectCert.java:121-221`).

### 2.1 Catálogo completo de parámetros reconocidos

| Parámetro | Tipo | Obligatorio | Valor por defecto | Descripción | Cita en código |
|---|---|---|---|---|---|
| `id` | Alfanumérico | Condicional | `null` | Identificador de la sesión en el servidor intermedio. Obligatorio si se usa servidor intermedio sin `fileid`. Longitud máxima 20 caracteres `[a-zA-Z0-9]`. | `UrlParametersToSelectCert.java:125, 134, 139` |
| `fileid` | Alfanumérico | Condicional | `null` | Identificador del fichero remoto en el servidor intermedio que contiene la configuración XML de la operación. Mismas reglas de validación que `id`. | `UrlParametersToSelectCert.java:128`, `UrlParameters.java:271` |
| `ver` | Numérico | No | `"0"` | Versión del protocolo requerida por la invocación. Si no se especifica, toma el valor `"0"` (`ProtocolVersion.VERSION_0`). | `UrlParametersToSelectCert.java:148-153` |
| `mcv` | Cadena | No | `null` | Versión mínima de la aplicación AutoFirma requerida (*Minimum Client Version*). Si la instalada es inferior, aborta con `SAF_41`. | `UrlParameters.java:260-262`, `ProtocolInvocationLauncherSelectCert.java:89-100` |
| `key` | Cadena (8 bytes) | No | `null` | Clave simétrica para el cifrado DES de la respuesta hacia el servidor intermedio. Debe tener exactamente 8 caracteres. | `UrlParameters.java:53, 255, 327-345` |
| `stservlet` | URL HTTP/HTTPS | Condicional | `null` | URL del servlet de almacenamiento temporal del resultado (`StorageService`). Obligatorio en transporte por servidor intermedio. | `UrlParametersToSelectCert.java:163-181`, `UrlParameters.java:351-380` |
| `rtservlet` | URL HTTP/HTTPS | Condicional | `null` | URL del servlet de recuperación remota (`RetrieveService`). Obligatorio si se proporciona `fileid`. | `UrlParameters.java:273-296` |
| `aw` | Booleano | No | `false` | Indicador de espera activa (*active waiting*). Si es `true` y se usa servidor intermedio, emite avisos periódicos de presencia. | `UrlParameters.java:257-258`, `ProtocolInvocationLauncher.java:407-409` |
| `properties` | Base64 | No | `null` | Conjunto de propiedades adicionales de configuración en formato clave-valor de Java Properties codificadas en Base64. Contiene los filtros de certificado y banderas como `headless`. | `UrlParametersToSelectCert.java:184-202` |
| `sticky` | Booleano | No | `false` | Si es `true`, conserva la `PrivateKeyEntry` seleccionada en memoria estática para operaciones consecutivas. Si es `false`, borra cualquier clave fijada previa. | `UrlParametersToSelectCert.java:205-209`, `ProtocolInvocationLauncherSelectCert.java:195-199` |
| `resetsticky` | Booleano | No | `false` | Si es `true`, ignora cualquier clave privada previamente fijada en memoria y fuerza la presentación del diálogo de selección. | `UrlParametersToSelectCert.java:212-217`, `ProtocolInvocationLauncherSelectCert.java:139` |
| `ksb64` | Base64 | No | `null` | Nombre del almacén de claves sugerido por la aplicación web, codificado en Base64. Opcionalmente admite la sintaxis `ALMACEN:LIB`. | `UrlParameters.java:66, 389-423` |
| `keystore` | Cadena | No | `null` | Nombre del almacén de claves en texto plano (obsoleto; mantenido por compatibilidad histórica con AutoFirma 1.4.x). | `UrlParameters.java:63, 386-388` |
| `jvc` | Numérico | No | `1` | Versión del código JavaScript llamante (`autoscript.js`). Se evalúa en el despachador general; si es inferior a 1 emite un diálogo de advertencia. | `ProtocolInvocationLauncher.java:196-214` |

### 2.2 Validación y restricciones sintácticas

1. **Identificador de sesión (`id` o `fileid`)**:
   En `UrlParametersToSelectCert.java:133-143`, si está presente cualquiera de los dos parámetros, se comprueba:
   - Que su longitud no exceda los 20 caracteres (`MAX_ID_LENGTH = 20`). Si la supera, se lanza `ParameterException("La longitud del identificador de la operacion es mayor de 20 caracteres.")`.
   - Que todos sus caracteres pertenezcan al rango alfanumérico ASCII (`c >= 'a' && c <= 'z'` o `c >= '0' && c <= '9'`, tras convertir a minúsculas en locale inglés). Si contiene caracteres no permitidos (guiones, barras, signos de puntuación), lanza `ParameterException("El identificador de la sesion debe ser alfanumerico.")`.
2. **Clave de cifrado (`key`)**:
   En `UrlParameters.java:341-343`, si se incluye el parámetro `key`, su longitud debe ser estrictamente de 8 caracteres (`CIPHER_KEY_LENGTH = 8`). Una longitud distinta produce `ParameterException("La longitud de la clave de cifrado no es correcta")`. Si el parámetro está presente pero vacío (`key=`), se interpreta como ausencia de cifrado (`null`).
3. **Validación de URLs de servlets (`stservlet` y `rtservlet`)**:
   En `UrlParameters.java:351-380`, cada URL de servlet:
   - Se decodifica mediante `URLDecoder.decode(url, "UTF-8")`.
   - Solo admite esquemas `http` y `https` (rechaza `file:`, `ftp:`, etc.).
   - Bloquea explícitamente el acceso a hosts locales: si el host es `"localhost"` o `"127.0.0.1"`, lanza `ParameterLocalAccessRequestedException("El host de la URL proporcionada para el Servlet es local")`.
   - No puede contener parámetros de consulta: si la cadena incluye `'?'` o `'='`, lanza `ParameterException("Se han encontrado parametros en la URL del servlet")`.
4. **Obligatoriedad condicional de servlets**:
   El constructor `UrlParametersToSelectCert(servicesRequired)` recibe un booleano que indica si la invocación exige servicios de red remotos (`!bySocket`, es decir, servidor intermedio). Si `servicesRequired == true`:
   - Si no se encuentra `stservlet` a pesar de haberse proporcionado `id`, lanza `ParameterException("No se ha recibido la direccion del servlet para el guardado del resultado de la operacion")` (`UrlParametersToSelectCert.java:179-181`).
   - Cuando se invoca vía socket local o WebSocket (`bySocket == true`), `servicesRequired` es `false`, por lo que `stservlet` es totalmente opcional y puede omitirse.

### 2.3 Tratamiento de datos (`dat`), configuración remota (`fileid`) y parámetros ignorados

En el constructor de `UrlParametersToSelectCert` (`UrlParametersToSelectCert.java:68-72`), el código incluye las siguientes inicializaciones explícitas:

```java
public UrlParametersToSelectCert(final boolean servicesRequired) {
    this.servicesRequired = servicesRequired;
    setData(null);
    setFileId(null);
    setRetrieveServletUrl(null);
}
```

Sin embargo, el orden de ensamblado de parámetros en `ProtocolInvocationUriParserUtil.getParametersToSelectCert` (`ProtocolInvocationUriParserUtil.java:171-177`) determina un comportamiento crítico respecto a estas variables:

```java
final UrlParametersToSelectCert ret = new UrlParametersToSelectCert(servicesRequired);
ret.setCommonParameters(params);
ret.setSelectCertParameters(params);
return ret;
```

1. **Configuración remota mediante `fileid` plenamente operativa**:
   `setFileId(null)` en el constructor no anula el parámetro `fileid`. Inmediatamente después, `ret.setCommonParameters(params)` (`UrlParameters.java:269-272`) lee `fileid` y `rtservlet` (`retrieveServletUrl`), asignándolos a la instancia. Esto permite que el despachador general en `ProtocolInvocationLauncher.java:384-403` detecte `if (params.getFileId() != null)` y descargue el XML con la configuración completa de la operación desde el servidor intermedio.
2. **Ingesta y consumo inútil de memoria ante el parámetro `dat`**:
   Las líneas `setData(null)` del constructor se ejecutan **antes** de llamar a `setCommonParameters`. Por tanto:
   - Si una invocación errónea incluye el parámetro `dat` (datos en Base64 o URL remota de descarga), `UrlParameters.java:306-314` ejecuta incondicionalmente `setData(DataDownloader.downloadData(dataPrm, Boolean.parseBoolean(params.get(GZIPPED_DATA_PARAM))))`.
   - Ni `setSelectCertParameters` ni `ProtocolInvocationLauncherSelectCert` vuelven a limpiar `data`, por lo que los bytes descargados o decodificados permanecen retenidos en memoria en la instancia de `UrlParametersToSelectCert` durante toda la vida de la operación, sin que `processSelectCert` llegue a consultarlos jamás.
   - Si la URL proporcionada en `dat` resulta inalcanzable, o si los datos locales apuntan a esquemas no permitidos (`file:/`), `DataDownloader` o `setCommonParameters` arrojan una `ParameterException`, abortando la operación prematuramente con el código de error `SAF_03` (`ERROR_PARAMS`), a pesar de que la selección de certificados no requería datos.
   - Si se proporcionan simultáneamente `dat` y `fileid`, la condición `if (!params.containsKey(DATA_PARAM))` en `UrlParameters.java:267` omite la lectura de `fileid`, impidiendo que `ProtocolInvocationLauncher` descargue el XML de configuración y haciendo fracasar la llamada.
3. **Parámetros de firma ignorados sin efecto**:
   Parámetros propios de la generación de firmas electrónicas tales como `op` (en la query), `cop`, `format`, `algorithm`, `filename`, `signProperties` o `batchpresignerurl` no son leídos por `setSelectCertParameters` ni transferidos al contexto de ejecución, siendo ignorados de forma silente.

---

## 3. Recuperación de parámetros por servidor intermedio (`fileid`)

Cuando la URL de invocación supera los límites seguros del sistema operativo o del navegador (habitualmente 2000 caracteres en entornos Windows), el cliente JavaScript no puede incluir todos los parámetros (especialmente `properties` cuando contiene filtros complejos o certificados codificados). En tal situación, el cliente web sube previamente la configuración en formato XML al servlet intermedio (`StorageService`), generando un identificador temporal `fileid`.

### 3.1 Flujo de descarga del XML de parámetros

En `ProtocolInvocationLauncher.java:384-403`:

```java
if (params.getFileId() != null) {
    final byte[] xmlData;
    try {
        xmlData = ProtocolInvocationLauncherUtil.getDataFromRetrieveServlet(params);
    } catch (final InvalidEncryptedDataLengthException e) {
        ...
        return ProtocolInvocationLauncherErrorManager.getErrorMessage(ProtocolInvocationLauncherErrorManager.ERROR_RECOVERING_DATA); // SAF_16
    } catch (final DecryptionException e) {
        ...
        return ProtocolInvocationLauncherErrorManager.getErrorMessage(ProtocolInvocationLauncherErrorManager.ERROR_DECRYPTING_DATA); // SAF_15
    }

    params = ProtocolInvocationUriParser.getParametersToSelectCert(xmlData, true);
}
```

1. **Descarga y descifrado**: `ProtocolInvocationLauncherUtil.getDataFromRetrieveServlet` invoca al `rtservlet` pasando `op=get`, la versión y el `fileid`. Si los datos venían cifrados con la clave `key`, se descifran con DES CBC/ECB. Un fallo en la longitud del buffer cifrado detona `SAF_16`, y un fallo criptográfico en el descifrado detona `SAF_15`.
2. **Análisis del documento XML**: Los bytes obtenidos se entregan a `ProtocolInvocationUriParser.getParametersToSelectCert(xmlData, true)` (`afirma-core/.../ProtocolInvocationUriParser.java:152-156`), que a su vez delega en `ProtocolInvocationUriParserUtil.parseXml` (`ProtocolInvocationUriParserUtil.java:91-137`).

### 3.2 Estructura del XML de parámetros remotos

El documento XML descargado debe presentar una raíz con elementos hijos `<e k="..." v="..."/>`:

```xml
<selectcert>
    <e k="id" v="12345678901234567890" />
    <e k="ver" v="1" />
    <e k="stservlet" v="https://sede.gob.es/afirma/StorageService" />
    <e k="properties" v="ZmlsdGVycz1leHBpcmVkOmZhbHNl..." />
    <e k="sticky" v="true" />
</selectcert>
```

`ProtocolInvocationUriParserUtil.parseXml` utiliza un parser XML seguro (`SecureXmlBuilder.getSecureDocumentBuilder()`) que deshabilita la resolución de entidades externas (evitando ataques XXE). Cada par clave (`k`) y valor (`v`) se inserta en un mapa `Map<String, String>` que se envía de nuevo a `ProtocolInvocationUriParserUtil.getParametersToSelectCert(map, true)` para reconstruir la instancia definitiva de `UrlParametersToSelectCert`.

---

## 4. Resolución de almacén, filtros y diálogo de selección

El núcleo de la ejecución reside en `ProtocolInvocationLauncherSelectCert.processSelectCert`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherSelectCert.java:60-291`).

### 4.1 Jerarquía de resolución del almacén de claves

AutoFirma determina qué almacén de certificados abrir según el siguiente orden de prelación estricto
(`ProtocolInvocationLauncherSelectCert.java:102-127`):

```
                   ┌──────────────────────────────────────────────────┐
                   │ ¿lastSelectedKeyStore != null?                   │
                   └─────────┬──────────────────────────────┬─────────┘
                             │ Sí                           │ No
                             ▼                              ▼
                 ┌───────────────────────┐      ┌─────────────────────────────┐
                 │ SimpleKeyStoreManager.│      │ ¿useDefaultStore == true?   │
                 │ getLastSelectedKeystore()    └──────┬──────────────────────┬─┘
                 └───────────────────────┘             │ Sí                   │ No
                                                       ▼                      ▼
                                           ┌──────────────────────┐  ┌──────────────────┐
                                           │ PreferencesManager:  │  │ ksb64 / keystore │
                                           │ defaultStore pref    │  │ en la llamada    │
                                           └──────────────────────┘  └────────┬─────────┘
                                                                              │ Si es null
                                                                              ▼
                                                                     ┌──────────────────┐
                                                                     │ Almacén defecto  │
                                                                     │ del SO           │
                                                                     └──────────────────┘
```

1. **Almacén previamente seleccionado en la misma sesión (`lastSelectedKeyStore`)**:
   Obtenido mediante `KeyStorePreferencesManager.getLastSelectedKeystore()`. Aplica cuando se encadenan múltiples llamadas a la aplicación dentro de la misma instancia en ejecución.
2. **Preferencia de almacén por defecto de la aplicación (`useDefaultStore`)**:
   Si la preferencia `useDefaultStoreInBrowserCalls` está habilitada en la configuración de AutoFirma y el almacén configurado es distinto de `"default"`, se utiliza dicho almacén.
3. **Almacén indicado en la invocación (`options.getDefaultKeyStore()`)**:
   Extraído de los parámetros `ksb64` (decodificado de Base64) o `keystore` (`UrlParameters.java:382-423`). Valores reconocidos: `WINDOWS`, `APPLE`, `PKCS11`, `PKCS12`, `MOZILLA`, etc.
4. **Almacén por defecto según el sistema operativo**:
   Si ninguna de las anteriores produce un almacén (`aoks == null`), se invoca `AOKeyStore.getDefaultKeyStoreTypeByOs(Platform.getOS())`:
   - Windows: `AOKeyStore.WINDOWS` (almacén CAPI/CNG del usuario `Windows-MY`).
   - macOS: `AOKeyStore.APPLE` (Keychain de macOS).
   - Linux / otros: `AOKeyStore.MOZILLA` (perfil NSS de Mozilla Firefox).

**Ruta de biblioteca (`aoksLib`)**: Para almacenes que requieren biblioteca nativa o fichero (PKCS#11 o PKCS#12), si `useDefaultStore` está activo se toma de `PreferencesManager.PREFERENCE_LOCAL_KEYSTORE_PATH`; en caso contrario, se toma del parámetro `options.getDefaultKeyStoreLib()` (obtenido tras los dos puntos en `ksb64=ALMACEN:LIB`).

### 4.2 Inicialización del gestor de almacenes (`AOKeyStoreManager`)

Una vez determinado el almacén y su biblioteca, se obtiene el manejador de contraseñas `PasswordCallback pwc = aoks.getStorePasswordCallback(null)` y se solicita la instancia del gestor mediante la factoría:

```java
ksm = AOKeyStoreManagerFactory.getAOKeyStoreManager(
    aoks,       // Store
    aoksLib,    // Lib
    null,       // Description
    pwc,        // PasswordCallback
    null        // Parent
);
```

Si la inicialización falla (por ejemplo, biblioteca PKCS#11 inexistente, almacén dañado o PIN erróneo al inicializar), se captura `Exception` (`ProtocolInvocationLauncherSelectCert.java:157-164`), emitiendo el código de error `SAF_08` (`ERROR_CANNOT_ACCESS_KEYSTORE`).

### 4.3 Gestión de filtros mediante `CertFilterManager`

Las propiedades recibidas en el parámetro `properties` se entregan al gestor de filtros
(`afirma-keystores-filters/.../CertFilterManager.java`):

```java
final CertFilterManager filterManager = new CertFilterManager(options.getExtraParams());
final List<CertificateFilter> filters = filterManager.getFilters();
final boolean mandatoryCertificate = filterManager.isMandatoryCertificate();
```

#### Omisión deliberada de `ExtraParamsProcessor.expandProperties`
En `UrlParametersToSelectCert.java:191`, las propiedades adicionales se cargan mediante `AOUtil.base642Properties(props)` en un objeto `Properties` directo, sin invocar `ExtraParamsProcessor.expandProperties(params, signedData, format)`. A diferencia de las operaciones de firma (`sign`, `signandsave`), donde `expandProperties` expande alias como `expPolicy=FirmaAGE` en los identificadores técnicos de política para CAdES, XAdES o PAdES, en `selectcert` **no existe formato de firma ni datos a firmar**. Si `selectcert` intentase ejecutar `ExtraParamsProcessor.expandProperties`, la ausencia de un formato válido causaría que `expandPolicyKeys` arrojase fatalmente `IncompatiblePolicyException("El formato de firma null no esta soportado por la politica")`. En `selectcert`, el diccionario `properties` transporta exclusivamente directivas de filtrado de certificados (`filter`, `filters`) y de interfaz (`headless`, `mandatoryCertSelection`, `disableopeningexternalstores`), las cuales son procesadas íntegramente por `CertFilterManager`.

#### Reglas de filtrado y autoselección:
* **Filtro automático de caducidad**: Si la lista de filtros resultante está vacía (`filters.isEmpty()`), `CertFilterManager` añade automáticamente un `ExpiredCertificateFilter(false)` conforme a los criterios de la directiva ETSI TS 119 102-1 (`CertFilterManager.java:133-135`), impidiendo que se muestren certificados caducados salvo que el integrador haya definido filtros específicos de forma explícita.
* **Control de selección automática y comportamiento `headless` (`mandatoryCertificate`)**:
  `CertFilterManager.isMandatoryCertificate` (`CertFilterManager.java:145-154`) evalúa dos propiedades:
  ```java
  final boolean headless = propertyFilters != null
          && Boolean.parseBoolean(propertyFilters.getProperty("headless"));
  final boolean omitSelection = propertyFilters != null
          && propertyFilters.containsKey("mandatoryCertSelection")
          && Boolean.FALSE.toString().equalsIgnoreCase(
                  propertyFilters.getProperty("mandatoryCertSelection"));
  return headless || omitSelection;
  ```
  Si se ha establecido `headless=true` o bien `mandatoryCertSelection=false`, `mandatoryCertificate` toma el valor `true`.
* **Bloqueo de almacenes externos**: La presencia de la propiedad `disableopeningexternalstores` inhabilita la capacidad del usuario de abrir ficheros de certificados externos desde la interfaz gráfica (`filterManager.isExternalStoresOpeningAllowed()`).

### 4.4 Presentación del diálogo gráfico (`AOKeyStoreDialog`)

AutoFirma configura y despliega el diálogo gráfico Swing para la selección del certificado:

```java
MacUtils.focusApplication();
final AOKeyStoreDialog dialog = new AOKeyStoreDialog(
    ksm,
    null,
    true,                      // checkPrivateKeys: solo muestra certificados con clave privada
    true,                      // showExpiredCertificates
    true,                      // checkValidity
    filters,
    mandatoryCertificate,
    libName
);
dialog.allowOpenExternalStores(filterManager.isExternalStoresOpeningAllowed());
dialog.show();
```

#### Comportamiento de `dialog.show()` (`AOKeyStoreDialog.java:720-745`):

1. **Selección automática si hay un único candidato**: Si `mandatoryCertificate == true` y la lista de certificados tras aplicar los filtros contiene **exactamente un único certificado** (`namedCertificates.length == 1`), `dialog.show()` selecciona inmediatamente dicho alias y retorna de inmediato sin desplegar ninguna interfaz gráfica (`AOKeyStoreDialog.java:726-729`).
2. **Presencia de múltiples certificados candidatos ante `headless=true`**: Si existen 2 o más certificados válidos que superen los filtros, la directiva `mandatoryCertificate` (activada por `headless=true` o `mandatoryCertSelection=false`) no realiza ninguna autoselección arbitraria. AutoFirma considera que no puede resolver la ambigüedad sin intervención y procede a invocar el diálogo interactivo `AOUIFactory.showCertificateSelectionDialog(...)`:
   - En sistemas con entorno de escritorio gráfico (X11, Wayland, Windows, macOS), se presenta la ventana visual de selección para que el usuario elija su certificado.
   - En entornos estrictamente desatendidos sin servidor de ventanas (servidores headless, pipelines de CI/CD), la llamada de Swing lanza `java.awt.HeadlessException`. Dicha excepción es absorbida por el bloque general `catch (final Exception e)` de `ProtocolInvocationLauncherSelectCert.java:217-225`, que registra `"Error al mostrar el dialogo de seleccion de certificados"` y devuelve el código de error `SAF_08` (`ERROR_CANNOT_ACCESS_KEYSTORE`). Asimismo, `ProtocolInvocationLauncherErrorManager.showError` consulta la propiedad de sistema de la JVM `es.gob.afirma.protocolinvocation.HeadLess` para decidir si suprime la ventana emergente modal de error.
3. **Ausencia de certificados**: Si ningún certificado supera los filtros (`namedCertificates.length == 0` o el diálogo lanza `IllegalStateException`), se lanza `AOCertificatesNotFoundException` (`AOKeyStoreDialog.java:736`), que en `ProtocolInvocationLauncherSelectCert.java:208-216` detona el error `SAF_19` (`ERROR_NO_CERTIFICATES_KEYSTORE`).
4. **Cancelación del usuario**: Si el usuario pulsa el botón «Cancelar» o cierra la ventana, `selectedAlias` es `null`, lanzando `AOCancelledOperationException` (`AOKeyStoreDialog.java:741`). En `ProtocolInvocationLauncherSelectCert.java:201-207`, esto detona la salida especial `"CANCEL"`.

#### Exigencia de clave privada (`PrivateKeyEntry`) y solicitud de PIN:

Tras cerrarse el diálogo con éxito, se recupera el contexto del almacén y se extrae la clave:

```java
final CertificateContext context = dialog.getSelectedCertificateContext();
final KeyStoreManager currentKsm = context.getKeyStoreManager();
pke = currentKsm.getKeyEntry(context.getAlias());
```

Dado que el diálogo fue instanciado con `checkPrivateKeys = true` (`ProtocolInvocationLauncherSelectCert.java:178`), `KeyStoreUtilities.getAliasesByFriendlyName` filtra activamente los alias mediante `ksm.isKeyEntry(al)` (`KeyStoreUtilities.java:248`), ocultando cualquier certificado público que carezca de clave privada en el almacén.

La posterior recuperación de `pke` mediante `currentKsm.getKeyEntry(context.getAlias())` obedece a dos necesidades arquitectónicas del protocolo:
1. Asegurar que la entrada elegida corresponde a una credencial de firma completa y no a un mero certificado público huérfano.
2. Permitir que, cuando la llamada incluye `sticky=true`, la `PrivateKeyEntry` pueda fijarse en la variable estática `ProtocolInvocationLauncher.stickyKeyEntry` (`ProtocolInvocationLauncherSelectCert.java:196`), quedando disponible para que operaciones consecutivas de firma (`sign`, `cosign`, etc.) en la misma sesión la reutilicen directamente sin volver a interactuar con el usuario.

**Impacto en tarjetas inteligentes y tokens criptográficos (DNIe, PKCS#11):**
Al invocar `currentKsm.getKeyEntry(context.getAlias())`, si el almacén o dispositivo hardware subyacente requiere autenticación para acceder a la clave privada o al slot criptográfico, el controlador PKCS#11 o el `PasswordCallback` de AutoFirma **solicita el PIN de la tarjeta al usuario durante la ejecución de `selectcert`**, aun cuando la operación no realiza ninguna firma electrónica y concluye devolviendo exclusivamente el certificado público X.509 (`certEncoded = pke.getCertificateChain()[0].getEncoded()`). Si el usuario cancela la introducción del PIN en el diálogo del controlador criptográfico, se produce una excepción de cancelación (`AOCancelledOperationException` o `BadPasswordProviderException`), abortando la operación.

**Divergencia observable en la suite de conformidad:**
La exigencia de clave privada en `selectcert` constituye una divergencia observable en el cable frente a rFirma (medida por el caso `selectcert_checks_private_key` de la suite de conformidad). Al tratarse de una comprobación deliberada de AutoFirma para evitar la selección de certificados huérfanos sin capacidad de firma, no es un defecto de AutoFirma y por tanto **no tiene ficha en el anexo A1**. El resultado de esta divergencia y su registro quedan en el **informe** de cada cliente, donde AutoFirma lo confirma (cancela al requerir PIN sin respuesta) y rFirma lo refuta (devuelve el certificado sin sesión).

---

## 5. Mecanismo de certificado pegajoso (*sticky signature*)

El protocolo de AutoFirma soporta la fijación de la clave privada y certificado del usuario para evitar que se le vuelva a solicitar la selección de certificado (y en ciertos casos el PIN) en invocaciones sucesivas.

### 5.1 Ciclo de vida de `stickyKeyEntry` en memoria

La clave privada fijada reside en un atributo estático en la clase `ProtocolInvocationLauncher`:

```java
private static PrivateKeyEntry stickyKeyEntry = null; // ProtocolInvocationLauncher.java:90
```

En `ProtocolInvocationLauncherSelectCert.java:139-143`:

```java
if (options.getSticky() && !options.getResetSticky() && ProtocolInvocationLauncher.getStickyKeyEntry() != null) {
    LOGGER.info("Se usa Sticky Signature y tenemos valor de clave privada");
    pke = ProtocolInvocationLauncher.getStickyKeyEntry();
} else {
    // ... Proceso completo de apertura de almacén y diálogo ...
    // Al finalizar:
    if (options.getSticky()) {
        ProtocolInvocationLauncher.setStickyKeyEntry(pke);
    } else {
        ProtocolInvocationLauncher.setStickyKeyEntry(null);
    }
}
```

### 5.2 Matriz de estados y transiciones de `sticky`

| Parámetro `sticky` | Parámetro `resetsticky` | `stickyKeyEntry` actual | Acción ejecutada | Nuevo estado de `stickyKeyEntry` |
|---|---|---|---|---|
| `true` | `false` | Distinto de `null` | **Reutiliza `stickyKeyEntry` directamente**. Omite diálogo y apertura de almacén. | Se mantiene sin cambios |
| `true` | `false` | `null` | Abre almacén y muestra diálogo de selección. | **Almacena** el nuevo `pke` |
| `true` | `true` | Distinto de `null` | **Ignora la clave fijada**. Abre almacén y muestra diálogo de selección. | **Sobrescribe** con el nuevo `pke` |
| `false` | Indiferente | Distinto de `null` | Abre almacén y muestra diálogo de selección. | **Se borra** (se establece a `null`) |
| `false` | Indiferente | `null` | Abre almacén y muestra diálogo de selección. | Se mantiene a `null` |

### 5.3 Limitación arquitectónica según el transporte

* **En Socket local y WebSocket**: El proceso Java de AutoFirma se mantiene en ejecución en segundo plano atendiendo múltiples peticiones en un bucle continuo (`ServiceInvocationManager` / `AfirmaWebSocketServerManager`). Por tanto, la variable estática `stickyKeyEntry` sobrevive entre sucesivas invocaciones a `launch()`, permitiendo que una llamada a `selectcert?sticky=true` fije el certificado para operaciones posteriores de firma (`sign`, `cosign`, etc.).
* **En Servidor intermedio**: Cada invocación de protocolo `afirma://` mediante el esquema del sistema operativo arranca un proceso JVM independiente desde cero (`SimpleAfirma.main`), que finaliza tras subir el resultado a `stservlet`. La memoria estática desaparece con el proceso.

#### Solución del cliente JavaScript (`autoscript.js`):
Para sortear esta limitación en el transporte por servidor intermedio en ordenadores de escritorio, `autoscript.js` implementa un mecanismo de fijación en el lado del cliente (`afirma-ui-miniapplet-deploy/.../autoscript.js:4559-4560, 4652-4655, 4660-4685`):
1. Al recibir la respuesta de `selectcert`, `autoscript.js` guarda el certificado en una variable JavaScript global:
   ```javascript
   stickyCertificate = !!stickySignatory ? certificate : null;
   ```
2. En las siguientes llamadas de firma, si `stickyCertificate` está fijado y `resetStickySignatory` es falso, la función `addSignatoryCertificateToExtraParams` inyecta automáticamente en `extraParams`:
   ```javascript
   newParamsList.push("filters=encodedcert:" + stickyCertificate);
   newParamsList.push("headless=true");
   ```
3. Esto fuerza a la nueva instancia independiente de AutoFirma a filtrar por ese certificado exacto y, al estar en modo `headless=true`, lo autoselecciona sin mostrar ninguna ventana al usuario.

### 5.4 Divergencia de rFirma

rFirma no reutiliza el certificado fijado sin preguntar (ADR-0010). Con `sticky=true`, `selectcert` abre siempre la ventana de consentimiento, y el certificado fijado solo llega preseleccionado; lo mismo en `batch` y en el lote local. El certificado fijado vive en la memoria del proceso de sede que atiende el trámite y muere con él: no se comparte con otro trámite ni se guarda en disco. Como rFirma arranca un proceso por invocación `afirma://`, en la práctica la sesión es esa invocación. Si no hay ninguno fijado en esa sesión, la fila preseleccionada es la del último certificado usado en el escritorio. `resetsticky` olvida solo el certificado fijado en la sesión, nunca el último certificado usado del escritorio.

En la matriz de 5.2, la primera fila es la que cambia: rFirma no omite el diálogo. En la suite de conformidad es una desviación deliberada: las comprobaciones `a_certificate_pinned_by_selectcert_signs_without_asking` y `a_pinned_certificate_ignores_the_filters_of_the_next_request` salen NO CONFORME para rFirma.

---

## 6. Codificación, cifrado y formato de la respuesta

Una vez obtenida la `PrivateKeyEntry` (bien por selección interactiva o reutilización de `stickyKeyEntry`), se extrae el certificado y se prepara la carga útil de respuesta.

### 6.1 Extracción del certificado hoja

En `ProtocolInvocationLauncherSelectCert.java:231-242`:

```java
byte[] certEncoded;
try {
    certEncoded = pke.getCertificateChain()[0].getEncoded();
} catch (final CertificateEncodingException e) {
    LOGGER.severe("Error en la decodificacion del certificado de firma: " + e);
    final String errorCode = ProtocolInvocationLauncherErrorManager.ERROR_DECODING_CERTIFICATE; // SAF_18
    ProtocolInvocationLauncherErrorManager.showError(errorCode, e);
    if (!bySocket) {
        throw new SocketOperationException(errorCode);
    }
    return ProtocolInvocationLauncherErrorManager.getErrorMessage(errorCode);
}
```

Se toma únicamente el primer elemento de la cadena de certificados (`pke.getCertificateChain()[0]`), correspondiente al certificado hoja del firmante en formato binario DER X.509 (`getEncoded()`). Si la codificación del certificado es inválida, se produce el error `SAF_18`.

### 6.2 Cifrado simétrico DES opcional

Si la invocación incluyó el parámetro `key` con una clave de 8 caracteres (`options.getDesKey() != null`), el resultado se cifra mediante `CypherDataManager.cipherData`
(`afirma-simple/.../crypto/CypherDataManager.java:71-76`):

```java
dataToSend = CypherDataManager.cipherData(certEncoded, options.getDesKey());
```

El algoritmo empleado es **DES simétrico en modo ECB sin relleno estándar** (`DES/ECB/NoPadding`, `DesCipher.java:31`), aplicando un relleno manual de ceros hasta el múltiplo de 8 bytes más próximo (`DesCipher.java:64-69`). La salida se compone de:

$$\text{dataToSend} = \text{padding} + \text{"."} + \text{Base64URLSafe}(\text{DES}_{\text{key}}(\text{certEncoded} \parallel \text{ceros}))$$

* El carácter separador entre la cuenta de padding y el criptograma es estrictamente el punto (`.` o `PADDING_CHAR_SEPARATOR`, `CypherDataManager.java:16`).
* La codificación Base64 resultante utiliza el alfabeto *URL-Safe* estándar (sustituyendo `+` por `-` y `/` por `_`).

Si no se proporcionó clave simétrica (`key`), se omite el cifrado y se codifica el certificado directamente en Base64 URL-Safe (`ProtocolInvocationLauncherSelectCert.java:262`):

```java
dataToSend = Base64.encode(certEncoded, true);
```

### 6.3 Comparativa de la carga útil frente a operaciones de firma

| Operación | Estructura devuelta en texto plano |
|---|---|
| `sign` / `cosign` / `countersign` | `Base64(Certificado) \| Base64(Firma) [ \| Base64(ExtraInfo) ]` |
| `signandsave` | `Base64(Certificado) \| Base64(Firma)` |
| `batch` | `XML_o_JSON_del_resultado` |
| `selectcert` | **`Base64(Certificado)`** (sin separadores `\|` ni firmas añadidas) |

### 6.4 Entrega del resultado según el transporte

```
                          ┌──────────────────────────┐
                          │     Resultado: dataToSend │
                          └─────────────┬────────────┘
                                        │
             ┌──────────────────────────┼──────────────────────────┐
             │                          │                          │
             ▼                          ▼                          ▼
 ┌───────────────────────┐  ┌───────────────────────┐  ┌───────────────────────┐
 │ Servidor Intermedio   │  │ Socket Local          │  │ WebSocket             │
 │ (!bySocket)           │  │ (bySocket == true)    │  │ (bySocket == true)    │
 ├───────────────────────┤  ├───────────────────────┤  ├───────────────────────┤
 │ 1. Interrumpe hilo aw │  │ Retorna dataToSend    │  │ Retorna dataToSend    │
 │ 2. Sincroniza semáforo│  │ como String a         │  │ a AfirmaWebSocket-    │
 │ 3. POST a stservlet   │  │ CommandProcessorThread│  │ ServerManager         │
 │ 4. Envía id y datos   │  │ Fragmentación en      │  │ Envío como frame      │
 │ 5. Retorna dataToSend │  │ paquetes HTTP SSL     │  │ de texto WebSocket    │
 └───────────────────────┘  └───────────────────────┘  └───────────────────────┘
```

1. **Transporte por servidor intermedio (`!bySocket`)**:
   - Se interrumpe el hilo de espera activa si estuviera activo: `waitingThread.interrupt()` (`ProtocolInvocationLauncherSelectCert.java:267-270`).
   - Se adquiere el cerrojo de sincronización global: `synchronized (IntermediateServerUtil.getUniqueSemaphoreInstance())` (`ProtocolInvocationLauncherSelectCert.java:272`).
   - Se realiza una petición HTTP POST a `stservlet` pasando los parámetros `op=put`, `v=1_0`, `id=options.getId()` y `dat=dataToSend` (`IntermediateServerUtil.java`).
   - Si la transmisión por red falla, se captura `Exception` y se emite `SAF_11` (`ERROR_SENDING_RESULT`).
2. **Transporte por socket local (`bySocket == true`)**:
   - `ProtocolInvocationLauncher.launch` retorna la cadena `dataToSend`.
   - El hilo `CommandProcessorThread` almacena el resultado en un búfer en memoria y responde a las peticiones GET/POST del cliente JavaScript dividiendo la carga útil en fragmentos (`send=@1@1idsession=...@EOF`).
3. **Transporte por WebSocket**:
   - El manejador `AfirmaWebSocketServer` o `AfirmaWebSocketServerV4` recibe la cadena devuelta por `launch()` y la transmite como mensaje de respuesta al navegador.

---

## 7. Integración con el cliente JavaScript de referencia (`autoscript.js`)

El cliente JavaScript de despliegue oficial de AutoFirma (`autoscript.js`) expone la función de alto nivel `AutoScript.selectCertificate`:

```javascript
AutoScript.selectCertificate(extraParams, successCallback, errorCallback);
```

### 7.1 Construcción de la petición en cada canal

#### 1. WebSocket (`afirma://websocket`)
En `autoscript.js:1818-1822` y `1941-1953`:
```javascript
function createSelectCertificateRequest(extraParams) {
    var data = new Object();
    data.op = createKeyValuePair("op", "selectcert");
    data.idsession = createKeyValuePair("idsession", idSession);
    data.properties = createKeyValuePair("properties", extraParams != null ? Base64.encode(extraParams, true) : null, true);
    data.ksb64 = createKeyValuePair("ksb64", defaultKeyStore != null ? Base64.encode(defaultKeyStore, true) : null, true);
    data.sticky = createKeyValuePair("sticky", stickySignatory);
    if (resetStickySignatory) {
        data.resetSticky = createKeyValuePair("resetsticky", resetStickySignatory);
    }
    return data;
}
```

#### 2. Socket local (`afirma://service`)
En `autoscript.js:2696-2709`:
```javascript
function selectCertByService(extraParams) {
    var data = new Object();
    data.op = generateDataKeyValue("op", "selectcert");
    data.properties = generateDataKeyValue("properties", extraParams != null ? Base64.encode(extraParams) : null);
    data.keystore = generateDataKeyValue("keystore", defaultKeyStore != null ? defaultKeyStore : null);
    data.ksb64 = generateDataKeyValue("ksb64", defaultKeyStore != null ? Base64.encode(defaultKeyStore) : null);
    data.sticky = generateDataKeyValue("sticky", stickySignatory);
    if (resetStickySignatory) {
        data.resetSticky = generateDataKeyValue("resetsticky", resetStickySignatory);
    }
    execAppIntent(buildUrl(data));
}
```

#### 3. Servidor intermedio (HTTP)
En `autoscript.js:3775-3817`:
```javascript
function selectCertificate(extraParams, successCallback, errorCallback) {
    currentOperation = OPERATION_SELECT_CERTIFICATE;
    var idSession = AfirmaUtils.generateNewIdSession();
    var cipherKey = generateCipherKey();
    var opId = "selectcert";
    var params = new Array();
    params[params.length] = {key:"ver", value:PROTOCOL_VERSION};
    params[params.length] = {key:"op", value:opId};
    params[params.length] = {key:"id", value:idSession};
    params[params.length] = {key:"key", value:cipherKey};
    if (defaultKeyStore != null) {
        params[params.length] = {key:"keystore", value:defaultKeyStore};
        params[params.length] = {key:"ksb64", value:Base64.encode(defaultKeyStore)};
    }
    if (storageServletAddress != null) {
        params[params.length] = {key:"stservlet", value:storageServletAddress};
    }
    if (!Platform.isAndroid() && !Platform.isIOS()) {
        params[params.length] = {key:"aw", value:"true"};
    }
    configureExtraParams(params, extraParams);
    ...
}
```

### 7.2 Procesamiento de la respuesta y normalización de Base64

Una particularidad esencial es cómo `autoscript.js` transforma la cadena devuelta por AutoFirma:
* En WebSocket (`autoscript.js:2462-2471`):
  ```javascript
  function processSelectCertificateResponse(data) {
      if (!!successCallback) {
          var responseSuccessCallback = successCallback;
          setCallbacks(null, null);
          responseSuccessCallback(data.replace(/\-/g, "+").replace(/\_/g, "/"));
      }
  }
  ```
* En Socket local (`autoscript.js:3358-3378`):
  ```javascript
  function successSelectCertServiceResponseFunction(data) {
      if (data == undefined || data == null || data == "CANCEL") {
          errorCallback("es.gob.afirma.core.AOCancelledOperationException", "Operacion cancelada por el usuario");
          return;
      }
      if (data.length > 4 && data.substr(0, 4) == "SAF_") {
          errorCallback("java.lang.Exception", data);
          return;
      }
      if (data == "NULL") {
          errorCallback("java.lang.Exception", "Error desconocido");
          return;
      }
      successCallback(data.replace(/\-/g, "+").replace(/\_/g, "/"));
  }
  ```
* En Servidor intermedio (`autoscript.js:4550-4565`):
  Si se usó clave de cifrado (`cipherKey`), se descifra la cadena mediante `decipher(html, cipherKey)`. Si no se usó clave, se convierte con `fromBase64UrlSaveToBase64(html)`. En ambos casos, el callback de éxito recibe el certificado X.509 en formato **Base64 estándar (RFC 4648 con `+` y `/`)**, habiendo deshecho la codificación URL-Safe generada por la aplicación de escritorio.

---

## 8. Catálogo completo de errores de la operación `selectcert`

Los errores generados durante la ejecución de `selectcert` se codifican mediante prefijos `SAF_nn`
definidos en `ProtocolInvocationLauncherErrorManager`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherErrorManager.java`).
La cancelación de usuario utiliza el literal no numérico `"CANCEL"`.

### 8.1 Tabla de errores

| Constante en código | Código | Mensaje literal (`protocolmessages.properties`) | Detonante en `selectcert` | Vía de notificación |
|---|---|---|---|---|
| `ERROR_NULL_URI` | `SAF_01` | La URL recibida es nula | Parámetro `options` es `null` en `processSelectCert`. | Diálogo nativo de error + respuesta al transporte |
| `ERROR_PARAMS` | `SAF_03` | Error en los parámetros de entrada | Parámetros inválidos: `id` > 20 caracteres o no alfanumérico; `key` != 8 caracteres; URL de servlet local o malformada; falta de `stservlet` en servidor intermedio. | Diálogo nativo (`showErrorDetail`) + respuesta al transporte |
| `ERROR_CANNOT_ACCESS_KEYSTORE` | `SAF_08` | Error accediendo al almacén de claves y certificados | Fallo al instanciar `AOKeyStoreManagerFactory` o excepción genérica durante el despliegue del diálogo `AOKeyStoreDialog`. | Diálogo nativo de error + respuesta al transporte |
| `ERROR_SENDING_RESULT` | `SAF_11` | Error en el envio del resultado de la operación. | Fallo en la conexión HTTP POST al enviar el resultado a `stservlet` en transporte por servidor intermedio. | Diálogo nativo de error + respuesta al transporte |
| `ERROR_ENCRIPTING_DATA` | `SAF_12` | Error en el cifrado de los datos a enviar | Excepción criptográfica en `CypherDataManager.cipherData` al cifrar el certificado con la clave DES. | Diálogo nativo de error + respuesta al transporte |
| `ERROR_DECRYPTING_DATA` | `SAF_15` | Error en el descifrado de los datos | Fallo al descifrar el XML descargado del servidor intermedio (`fileid`). | Diálogo nativo de error + respuesta al transporte |
| `ERROR_RECOVERING_DATA` | `SAF_16` | Error al recuperar los datos del servidor intermedio | Fallo de red o longitud inválida (`InvalidEncryptedDataLengthException`) al descargar el XML de configuración desde `rtservlet`. | Diálogo nativo de error + respuesta al transporte |
| `ERROR_DECODING_CERTIFICATE` | `SAF_18` | Error al descodificar el certificado de firma | `CertificateEncodingException` al obtener `pke.getCertificateChain()[0].getEncoded()`. | Diálogo nativo de error + respuesta al transporte |
| `ERROR_NO_CERTIFICATES_KEYSTORE` | `SAF_19` | No hay ningun certificado válido en su almacén. Compruebe las fechas de caducidad e instale un certificado válido. | Ningún certificado en el almacén supera los filtros (o todos están caducados si no había filtros explícitos). `AOCertificatesNotFoundException`. | Diálogo nativo de error + respuesta al transporte |
| `ERROR_UNSUPPORTED_PROCEDURE` | `SAF_21` | La versión de Autofirma instalada no es compatible con este trámite. Actualice a la última versión disponible. | La versión de protocolo requerida en la URL (`protocolVersion`) supera `MAX_PROTOCOL_VERSION_SUPPORTED` (versión 3). | Diálogo nativo de error + respuesta al transporte |
| `ERROR_MINIMUM_VERSION_NON_SATISTIED` | `SAF_41` | El uso de este trámite web requiere una versión más reciente de Autofirma. Actualice a la última versión disponible. | El parámetro `mcv` especifica una versión de aplicación estrictamente superior a la versión actual de AutoFirma (`SimpleAfirma.getVersion()`). | Diálogo nativo de error + respuesta al transporte |
| `RESULT_CANCEL` | `"CANCEL"` | *(Sin mensaje modal; cancelación explícita)* | El usuario pulsa «Cancelar» o cierra la ventana de selección de certificados (`AOCancelledOperationException`). | Cadena `"CANCEL"` sin cuadro de error nativo |

### 8.2 Mecanismo de notificación según el canal de transporte

1. **Transporte por servidor intermedio (`!bySocket`)**:
   Cuando se produce un error o una cancelación dentro de `ProtocolInvocationLauncherSelectCert`, la clase lanza una `SocketOperationException` envolviendo el código de error (`errorCode` o `"CANCEL"`).
   En `ProtocolInvocationLauncher.java:419-428`:
   ```java
   catch (final SocketOperationException e) {
       final String msg = e.getErrorCode() == ProtocolInvocationLauncherSelectCert.getResultCancel()
               ? e.getErrorCode()
               : URLEncoder.encode(
                       ProtocolInvocationLauncherErrorManager.getErrorMessage(e.getErrorCode()),
                       StandardCharsets.UTF_8.toString());
       sendDataToServer(msg, params.getStorageServletUrl().toString(), params.getId());
       return ProtocolInvocationLauncherErrorManager.getErrorMessage(e.getErrorCode());
   }
   ```
   Si es una cancelación, sube la cadena `"CANCEL"` en texto plano; si es un error, sube el mensaje en español URL-encoded al `stservlet`.
2. **Transporte por socket local o WebSocket (`bySocket == true`)**:
   No se lanza `SocketOperationException`. El método retorna directamente el código de error (`ProtocolInvocationLauncherErrorManager.getErrorMessage(errorCode)`) o `"CANCEL"`. El socket o websocket lo envía al navegador, donde `autoscript.js` evalúa si la respuesta comienza por `"SAF_"` o es igual a `"CANCEL"`, ejecutando el callback de error correspondiente.



