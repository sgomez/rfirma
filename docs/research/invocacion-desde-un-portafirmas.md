# Cómo llama un portafirmas de la Administración a la aplicación de firma (issue #2165)

## La pregunta

Cuando una persona firma en un portafirmas de la Administración y la firma falla en rFirma, pero
sale con AutoFirma, ¿qué le llegó a la aplicación por `afirma://`? Hay que saber qué operación
usa (lote remoto con `batchpresignerurl`/`batchpostsignerurl`, `sign` con servidor trifásico u
otra), si el lote es XML o JSON, qué parámetros lleva, por qué transporte llega y si cambia según
la versión o el despliegue. Con eso se decide si la suite de conformidad ya reproduce el flujo o
le falta un escenario.

Hay más de un portafirmas, y no son el mismo producto:

* **Portafirmas de la AGE** (SGAD, solución 132 del CTT). Se publica como distribuible para que
  otros organismos lo desplieguen, y la SGAD lo opera en `pf.redsara.es` (antes
  `pf.seap.minhap.es`) y en `portafirmas.redsara.es`.
* **Port@firmas de la Junta de Andalucía** (Agencia Digital de Andalucía, v3.5), con su propio
  manual de integración.
* **Las apps móviles del portafirmas de la AGE** (Android, iOS, «Mi Portafirmas»), que no pasan
  por `afirma://`: se explica más abajo.

## Fuentes y su alcance

| Fuente | Qué es | Cómo se obtuvo |
|---|---|---|
| `ctt-gob-es/clienteafirma`, `master` en `bd9254be70004be095b1251b5a4af15b9d392dd9` (19-10-2026) | El código de AutoFirma y de su `autoscript.js` (`afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js`, `VERSION = "1.10.1"`) | `git clone` |
| `ctt-gob-es/clienteafirma-docs` en `c8d1ec4c2fa991e897839e6512e06aaedfcf0d62` | Manuales del CTT, entre ellos los del portafirmas móvil (`PF_*`) y el *whatsnew* de AutoFirma | `git clone` |
| `ctt-gob-es/portafirmas-proxy` en `714080af5165d4cb88172f281cd5b5594ee84cf9` | El servicio intermedio de las apps móviles (`afirma-signfolder-proxy`) | `git clone` |
| Lista `portafirmas-avisos` del CTT, <https://listas-ctt.administracionelectronica.gob.es/pipermail/portafirmas-avisos/> | Los avisos de versión que publica el equipo de Portafirmas de la AGE | Archivos mensuales, descargados el 24-10-2026 |
| Página de acceso de <https://portafirmas.redsara.es/pf/> | La portada pública de la instancia SARA, que declara versión y requisitos | Descargada el 24-10-2026 |
| `https://ws024.juntadeandalucia.es/afirma-server-html/js/batchscript.js` y `.../autoscript.js` | Los scripts corporativos de firma de la Junta de Andalucía, servidos en producción | Descargados el 24-10-2026; `sha256` `34462c0a…c8af` y `f5551fac…dd05` |
| «Plataforma @firma/Autofirma. Manual de Integración Firma por Lotes», v01r001, 11-08-2025 (Junta de Andalucía), <https://desarrollo.juntadeandalucia.es/sites/default/files/2026-01/20250811-Autofirma-Manual%20del%20Integraci%C3%B3n%20de%20firma%20por%20lotes.pdf> | El manual que documenta `batchscript.js` | Descarga directa |
| «Manual de integración Port@firmas v3.5», v01r03, 20-09-2024 (Junta de Andalucía) | Los servicios web de Port@firmas de la Junta para las aplicaciones que le envían peticiones | Descarga directa |
| `portafirmas-src` 7.10.8 (`whatsNew_7.10.8.txt:1`), el código fuente del distribuible del Portafirmas de la AGE | Su interfaz web, su `autoscript.js`, su configuración por defecto y sus scripts de base de datos | Descargado por el titular del área de descargas del PAe; leído como datos, sin compilar ni ejecutar |
| `ctt-gob-es/clienteafirma`, etiquetas `Version_1.7` (`1c35e0d0e103c50317f4f94abb5e2920e9904078`) y `v1.7.2` (`a20146191a036c50bbc2233b83b07da27c75b355`) | El `autoscript.js` publicado con el que se compara el del Portafirmas | El mismo clon, por etiqueta |

