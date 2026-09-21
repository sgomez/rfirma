# 10. Operaciones `save` y `load`

Este capítulo describe las operaciones utilitarias de gestión de ficheros
`save` y `load` (junto con su variante multicarga `multiload`) del protocolo
`afirma://` de AutoFirma 1.9.2. A diferencia de las operaciones de firma digital
(`sign`, `cosign`, `countersign`, `signandsave`, `batch`) o de selección de
credenciales (`selectcert`), estas operaciones no efectúan procesamiento
criptográfico ni acceden a los almacenes de claves: su propósito es salvar las
restricciones de la sandbox del navegador web permitiendo, respectivamente, el
almacenamiento en el disco local de datos descargados o generados por una
aplicación web, y la lectura local de ficheros del usuario para su transmisión
codificada en Base64 hacia la aplicación web invocadora.

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. Visión general y ciclo de vida de `save` y `load`

Las operaciones `save` y `load` fueron concebidas para proporcionar a los
despliegues web de administración electrónica una pasarela de lectura y escritura
en disco, heredada conceptualmente de las capacidades del antiguo Applet Java
(`MiniApplet`), pero ejecutada a través de la aplicación de escritorio AutoFirma.

* **`save`:** Toma un flujo de datos (recibido directamente en la URL en Base64
  o descargado de un servidor intermedio mediante `fileid`) y despliega un diálogo
  gráfico nativo `Guardar como...` (`JFileChooser`) para que el usuario escoja
  dónde guardarlo, con detección de colisiones de nombre y confirmación de
  sobrescritura. Devuelve únicamente una confirmación de éxito (`OK` / `SAVE_OK`)
  o un código de cancelación/error.
* **`load`:** Despliega un diálogo gráfico nativo `Abrir fichero...`
  (`JFileChooser`) con filtros de extensión configurables, permitiendo seleccionar
  un fichero (o varios si se activa `multiload`), lee su contenido del sistema de
  archivos y lo devuelve codificado en Base64 junto al nombre del fichero con el
  formato `nombre:base64` (o `nombre1:base64_1|nombre2:base64_2`).

### 1.1 Diagrama del ciclo de vida de la operación `save`

```
           ┌─────────────────────────────────────────────────────────────┐
           │ Invocación: afirma://save?dat=...&filename=...              │
           │ o afirma://save/?fileid=...&key=...                         │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 1. ¿Configuración remota?                                   │
           │    - Si hay fileid: descarga XML de rtservlet y parsea      │
           │    - Si hay dat: decodifica Base64 / descomprime GZIP       │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 2. Enfoque de ventana en macOS: MacUtils.focusApplication() │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 3. Diálogo de guardado (AOUIFactory.getSaveDataToFile):     │
           │    - Selector JFileChooser con filtros GenericFileFilter    │
           │    - Detección de colisión: confirmación de sobrescritura   │
           │    - Escritura física en disco (FileOutputStream)           │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 4. Devolución de resultado:                                 │
           │    - Socket local: responde SAVE_OK (HTTP 200)              │
           │    - WebSocket: devuelve OK en broadcast                    │
           │    - Servidor intermedio: sube OK a stservlet con semáforo  │
           └─────────────────────────────────────────────────────────────┘
```

### 1.2 Diagrama del ciclo de vida de la operación `load`

```
           ┌─────────────────────────────────────────────────────────────┐
           │ Invocación: afirma://load?exts=pdf,txt&multiload=true...    │
           │ o afirma://load/?title=...&filePath=...                     │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 1. Enfoque de ventana en macOS: MacUtils.focusApplication() │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 2. Diálogo de selección (AOUIFactory.getLoadFiles):         │
           │    - Selector JFileChooser en modo simple o múltiple        │
           │    - Aplicación de FileNameExtensionFilter si hay exts      │
           │    - Directorio inicial según filePath o preferencia        │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 3. Lectura física y codificación:                           │
           │    - Lectura de bytes vía FileInputStream                   │
           │    - Codificación en Base64 estándar                        │
           │    - Formateo: nombre:base64[|nombre2:base64_2...]          │
           └──────────────────────────────┬──────────────────────────────┘
                                          │
                                          ▼
           ┌─────────────────────────────────────────────────────────────┐
           │ 4. Devolución de resultado:                                 │
           │    - Socket local: respuesta fragmentada HTTP               │
           │    - WebSocket: broadcast directo del texto estructurado    │
           │    - Servidor intermedio: NO SOPORTADO (lanza error)        │
           └─────────────────────────────────────────────────────────────┘
```

### 1.3 Prefijos de URI reconocidos

El despachador central `ProtocolInvocationLauncher.launch`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncher.java:443,753`)
identifica ambas operaciones mediante coincidencia estricta de prefijos:

| Operación | Prefijos aceptados | Cita en código |
|---|---|---|
| `save` | `afirma://save?`<br>`afirma://save/?` | `ProtocolInvocationLauncher.java:443` |
| `load` | `afirma://load?`<br>`afirma://load/?` | `ProtocolInvocationLauncher.java:753` |

Cualquier variante sin signo de interrogación (como `afirma://save`) o con otros
separadores no es reconocida y genera el error genérico `SAF_04` («Operación no
soportada»).

---

## 2. Operación `save`: Invocación y Parámetros

