# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).
No se reconstruye el histórico anterior a la v0.4.0: este fichero empieza
aquí (ID-153).

Desde la v0.11.2 cada sección la genera `just release <version>` a partir de
los títulos de las PR fusionadas en `main` desde la etiqueta anterior: los
`feat`, `fix` y `perf` que no son de ámbito interno.

## [0.12.0] - 2026-09-30

### Added
- Una firma de sede con SHA-1 se rechaza y la ventana explica por qué (#1208).
- Un lote con SHA-1 se rechaza explicado y rFirma deja de pedir SHA-1 al token (#1209).
- RFirma funciona en Windows, con su instalador y el almacén de certificados del sistema (#1160).
- Canal windows que comprueba la versión contra latest.json (#1239).
- Firma minisign del instalador de Windows en la Release (#1240).
- La clave pública minisign de las actualizaciones de Windows (#1241).
- El árbol del servidor sirve el instalador de Windows en /windows/ (#1242).
- Instalar la actualización de Windows desde el backend (#1243).
- «Actualizar ahora» en la franja de versión nueva y en Acerca de (#1244).

## [0.11.1] - 2026-09-29

### Fixed
- El lote remoto manda sus parámetros en el cuerpo del POST, como AutoFirma, y ya no falla con `SAF_03` en las sedes cuyos servlets solo leen el cuerpo (#1161).
- Un servicio de la sede con un certificado TLS que el sistema no reconoce
  falla con un error que lo dice y nombra el servidor, en vez de un fallo de
  conexión genérico (#1174).
- Los servicios de la sede (lote remoto, servidor trifásico, servidor
  intermedio y descarga de datos) ya no fallan a los 30 s si tardan en
  contestar: el límite es solo para conectar, y la espera de la respuesta no
  tiene límite, como en AutoFirma. Cerrar la ventana de sede cancela el
  trámite sin esperar (#1176).
- Cuando un servicio de la sede contesta con un error HTTP, el registro guarda
  el cuerpo de la respuesta además del código de estado (#1177).
- Los `properties` y `extraParams` de la sede se leen como `Properties.load`
  de Java: con continuación de línea (`\` al final), escapes `\uXXXX` y el
  espacio como separador entre clave y valor (#1179).
- rFirma guarda y reenvía las cookies de sesión entre las peticiones de una
  misma operación de sede, como AutoFirma, así que los servicios que dependen
  de ellas (por ejemplo, la afinidad de sesión de un balanceador) ya no fallan
  entre la prefirma y la postfirma del lote ni en el servidor intermedio
  (#1180).
- La ventana de sede nombra a quien pide la firma («sede.ejemplo.gob.es pide tu
  firma…») cuando la operación llega por el canal WebSocket desde una página
  `https`; antes decía siempre «Una página sin identificar» (#1185, #1186).

## [0.11.0] - 2026-09-28

### Added
- Aviso de página desactualizada cuando la sede declara `jvc` menor que 1; el
  trámite sigue al pulsar Entendido (#853).
- En el consentimiento de la ventana de sede, «Firmar» (o «Enviar mis datos»)
  nace desactivado con una cuenta atrás de tres segundos y después se lleva el
  foco, así que se firma con Intro. Se apaga en Preferencias › Firma con
  «Protección contra firmas accidentales» (#849).
- La ventana de sede se maneja con el teclado: Intro pulsa el botón por defecto
  de cada momento y Escape equivale a cancelar o cerrar donde hay ese botón
  (#965).
- La firma `CAdEStri` pasa por el `serverUrl` de la sede, con `SAF_03` sin
  servidor y `SAF_40` si el servidor falla (#911).
- La firma `PAdEStri`, `XAdEStri` y `FacturaEtri` pasa por el `serverUrl` de la
  sede, como ya lo hacía `CAdEStri` (#913).
- `signandsave` con un formato trifásico firma por el `serverUrl` de la sede y
  guarda la firma que devuelve el servidor (#913).
- La sede puede pedir el formato `NONE` (también `PKCS1` y `PKCS#1`) en `sign`,
  `signandsave` y el `format` del lote local: lo que vuelve es la firma PKCS#1
  de los datos con la clave del certificado, sin envoltorio CMS, y se hace en
  el token sin pasar por el puente. La cofirma y la contrafirma de `NONE` salen
  con `SAF_04`, como en AutoFirma (#916).
- El lote local admite `countersign` como suboperación: contrafirma el árbol con
  `target=tree` y las hojas en cualquier otro caso, también sin `target` (#918).
- Cuando la sede pide la firma visible con `visibleSignature`, la ventana de
  sede enseña el PDF para marcar dónde va la firma antes de elegir certificado
  (#909).
- Preferencia «Usar el certificado que elija la sede», apagada por
  omisión. Encendida, una petición con `headless=true` o
  `mandatoryCertSelection=false` y un solo certificado candidato se atiende sin
  elegirlo en la ventana, como AutoFirma: `selectcert` contesta sin ventana y
  las firmas y los lotes siguen con ese certificado. El PIN, elegir o guardar
  el fichero y el área de la firma visible se siguen pidiendo (#959).
- La sede que pide el almacén `PKCS11:<ruta>` ve solo los certificados de ese
  módulo si es uno de los que rFirma descubre, ahora también los registrados en
  p11-kit; si no lo es, recibe `SAF_08`. rFirma nunca carga un módulo porque lo
  nombre la sede (#930).
- El contenido de la firma visible se elige por modelo: *Completa* («Firmado
  por … el día … con un certificado emitido por …», la frase de AutoFirma) o
  *Solo rúbrica*, con una línea «Con rúbrica» común a todos los modelos
  (#969, #975).
- rFirma recuerda el modelo, la frase y «Con rúbrica» de la última firma
  visible, y el documento siguiente arranca con ellos (#971, #978).
- Un pie fijo que no se mueve al desplazar el panel: «Guardar en» enseña la
  carpeta y el nombre del fichero firmado, y un solo «Cambiar» elige los dos
  para esa firma (#970, #977).
- La fila de un documento reciente enseña debajo la carpeta en la que está
  (#996, #997).
- Si la firma falla, el panel lo dice, aclara que el documento sigue como
  estaba y ofrece «Reintentar» o «Volver» (#979).
- El modelo Personalizada de la firma visible: una frase editable con el firmante, el emisor y la fecha como pastillas que se borran y se mueven como una palabra, insertadas desde «+ Dato» (#976).
- Al abrir un PDF que ya lleva firmas, el panel avisa «Firmarás junto a N firmas
  anteriores» y, al desplegarlo, dice quién firmó cada una y cuándo (#1010).
- Cada firma previa lleva su estado y su motivo: válida, certificado caducado,
  certificado aún no válido, rota, no se puede validar o no se ha podido
  comprobar del todo. Se comprueba sin consultar la red; una firma longeva con
  el certificado caducado cuenta como «no se ha podido comprobar del todo», y
  una firma añadida a un PDF certificado sin cambios permitidos, como rota
  (#1012, #1020, #1021, #1022, #1023, #1026).
- El aviso plegado suma los avisos («M avisos») y toma el color del peor de
  ellos; si el documento ha cambiado después de la última firma, esa firma lo
  dice (#1017, #1025).
- Al elegir certificado, una franja avisa si ya firmaste el documento, con ese
  mismo certificado o con otro tuyo (#1013).
- Si alguna firma previa no es válida, «Firmar» pregunta antes «¿Firmar de
  todos modos?» con la lista de las firmas que no se sostienen; las
  que no se han podido comprobar del todo o un documento cambiado no lo
  piden (#1014).
- La ventana de sede muestra el mismo aviso de firmas previas y la misma franja
  de «ya lo firmaste tú», sin añadir ninguna confirmación: «Firmar» firma como
  antes (#1015).
- En Linux, los certificados que se instalan van al Almacén de rFirma, cifrado
  y abierto con la sesión: su clave se guarda en el llavero del escritorio, así
  que se firma con ellos sin teclear nada, y ninguno queda en claro en el disco
  ni en una copia de seguridad (#1055, #1057, #1059).
- Se pueden instalar y usar para firmar certificados `.p12` de curva elíptica
  (#1049).
- Si el llavero ha perdido la clave del Almacén de rFirma, Preferencias lo
  avisa y ofrece vaciarlo tras pedir confirmación; nunca lo vacía por su cuenta
  (#1062).
- En la ventana de sede, la pantalla en la que la sede excluye todos tus
  certificados también ofrece instalar uno (#1058).
- Un único selector de certificado, con el rótulo «Certificado», en la ventana
  principal y en la de sede. Cada fila de un certificado de representante
  empieza por la entidad y su NIF y sigue con quién la representa; un
  certificado personal lleva el nombre del titular. Cada fila muestra además
  una etiqueta por almacén y el mes en que caduca (#1092, #1095, #1096).
- El selector también sirve de buscador: al abrirlo se puede filtrar por
  nombre, empresa, NIF, almacén o por la palabra «personal», con un contador
  de coincidencias, y se maneja entero con el teclado (#1095, #1097).
- Ctrl+O (Cmd+O en macOS) abre un PDF igual que el menú; no hace nada mientras
  hay un diálogo, la firma en curso, Preferencias, Estado de rFirma o el
  asistente del primer arranque delante (#1111).
- Cuando las pestañas no caben, la barra enseña «+N ▾» con el número de las
  ocultas; su menú las lista, con la ✓ si están firmadas, y elegir una la trae
  a la barra como activa (#1115, #1116).
- El botón de menú lleva el aviso de Estado de rFirma aunque el menú esté
  cerrado (#1109).
- Al marcar dónde va la firma visible que pide una sede, se puede elegir en qué
  páginas va el recuadro: una, varias o todas (#1130).
- Preferencias › Firma deja elegir dónde se guarda el documento firmado:
  «Junto al documento original» o «En esta carpeta». En el flatpak, que no
  puede escribir junto al original, solo se ofrece la carpeta (#1133, #1134,
  #1136).
- El asistente del primer arranque incluye el interruptor «Protección contra
  firmas accidentales», que se guarda en cuanto se toca (#1135, #1139).

### Changed
- El canal WebSocket escucha en los dos bucles locales, `127.0.0.1` y `::1`,
  sin reinstalar la CA local (#908).
- El canal `service` atiende operaciones seguidas, como el WebSocket, en vez de
  cerrarse tras la primera o tras enseñar un rechazo, y se cierra a los 90 s sin
  órdenes válidas, como AutoFirma (#907).
- Una firma XAdES explícita, que solo firmaría la huella SHA-1 del documento,
  ya no se rechaza en silencio: la ventana explica por qué rFirma no la hace y
  qué puede cambiar la sede, y después la sede recibe `SAF_06` (#915).
- La cofirma o contrafirma de una factura electrónica y la contrafirma fuera de
  CAdES, CMS y XAdES también se explican en la ventana antes de responder a la
  sede. Si la persona había elegido el fichero, la ventana ya no se queda
  colgada en la pantalla de selección (#915).
- `sticky=true` ya no entrega el certificado recordado sin preguntar: la ventana
  se abre siempre, con el certificado fijado en la sesión preseleccionado.
  `sign` y `signandsave` también atienden `sticky` y `resetsticky`, y el fijado
  solo se preselecciona si el filtro de la petición nueva lo acepta. El recuerdo
  dura lo que la sesión de sede, y `resetsticky` ya no olvida el último
  certificado usado en el escritorio (#845, #933).
- `mandatoryCertSelection=false` ya no equivale a `headless=true`: con él, un
  PDF certificado, la contraseña de un PDF o las firmas no registradas se
  preguntan a la persona en vez de salir con `SAF_50` (#959).
- El certificado sale de la cabecera y se elige en el panel; sin certificado
  recordado no se preselecciona ninguno, aunque haya uno solo (#973).
- La firma visible es un interruptor apagado por defecto: al encenderlo, el
  recuadro aparece en la página que se está viendo; sin certificado elegido
  queda desactivado hasta que se elige uno (#974).
- La interfaz dice siempre «firma visible»: la palabra «sello» desaparece de
  todos los textos (#980).
- La bandeja lateral desaparece: cada documento abierto tiene su pestaña; sin documentos, el visor enseña la zona de soltar con los recientes debajo (#972).
- Soltar varios PDF a la vez abre una pestaña por cada uno (#972).
- La ventana de sede presenta primero el origen y el certificado con el que
  firmarás, y después el documento y sus avisos. Sin un origen identificado,
  en lugar de la caja «Origen sin identificar» queda una sola línea: «Una
  página sin identificar pide tu firma.» o «…pide tus datos de identidad.»
  (#1011).
- Instalar un certificado pide primero el fichero y después su contraseña, en
  un diálogo que nombra el fichero; si la contraseña es incorrecta la vuelve a
  pedir con un aviso, y cancelar o cerrar el selector no instala nada ni da
  error. El gesto es el mismo en Preferencias y en la ventana de sede (#1054).
- El diálogo de un PIN o una contraseña aparece sobre la ventana desde la que
  se ha pedido, aunque estén abiertas la principal y una de sede (#1053).
- Sin llavero en el escritorio, rFirma se niega a instalar un certificado y
  explica por qué, en lugar de guardarlo sin proteger (#1062).
- Si un certificado recién instalado desde la sede tampoco vale para ella, la
  pantalla lo dice y deja instalar otro o cerrar: el trámite solo termina
  cuando cierras tú (#1058).
- Quitar un certificado instalado lo borra del almacén, y si era el
  certificado recordado deja de salir preseleccionado (#1061).
- El diálogo «Acerca de» se ordena en tres zonas: la versión con su estado, una
  ficha técnica con enlace al código fuente y el aviso de independencia. Las
  licencias están siempre a la vista, sin el botón «Ver las licencias» (#1084).
- Un certificado que está en varios almacenes, por ejemplo en la tarjeta y en
  el navegador, sale una sola vez en la lista; se firma con la copia recordada
  o, si no la hay, con la de la tarjeta antes que con la de otro almacén
  (#1093).
- Los certificados caducados, revocados o ilegibles se agrupan aparte, en «No
  se pueden usar», con el motivo a la vista y sin que se puedan elegir; la
  sede ya no preselecciona uno de ellos (#1095).
- El pie de la ventana principal lleva un solo botón, «Firmar», que queda
  desactivado mientras no se haya elegido un certificado que se pueda usar; el
  selector pasa al principio del panel, también con documentos ya firmados
  (#1096).
- El «+» de la barra desaparece: en su lugar, junto a rFirma, un botón partido
  fijo cuyo «Abrir PDF…» abre el diálogo del sistema a la primera, y cuya
  flecha despliega «Abiertos recientemente». Sin recientes, o con «Recordar mi
  actividad» apagado, queda solo «Abrir PDF…», sin flecha (#1113, #1114).
- «Recientes» pasa a llamarse «Abiertos recientemente», en el menú y en el
  visor sin documentos (#1113, #1114).
- Las pestañas miden de 160 a 200 px según el sitio que haya y la activa se
  marca con un subrayado; ya no hay flechas ‹ › para recorrerlas (#1112,
  #1115).
- La cabecera y la tira de pestañas pasan a ser una sola barra de 44 px: las pestañas y el botón para abrir viven dentro de la cabecera, y el visor gana el alto que sobra (#1110).
- Por omisión, el documento firmado se guarda junto al original. Si esa
  carpeta no se puede escribir, el pie lo avisa y la firma no se guarda en
  otra carpeta sin decirlo (#1133).
- El asistente del primer arranque vuelve a salir una vez a quien ya lo había
  visto, y su segundo paso se presenta en pasos numerados (#1137, #1138).
- La versión nueva se comprueba cada vez que se abre la ventana principal,
  «Acerca de» o Estado de rFirma, y no como mucho una vez al día. Sin red, se
  usa la última versión conocida, y la consulta espera como mucho 4 s. La
  ventana de sede no la comprueba nunca (#1142, #1143, #1144, #1146, #1147, #1148).

### Removed
- Las casillas sueltas y el motivo de la firma visible, sustituidos por los
  modelos (#981).
- Desaparece el almacén anterior, en el que cada `.p12` instalado quedaba en
  claro en su propia carpeta; no se migra, porque aún no se usaba (#1060).

### Fixed
- Un `ver` dentro de una operación que llega por un canal ya abierto se
  ignora, como en AutoFirma: la versión la fija `v` al abrir el canal. Solo lo
  comprueban el servidor intermedio y el canal WebSocket v3 (#841, #908).
- El canal WebSocket atiende operaciones sucesivas por la misma conexión en vez
  de cerrarla tras la primera, como espera `autoscript.js` (#853).
- Las órdenes que llegan por el canal pasan las mismas comprobaciones de
  parámetros que AutoFirma (`key`, `fileid`, `rtservlet`, identificador) (#853).
- Un `dat` que no es Base64 se firma tal cual, como su texto (#853).
- Un rechazo del análisis de la petición se enseña en la ventana de sede y la
  sede lo recibe al cerrarla; el que nace al procesarla, como la cofirma de una
  factura, se contesta en el acto (#853).
- El canal WebSocket v3 ya no exige el `idsession` en cada mensaje, y tras un
  `SAF_46` sigue atendiendo por la misma conexión, como AutoFirma (#840).
- Un arranque `websocket` con una versión distinta de 3 y 4, o `service` con una
  distinta de 1, 2 y 3, se rechaza en la ventana sin abrir puerto, con `SAF_21`
  en WebSocket. Por WebSocket v3, una operación que exige una versión posterior
  a la 4 sale con `SAF_21` y no con `SAF_06` (#907, #908).
- El canal `service` contesta con los códigos de AutoFirma: `SAF_03` a una
  sesión ajena o una orden desconocida, y `SAF_11` a un `cmd=` que no es una
  operación válida, a un `send=` fuera de rango o a un guardado que no acaba en
  `SAVE_OK` ni `CANCEL`. Un guardado por el canal contesta `SAVE_OK`, que el
  cliente publicado ya no toma por una firma (#907).
- Si otra aplicación ocupa los puertos del canal, la ventana lo dice en un aviso
  que no se cierra solo, en vez de culpar al permiso del navegador (#907).
- El servidor intermedio ya no exige `dat` ni `fileid` al arrancar, no depende
  de la CA local y no deja una ventana en «Conectando con la sede» cuando la
  operación se contesta al llegar. Un rechazo del arranque se enseña aunque la
  sede ya lo haya recibido, un `stservlet` rechazado con `SAF_13` no recibe ese
  rechazo, y el detalle de un rechazo empieza por su código SAF (#906).
- Un prefirmador o postfirmador del lote remoto que contesta 400 da `SAF_03`,
  otro 4xx `SAF_26` y un 5xx `SAF_27`, en vez de `SAF_27` siempre. El algoritmo
  del lote remoto se lee al firmar, como en el original: un lote pedido con
  `jsonBatch` llega al prefirmador como XML, y un algoritmo ausente o ilegible
  sale con `SAF_27` (#946, #948).
- Un lote local mal escrito sale con `SAF_20` después de elegir certificado,
  como en AutoFirma, y no con `SAF_03` (#948).
- Una clave que no es RSA ni de curva elíptica sale con `SAF_51` en vez de
  firmarse como si fuera RSA (#946).
- `format=auto` sobre una firma CMS que no es CAdES la cofirma como CMS (#946).
- Si el guardado de la sede no puede escribir en el destino elegido, la ventana
  lo avisa y vuelve a abrir el diálogo, como AutoFirma; cancelar entonces da
  `CANCEL`, en vez de acabar con `SAF_05` (#948).
- El documento elegido para `sign` o `signandsave` que no se puede leer responde
  `SAF_00`, como el original, y no `SAF_25` (#966).
- Un rechazo de los datos por el firmador llega a la sede con el código del
  original (`SAF_28`, `29`, `30`, `31`, `32`, `38` y `44`) en vez de `SAF_09` (#911).
- La envoltura XAdES que la sede declara en `format=` se respeta, incluida
  `XAdES Externally Detached`, y la política AGE ya no se pisa (#911).
- `userPassword`, `ownerPassword` y `allowSigningCertifiedPdfs=true` abren el PDF
  protegido; con `headless=true` y sin ellos, la sede recibe `SAF_50` (#911).
- La ventana de sede nombra los fallos del servidor trifásico en vez de mostrar
  un error desconocido (#913).
- El diálogo de guardar o cargar de una sede que no declara carpeta se abre en
  el directorio personal, no en el directorio desde el que se lanzó rFirma (#913).
- Una firma CAdES con huella precalculada ignora el `mode` que pida la sede (#914).
- Una contrafirma con un `target` distinto de `tree` contrafirma solo las hojas,
  como AutoFirma (#914).
- La firma sobre un documento elegido en disco devuelve su nombre a la sede en
  un tercer componente, por WebSocket v3 y v4 (#914).
- La política de firma de la AGE funciona en CAdES y PAdES (#914).
- Con la política de la AGE, un documento de menos de 1 MB se firma en CAdES
  implícita, con los datos dentro (#914).
- El nombre del documento elegido en disco sale en el tercer componente de la
  respuesta de `sign` y `signandsave` también por el canal `service` con `v=3`
  y por el servidor intermedio con `ver` 3 o mayor, cifrado si la sede da
  clave (#917).
- Con `checkSignatures=true`, una cofirma o una contrafirma sobre un documento
  sin firmas se rechaza con `SAF_39`, como en el original (#918).
- Un PDF certificado que la sede manda firmar sin `headless` ni
  `allowSigningCertifiedPdfs` pregunta a la persona antes de firmarlo, como el
  original, en vez de rechazarse con `SAF_35`; si dice que no, la sede recibe
  `CANCEL` (#919).
- La ventana de sede nombra los fallos del token, del puente y del documento con
  el mismo título que la ventana principal, y los propios del trámite con su
  frase, en vez de mostrar un error desconocido (#920).
- Cancelar el área de la firma visible ya no aborta la firma cuando la petición
  trae su propia área o la firma visible es opcional: se firma en el área de la
  petición o sin firma visible, como en AutoFirma (#909).
- Un PDF cifrado que llega por la sede con una contraseña que no lo abre, o
  sin ninguna, pide la contraseña a la persona después de elegir certificado,
  como AutoFirma. Con `headless` se rechaza con `SAF_50` y no con `SAF_09` (#922).
- Cerrar la ventana de sede mientras se marca el área de la firma visible hace
  lo mismo que pulsar «Cancelar» en ese paso, como en AutoFirma: se firma en el
  área de la petición o sin firma visible, y solo con `visibleSignature=want`
  sin área la sede recibe `SAF_43` (#924).
- Si falla el token durante un lote de sede, la ventana dice qué ha pasado
  («El PIN no es correcto», token ausente…) en lugar de la frase genérica del
  lote fallido (#924).
- Una contrafirma que llega por la sede con un `target` distinto de `tree`
  (`signers`, por ejemplo) contrafirma las hojas, como AutoFirma, en vez de
  fallar con `SAF_09` (#925).
- Una firma CAdES o XAdES que la sede pide con `tsaURL` sale con su sello de
  tiempo. Si no se puede sellar, la sede recibe `SAF_09` en vez de una firma
  sin el sello que pidió (#927).
- Una cofirma, una contrafirma, una firma `XAdEStri` o una firma con
  `useManifest=true` en XAdES con `mode=explicit` se firman como en AutoFirma,
  en vez de salir con `SAF_06`. Solo se rechaza la firma de la huella SHA-1 (#928).
- Un almacén que no se puede abrir, un token ausente o un módulo PKCS#11 que no
  se encuentra llegan a la sede con `SAF_08`, como en AutoFirma, y no con
  `SAF_07`, que el original no emite nunca (#930, #936).
- Con `headless`, un certificado caducado ya no se entrega aunque el filtro de
  la sede lo admita: si no queda ninguno vigente, la sede recibe `SAF_19`
  (#935).
- Un token al que no se ha abierto sesión ya no ofrece certificados de CA ni los
  que por su uso de clave no pueden firmar (#934).
- Con la firma visible apagada, el PDF firmado ya no lleva el recuadro de una
  colocación anterior o recordada, y «Firmar» ya no se queda sin hacer nada
  cuando no hay ninguna colocación (#968).
- «Ajustar a la página» encaja la página entera en el visor sin dejar barra de
  desplazamiento (#968).
- Un `.p12` exportado sin nombre amistoso, como los de un `openssl pkcs12
  -export` sin `-name` o muchas exportaciones de Windows, ya se instala en vez
  de fallar siempre pidiendo que se compruebe la contraseña (#1048).
- Al instalar se distingue una contraseña incorrecta de un fichero que no es
  un certificado válido y de un `.p12` sin clave privada, cada uno con su
  mensaje (#1050).
- Instalar dos veces el mismo fichero ya no lo duplica en la lista ni da error
  (#1057).
- Desde la ventana de sede se instala un `.p12` con contraseña, y si la
  instalación falla la pantalla dice por qué en lugar de quedarse igual
  (#1054, #1056).
- Abrir rFirma con una URL de otro esquema que no sea `afirma://` ya no abre la
  ventana principal: rFirma termina sin hacer nada, como AutoFirma. Una URL
  `file://` abre la ventana principal con ese documento (#1128).
- Una sede que pide el almacén NSS del sistema (`keystore=SHARED_NSS`) ve solo
  los certificados de NSS, sin las tarjetas ni los tokens registrados por
  p11-kit, así que la selección desatendida vuelve a elegir sin preguntar
  (#1128).
- Un filtro de certificados de la sede en el que no se reconoce ningún criterio
  se trata como si no hubiera filtro, y los certificados caducados siguen
  ocultos (#1128).
- El recuadro de la firma visible que pide una sede lleva el texto que la sede
  declara, o el de AutoFirma si no declara ninguno; antes salía en blanco
  (#1130).
- Los pasos del asistente arrancan en su estado real: el certificado espera a
  que se compruebe, y el paso de las sedes ya no dice «Ahora abren rFirma.»
  cuando no aplica (#1139).

### Security
- La traza de una operación ya no imprime el valor de `properties`, que podía
  llevar la contraseña de un PDF (#911).

## [0.10.0] - 2026-09-21

### Added
- «Comentarios y ayuda» en el menú abre el foro del proyecto, y los fallos
  propios de rFirma y los rechazos desconocidos de una sede enlazan con él
  (#787, #788).
- «Estado de rFirma» en el menú: una vista que dice de un vistazo si rFirma
  está lista para firmar, con una fila por señal —versión, tus certificados,
  el certificado de rFirma en cada almacén de los navegadores y qué programa
  abre las sedes— y la cabecera siempre a mano (#789, #790, #791, #792, #794).
- Desde el panel de estado se instala el certificado de rFirma cuando falta o
  está a medias, y la fila avisa de reiniciar Firefox si está abierto (#793).
- Desde el panel de estado se elige qué programa abre las sedes cuando hay
  más de uno instalado; elegir rFirma instala también su certificado, sin el
  que no puede firmar en sedes (#795).
- El botón del menú lleva un triángulo de atención cuando hay algo que rFirma
  puede y debe arreglar: el certificado ausente o a medias, o ningún programa
  atendiendo las sedes (#796).
- «Retirar…» en el panel de estado deshace lo que rFirma instaló fuera de sus
  carpetas: su certificado en cada almacén de los navegadores y su registro
  como programa que abre las sedes. Si algún almacén falla, lo dice uno a uno
  con su motivo y «Reintentar» repite solo lo que falló (#801, #802, #803).
- La primera vez que se abre, rFirma se presenta con un asistente de dos
  pantallas: una bienvenida que explica qué es, y otra con las dos acciones
  que necesita para firmar en sedes —instalar su certificado en los
  navegadores y que las sedes lo abran—, cada una con su «Ahora no». Arranca en
  el idioma del sistema, deja cambiarlo desde la propia bienvenida y se puede
  omitir; al terminar no vuelve a salir (#804).

### Changed
- Cada trámite de sede corre en su propio proceso, aparte de la ventana de
  escritorio: dos trámites simultáneos ya no se pisan, cada proceso tiene su
  carpeta de paso y la limpia al salir, y las preferencias se releen de disco
  en cada cambio (#671, #672, #673, #674).
- Preferencias pasa a ser una vista de la ventana, con un índice permanente a
  la izquierda —General, Firma, Certificados, Apariencia— y solo la sección
  activa a la derecha, en vez de una columna larga con scroll (#798, #799).

### Removed
- La franja que preguntaba si rFirma debía abrir las sedes, y el grupo
  «Sedes» de Preferencias: eso se elige ahora en el panel de estado (#797).
- El aviso de confianza del primer arranque, al que sustituye el asistente
  (#804).

### Fixed
- Cerrar la ventana de sede con la X cancela el trámite ante la sede, en vez
  de dejarla esperando (#676).
- Los lotes de firma de una sede vuelven a firmarse desde la ventana con el
  PIN del diálogo nativo, roto desde la v0.9.1, y un PIN equivocado se vuelve
  a pedir (#700).
- Compatibilidad con las sedes que usan servidor intermedio: rFirma avisa cada
  diez segundos de que sigue esperando a la persona, y la sede ya no cancela
  el trámite a los treinta (#713). Los rechazos al arrancar se suben al
  servlet de guardado, como hacía AutoFirma, y si la subida falla se enseñan
  en la ventana (#705).
- Paridad con AutoFirma 1.9.2 en el protocolo de sede: se aceptan las URIs
  `afirma://` con barra final en el verbo (#711), una versión de protocolo
  demasiado nueva responde `SAF_21` (#712), una multifirma con `format=auto`
  sobre datos sin firmar se rechaza con `SAF_17` (#714), y el nombre propuesto
  en `signandsave` lleva la extensión del formato de firma (#715).
- La ventana que atiende una petición de certificado de una sede deja de
  presentarse como una identificación: la sede solo recibe el certificado
  elegido, y los textos lo dicen así en los cinco idiomas (#730).
- Un fallo al pintar la interfaz enseña una pantalla que lo explica, con el
  detalle copiable y un botón para recargar, en vez de dejar la ventana en
  blanco; en la ventana de sede, esa pantalla no se cierra sola (#806).
- Si rFirma no puede arrancar, lo dice en un diálogo nativo con el motivo y el
  detalle, en vez de cerrarse sin enseñar nada (#807).

## [0.9.1] - 2026-09-11

### Changed
- El PIN de un token PKCS#11 o la contraseña de un almacén protegido ya no se
  piden en un diálogo web (`PinDialog`) dentro de WebKitGTK: se piden en un
  diálogo nativo de GTK3, gestionado por el backend, cuyo buffer de memoria
  queda fijado con `mlock` y excluido de volcados con `MADV_DONTDUMP`, y se
  borra de forma segura al terminar (#648). AutoFirma solo hacía un borrado
  parcial e inconsistente del `char[]` con `Arrays.fill`, sin `mlock` ni
  exclusión de volcados, así que esto es una mejora de seguridad frente al
  original, no paridad de funcionalidad.

## [0.9.0] - 2026-09-10

### Added
- Mecanismo de CHANGELOG por fragmentos: `changelog.d/`, `CHANGELOG.md` desde
  la v0.4.0 y la receta `just changelog-release` (#252).
- rFirma tiene icono propio: un SVG maestro versionado y los ocho tamaños que
  instala el paquete, de 16 a 512, rasterizados desde él y no reescalados al
  construir (#254).
- Un almacén cuyo secreto se teclea en el teclado del lector se rechaza con su
  propio aviso antes de intentar firmar, en vez de pedir el secreto por
  pantalla y fallar contra el token (#257).
- Un certificado en fichero `.p12` se puede instalar en rFirma y firmar con él:
  cada fichero pasa a ser un almacén propio, y sus certificados aparecen en la
  lista junto a los del navegador sin volver a teclear su contraseña (#261).
- Preferencias tiene una sección de certificados en fichero: la lista de los
  `.p12` instalados —titular, DNI, emisor y fecha de caducidad— y sus dos
  gestos, «Añadir…» y «Quitar». Sin casillas por almacén ni diálogos anidados
  (#262).
- Instalar un `.p12` con clave elíptica lo dice en la propia sección y en un
  solo renglón, sin la lista cambiar (#262).
- Preferencias avisa de que el documento firmado cae «Junto al documento
  original» cuando el entorno sabe devolver su ruta real; donde no la sabe,
  el destino se queda en la carpeta con su «Cambiar carpeta…», como antes
  (#264).
- `packaging/verifica-contenido.sh`, la puerta independiente del formato que comprueba la
  invariante del ADR-0012 (un solo `librfirma_crypto.so`, `libawt.so` en ninguna parte) sobre
  el paquete construido, hoy el `.flatpak` (#265).
- `just check-glibc`, la puerta que comprueba el suelo `GLIBC_2.34` de la librería nativa sin
  adoptar un contenedor de construcción (#265).
- Los paquetes `.deb` y `.rpm`, producidos por el *bundler* de Tauri a partir de la misma
  construcción que el flatpak, con la librería nativa en `/usr/lib/rfirma/librfirma_crypto.so`
  y sin ninguna dependencia de PKCS#11 (#266).
  La receta `just bundle` los construye y les pasa `packaging/verifica-contenido.sh`.
- rFirma se abre desde fuera con un documento, `rfirma /ruta/documento.pdf`: la
  ventana completa con ese PDF cargado, el mismo estado en que la deja
  arrastrarlo (#267).
- «Firmar con rFirma» en el menú contextual de KDE sobre un PDF, al primer nivel y sólo con un
  fichero seleccionado. Lo instalan el `.deb` y el `.rpm` en
  `/usr/share/kio/servicemenus/`; el flatpak no lo lleva, porque ahí el gestor de ficheros
  entregaría una ruta del portal (#268).
- El ADR-0018, que explica por qué la firma empieza por un verbo y no por un tipo de fichero:
  rFirma no declara `application/pdf` en ningún lanzador y, a cambio, arrastrar un PDF sobre el
  icono del *dock* no funciona (#268).
- «Firmar con rFirma» en el menú contextual de Nautilus sobre un PDF, al primer nivel y sólo con
  un fichero seleccionado. Es una extensión de `nautilus-python` que el `.deb` y el `.rpm`
  instalan en `/usr/share/nautilus-python/extensions/`; el flatpak no la lleva, por lo mismo que
  no lleva el menú de KDE (#269).
- El paquete de `nautilus-python` viaja como **recomendación**, con el nombre que le da cada
  familia (`python3-nautilus` en el `.deb`, `nautilus-python` en el `.rpm`). Sin él la extensión
  queda inerte y no pasa nada más (#269).
- rFirma comprueba al arrancar si hay una versión nueva publicada, preguntándole
  a GitHub una vez cada 24 horas. Es sólo un aviso: no descarga ni instala nada
  (#270).
- Cuando hay una versión nueva publicada, rFirma lo dice en una franja bajo la
  cabecera, con una acción que lleva a *Acerca de* y una `×` que la descarta.
  Nada de esto interrumpe el trabajo: no hay ningún diálogo, y sin versión
  nueva la franja no ocupa ni un píxel (#271).
- *Acerca de* enseña cómo actualizar: las órdenes de alta del repositorio,
  copiables, para Flatpak, Debian y Ubuntu, y Fedora y openSUSE. No hay botón
  de descarga (#272).
- En Preferencias, sección *Privacidad*, un interruptor para dejar de avisar
  de las versiones nuevas. Siempre visible, sin condición (#272).
- La landing de `rfirma.sgomez.me`: `index.html` escrito a mano y la imagen `nginx` que la
  sirve, construida por Coolify desde `main` (#273).
- Sección «Elige un canal» en la instalación del README (#273).
- El workflow de construcción de la entrega, `.github/workflows/build.yml`: invocable desde otro
  workflow, con permisos de sólo lectura y sin ver ningún secreto, produce el flatpak, el `.deb` y
  el `.rpm` de una sola construcción y les pasa `packaging/verifica-contenido.sh` a los tres antes
  de subirlos con su `SHA256SUMS` (#274).
- Los otros dos workflows de la tubería de entrega: `release.yml`, que ante una etiqueta `v*`
  construye, firma y deja la Release **en borrador** con los tres paquetes, su `SHA256SUMS.asc`,
  la atestación de procedencia y el PDF de la puerta manual adjunto; y `publish.yml`, que sólo
  reacciona a una Release **publicada** que no sea candidata y comprueba la firma, los resúmenes
  y la puerta del contenido antes de que se sirva nada (#275).
- `SECURITY.md`: el aviso privado de GitHub como única vía de reporte, las claves de larga vida
  con lo que firma cada una, cómo verificar un paquete descargado, y la comprobación de versión
  declarada como la primera —y hoy única— conexión saliente de rFirma, con su ajuste para
  apagarla (#275).
- El mecanismo de publicación de `rfirma.sgomez.me`: `publish.yml` reconstruye el árbol servido
  **entero** desde las Releases de la serie vigente, lo sube al anfitrión por `rsync` sobre SSH con
  orden forzada (`rrsync`) y lo pone en servicio **intercambiando un enlace simbólico**. Un
  despliegue a medias no llega a verse, la vuelta atrás es reapuntar el enlace y el volumen se puede
  tirar y rehacer: la fuente de verdad son las Releases. En el anfitrión se quedan el árbol vigente y
  el anterior, y ni uno más (#276).
- Caddy sirve los tres repositorios y la clave pública desde el montaje del anfitrión a través del
  enlace `actual`, con las rutas que fija el ADR-0015 —`/flatpak/`, `/apt/`, `/rpm/`, `/rfirma.asc`,
  `/rfirma.flatpakref`— y sólo esas; la landing sigue viniendo dentro de la imagen (#276).
- `just check-publish`: las pruebas del mecanismo, que levantan el mismo `rrsync` del `authorized_keys`
  del VPS detrás de un `ssh` de mentira. Es la única parte de la tubería que no se puede ensayar con
  una etiqueta `v*-rc.N`, porque el ensayo se detiene justo antes de tocar el anfitrión (#276).
- Los tres repositorios de `rfirma.sgomez.me`, montados encima del mecanismo de publicación:
  el **ostree** de `/flatpak/` con todos los bundles de la serie vigente importados en orden y
  su `rfirma.flatpakref` de un clic; el **apt** de `/apt/` con suite `stable` —no un
  repositorio plano— y su fichero deb822 con `Signed-By`; y el **dnf** de `/rpm/` con la URL
  literal, `gpgcheck` y `repo_gpgcheck` a 1 y su `repodata` firmado (#277).
- Reconstruir el árbol **no obliga a nadie a redescargar**: los bundles reimportados en un
  ostree vacío dan el mismo commit, y `just check-publish` lo comprueba construyendo los tres
  repositorios dos veces y comparando. El repositorio se re-firma en cada reconstrucción,
  porque la firma es metadato desacoplado que no viaja dentro del bundle (#277).
- Los filtros de certificado que manda una sede se aplican con el motor del
  original: `afirma-keystores-filters` entra en el puente por una llamada sin
  estado y sin sello, y la expresión cruza literal, sin reinterpretar (#350).
- Lista blanca de criterios antes de llamar al motor: un criterio que rFirma no
  reconoce se rechaza con `SAF_03` en vez de ignorarse en silencio, que es lo
  que hace el original y deja el listado más ancho de lo que la sede pidió
  (#350).
- El trámite de sede de punta a punta para `selectcert`: la invocación abre el
  canal, la operación se lee de la URL, el listado se acota con el filtro que
  mandó la sede y lo que ella recibe es el certificado en Base64 URL-safe, o
  `CANCEL`, o su código `SAF_` (#352).
- El consentimiento no se salta nunca: `headless` y `mandatoryCertSelection` se
  ignoran los dos, también cuando queda un solo certificado, y en `selectcert`
  ese momento consiente entregar identidad (#352).
- Lo que la sede recibe sale de inmediato, sin esperar a que se cierre ninguna
  ventana: la precisión de lo que pasó se queda en la ventana y el código, en el
  cable (#352).
- El trámite de sede atiende `sign` y `cosign` en PAdES: la operación se lee de
  la URL con su formato, su algoritmo y el documento, la persona consiente con
  el documento delante y lo que la sede recibe es el certificado y la firma en
  Base64 URL-safe, separados por `|` (#353).
- `expPolicy=FirmaAGE` lo expande `ExtraParamsProcessor` de `afirma-core`, por
  una entrada nueva del puente, y no una reimplementación: expandirlo mal sería
  firmar con una política distinta de la declarada (#353).
- Una política que no se puede aplicar al formato pedido llega con nombre propio
  desde el puente y la sede recibe `SAF_23`, en vez de colapsarse en «la firma
  no ha salido» (#353).
- La firma visible que pide una sede se atiende sin visor y sin hacerla esperar:
  con posición y página se firma con recuadro, sin ellas `visibleSignature=optional`
  firma invisible y `want` cancela con `SAF_43`, y un `visibleAppearance=custom`
  del que no vienen datos estampa el aspecto por omisión (#354).
- `signaturePages` admite la gramática entera del puente, índices contados desde
  el final incluidos —`-1` es la última—, y **rechaza `append`**: añadir una
  página en blanco es modificar el documento antes de firmarlo (#354).
- rFirma se registra como manejador del esquema `afirma://`: pulsar un enlace del
  protocolo en el navegador la arranca con la URL entera, en los tres canales (#356).
- rFirma detecta el canal de distribución (`/.flatpak-info`) y, fuera del flatpak,
  puede leer qué aplicaciones dice el escritorio que atienden `afirma://`, sin
  nombrar ninguna en el código; dentro del sandbox no lo intenta, porque la
  respuesta no es de fiar (#357).
- Preferencias trae una sección *Sedes* con un solo control: quién atiende los
  enlaces `afirma://`, elegido entre lo que el escritorio diga que hay
  registrado, con el aviso de que Firefox impone la elección que guarda aparte.
  En el flatpak, donde no hay portal que lo permita, hay una frase fija que
  remite a los ajustes del escritorio en vez de un control que no cumpliría
  (#364).
- Al arrancar, un banner bajo la cabecera pregunta si rFirma debe atender esos
  enlaces —*Sí*, *Ahora no*, *No volver a preguntar*—, y «No volver a
  preguntar» se deshace en Preferencias (#364).
- La ventana de sede enseña los momentos de guardar y de cargar, con el nombre
  que propone la sede y nunca su ruta, y abre sola el diálogo del portal (#500).
- Las cuatro variantes XAdES —Enveloping, Detached, Enveloped y ASiC-S— cruzan
  la frontera nativa: `Format::Xades(_)` va a las entradas XAdES del puente y
  deja de ser un formato sin resolver (#537).
- La sede puede pedir SHA-256, SHA-384 o SHA-512 y rFirma compone el algoritmo
  con la clave del certificado, como hace el original (#544).
- rFirma firma con certificados de curva elíptica: el token puede pedir el
  mecanismo compuesto o el crudo sobre el resumen, y la firma sale en el DER
  que esperan CAdES y XAdES (#545).
- `just token` provisiona también el token de pruebas `rfirma-test-ecc`, con un
  certificado P-256 del kit FNMT (#545).
- `format=CAdES-ASiC-S` cruza el puente por las entradas de CAdES y devuelve el
  contenedor ASiC-S con la firma CAdES dentro, como AutoFirma (#591).
- Una sede que pide `checkSignatures=true` obtiene la validación de las firmas que el documento ya trae antes de pedir consentimiento: si alguna no vale, recibe `SAF_39`; si hace falta confirmar y mandó `headless=true`, recibe `SAF_50`; y si no, el trámite se para en un momento nuevo de confirmación con dos salidas, seguir o cancelar (#595).
- Cuando la validación previa de `checkSignatures` necesita una decisión de la persona, la ventana de sede la pide con las palabras del original —traducidas a los cinco idiomas— y dos salidas: continuar, que repite la validación y sigue al consentimiento, o cancelar, que contesta `CANCEL` a la sede (#596).
- El `dat` de la sede puede ser una URL `http(s)`: rFirma se descarga el
  documento y sigue el trámite con él, como AutoFirma (#612).
- `sign`, `cosign` y `countersign` sin `dat` dejan de ser un `SAF_03`: se pide
  el documento a la persona con las claves de carga de `properties`, y
  cancelar contesta `CANCEL` (#612).
- Catálogos completos en catalán, euskera y gallego: el selector de idioma ofrece ya los cinco (#641).
- La landing de `rfirma.sgomez.me` se reescribe con Astro en `packaging/repo/site/`:
  las diez secciones de la página validada en el lienzo, en los cinco idiomas de la
  aplicación (`es`, `ca`, `eu`, `gl`, `en`) con `/` en castellano y `/ca/`, `/eu/`,
  `/gl/` y `/en/` para el resto, y una prueba que exige los cinco diccionarios al
  100 % (#642).

### Changed
- La versión se cambia en un solo sitio, `tauri.conf.json`, y `just check-version` pone en rojo cualquier divergencia con `package.json`, `Cargo.toml` y el metainfo (#253).
- El README enlaza a `releases/latest/download/…` en vez de llevar el número de versión dentro (#253).
- El metainfo declara `version`, `date` y un enlace al CHANGELOG en vez de copiar sus notas (#253).
- El secreto que desbloquea un almacén deja de ser una cadena y pasa a ser de
  tres clases —sin sesión, tecleado en pantalla, tecleado en el teclado del
  lector—, que la prefirma lee de la ranura y devuelve a la ventana (#257).
- Sin necesidad de sesión no se abre ningún diálogo: la firma sigue directa,
  con el secreto vacío (#258).
- El diálogo del secreto ya no nombra la clase de almacén PKCS#11 ni el
  nombre del token, ni lleva ninguna frase tranquilizadora; la palabra la
  elige el almacén —«PIN» para un módulo, «contraseña» para un fichero— y
  «Tarjeta bloqueada» deja de resolverse ahí dentro (#258).
- Fuera del sandbox, el documento abierto cruza a la ventana con su **ruta
  real**, como la enseña cualquier aplicación de escritorio (#263).
- «Guardar junto al original» lo contesta **el documento**: uno de ruta directa
  ofrece la carpeta en la que está, y uno que entró por el portal responde que
  no hay carpeta original (#263).
- La guarda del ADR-0011 vigila **valor y no texto**: comprueba sobre los
  valores que produce la aplicación que la ruta del portal (`/run/user/*/doc/`)
  no sale a la ventana por ningún campo, en ningún canal (#263).
- `packaging/flatpak/verifica.sh` deja de comprobar la invariante del ADR-0012: ahora vive en
  `packaging/verifica-contenido.sh` (#265).
- Solo hay una rFirma abierta: invocarla otra vez con un documento sustituye el
  que hubiera delante, sin preguntar; con una firma a medias no sustituye nada
  (#267).
- Una invocación `afirma://` **nunca sustituye** el documento que la ventana
  tuviera delante: abre lo suyo, y una firma local a medias tampoco la detiene
  (#352).
- Un segundo `afirma://` con un trámite de sede vivo se rechaza con `SAF_45` por
  su propio socket mientras el primero siga vivo (#352).
- El documento que manda una sede no deja rastro: entra por la puerta que no
  recuerda, su fichero de paso se borra en cuanto el trámite contesta, y la
  postfirma del trámite devuelve los bytes sin escribir fila en Recientes, ni
  colocación del recuadro, ni «último documento» (#353).
- La firma de un trámite de sede vuelve a pasar el filtro de la sede justo antes
  del PIN, y no resuelve el certificado por el camino local (#353).
- Los `extraParams` que declara la sede van **debajo** de los seis ajustes de
  rFirma: la sede decide la política y rFirma el recuadro que la persona vio
  (#353).
- Los dos caminos del recuadro dejan de compartir conversión: cuando el sitio lo
  elige la persona con el ratón se aplica la corrección de rotación, y cuando lo
  elige la sede los `signaturePositionOnPage*` cruzan al puente **crudos**, que
  es lo que hace AutoFirma y contra lo que las sedes ajustaron esos números
  (#354).
- Un algoritmo que el token no ofrece se rechaza antes de pedir el PIN, con el
  `SAF_09` del original y una vista que lo dice en castellano (#544).
- El trámite de sede deja de contestar `CAdES-ASiC-S` como formato sin puente:
  `Format::bridged()` lo admite (#591).
- El almacén de certificados que la sede nombra en `keystore` o `ksb64` ya no se ignora: se obedece el de la familia NSS, que es el que rFirma abre, y cualquier otro se rechaza con `SAF_07` diciéndolo en la ventana (#617).
- Las cadenas hablan de tú en los cinco idiomas, con un registro cercano y directo fijado en el ADR-0009; las confirmaciones de sede dejan el «¿Desea continuar?» heredado de AutoFirma, y el inglés pasa a la voz de la aplicación con contracciones (#641).
- La imagen de `rfirma.sgomez.me` se construye en dos etapas y sirve la salida de
  `astro build` en lugar de un `index.html` suelto; su contexto de construcción pasa a
  ser la raíz del repositorio (#642).

### Removed
- Se retira la fontanería de tarjeta, que nunca se había publicado: el
  cliente PC/SC y el módulo PKCS#11 de OpenSC salen del flatpak, y las siete
  rutas de OpenSC salen de la colección de almacenes. Tarjetas y DNIe no
  están soportados en la v0.4 (#256).
- El contador de intentos restantes del secreto incorrecto: PKCS#11 no lo
  cuenta nunca, así que deja de fingirse (#258).
- `countersign` se contesta con `SAF_04`: en PAdES contrafirmar no existe
  (#353).
- `save` y `signandsave` se rechazan con nombre propio, por seguridad y no por
  coste: que una sede escriba ficheros en el equipo es una decisión (#353).

### Fixed
- El lanzador enseña `rFirma`, no `rfirma` (#253).
- El lanzador enseña el icono al tamaño que pide, en vez de estirar el único
  PNG de 64×64 que se instalaba antes (#254).
- La colección de almacenes de certificados vuelve a encontrar los de Firefox
  147 y Chrome M146, que se habían mudado a rutas XDG
  (`~/.config/mozilla/firefox` + `~/.local/share/mozilla/firefox`, y
  `~/.local/share/pki/nssdb`); el
  manifiesto del flatpak las declara en sólo lectura (#255).
- Un perfil de Firefox con contraseña maestra ya no enseña sus autoridades
  sueltas al listar certificados: la sesión se inicia antes de listar, y se
  retira la vuelta atrás que devolvía el almacén entero sin filtrar cuando
  ninguna clave privada era visible (#259).
- La ventana ya no crece en cada arranque hasta salirse de la pantalla. Se
  recordaba el tamaño con la medida equivocada —la superficie con las sombras
  del CSD dentro, que `set_size` no cuenta—, y eso sumaba 52x99 px cada vez. Se
  retira la memoria de tamaño de ventana: rFirma abre siempre a 1280x720 y quien
  quiera otra cosa maximiza (ADR-0010, enmienda) (#306).
- La imagen nativa firma XAdES Enveloped: le faltaban los canonicalizadores
  `REC-xml-c14n-20010315` y la tabla de funciones XPath, que solo se alcanzan
  por reflexión (#537).
- Las sedes que fuerzan el modo servidor intermedio ya pueden firmar: cuando la
  operación no cabe en la URL, la sede solo manda `fileid`, `rtservlet` y `key`,
  y rFirma recupera con ellos el XML de parámetros de la operación —de donde
  salen el verbo, el documento, `stservlet` e `id`— en vez de rechazar la
  invocación (#601).
- El algoritmo se normaliza con el criterio del original en vez de contra una
  lista cerrada: se atienden los nombres compuestos (`SHA256withRSA`,
  `SHA384withRSA`, `SHA512withRSA` y sus variantes `withECDSA`) en la cabecera
  del lote —tanto en XML heredado como en JSON remoto y local— y las variantes
  con guion (`SHA-256`, `SHA-384`, `SHA-512`), OID y URI de XMLDSig en todas
  las operaciones (#602).
- Los algoritmos no atendidos como SHA1 se rechazan al leer la operación con
  `SAF_03` nombrando `algorithm` (#602).
- El arranque por servidor intermedio conserva el momento del trámite al abrir
  la ventana en vez de quedarse en «Conectando con la sede», permitiendo ver los
  certificados y consentir la firma (#606).
- El cliente de servidor intermedio desacopla las llamadas bloqueantes HTTP del
  runtime de Tokio, evitando el pánico al subir la firma al servlet de guardado (#610).
- La versión mínima de protocolo que la sede declara en `ver` se lee en toda
  operación: si pide más de la que rFirma habla, el trámite sale con `SAF_21`
  antes de firmar nada, y en el camino del servidor intermedio es ella la que
  fija la versión de la operación (#618).
- Se firman los lotes y las peticiones de sede que declaran SHA-1, que es lo
  que acepta AutoFirma: rechazarlos dejaba sin atender a sedes en producción
  (#647).

### Security
- Un `.p12` con clave elíptica se rechaza al instalarlo, y no al firmar, porque
  rFirma solo firma con claves RSA (#261).
- Sin red, la comprobación no dice nada: ni aviso, ni error, ni reintento hasta
  el siguiente arranque. Es la única conexión saliente de rFirma, y el flatpak
  gana por ella —y sólo por ella— el permiso de red (#270).
- Todas las acciones de GitHub Actions quedan fijadas por SHA, con `dependabot.yml` revisándolas
  una vez al mes en una sola PR agrupada, y una puerta —`just check-actions`— que impide que
  vuelva a entrar ninguna por etiqueta (#274).
- Una sola clave GPG para todo lo verificable por humanos —Releases, ostree, apt y dnf—, con la
  **subclave de firma en el CI y la maestra fuera de línea**: el CI puede firmar, no puede
  certificar, y una filtración se resuelve revocando la subclave sin que nadie tenga que volver a
  dar de alta el repositorio. La genera `packaging/setup-signing-key.sh`, la importa una acción
  local que comprueba la huella contra la que el repositorio declara, y tres puertas nuevas de
  `just check-actions` impiden que se pierdan el borrador, la guarda de las candidatas o la
  frontera del secreto (#275).
- **La huella de la clave, publicada por dos caminos**: `SECURITY.md` y la portada de
  `rfirma.sgomez.me`. Una huella sólo sirve si quien descarga la clave pública puede
  contrastarla con algo que ya sabía, así que `SECURITY.md` prometía una portada que no la
  enseñaba (#275).
- `packaging/setup-publish-access.sh`, hermano del de la clave de firma: da de alta el acceso
  de publicación al anfitrión —clave ed25519 encerrada con `command="rrsync …",restrict`, el
  secreto del entorno `release` y las tres variables—, y comprueba antes de guardar nada que
  esa clave habla por rsync y **no** da consola (#275).
- La Release **se para si la huella publicada no es la de la clave que firma**. La acción de
  importación ya ataba el secreto a `vars.GPG_FINGERPRINT`, pero `SECURITY.md` y la portada
  eran texto suelto: una huella equivocada ahí no rompía nada y dejaba verificando contra nada
  a quien se fiara de ella (#275).
- Dos puertas nuevas de `just check-actions`: el `rsync` de la publicación tiene que ir por el guion
  probado y no suelto dentro del YAML, y **ningún workflow puede tocar Docker ni un registro de
  imágenes** —los repositorios no van dentro de la imagen, así que la tubería de entrega no toca
  Docker en ningún momento— (#276).
- El repositorio dnf **rechaza un `.rpm` que no lleve la firma dentro**, que es lo que
  verifica `gpgcheck=1` en la máquina de quien instala. Y una puerta nueva de
  `just check-actions` fija el orden de `release.yml` —firmar cada `.rpm`, luego el
  `SHA256SUMS`, luego la atestación y sólo entonces adjuntar—: firmar modifica el fichero, así
  que reordenar esos pasos dejaría el `.rpm` de la Release y el del repositorio con bytes
  distintos sin poner en rojo ninguna ejecución (#277).
- Otras dos puertas de `just check-actions`: la publicación tiene que pasarle la huella de
  firma a `build-tree.sh`, y el modo sin firma del constructor —que existe porque las claves
  las crea una persona y ninguna prueba puede fabricarse una— no puede aparecer en ningún
  workflow (#277).