**Del Portafirmas web de la AGE se ha leído el código de la 7.10.8, y solo de esa.** El aviso del
27-10-2021 («Publicado distribuible 7.6») dice que el código fuente y el distribuible están en el
área de descargas de la solución en el PAe
(`administracionelectronica.gob.es/ctt/verPestanaDescargas.htm?idIniciativa=portafirmas`), que
contesta con un desafío anti-bot que no se puede pasar desde aquí; el titular descargó de allí el
zip de la 7.10.8. La 7.10 es anterior al lote de peticiones de la 7.12 y al cambio de la llamada a
AutoFirma de 2025-11, así que lo que sigue de su código no se extrapola a la 7.14 que sirve hoy
`portafirmas.redsara.es`. En GitHub, `ctt-gob-es/Portafirmas` es un repositorio vacío.

En las citas del distribuible, `webapp/` es `fuentes/portafirmas/src/main/webapp/`, `java/` es
`fuentes/portafirmas/src/main/java/es/seap/minhap/portafirmas/` y `datos.sql` es
`resources/script BBDD/creacion/5_script_datos.sql`, todo relativo a la raíz del zip. El árbol
`fuentes/portafirmas/bin/` repite `webapp/js/` sin cambios.

## El Portafirmas web de la AGE

Lo que sí consta en fuentes primarias:

* **Firma con AutoFirma, sin applet**, desde la v7.1: «Se desactiva el uso de applets en
  Internet Explorer. A partir de ahora, todas las firmas en cliente se realizarán con
  Autofirma» (aviso «[Portafirmas AGE - PRO] Subida versión 7.1», 05-11-2019).
* **Firma trifásica en la web** desde el distribuible 7.6: «Mejorado el flujo de Autofirma» y
  «Firma trifasica en portafirmas web» (aviso «[Portafirmas PRO]», 21-01-2022).
* **Lote de peticiones** desde la v7.12: «Nueva funcionalidad de firmado de lote de peticiones»,
  con el requisito de «la versión 1.8.0 o superior de Autofirma» (avisos del 17-09-2024 y del
  06-11-2024 sobre la v7.12).
* **Cambios en la llamada a AutoFirma sin versión nueva**: «actualización de su llamada a
  Autofirma» en Portafirmas AGE el 12-11-2025 (aviso del 11-11-2025). El aviso no dice qué cambió.
* **Versión y requisitos de hoy**: la portada de `portafirmas.redsara.es/pf/` declara
  «Portafirmas Versión: 7.14» y, en requisitos, «Autofirma 1.8.2». Allí se entra solo por Cl@ve
  (`/pf/login/accesoRedirectClave`, Autentica); el texto avisa de que «el certificado elegido al
  ingresar en la aplicación será el utilizado en las operaciones de firma». La portada no carga
  `autoscript.js`: el script de firma va en las páginas de dentro, a las que se llega con sesión.

### Lo que hace el distribuible 7.10.8

* **Una serie de operaciones sueltas, sin lote.** La bandeja firma las peticiones de una en una
  (`webapp/js/inbox/inbox.js:402-421`, con `nRequestLaunch < 1`) y, dentro de cada petición, los
  documentos de uno en uno: el siguiente sale al volver el anterior
  (`webapp/js/autoscript1.7.0/firma.js:27-88`). Por documento llama a `AutoScript.sign`,
  `coSign` o `counterSign` según la operación que decide el servidor (`firma.js:55-67`). Esa
  operación es `sign` si el documento no trae firmas, `countersign` si la petición es en cascada y
  el documento ya es una firma XAdES o CAdES, y `cosign` si es en paralelo
  (`java/business/SignBO.java:267-301`). Un formato con `PDF` sale siempre como `sign`, aunque el
  servidor pida otra cosa (`firma.js:55`). No hay `batch`, ni `batchpresignerurl` ni
  `batchpostsignerurl` en todo `webapp/`.