El procesador de la operación de guardado es `ProtocolInvocationLauncherSave`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherSave.java:24-146`),
cuyos parámetros son analizados y encapsulados en la clase `UrlParametersToSave`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSave.java:18-260`),
heredera de `UrlParameters`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java:26-458`).

### 2.1 Catálogo de parámetros aceptados en `save`

| Parámetro | Tipo / Formato | Obligatorio | Descripción | Cita en código |
|---|---|---|---|---|
| `dat` | Base64 URL-safe / URL | Condicional* | Datos binarios que se escribirán en el disco. Admite Base64, URLs HTTP/HTTPS o texto plano. | `UrlParameters.java:34,298-320` |
| `fileid` | Alfanumérico (máx. 20 car.) | Condicional* | Identificador del fichero temporal en el servidor intermedio cuando los datos no viajan en la URL. | `UrlParameters.java:59,269-272` |
| `rtservlet` | URL HTTP/HTTPS | Sí, si hay `fileid` | URL del servlet de recuperación (`RetrieveService`) para descargar los datos o el XML de configuración. | `UrlParameters.java:43,273-295` |
| `stservlet` | URL HTTP/HTTPS | Sí, con servidor intermedio | URL del servlet de almacenamiento (`StorageService`) donde AutoFirma deposita el `OK` o el error. | `UrlParametersToSave.java:178-199` |
| `id` | Alfanumérico (máx. 20 car.) | Sí, con servidor intermedio | Identificador de la sesión de comunicación con el servidor intermedio. Si no se pasa, se toma de `fileid`. | `UrlParametersToSave.java:147-167` |
| `key` | 8 caracteres ASCII | Opcional | Clave simétrica DES para descifrar los datos recuperados de `rtservlet`. | `UrlParameters.java:56,327-345` |
| `title` | Cadena de texto | No | Título mostrado en la barra superior de la ventana del diálogo nativo `Guardar como...`. Por defecto: `"Guardar fichero"`. | `UrlParametersToSave.java:21,235-240` |
| `filename` | Cadena de texto | No | Nombre de fichero propuesto por defecto en la caja de texto del diálogo. No puede contener caracteres ilegales de sistema de ficheros. | `UrlParametersToSave.java:27,207-219` |
| `exts` | Lista separada por comas | No | Extensiones sugeridas para el filtrado de ficheros en el diálogo (p. ej. `pdf,txt`). | `UrlParametersToSave.java:30,221-233` |
| `desc` | Cadena de texto | No | Descripción legible del tipo de fichero asociado al filtro de extensiones (p. ej. `Documentos PDF`). | `UrlParametersToSave.java:24,242-258` |
| `gzip` | Booleano (`true`/`false`) | No | Indica si los datos contenidos en `dat` fueron comprimidos con GZIP antes de codificarse en Base64. | `UrlParameters.java:40,311` |
| `ver` | Entero (`0`–`4`) | No | Versión de protocolo de la operación; solo se lee sin canal abierto (servidor intermedio). Por defecto: `"0"`. Ver [14 §2.3](14-versiones.md). | `UrlParametersToSave.java:36,170-175` |
| `v` | Entero (`1`–`4`) | No | Versión de protocolo declarada en la URL (extraída por `ProtocolInvocationLauncher.getVersion`). | `ProtocolInvocationLauncher.java:923-939` |
| `mcv` | Cadena versionada (p. ej. `1.9.2`) | No | Versión mínima de la aplicación AutoFirma requerida. Si la versión actual es menor, falla con `SAF_41`. | `UrlParameters.java:73,260-262`, `ProtocolInvocationLauncherSave.java:62-73` |
| `aw` | Booleano (`true`/`false`) | No | Habilita la espera activa (*active waiting*) en el servidor intermedio mientras se guarda el fichero. | `UrlParameters.java:70,257-258`, `ProtocolInvocationLauncher.java:483-485` |

*\*Es obligatorio proporcionar al menos `dat` o `fileid`. Si ambos están ausentes,
`UrlParametersToSave.java:141-143` lanza de inmediato una `ParameterException`:*
*`"No se ha proporcionado el identificador de fichero ni los datos a guardar"`.*

### 2.2 Validación y restricciones sintácticas en `save`

1. **Restricción de lectura local en `dat`:**
   Si el valor de `dat` comienza por `file:/`, `UrlParameters.java:300-304`
   rechaza la petición arrojando una `ParameterException` con el mensaje
   `"No se permite la lectura de ficheros locales: ..."`. Esto impide que una
   página maliciosa use el parámetro para forzar la lectura indebida de archivos
   del sistema.
2. **Caracteres prohibidos en `filename`:**
   `UrlParametersToSave.java:212-216` comprueba que el nombre propuesto no
   contenga ninguno de los caracteres:
   ```
   \ / : * ? " < > |
   ```
   Si se detecta alguno de ellos, lanza `ParameterException` indicando el
   carácter inválido.
3. **Caracteres prohibidos en `exts`:**
   `UrlParametersToSave.java:226-230` valida la lista de extensiones frente al
   conjunto de caracteres no admitidos:
   ```
   \ / : * ? " < > | ; [espacio]
   ```
   Obsérvese que incluye explícitamente el punto y coma (`;`) y el espacio en blanco.
4. **Transformación automática de `desc`:**
   En `UrlParametersToSave.java:247-255`, si se han definido `exts` y la
   descripción proporcionada no finaliza ya en `")"`, el parser concatena las
   extensiones entre paréntesis al final de la descripción.
   *Mecanismo de formateo:* La rutina itera las extensiones concatenando `*.`
   y cada extensión sin intercalar ningún delimitador (p. ej., para `exts=pdf,txt`
   y `desc=Documentos`, ejecuta `sb.append("*.").append(ext)` en bucle, generando
   la cadena `"Documentos (*.pdf*.txt)"` en lugar de utilizar comas o espacios).
   Esta cadena resultante es la que se asigna como descripción al filtro nativo
   y se muestra en el desplegable de tipos de archivo de la interfaz gráfica.
5. **Validación de identificadores de sesión (`id` / `fileid`):**
   `UrlParametersToSave.java:154-165` restringe la longitud máxima a 20 caracteres
   (`MAX_ID_LENGTH = 20`) y verifica que todos los caracteres sean alfanuméricos
   ASCII estrictos (`[a-z0-9]` tras conversión a minúsculas en locale inglés).
   Si contiene caracteres no alfanuméricos, arroja una `ParameterException` con
   el mensaje literal:
   `"El identificador de la firma debe ser alfanumerico."`
   A pesar de tratarse de una operación utilitaria de guardado de ficheros (`save`)
   y no de firma digital, este mensaje hace referencia explícita a la «firma»
   debido a que el bloque de validación fue reutilizado directamente del parser
   de firma (`UrlParametersToSign.java:188-193`). El error es capturado por
   `ProtocolInvocationLauncher.java:516`, que despliega el diálogo modal de error
   al usuario con dicho texto y retorna el código `SAF_03`.
6. **Validación de URL de servlets (`stservlet`, `rtservlet`):**
   `UrlParameters.validateURL` (`UrlParameters.java:351-379`) exige protocolo
   `http` o `https`, prohíbe conexiones a direcciones locales (`localhost` o
   `127.0.0.1`, lanzando `ParameterLocalAccessRequestedException`) y rechaza
   cualquier URL que contenga caracteres `?` o `=`, asegurando que los servlets no
   reciban parámetros adicionales no controlados.
7. **Clave de cifrado `key`:**
   Si se especifica el parámetro `key`, `UrlParameters.verifyCipherKey`
   (`UrlParameters.java:327-345`) exige que tenga una longitud exacta de 8
   caracteres (64 bits, tamaño de clave DES).

---

## 3. Operación `save`: Interfaz de usuario y guardado en disco

Una vez superadas las validaciones de protocolo y de versión mínima de cliente
(`ProtocolInvocationLauncherSave.java:49-73`), la ejecución procede al guardado
interactivo:

### 3.1 Activación de foco en macOS

En sistemas macOS, se invoca `MacUtils.focusApplication()`
(`ProtocolInvocationLauncherSave.java:76-78`) antes de desplegar cualquier diálogo,
forzando a que la ventana de guardado aparezca en primer plano sobre el navegador web.

### 3.2 Despliegue del diálogo `Guardar como...`

La llamada se canaliza a través de `AOUIFactory.getSaveDataToFile`
(`afirma-core/src/main/java/es/gob/afirma/core/ui/AOUIFactory.java:299-313`), que
delega en la implementación Swing `JSEUIManager.saveDataToFile`
(`afirma-ui-core-jse/src/main/java/es/gob/afirma/ui/core/jse/JSEUIManager.java:695-838`):

```java
// ProtocolInvocationLauncherSave.java:79-91
AOUIFactory.getSaveDataToFile(
    options.getData(),
    options.getTitle(),
    null,
    options.getFileName(),
    Collections.singletonList(
        new GenericFileFilter(
            options.getExtensions() != null ? new String[] { options.getExtensions() } : null,
            options.getFileTypeDescription()
        )
    ),
    null
);
```

#### Defecto en la construcción del filtro de extensiones múltiples (BUG-17):
En `ProtocolInvocationLauncherSave.java:86`, el array de extensiones se construye
como `new String[] { options.getExtensions() }` pasando la cadena cruda (por
ejemplo `"pdf,txt"`), en lugar de dividirla mediante `.split(",")` como realiza
correctamente `ProtocolInvocationLauncherLoad.java:98`. Como consecuencia, si se
especifican múltiples extensiones en una operación `save`, el `FileNameExtensionFilter`
subyacente de Java busca una extensión literal `.pdf,txt`. Esto causa que los ficheros
con extensiones válidas (`.pdf` o `.txt`) no se muestren en el diálogo nativo y que,
al guardar, AutoFirma anexe `.pdf,txt` al nombre (p. ej. `informe.pdf.pdf,txt`),
corrompiendo el nombre de archivo en disco — ver detalle y mecanismo en
[BUG-17 en A1-bugs-autofirma.md](A1-bugs-autofirma.md#bug-17-corrupción-de-nombres-y-fallo-de-filtrado-en-save-por-omisión-de-división-de-extensiones-múltiples-exts).

### 3.3 Directorio inicial y preferencias

El diálogo configura el directorio por defecto mediante `configureDefaultDir`
(`JSEUIManager.java:848-870`). Si no se proporciona un directorio inicial
explícito (en `save` siempre se pasa `null`), recupera la ruta de la preferencia
`current.dir` (`PREFERENCE_DIRECTORY = "current.dir"`). Si el usuario completa
el guardado con éxito, `JSEUIManager.java:828` actualiza dicha preferencia con el
directorio finalmente utilizado:
```java
put(PREFERENCE_DIRECTORY, fileChooser.getCurrentDirectory().getPath());
```

### 3.4 Control de colisiones y confirmación de sobrescritura

Si el fichero elegido por el usuario ya existe en disco (`file.exists()`),
`JSEUIManager.java:781-803` muestra un diálogo de confirmación
(`JOptionPane.showConfirmDialog`) con las opciones **Sí**, **No** y **Cancelar**
(`JOptionPane.YES_NO_CANCEL_OPTION`):

| Opción seleccionada | Comportamiento en `JSEUIManager` |
|---|---|
| **Sí** (`JOptionPane.YES_OPTION`) | Continúa normalmente y sobrescribe el fichero existente. |
| **No** (`JOptionPane.NO_OPTION`) | Establece `tryAgain = true`, rompe el switch y reabre el diálogo de guardado para que el usuario elija otro nombre. |
| **Cancelar** (`JOptionPane.CANCEL_OPTION`) | Lanza `AOCancelledOperationException`. |

### 3.5 Escritura física de los datos

Si el usuario confirma la ruta, `JSEUIManager.java:808-827` abre un
`FileOutputStream` sobre el archivo canónico y escribe la totalidad del array
de bytes `data`. Si ocurre cualquier error de entrada/salida (p. ej. disco lleno
o permisos denegados):
1. Captura la excepción y muestra una ventana emergente de error (`showErrorMessage`).
2. Marca `tryAgain = true` y vuelve a mostrar el diálogo de guardado para permitir
   al usuario seleccionar otra ubicación o unidad de almacenamiento.

Si el usuario pulsa `Cancelar` en el selector de ficheros, se lanza
`AOCancelledOperationException`.

---

## 4. Operación `save`: Notificación de resultado por transporte

El resultado de la operación `save` se notifica al entorno invocador de forma
diferenciada según el canal de transporte utilizado:

### 4.1 Devolución en WebSocket (`afirma://websocket`)