* **Antes de la serie, un `sign` para saber el certificado.** Si la sesión no tiene todavía el
  número de serie del certificado (`inbox.js:1645-1652`), `obtenerCertificado` firma un número
  aleatorio con `AutoScript.sign(…, "SHA1withRSA", "XAdES", null, …)`: sin `properties`, así que
  sin filtros ni `headless` (`webapp/js/autoscript1.7.0/authenticator.js:2-5`). El certificado que
  vuelve va al servidor, que lo valida, comprueba que es de la persona de la sesión y guarda su
  número de serie en hexadecimal (`inbox.js:338-364`;
  `java/web/controller/InboxController.java:674-782`). Ese número dura lo que la sesión.
* **Filtro `qualified:` en cada firma de la serie.** Cada `sign`, `cosign` o `countersign` añade a
  los `extraParams` la línea `filter=qualified:<número de serie>`: la clave en singular, con el
  comentario «Como indicas un sólo certificado, no te lo vuelve a pedir» (`firma.js:36-37`).
* **`sticky` siempre, por un cambio en el `autoscript.js`.** El Portafirmas no llama a
  `setStickySignatory` ni a `setResetStickySignatory`: cambia el valor inicial en su copia del
  script (`autoscript.js:34`, `var stickySignatory = true;`), y así todas las peticiones salen con
  `sticky=true` (`autoscript.js:930, 946, 974, 993`). La nota de la 7.9 lo explica: «Firma masiva
  de peticiones con tarjeta electronica (Revisar parametro AutoScript.setStickySignatory(true); en
  llamada a Autofirma)» (`whatsNew_7.10.8.txt:72`). `resetsticky` solo saldría tras un
  `setKeyStore` o un `setStickySignatory(false)` (`autoscript.js:346-351, 426-433`), que la página
  no llama. El primer `sign`, el del número aleatorio, es el que fija el certificado en AutoFirma, y
  los de la serie lo reutilizan.
* **`headless`.** `headless=true` entra cuando la petición tiene más de un documento o la persona no
  tiene permitido aceptar firmas no válidas (`java/utils/UtilComponent.java:289-290`; el valor, en
  `resources/potafirmasConf/properties/autofirma.properties:2`).
* **Los demás `extraParams`** los monta `UtilComponent.getSignatureParameters`
  (`UtilComponent.java:287-351`) con los valores de `autofirma.properties:1-16`:
  `expPolicy=FirmaAGE`; `mimeType=<tipo>` en XAdES implícita; `mode=explicit` en XAdES explícita;
  `mode=implicit` en CAdES; `signatureSubFilter=ETSI.CAdES.detached` en PAdES; nada en FacturaE; y,
  si la petición pide firma visible, el recuadro en la página 1 con `layer2Text`. En XAdES
  Enveloping y Enveloped no va la política: la cadena se reinicia a `null` y se le concatena el
  resto (`UtilComponent.java:305-322`), así que la primera línea empieza por el literal `null`
  (`nullheadless=true` o, sin `headless`, `nullformat=XAdES Enveloping`). Al final va la línea
  `CSVStorage.properties`, sin `=` (`autofirma.properties:16`, `UtilComponent.java:347-348`). En
  XAdES, la persona puede añadir `signerClaimedRoles=<comentario>` (`firma.js:38-40`).
* **Formato y algoritmo.** El formato sale de la configuración de la aplicación que envió la
  petición: `XAdES`, `CAdES`, `PAdES` o `FacturaE` (`UtilComponent.java:354-372`), y la de por
  defecto es `PDF`, que pasa a XAdES implícita si el documento no es un PDF (`datos.sql:774`;
  `SignBO.java:323-329`). El algoritmo es `SHA256withRSA`, salvo que la configuración diga SHA-384
  o SHA-512 (`UtilComponent.java:386-394`; `datos.sql:770`). `dat` lleva el documento o la firma
  previa en Base64 (`FIRMA.SIGNATURE.MODE=BINARIO` por defecto, `datos.sql:825`), que el navegador
  baja antes de su propio servidor (`firma.js:44-49, 112-130`).