* En modo WebSocket, `AfirmaWebSocketServer`
  (`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/AfirmaWebSocketServer.java:113`)
  y `AfirmaWebSocketServerV4.java:91` envían el resultado devuelto por
  `ProtocolInvocationLauncher.launch`, que para una operación `save` exitosa es
  la cadena `"OK"` (`ProtocolInvocationLauncherSave.java:135`).
* Si el usuario cancela, devuelve `"CANCEL"`.
* Si ocurre un error, devuelve el código `SAF_nn` correspondiente.

#### Defecto de respuesta en guardado por WebSocket (BUG-18):
En `autoscript.js:3423`, la rutina de recepción del WebSocket evalúa:
```javascript
// autoscript.js:3423-3428
if (data == "SAVE_OK") {
    if (successCallback) {
        successCallback(data);
    }
    return;
}
```
Dado que el servidor WebSocket de AutoFirma devuelve directamente la cadena `"OK"`
devuelta por `ProtocolInvocationLauncher.launch` (a diferencia del socket HTTP local
donde `CommandProcessorThread.java:293` la transforma en `"SAVE_OK"`), la condición
`data == "SAVE_OK"` resulta falsa. El flujo desciende hasta la línea 3482, donde
`autoscript.js` asume que la respuesta es una firma en Base64, intenta decodificar `"OK"`
mediante `Base64.decode(data, true)` e invoca `successCallback(signature, certificate)`
entregando los bytes residuales de `"OK"` como firma y `null` como certificado — ver
detalle y mecanismo en [BUG-18 en A1-bugs-autofirma.md](A1-bugs-autofirma.md#bug-18-incoherencia-de-respuesta-en-save-por-websocket-ok-frente-a-save_ok-provoca-procesamiento-erróneo-como-firma-en-autoscriptjs).

### 4.2 Devolución en Socket local (`afirma://service`)

En la comunicación mediante socket local HTTP, el procesador de comandos
`CommandProcessorThread`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/CommandProcessorThread.java:290-305,333-346`)
intercepta explícitamente la invocación de `save`:

```java
// CommandProcessorThread.java:291-305
if (totalhttpRequest.toString().startsWith("afirma://save?") || totalhttpRequest.toString().startsWith("afirma://save/?")){
    isSave = true;
    final String operationResult = ProtocolInvocationLauncher.launch(totalhttpRequest.toString(), this.protocolVersion, true);
    if (operationResult.equals(OK)){
        sendData(createHttpResponse(true, SAVE_OK), socket, "Operacion save realizada con exito");
    }
    else if (operationResult.equals(CANCEL)){
        sendData(createHttpResponse(true, CANCEL), socket, "Cancelado por el usuario");
    }
    else {
        throw new IllegalArgumentException("Error al realizar la operacion save");
    }
}
```

* Si `ProtocolInvocationLauncher.launch` devuelve `"OK"`, `CommandProcessorThread`
  lo transforma expresamente en `"SAVE_OK"` antes de enviarlo codificado en Base64
  en el cuerpo de la respuesta HTTP 200.
* En `autoscript.js:3243-3248`, el cliente comprueba `isSaveOperation` y entrega
  `"SAVE_OK"` directamente a la función callback de éxito.
* Si la operación fue cancelada por el usuario, envía `"CANCEL"`.

### 4.3 Devolución en Servidor intermedio (HTTP)

Cuando la invocación se realiza sin socket (`bySocket == false`):
1. Si se configuró `storageServletUrl` (`ProtocolInvocationLauncherSave.java:110-128`),
   AutoFirma interrumpe el hilo de espera activa (`waitingThread.interrupt()`).
2. Adquiere el semáforo global exclusivo de red:
   `synchronized (IntermediateServerUtil.getUniqueSemaphoreInstance())`.
3. Sube la cadena `"OK"` al servlet de almacenamiento:
   ```java
   IntermediateServerUtil.sendData(RESULT_OK, options.getStorageServletUrl().toString(), options.getId());
   ```
4. Si el usuario canceló el diálogo (`AOCancelledOperationException`),
   `ProtocolInvocationLauncherSave` arroja una `SocketOperationException("CANCEL")`.
   En `ProtocolInvocationLauncher.java:494-503`, esta excepción es capturada y
   se envía la cadena `"CANCEL"` al servidor intermedio:
   ```java
   sendDataToServer(msg, params.getStorageServletUrl().toString(), params.getId());
   ```
5. Tras completar el envío, `SimpleAfirma.java:978-980` ejecuta
   `forceCloseApplication(0)` para cerrar el proceso de la aplicación de escritorio.

---

## 5. Operación `load`: Invocación y Parámetros

La operación `load` es gestionada por `ProtocolInvocationLauncherLoad`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherLoad.java:29-158`),
utilizando los parámetros encapsulados en `UrlParametersToLoad`
(`afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToLoad.java:15-202`).