* **Transporte, sin forzar.** La página llama a `AutoScript.cargarAppAfirma(baseDownloadURL, null)`
  (`inbox.js:206`) y no llama a `setForceWSMode` ni a `setServlets`. En el `autoscript.js` 1.7.0
  la cascada da WebSocket a cualquier navegador que lo soporte y no sea Internet Explorer, con
  `afirma://websocket?v=3` (`autoscript.js:564-583, 737, 1089`); servidor intermedio solo en iOS y
  Android, o en navegadores sin WebSocket que tampoco admitan el socket. En ese caso los servlets
  se deducen del origen de la página: `<origen>/afirma-signature-storage/StorageService` y
  `<origen>/afirma-signature-retriever/RetrieveService` (`autoscript.js:2628-2637`). La página no
  manda `appname`: el 1.7.0 no lo conoce.
* **La firma con FIRe** es otro botón (`inbox.js:67-85, 746-757`) y no pasa por `afirma://`.

**El `autoscript.js` es el 1.7.0 del CTT con una línea cambiada.** Declara `VERSION = "1.7.0"`
(`autoscript.js:17`). Comparado con el de `ctt-gob-es/clienteafirma` en `Version_1.7` y en
`v1.7.2` (iguales entre sí en ese fichero), y sin contar finales de línea, la única diferencia es
la línea 34: `stickySignatory = true` en el del Portafirmas, `false` en el del CTT.

### La «firma trifásica en portafirmas web»

Es un `sign` (o `cosign`, `countersign`) suelto con formato `*tri` y `serverUrl` en los
`extraParams`, contra el servicio de firma trifásica del Cliente @firma, no un lote. La activa el
parámetro global `FIRMA.TRIFASICA.ACTIVA`, que nace a `N` (`datos.sql:2208-2212`;
`java/utils/Constants.java:240-242`). Con él activo, al cargar las peticiones
(`InboxController.java:642-643`) `comprobarFirmaTrifasica` cambia cada documento
(`SignBO.java:1425-1436`):

* añade el sufijo `tri` al formato (`XAdEStri`, `CAdEStri`, `PAdEStri`, `FacturaEtri`);
* añade `serverUrl=<FIRMA.TRIFASICA.URL>` a los `extraParams`, el parámetro «donde se despliegue el
  SFT», que nace a `-` (`datos.sql:2217-2221`);
* pone el modo `HASH`, con lo que `firma.js:45-46` pasa `documento.data` sin bajar nada.

En ese modo `documento.data` no es el documento. El servidor escribe el documento, o la firma
previa, en crudo en la carpeta `FIRMA.TRIFASICA.PATH_DOC` (con `SGTICCONFIGPATH` delante si
`FIRMA.TRIFASICA.ANADIRSGTIC=S`) y pasa como datos el nombre del fichero,
`<sesión>-<huella>-<instante>.data` (`SignBO.java:651-700, 716-772`;
`datos.sql:2229-2251`). Así que `dat` lleva ese nombre tal cual, sin Base64. AutoFirma toma como
texto un `dat` que no es Base64 (el `.` lo descarta), con sus octetos (`clienteafirma`
`Version_1.7`, `afirma-core/…/DataDownloader.java:109-124`), y la firma trifásica lo manda al
servidor como identificador del documento. El `FileSystemDocumentManager` del servidor trifásico de referencia
lee el fichero con ese nombre de su carpeta de entrada y devuelve el nombre del fichero de firma en
Base64 (`clienteafirma` `Version_1.7`,
`afirma-server-triphase-signer/…/FileSystemDocumentManager.java:54-58, 100-145`). El Portafirmas
descodifica ese resultado y lee la firma de `FIRMA.TRIFASICA.PATH_FIRMAS` (`SignBO.java:483-499,
897-900`). El Portafirmas y el servicio trifásico comparten, pues, el sistema de ficheros; qué
gestor de documentos usa de verdad un despliegue es configuración del servicio, que no va en el
zip.

En las notas de versión del distribuible, «Firma trifasica en portafirmas web» figura en la 7.4
(`fuentes/portafirmas/InfoVersiones.txt:323-325`), y el aviso de la lista la anuncia con el
distribuible 7.6.

### Lo que sigue sin constar (7.12 y posteriores)

* qué operación manda la v7.12 para el lote de peticiones (`batch` con `jsonbatch=true`, `batch`
  XML o una serie de `sign`). AutoFirma 1.8 trae «un nuevo mecanismo de firma de lotes más
  óptimo y fácil de usar» y «un nuevo cojunto de funciones para la firma de lotes»
  (`clienteafirma-docs/AF_whatsnew_ES.txt:82,91`), y el lote del Portafirmas exige esa versión
  justo a la vez. Eso apunta al lote JSON, pero es una coincidencia de fechas, no una prueba;
* si la 7.12 y la 7.14 conservan la serie, el `sign` previo sin filtros, el filtro `qualified:`,
  el `sticky` por defecto y la trifásica por nombre de fichero de la 7.10.8;
* qué cambió en la llamada a AutoFirma del 12-11-2025.

Con AutoFirma 1.8.2 como requisito, el `autoscript.js` de la 7.14 será de la rama 1.8 o posterior,
y no el 1.7.0 de la 7.10.8. La forma de la URL que construye esa rama se describe a continuación.

## Lo que hace `autoscript.js` con un lote y con una firma suelta

Esto vale para cualquier portafirmas que use el cliente JavaScript del CTT, y ya está documentado
en el manual de referencia. Aquí se resume:

* **Lote remoto JSON.** `createBatch`/`addDocumentToBatch` montan el objeto con `algorithm`,
  `format`, `suboperation`, `extraparams` en Base64 y `singlesigns[]` con `id` y `datareference`;
  `signBatchProcess` es `signBatchJSON` (`autoscript.js:727-772,5671`). La petición es
  `op=batch` con `batchpresignerurl`, `batchpostsignerurl`, `properties` (los filtros de
  certificado en Base64), `ksb64`, `sticky`, `resetsticky` si toca, `appname`, `needcert=true`,
  `dat` (el lote en Base64), `jsonbatch=true`, `localBatchProcess` si toca y `servicetimeout` si se
  fijó (`autoscript.js:2319-2342`). El contrato del lote y de los servlets está en
  [08 §6](../afirma/1.9.2/08-operacion-batch.md); en el servidor de referencia, los servlets
  son `JSONBatchPresigner` en `/presign` y `JSONBatchPostsigner` en `/postsign`
  (`afirma-server-triphase-signer/src/main/webapp/WEB-INF/web.xml:23-39`).
* **Lote remoto XML**, el heredado: el mismo `op=batch` sin `jsonbatch`
  ([08 §5](../afirma/1.9.2/08-operacion-batch.md)).
* **Firma suelta trifásica**: `sign` con formato `CAdEStri`, `XAdEStri`, `PAdEStri` o
  `FacturaEtri` y `serverUrl` en los `extraParams`
  ([06 §6.3](../afirma/1.9.2/06-operaciones-firma.md)).
* **Transporte.** Lo decide `cargarAppAfirma` una vez, al cargar: servidor intermedio si
  `forceWSMode` o si el sistema es iOS o Android; si no, WebSocket cuando el navegador lo soporta y
  no es Internet Explorer ni Firefox 60 o anterior; si no, socket local `afirma://service`; y, en
  navegadores muy antiguos, servidor intermedio (`autoscript.js:1086-1125`;
  [16 §2](../afirma/1.9.2/16-cliente-javascript.md)). Ni el portafirmas ni la operación lo
  eligen después.
* **`sticky`.** Por WebSocket y socket viaja como `sticky=true` y lo guarda el proceso vivo de
  AutoFirma; por servidor intermedio lo imita el script, que reescribe los `extraParams` con
  `filters=encodedcert:<certificado>` y `headless=true`
  ([16 §9.1](../afirma/1.9.2/16-cliente-javascript.md)). En AutoFirma, un `sign` con `sticky`
  fija la clave que se eligió (`ProtocolInvocationLauncherSign.java:686`), y el siguiente `sign`
  con `sticky` y sin `resetsticky` la reutiliza sin preguntar (`:557-560`).

## Un despliegue real con código público: los scripts de la Junta de Andalucía

Es el único despliegue de una Administración cuyo código de firma se ha podido leer entero. **No
es el Port@firmas de la Junta**: el manual de integración de Port@firmas v3.5 trata los servicios
web con que las aplicaciones le envían peticiones y no describe cómo firma su interfaz. Pero es
un cliente corporativo de firma múltiple, de los que usa un portafirmas, y enseña que el mismo
botón produce operaciones distintas según el dispositivo.