A diferencia de `save`, `load` nunca procesa ni requiere datos binarios de entrada
(`dat`), limitándose a recibir los criterios de filtrado y el modo de selección.

### 5.1 Catálogo de parámetros aceptados en `load`

| Parámetro | Tipo / Formato | Obligatorio | Descripción | Cita en código |
|---|---|---|---|---|
| `multiload` | Booleano (`true`/`false`) | No | Si es `true`, permite la selección múltiple de ficheros en el diálogo. Por defecto: `false` (selección única). | `UrlParametersToLoad.java:21,165-170` |
| `title` | Cadena de texto | No | Título de la ventana del diálogo de selección de ficheros. Por defecto: `null` (Swing utiliza el título nativo del sistema, p. ej. `"Abrir"`). | `UrlParametersToLoad.java:24,172-177` |
| `exts` | Lista separada por comas | No | Extensiones de fichero permitidas en el selector (p. ej. `pdf,xml,xsig`). | `UrlParametersToLoad.java:27,179-184` |
| `desc` | Cadena de texto | No | Descripción textual del tipo de fichero que acompaña a las extensiones en el desplegable de filtros. | `UrlParametersToLoad.java:30,186-191` |
| `filePath` | Ruta de fichero o directorio | No | Ruta local inicial sugerida donde se abrirá el diálogo selector de ficheros. | `UrlParametersToLoad.java:33,193-199` |
| `ver` | Entero (`0`–`4`) | No | Versión de protocolo de la operación; solo se lee sin canal abierto (servidor intermedio). Por defecto: `"0"`. Ver [14 §2.3](14-versiones.md). | `UrlParametersToLoad.java:18,155-161` |
| `v` | Entero (`1`–`4`) | No | Versión de protocolo declarada en la URL (extraída por `ProtocolInvocationLauncher.getVersion`). | `ProtocolInvocationLauncher.java:923-939` |
| `mcv` | Cadena versionada (p. ej. `1.9.2`) | No | Versión mínima de la aplicación AutoFirma requerida. Si la versión instalada es inferior, falla con `SAF_41`. | `UrlParametersToLoad.java:153`, `UrlParameters.java:260-262`, `ProtocolInvocationLauncherLoad.java:73-84` |