`batchscript.js` (`VERSION : "1.0.1"`) expone `doSignBatch(stopOnError, commonAlgorithm,
commonFormat, commonSuboperation, commonExtraparams, certFilters, documentArray, formatArray,
suboperationArray, extraparamArray, …)` (líneas 58-171), y lo documenta el manual de firma por
lotes de la Junta (§2). Se apoya en un `autoscript.js` que declara `VERSION = "1.8.2.1"` y
`VERSION_CODE = 2` (líneas 17-18).

* **En escritorio, una serie de `sign` con `sticky`, no un lote.** Si no es iOS ni Android,
  `doSignBatch` llama a `doMassiveSign` (líneas 140-164). Esa función activa
  `AutoScript.setStickySignatory(true)` (línea 447) y encadena un `AutoScript.sign` (o `coSign` o
  `counterSign`) por documento con el formato, el algoritmo y los `extraParams` comunes (líneas
  473-479). Al terminar desactiva el `sticky` (línea 486). En esta rama no se usan `certFilters`,
  `formatArray` ni `extraparamArray`. El manual lo justifica así: «en entornos de escritorio,
  donde la firma recursiva tiene mejor rendimiento» (§2). Con este `autoscript.js`, en escritorio
  el transporte es WebSocket v4 (`autoscript.js` de la Junta, 640-700 y 979), y cada `sign` lleva
  `sticky=true` (línea 1223).
* **En móvil, lote remoto JSON por servidor intermedio.** En iOS o Android sube antes cada
  documento a un `BatchStorageService` propio, dentro de `afirma-server-triphase-signer-2.7.3`
  (líneas 109 y 213-274). Con `createBatch`/`addDocumentToBatch` monta el lote usando el mismo
  UUID como `id` y como `datareference` (líneas 95-127). Después llama a `signBatchProcess` con
  `.../afirma-server-triphase-signer-2.7.3/presign` y `/postsign` (líneas 75-76 y 132-138),
  y recoge las firmas de un `BatchRetrieveService` propio (líneas 277-341). Los servlets del
  servidor intermedio son los de siempre (`afirma-signature-storage/StorageService`,
  `afirma-signature-retriever/RetrieveService`, líneas 78-79).

Este `autoscript.js` 1.8.2.1 no manda `appname` ni `servicetimeout` en el lote
(`createBatchRequest`, líneas 1264-1287), y el de `master` sí. Las sedes en producción no llevan
el script del CTT de la última versión: lleva el que cada organismo copió en su día.

## Las apps móviles del portafirmas no pasan por `afirma://`

Las apps de Android e iOS firman con el certificado instalado en el dispositivo, contra un
servicio intermedio (`afirma-signfolder-proxy.war`) que habla con el Portafirmas web y con el
servicio de firma trifásica del Cliente @firma
(`clienteafirma-docs/PF_manual-configuracion_servicios_ES.docx`, «Servicio Proxy» y «Servicio de
firma trifásica»; «Diagramas de secuencia de los procesos de firma móvil.docx», «Firma trifásica
en portafirmas»). El proxy llama él mismo a la prefirma y a la postfirma, con `op=pre`/`op=post`
y `cop=sign|cosign|countersign` (`portafirmas-proxy/.../TriSigner.java:44-57,145-166`). En ese
camino no hay aplicación de escritorio, así que no afecta a rFirma.

Los paquetes `es.gob.afirma.signfolder.server.proxy` de `afirma-signature-storage` y
`afirma-signature-retriever` (el servidor intermedio de AutoFirma) llevan el nombre de su origen,
el portafirmas móvil. El protocolo que implementan es el de
[03](../afirma/1.9.2/03-transporte-servidor-intermedio.md), el de cualquier sede.

## Lo que queda sin confirmar

* La operación, los parámetros y el formato del lote del Portafirmas web de la AGE (v7.12 y
  posteriores), y si la 7.14 sigue firmando como la 7.10.8. Para confirmarlo hace falta su código
  o una URL `afirma://` capturada en un trámite real, según
  [casos-de-sedes-reales](../casos-de-sedes-reales.md).
* Si la 7.12 o posteriores fuerzan el servidor intermedio (`setForceWSMode`) en escritorio. La
  7.10.8 no lo fuerza.