*Nota sobre parámetros ausentes y gestión de `key` en `load`:*
1. A diferencia de `UrlParametersToSave`, `UrlParametersToLoad` no define ni
   analiza los parámetros de servidor intermedio `stservlet` ni `id`.
2. Por herencia de la clase base `UrlParameters`, `setCommonParameters` sí analiza
   y valida el parámetro de clave de cifrado `key` (`verifyCipherKey`), exigiendo
   que tenga exactamente 8 caracteres (arrojando `SAF_03` si la longitud difiere).
   Sin embargo, el objeto descifrador `desKey` resultante **nunca es consultado ni
   utilizado** por `ProtocolInvocationLauncherLoad`: los ficheros leídos se
   transmiten siempre en Base64 plano sin cifrar (ver secciones 6.3 y 7.1).

---

## 6. Operación `load`: Interfaz de usuario y lectura de ficheros

El flujo de ejecución de `load` en `ProtocolInvocationLauncherLoad.processLoad`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherLoad.java:55-152`)
sigue las siguientes etapas:

### 6.1 Enfoque en macOS

Al igual que en `save`, si el sistema operativo es macOS, se ejecuta
`MacUtils.focusApplication()` (`ProtocolInvocationLauncherLoad.java:91-93`)
para asegurar que el diálogo de apertura adquiera el foco sobre la ventana del
navegador.

### 6.2 Despliegue del selector interactivo `AOUIFactory.getLoadFiles`

Se invoca `AOUIFactory.getLoadFiles` (`afirma-core/src/main/java/es/gob/afirma/core/ui/AOUIFactory.java:264-284`),
que delega en `JSEUIManager.getLoadFiles`
(`afirma-ui-core-jse/src/main/java/es/gob/afirma/ui/core/jse/JSEUIManager.java:627-691`):

```java
// ProtocolInvocationLauncherLoad.java:95-103
selectedDataFiles = AOUIFactory.getLoadFiles(
    options.getTitle(),
    options.getFilepath(),
    null,
    options.getExtensions() != null ? options.getExtensions().split(",") : null,
    options.getDescription(),
    false,
    options.getMultiload(),
    DesktopUtil.getDefaultDialogsIcon(),
    null
);
```

#### Parámetros aplicados al diálogo Swing `JFileChooser`:
1. **Directorio y ruta inicial:** `configureDefaultDir` (`JSEUIManager.java:848-870`)
   asigna `options.getFilepath()`. Si `filePath` es nulo, recupera la preferencia
   persistida `current.dir`.
2. **Selección múltiple:** `jfc.setMultiSelectionEnabled(multiSelect)` activa o
   desactiva la selección de varios archivos según el valor del booleano `multiload`.
   El parámetro `selectDirectory` se fija en `false` (no se permite seleccionar
   carpetas).
3. **Filtro de extensiones:** A diferencia del error presente en `save`, en `load`
   las extensiones **sí se dividen correctamente por comas** mediante
   `options.getExtensions().split(",")` (`ProtocolInvocationLauncherLoad.java:98`).
   Si se definieron extensiones, se instala un `FileNameExtensionFilter(description, extensions)`.
4. **Almacenamiento del directorio:** Si el usuario pulsa `Aceptar`
   (`JFileChooser.APPROVE_OPTION`), la ruta de la carpeta actual se guarda en las
   preferencias del sistema: `put(PREFERENCE_DIRECTORY, jfc.getCurrentDirectory().getPath())`.
5. **Cancelación:** Si el usuario pulsa `Cancelar` o cierra la ventana,
   `JSEUIManager.java:690` lanza `AOCancelledOperationException`.

### 6.3 Lectura binaria y codificación en memoria

Tras la selección, `selectedDataFiles` contiene los objetos `File[]` elegidos.
`ProtocolInvocationLauncherLoad.java:118-140` itera secuencialmente sobre cada archivo:

1. Abre un flujo de entrada `FileInputStream` envuelto en un `BufferedInputStream`.
2. Lee la totalidad de los bytes del fichero mediante `AOUtil.getDataFromInputStream(bis)`.
3. Si la lectura produce `null`, lanza `IOException("La lectura de datos para cargar ha devuelto un nulo")`.
4. Codifica el array binario en Base64 estándar usando `Base64.encode(data)`.
   Los datos se codifican directamente en claro sin aplicar cifrado simétrico
   (ni DES ni AES), con independencia de si se especificó el parámetro `key`.
5. Si ocurre cualquier fallo de acceso al sistema de ficheros o error de memoria,
   captura la excepción y lanza el error `SAF_25` (`ERROR_CANNOT_LOAD_DATA`).

---

## 7. Formato de la respuesta en `load` y entrega por transporte

### 7.1 Estructura textual de la respuesta

La respuesta generada por `ProtocolInvocationLauncherLoad.java:132-139` es un texto
estructurado delimitado por caracteres especiales fijos:

* **Separador de fichero y contenido:** dos puntos `:` (`LOAD_SEPARATOR = ':'`, línea 34).
* **Separador de elementos en selección múltiple:** barra vertical `|` (`MULTILOAD_SEPARATOR = '|'`, línea 36).

```
<nombre_fichero_1>:<base64_1>[|<nombre_fichero_2>:<base64_2>...]
```

#### Ejemplo de respuesta simple (`multiload=false`):
```text
contrato.pdf:JVBERi0xLjQKJcOkw7zDtsOfCjIgMCBvYmoKPDwvTGVuZ3RoIDM...
```

#### Ejemplo de respuesta múltiple (`multiload=true`):
```text
anexo1.pdf:JVBERi0xLjQK...|datos.xml:PD94bWwgdmVyc2lvbj0iMS4wIi...|factura.xsig:MIIH...
```

*Nota sobre seguridad de la ruta:* `selectedDataFiles[i].getName()`
(`ProtocolInvocationLauncherLoad.java:132`) extrae exclusivamente el nombre simple
del fichero con su extensión; **nunca incluye la ruta absoluta ni el directorio
del sistema de ficheros local**, protegiendo la privacidad del árbol de directorios
del usuario.

*Nota sobre transmisión en texto plano y ausencia de cifrado:* A diferencia de las
operaciones de firma (`sign`, `cosign`, `countersign`) o de selección de
credenciales (`selectcert`), donde los datos sensibles devueltos se cifran
simétricamente con la clave `key` proporcionada por el llamante mediante DES-ECB,
en `load` los contenidos de los ficheros leídos del disco del usuario se devuelven
invariablemente en Base64 plano sin cifrar. No existe soporte en el código de
`ProtocolInvocationLauncherLoad` ni en los métodos receptores de `autoscript.js`
para descifrar ficheros cargados. Por tanto, la confidencialidad de la lectura
descansa por completo en el aislamiento del canal de comunicación local (socket TCP
o WebSocket vinculado a `127.0.0.1`).

### 7.2 Entrega por Socket local (`afirma://service`)

En el transporte por socket HTTP local:
1. `CommandProcessorThread.java:308-310,348-353` ejecuta
   `ProtocolInvocationLauncher.launch` y obtiene la cadena estructurada.
2. Al no tratarse de un comando `save`, la respuesta entra en la rama estándar
   `calculateNumberPartsResponse(operationResult)`.
3. Si la longitud de la cadena excede el tamaño máximo del búfer de fragmento,
   se divide en partes numeradas.
4. AutoFirma responde con el número total de fragmentos (`parts`).
5. El cliente JavaScript solicita sucesivamente cada fragmento mediante `send=n` y
   reconstruye la cadena completa en memoria.

### 7.3 Entrega por WebSocket (`afirma://websocket`)

En el transporte WebSocket, `AfirmaWebSocketServer.java:113` y
`AfirmaWebSocketServerV4.java:91` transmiten la cadena completa devuelta por
`launch()` en una única trama de texto (*broadcast* hacia la conexión del cliente).

### 7.4 Incompatibilidad absoluta con Servidor Intermedio

La operación `load` **no funciona a través del transporte por servidor intermedio**:

1. `UrlParametersToLoad` (`UrlParametersToLoad.java`) no parsea ni almacena
   `stservlet` ni `id`.
2. `ProtocolInvocationLauncherLoad.processLoad` devuelve la cadena con los datos,
   pero **nunca efectúa ninguna llamada para subirla a un servidor HTTP**.