* Qué cambió en la «llamada a Autofirma» del 12-11-2025.
* Qué servicio trifásico, con qué gestor de documentos, despliega cada organismo con
  `FIRMA.TRIFASICA.ACTIVA=S`, y si la instancia de la SGAD lo tiene activo.
* Cómo firma la interfaz del Port@firmas de la Junta de Andalucía, y si usa `batchscript.js`.
* Si otros organismos que despliegan el distribuible de la AGE cambian su script de firma.

## Decisión sobre la suite de conformidad

La suite ya cubre los dos caminos posibles del lote:

* el lote remoto JSON y el XML contra `batchpresignerurl`/`batchpostsignerurl`, con la cadena al
  prefirmador, PK1 al postfirmador, `needcert`, la cookie entre fases y los fallos parciales
  (conjunto `lote`, `rfirma-conformance/catalogue/lote.toml`);
* el resultado del lote por servidor intermedio (`canal.intermedio`,
  `the_batch_result_reaches_the_site_through_the_intermediate_server`);
* la firma suelta trifásica de cada formato contra `serverUrl` (las celdas trifásicas de
  `catalogue/matrix.toml`);
* el `sticky` que fija `selectcert` y reutiliza la firma siguiente
  (`a_certificate_pinned_by_selectcert_signs_without_asking` en `certificado.toml`), su
  liberación y su indiferencia a los filtros
  (`a_pinned_certificate_ignores_the_filters_of_the_next_request`);
* el `sticky` que fija un `sign` sin `selectcert` previo
  (`a_signature_with_sticky_pins_the_certificate_for_the_next_signature`), que es lo que hacen en
  escritorio el cliente de la Junta y la 7.10.8 con su `sign` del número aleatorio;
* las piezas de cada firma de la serie de la 7.10.8: el filtro `qualified:`
  (`qualified_filter_selects_by_serial`), la clave `filter` en singular
  (`the_singular_filter_takes_precedence_over_filters`), XAdES con `SHA1withRSA`, `expPolicy` de
  la AGE, la cofirma y la contrafirma trifásicas y el canal WebSocket v3 (`canal.websocket`);
* un `dat` que no es Base64 se firma como los octetos de la cadena
  (`dat_that_is_not_base64_is_signed_literally`, en `peticion.toml`).

Al flujo de la 7.10.8 le falta una comprobación: **la firma trifásica cuyo `dat` es un
identificador del servidor y no el documento**. Las comprobaciones trifásicas mandan el documento
en Base64, y la del `dat` literal es monofásica. Ninguna mide que, con un formato `*tri`, un `dat`
que no es Base64 llegue al servidor trifásico como identificador del documento con los octetos de
la cadena, sin que el cliente lo trate como un PDF, un XML o una firma previa. Se propone añadir al
conjunto `firma`, sin implementarla aquí:

* `id`: `a_triphase_signature_sends_a_textual_dat_to_the_server_as_the_document_id`
* `statement`: «En una firma `*tri`, un `dat` que no es Base64 llega al `serverUrl` como
  identificador del documento con los octetos de la cadena, y la firma que vuelve es la que
  devuelve el servidor.»
* `drive`: modo `v3`, un `sign` con `PAdEStri` y otro con `XAdEStri`, `dat` igual a un nombre de
  fichero con `.data`, y un servidor trifásico de pruebas que solo conteste a ese identificador.
  La cita sale de `DataDownloader` y del firmador trifásico de cada formato en 1.9.2, que hay que
  leer antes de escribirla.

La serie entera de la 7.10.8 (el `sign` sin filtros, los `sign`, `cosign` o `countersign` con
`filter=qualified:`, `headless` y `sticky`, y las líneas `nullheadless=true` y
`CSVStorage.properties`) cabe como guion de sede escrita a mano, con el `autoscript.js` 1.7.0 y su
línea 34 cambiada. No se propone todavía: cada pieza ya tiene su comprobación, y el guion solo
valdría para la 7.10, que no es la que sirve la SGAD. Cuando llegue el informe del #2158 con la URL
capturada, se compara con lo de arriba: si la 7.14 hace lo mismo que la 7.10.8, el guion pasa a
tener sentido; si hace otra cosa, la URL redactada entra como guion de sede, según
[casos-de-sedes-reales](../casos-de-sedes-reales.md).