3. En `ProtocolInvocationLauncher.java:797`, el método `launch` retorna la cadena a
   `SimpleAfirma.java:962`, que ignora el valor devuelto y llama de inmediato a
   `forceCloseApplication(0)` (`SimpleAfirma.java:979`), finalizando la JVM sin
   haber transmitido los datos cargados a ninguna parte.
4. Si se produce un error o cancelación y se lanza `SocketOperationException`,
   `ProtocolInvocationLauncher.java:808` intenta notificar el fallo invocando:
   ```java
   sendDataToServer(msg, params.getStorageServletUrl().toString(), params.getId());
   ```
   Como `params.getStorageServletUrl()` es incondicionalmente `null`, esta línea
   arroja de inmediato un `NullPointerException`.
5. Por este motivo, el cliente oficial `autoscript.js:4149,4163` bloquea de raíz
   la ejecución de `load` cuando se utiliza el modo de servidor intermedio, lanzando
   directamente una excepción en JavaScript sin llegar a invocar a AutoFirma:
   ```javascript
   // autoscript.js:4149-4156
   var errorType = "java.lang.UnsupportedOperationException";
   var errorMessage = "La operacion de carga de fichero no esta disponible por servidor intermedio";
   errorCallback(errorType, errorMessage);
   ```

---

## 8. Integración en el cliente JavaScript de referencia (`autoscript.js`)

El script de integración web `autoscript.js` expone tres funciones públicas para
estas operaciones en los objetos `AppAfirmaJS`, `AppAfirmaJSSocket` y
`AppAfirmaJSWebSocket`:

### 8.1 API JavaScript disponible

```javascript
// Guardado de datos en disco
AutoScript.saveDataToFile(
    dataB64,                  // Datos en Base64 a guardar
    title,                    // Título del diálogo (opcional)
    filename,                 // Nombre de fichero por defecto
    extension,                // Extensiones permitidas (exts)
    description,              // Descripción del tipo de archivo
    successCallback,          // Callback function(result)
    errorCallback             // Callback function(errorType, errorMessage)
);

// Carga de un único fichero
AutoScript.getFileNameContentBase64(
    title,                    // Título del diálogo
    extensions,               // Extensiones permitidas separadas por comas
    description,              // Descripción textual del tipo de fichero
    filePath,                 // Ruta de fichero o carpeta por defecto
    successCallback,          // Callback function(filename, dataB64)
    errorCallback             // Callback function(errorType, errorMessage)
);

// Carga múltiple de ficheros
AutoScript.getMultiFileNameContentBase64(
    title,                    // Título del diálogo
    extensions,               // Extensiones permitidas separadas por comas
    description,              // Descripción textual del tipo de fichero
    filePath,                 // Ruta de fichero o carpeta por defecto
    successCallback,          // Callback function(filenamesArray, dataB64Array)
    errorCallback             // Callback function(errorType, errorMessage)
);
```

### 8.2 Procesamiento de la respuesta de carga (`processLoadResponse`)

En `autoscript.js:2377-2434`, la función `processLoadResponse(data, multi)` procesa
el texto recibido:

```javascript
// autoscript.js:2385-2430
if (data.indexOf(":") <= 0) {
    processErrorResponse("java.lang.Exception", "Respuesta no valida");
    return;
}

var fileNamesDataBase64 = data.split("|");

if (!multi) {
    var sepPos = fileNamesDataBase64[0].indexOf(":");
    filenames = fileNamesDataBase64[0].substring(0, sepPos);
    datasB64 = fileNamesDataBase64[0].substring(sepPos + 1).replace(/\-/g, "+").replace(/\_/g, "/");
    responseSuccessCallback(filenames, datasB64);
}
else {
    var filenames = new Array();
    var datasB64 = new Array();
    for (i = 0; i < fileNamesDataBase64.length; i++) {
        var sepPos = fileNamesDataBase64[i].indexOf(":");
        filenames.push(fileNamesDataBase64[i].substring(0, sepPos));
        datasB64.push(fileNamesDataBase64[i].substring(sepPos + 1).replace(/\-/g, "+").replace(/\_/g, "/"));
    }
    responseSuccessCallback(filenames, datasB64);
}
```

* **Diferenciación de tipos en el callback:**
  * En carga simple (`multiload=false`), los argumentos entregados a
    `successCallback` son dos cadenas de texto primitivas: `(filename, dataB64)`.
  * En multicarga (`multiload=true`), los argumentos entregados son dos vectores
    paralelos: `(filenamesArray, datasB64Array)`.
* **Normalización de Base64:** `autoscript.js` reemplaza sistemáticamente los
  caracteres seguros para URL (`-` y `_`) generados por AutoFirma por los
  caracteres estándar de Base64 (`+` y `/`).

### 8.3 Incoherencia de nombres de parámetros en `AppAfirmaJSWebService.saveDataToFile` (BUG-19)

En la implementación para servidor intermedio `AppAfirmaJSWebService`
(`autoscript.js:4122-4123`), los parámetros de extensión y descripción se
configuran erróneamente con nombres no reconocidos:
```javascript
params[params.length] = {key:"extension", value:extension};
params[params.length] = {key:"description", value:description};
```
A diferencia de las variantes de WebSocket (`AppAfirmaWebSocketClient`, línea 2057)
y de socket HTTP local (`AppAfirmaJSSocket`, línea 3580) que empaquetan correctamente
`"exts"` y `"desc"`, `AppAfirmaJSWebService` genera `"extension"` y `"description"`.
Dado que `UrlParametersToSave.java:24,30` busca exclusivamente `FILENAME_EXTS_PARAM = "exts"`
y `FILETYPE_DESCRIPTION_PARAM = "desc"`, estos parámetros son ignorados de forma
silenciosa. Como consecuencia, al invocar `saveDataToFile` a través de servidor
intermedio, el diálogo nativo de guardado se abre sin filtros de extensión ni
descripción — ver detalle y mecanismo en [BUG-19 en A1-bugs-autofirma.md](A1-bugs-autofirma.md#bug-19-discrepancia-de-nombres-de-parámetros-extensiondescription-vs-extsdesc-en-appafirmajswebservicesavedatatofile-ignora-los-filtros-en-servidor-intermedio).

---

## 9. Catálogo completo de errores de `save` y `load`

La siguiente tabla recoge todos los códigos de error susceptibles de ser emitidos
durante la ejecución de las operaciones `save` y `load`, clasificados por su origen
en `ProtocolInvocationLauncherErrorManager`
(`afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherErrorManager.java:31-84,87-120`)
y `protocolmessages.properties`:

| Código | Constante interna | Operación | Causa técnica en el código | Mensaje mostrado al usuario / devuelto |
|---|---|---|---|---|
| `SAF_01` | `ERROR_NULL_URI` | `save` / `load` | La URI de invocación recibida en `args[0]` es `null`. | Protocolo no soportado / URI nula (`ProtocolLauncher.1`). |
| `SAF_02` | `ERROR_UNSUPPORTED_PROTOCOL` | `save` / `load` | La URI no comienza por el esquema `afirma://`. | Formato de llamada incorrecto (`ProtocolLauncher.2`). |
| `SAF_03` | `ERROR_PARAMS` | `save` / `load` | Parámetros obligatorios ausentes (p. ej. ni `dat` ni `fileid` en `save`), sintaxis de URL no válida, caracteres prohibidos en `filename` o `exts`, `id` no alfanumérico (reportando el mensaje literal «El identificador de la firma debe ser alfanumerico.» en `UrlParametersToSave.java:162`), o clave `key` con longitud distinta de 8 caracteres. | Error en los parámetros de entrada (`ProtocolLauncher.3`). |
| `SAF_05` | `ERROR_CANNOT_SAVE_DATA` | `save` | Excepción de E/S o fallo al escribir los bytes en disco en `ProtocolInvocationLauncherSave.java:102`. | No se ha podido guardar los datos (`ProtocolLauncher.5`). |
| `SAF_11` | `ERROR_SENDING_RESULT` | `save` | Fallo de conexión de red al enviar el resultado `OK` al servlet de almacenamiento en `ProtocolInvocationLauncherSave.java:124`. | Error en el envío del resultado de la operación (`ProtocolLauncher.11`). |
| `SAF_13` | `ERROR_LOCAL_ACCESS_BLOCKED` | `save` | Se especificó un servlet con dirección `localhost` o `127.0.0.1`, bloqueado por seguridad en `UrlParameters.java:370`. | Se ha pedido acceso a una dirección local, pero por seguridad se ha bloqueado el acceso (`ProtocolLauncher.13`). |
| `SAF_14` | `ERROR_OBSOLETE_APP` | `save` / `load` | Parámetros que requieren una versión de protocolo superior a la soportada por la versión actual. | La aplicación está obsoleta y no puede procesarse la petición (`ProtocolLauncher.14`). |
| `SAF_15` | `ERROR_DECRYPTING_DATA` | `save` | Fallo criptográfico al descifrar los datos descargados de `rtservlet` con la clave DES `key`. | Error en el descifrado de los datos (`ProtocolLauncher.15`). |
| `SAF_16` | `ERROR_RECOVERING_DATA` | `save` | Fallo HTTP o de conexión al intentar descargar los datos remotos de `rtservlet` indicados por `fileid`. | Error al recuperar los datos del servidor intermedio (`ProtocolLauncher.16`). |
| `SAF_21` | `ERROR_UNSUPPORTED_PROCEDURE` | `save` / `load` | La versión de protocolo solicitada en `ver` o `v` es mayor que la máxima soportada (versión 4). | La versión de Autofirma instalada no es compatible con este trámite (`ProtocolLauncher.21`). |
| `SAF_25` | `ERROR_CANNOT_LOAD_DATA` | `load` | Excepción al leer físicamente el fichero seleccionado en disco mediante `FileInputStream` en `ProtocolInvocationLauncherLoad.java:143`. | Error en la lectura de los datos a cargar (`ProtocolLauncher.35`). |
| `SAF_41` | `ERROR_MINIMUM_VERSION_NON_SATISTIED` | `save` / `load` | El parámetro `mcv` exige una versión de AutoFirma estrictamente superior a la versión actual de la aplicación. | El uso de este trámite web requiere una versión más reciente de Autofirma (`ProtocolLauncher.53`). |
| `CANCEL` | `RESULT_CANCEL` | `save` / `load` | El usuario pulsó el botón Cancelar o cerró la ventana del diálogo selector de ficheros. | `CANCEL` (sin código `SAF_`, interceptado por `autoscript.js` para emitir `AOCancelledOperationException`). |
| `MEMORY_ERROR` | `MEMORY_ERROR` | `save` | La JVM se quedó sin memoria intentando procesar un fichero de tamaño excesivo. | El fichero que se pretende firmar o guardar excede de la memoria disponible para la aplicación. |

