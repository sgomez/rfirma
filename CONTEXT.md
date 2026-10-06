# rfirma

Aplicación nativa de firma electrónica que sustituye la interfaz de **AutoFirma**
para ciudadanos y empresas que firman ante la Administración española. Este
documento es el glosario del dominio de la aplicación: define **qué es** cada
término, no cómo está implementado (eso vive en `docs/adr/`). Entre paréntesis, el
nombre que le da el código, para encontrarlo; sin prefijo, es el mismo en
TypeScript y en Rust. La suite de
conformidad tiene el suyo; los dos están en `CONTEXT-MAP.md`.

## Language

### Proceso de firma

**Firma trifásica** (TS `SIGNING_STAGES` · Rust `cycle`):
Procedimiento de firma partido en tres etapas —prefirma, firma y postfirma— de
modo que la clave privada nunca sale del dispositivo que la custodia.
_Avoid_: firma en tres pasos, firma distribuida, triphase

**Prefirma** (`presign` · Rust `PreSignature`):
Primera etapa: a partir del documento y del certificado del firmante se calculan
los datos que hay que firmar (típicamente un hash) y los metadatos necesarios
para reensamblar la firma después.
_Avoid_: pre-proceso, presign, preparación

**Firma** (`sign` · Rust `TokenSignature`):
Segunda etapa: la operación criptográfica que aplica la clave privada sobre los
datos calculados en la prefirma. Es la única etapa que toca la clave privada.
_Avoid_: firmado, sign, cifrado del hash

**Postfirma** (`postsign`):
Tercera etapa: ensamblado del documento firmado final incorporando el resultado
de la firma en el formato de firma correspondiente.
_Avoid_: post-proceso, postsign, ensamblado

**Recorrido de firma** (TS `SigningJourney`):
Todo lo que la persona hace en la ventana principal con el documento que tiene
delante, desde que elige certificado hasta que ve el acuse o el fallo: decidir
la firma visible, atender los avisos que se interponen antes del PIN y las tres
etapas de la firma trifásica. El acuse y el fallo son del documento que los
produjo, y cambiar de pestaña los cierra; el certificado elegido y la firma
visible siguen de un documento a otro. En un trámite de sede no hay recorrido:
lo conduce el backend y la ventana solo lo sigue.
_Avoid_: sesión de firma (choca con el sello de sesión), operación de firma, proceso de firma, flujo de firma

**Panel de firma** (TS `SigningPanel`):
La columna derecha de la ventana principal antes de firmar: todo lo que se
decide sobre el documento que se tiene delante y el botón que lo firma.
_Avoid_: panel lateral, formulario de firma

**Acuse** (TS `Acknowledgement`):
Lo que la ventana enseña del documento recién firmado en ella: sus firmas, dónde
quedó y cómo llegar hasta él. Es del documento que lo produjo.
_Avoid_: resumen, justificante, recibo, panel firmado

**Lectura de firmas** (TS `SignatureReading`):
Las firmas de un documento abierto solo para verlas, con su validez, sin haberlo
firmado en la ventana.
_Avoid_: verificación (promete validar contra una autoridad), vista de firmas

**Configuración de firma** (Rust `SignatureConfig`):
Conjunto de parámetros con que rFirma pide la firma de un PDF: el subfiltro, el
recuadro y su contenido. Es lo que distingue una firma de
otra a igualdad de documento y certificado. No incluye el certificado ni el
documento, que son entradas por su cuenta.
_Avoid_: extraParams, opciones de firma, perfil de firma

**Cofirma** (Rust `SignatureOperation::Cosign`):
Firma de un PDF que ya lleva otras: la nueva se añade detrás y las anteriores
siguen siendo válidas, salvo que la primera no admita cofirmas, y entonces la
nueva es no válida. Si alguna de las que ya tiene no la reconoce rFirma, la
cofirma pide antes el consentimiento de la persona.
_Avoid_: contrafirma (es otra cosa), multifirma, segunda firma

**Sello de sesión** (Rust `SessionSeal`):
Bloque que la prefirma devuelve y que la postfirma exige recibir idéntico:
lleva la configuración de firma tal y como quedó tras la prefirma, el instante
de la firma, la zona horaria y el algoritmo. rFirma lo transporta sin leerlo.
Existe porque la postfirma regenera el documento entero y cualquier diferencia
invalida la firma sin dar error.
_Avoid_: contexto de firma, sesión trifásica, sello de tiempo (es otra cosa)

**Formato de firma** (TS `SignatureFormat` · Rust `Format`):
Estándar que define cómo se estructura y se incrusta una firma en un documento:
CAdES, PAdES, XAdES y FacturaE.
_Avoid_: tipo de firma, perfil de firma

**Firma visible** (`VisibleSignature`):
Recuadro que se estampa sobre una o varias páginas del PDF para que la firma se
vea al abrir el documento. Es opcional y no aporta validez: la firma electrónica está
en la estructura del PDF, se dibuje o no. Su apariencia forma parte del
documento cuyo hash se firma, así que se decide antes de la prefirma. Dentro
del recuadro puede haber texto, la rúbrica del titular o las dos cosas; el
texto lo redacta rFirma y sigue al idioma de la aplicación.
_Avoid_: sello, marca de agua, firma gráfica

**Colocación** (`Placement`):
Dónde y en qué páginas se estampa el recuadro de la firma visible: un
rectángulo en espacio de usuario y el conjunto de páginas que lo llevan. No hay
colocación «vacía»: encender la firma visible la coloca en la página a la
vista, y apagarla deja firmar sin ella; no existe «encendida y sin colocar». El conjunto
puede ser una página, algunas o todas, y el recuadro se dibuja idéntico en
todas ellas y en ninguna más, porque el PDF lleva un solo campo de firma con su
widget replicado. En la ventana principal se recuerda por documento: «las
páginas 3, 7 y 9» no significa nada en otro PDF. En un trámite de sede que pide
firma visible sin traer recuadro, la persona la coloca igual y dura lo que el
trámite: no se guarda.
_Avoid_: ancla, posición de la firma, página de firma

**Modo de páginas** (`PageMode`):
Cuál de las tres maneras de nombrar las páginas de una colocación está activa:
una página, varias o todas. Cada modo recuerda su propio conjunto, y cambiar de
modo no reescribe el que se deja: volver a él trae lo que tenía. Un modo que se
estrena arranca con lo que nombraba el anterior. El recuadro es uno solo y no
cambia con el modo; a la firma solo llega el conjunto del modo activo. La
memoria de los otros modos dura mientras el documento sigue delante: al cambiar
de documento o volver a abrirlo solo queda la colocación que se firmaría.
_Avoid_: opción de páginas, tipo de colocación, selección de páginas

**Recuadro que pide la sede** (Rust `SiteVisibleSignature::PlacedByTheSite`):
La firma visible que un trámite de sede trae ya puesta en sus `extraParams`:
cuatro esquinas y una página, calibradas contra AutoFirma. No es una
colocación: no nace de un arrastre, no se guarda y no se convierte. Cruza al
puente tal y como vino, y rFirma solo decide si la petición lleva recuadro, si
no lo lleva o si lo que pide no se atiende.
_Avoid_: colocación de la sede, posición remota, recuadro del protocolo

**Espacio de usuario** (`UserSpaceRect`):
Sistema de coordenadas del propio PDF, en puntos, con el origen donde lo ponga
la MediaBox de la página. Es donde rFirma guarda el recuadro de la firma
visible: los píxeles del visor se derivan de él en cada pintada, nunca al
revés, porque un recuadro guardado en píxeles se desplaza sobre el documento
en cuanto cambia el zoom.
_Avoid_: coordenadas del PDF, puntos de pantalla, píxeles

**Rúbrica** (TS `Rubric` · Rust `NormalizedRubric`):
Imagen de la firma manuscrita del titular, escaneada, que puede mostrarse
dentro del recuadro de la firma visible. Es un adorno del recuadro, no la
firma: sin rúbrica la firma sigue siendo válida, y una rúbrica sin firma
electrónica no es nada. Rúbrica es **siempre** una imagen: el texto que
acompaña al recuadro no es una rúbrica, es texto de la firma visible.
_Avoid_: firma manuscrita (a secas), imagen de firma, sello, rúbrica de texto

**Modelo** (`VisibleContent`):
La forma del contenido de la firma visible, una de tres: *Completa* (firmante,
fecha y emisor), *Solo rúbrica* (la imagen y ningún texto) y *Personalizada*
(una frase que la persona escribe con los datos que quiera). «Con rúbrica» se
suma a cualquiera de los tres, y *Solo rúbrica* la lleva siempre. El texto lo
compone rFirma y el puente no sustituye nada en él.
_Avoid_: plantilla, casillas, formato del sello

**Dato** (`Datum`):
Un valor que la firma visible toma del certificado o de la firma: el firmante
—con el número de identidad enmascarado—, el emisor o la fecha. En la frase de
*Personalizada* viaja como dato, nunca como comodín `$$…$$`.
_Avoid_: comodín, variable, etiqueta, campo

### Identidad y claves

**Certificado** (TS `Certificate` · Rust `TokenCertificate`):
Certificado X.509 que identifica al firmante y que la Administración acepta como
prueba de su identidad.
_Avoid_: credencial, identidad digital

**Clave privada**:
Material criptográfico asociado a un certificado con el que se produce la firma.
Puede residir en un fichero, en el almacén del sistema operativo o dentro de una
tarjeta criptográfica.
_Avoid_: clave secreta, llave

**Clave no exportable**:
Clave privada que su custodio (una tarjeta criptográfica o el almacén del
sistema) no permite extraer: solo puede usarse delegando la operación de firma
en el propio dispositivo.
_Avoid_: clave protegida, clave bloqueada

**Tarjeta criptográfica** (Rust `StoreClass::Card`):
Dispositivo físico que custodia una clave no exportable y ejecuta la firma en su
interior, protegido por un PIN. El caso principal en España es el **DNIe**.
_Avoid_: smartcard, token, tarjeta inteligente

**Almacén** (Rust `Store`):
**Un** origen de certificados, no todos: una tarjeta criptográfica, el perfil de
Firefox, la base de datos de Chrome, el Almacén de rFirma. Son varios a la vez
y se abren por separado, así que uno que no cargue no deja sin certificados a
los demás. El mismo certificado —mismo emisor y número de serie— en varios
almacenes se muestra una vez, con la lista de almacenes donde está; se firma con
la copia recordada o, si no la hay, con la del primer almacén por este orden:
tarjeta, Almacén de rFirma, NSS del sistema, Firefox, Chrome.
_Avoid_: keystore, repositorio de certificados, llavero, «el conjunto de
certificados de la máquina»

**Almacén NSS** (Rust `StoreClass::Nssdb`):
El almacén de un navegador —el perfil de Firefox, la base de datos de Chrome—,
que es a la vez de donde salen certificados para firmar y **donde la aplicación
registra la CA local** para que ese navegador confíe en el servidor local. No es
el único almacén en el que rfirma escribe: el Almacén de rFirma también es una
base NSS, pero propia de la aplicación, no de un navegador.
_Avoid_: nssdb, base de datos de certificados, almacén del navegador

**Almacén de rFirma** (Rust `StoreClass::Installed`):
El almacén propio de rFirma en Linux: una única base NSS cifrada con un PIN
aleatorio que la persona nunca ve ni teclea, guardado en el llavero del
escritorio. Es donde caen los certificados personales que la persona instala.
El gesto que los lleva ahí se llama, en la interfaz, «Instalar certificado».
_Avoid_: almacén NSS (es otra cosa), llavero (el llavero guarda el PIN, no el
certificado), keystore

**CA local** (Rust `LocalCa`):
Certificado que rfirma genera en la máquina de la persona y registra en sus
almacenes NSS. No identifica a nadie ni firma documentos: su único trabajo es
firmar el certificado del servidor local. Es lo que se queda dentro del
navegador y puede sobrevivir a la desinstalación, así que su caducidad es la
red.
_Avoid_: ancla, ancla de confianza, CA raíz, certificado raíz

**Solape** (Rust `Stage::Overlapping`):
Los meses en los que hay dos CA locales de confianza en los almacenes NSS: la
que sirve y la siguiente. La siguiente se fabrica e instala cuando a la vigente
le queda poca vida y espera en su propia ranura; la vigente sigue firmando el
certificado del servidor local hasta que caduca, y entonces la siguiente toma el
relevo sin instalar nada y sin reiniciar ningún navegador. Instalar solo añade:
nada se borra durante el solape.
_Avoid_: rotación, renovación en caliente, rollover

**Certificado del servidor local** (Rust `LocalServerCertificate`):
El que rfirma presenta en cada saludo TLS del servidor local, firmado por la CA
local. No se guarda en ningún sitio: se genera al arrancar y vive lo que vive el
proceso.
_Avoid_: hoja, certificado de servidor, certificado TLS

**Representante**:
El titular de un certificado que firma en nombre de una entidad, no en el suyo
propio. Su sujeto trae el identificador de organización de la entidad
(`organizationIdentifier`) junto al nombre y apellidos de una persona física; un
certificado de entidad sin persona física —un sello— no es de representante.
_Avoid_: apoderado, poder notarial, persona jurídica

**Firmante**:
El titular del certificado de una firma que ya trae el documento, leído de ella;
distinto del certificado con el que tú firmas.
_Avoid_: firmador, autor

**Validez** (`Validity`):
El juicio sobre una firma que ya trae el documento, en una lista cerrada: válida,
caducada o no válida. Es la misma dondequiera que se enseñe la firma; el motivo
que la explica es detalle, no una validez más.
_Avoid_: estado, veredicto, resultado, «no se ha podido comprobar del todo»

**Hallazgo del documento** (`DocumentFinding`):
Lo que la validación encuentra en el documento entero sin poder atribuirlo a una
firma, como un cambio posterior a la última. Pesa como una firma no válida, pero
no se cuelga de ninguna.
_Avoid_: aviso, validez del documento

### Invocación

**Compatible con AutoFirma**:
Que una sede que hoy habla con AutoFirma 1.9.2 hable con rFirma sin cambiar nada,
con **cinco desviaciones a propósito**, y ninguna más: rFirma **no firma con
SHA1** salvo que la persona lo permita en sus preferencias, y nunca en un XML
que firme ella misma, porque por defecto no produce firmas con una huella rota
(ADR-0023);
**no atiende XMLDSig**, porque el original lo firma en una sola fase con la
clave privada dentro de Java y eso lo prohíbe el ADR-0001; **no reproduce la
XAdES explícita** (`mode=explicit`), que el propio original da por obsoleta y
resuelve hasheando el dato con SHA1; **solo admite el lote local en JSON**, no
el XML heredado que el original todavía acepta; y **no soporta tarjetas
criptográficas ni el DNIe**, cuya fontanería PC/SC y PKCS#11 no se distribuye en
ningún paquete (ADR-0004). Alcanza también a la **línea de órdenes** del
original, con una desviación más propia de ella: **no acepta la contraseña como
argumento** (ADR-0041). El `--json` de esa línea de órdenes es una **extensión**
de rFirma, no una desviación: el original no lo tiene y nada que lo use deja de
funcionar (ADR-0041).
_Avoid_: equivalente, clon de AutoFirma, drop-in replacement

**Petición de firma** (TS `SiteOperation` · Rust `SiteRequest`):
Solicitud, originada normalmente en una sede electrónica abierta en el
navegador, que pide firmar unos datos concretos con un certificado que el
usuario debe elegir.
_Avoid_: request, encargo, trabajo de firma

**Sede electrónica** (Rust `site`):
Sitio web de la Administración que origina la petición de firma y recibe el
documento firmado.
_Avoid_: portal, cliente web, tercero

**Trámite de sede** (`Errand`):
Lo que rFirma atiende desde que llega una URL `afirma://` hasta que la sede
tiene su respuesta: la negociación de arranque, el consentimiento de la
persona, la firma y la entrega. Uno por **proceso de sede**. Con WebSocket
abarca todas las operaciones que llegan por el canal mientras siga conectado
su primer cliente.
_Avoid_: errand en prosa, sesión, operación (que es cada verbo del protocolo)

**Rol del proceso** (Rust `Role`):
Lo que un proceso de rFirma es desde que arranca, decidido por su línea de
órdenes y sin cambiar después. El **proceso de escritorio** es la aplicación
que abre la persona, único en el equipo, con la ventana principal y la
colocación de la firma; el **proceso de sede** es el que arranca una URL
`afirma://`, uno por invocación, con su ventana de sede y sin ventana
principal, y termina con su trámite. Dos procesos de sede conviven, cada uno
en el puerto que sorteó su navegador; ninguno se une al de escritorio ni lo
cierra. El **proceso de terminal** es el que arranca una orden de la línea de
órdenes (`sign`, `cosign`, `verify`, `listaliases`): no abre más ventana que la de sede,
y solo si se le pide elegir el certificado en ella; no se une al de escritorio y
termina con su orden.
_Avoid_: modo, instancia, app de navegador, dos aplicaciones, CLI, modo consola,
modo desatendido

**Canal** (Rust `OpenChannel`):
La conexión `wss://` que la sede abre contra el servidor local, y lo que hace
falta para sostenerla: escuchar en el *loopback*, el saludo TLS y comprobar de
dónde viene la petición. Lo que lo cierra es la **credencial de canal**, abajo.
_Avoid_: socket, conexión, túnel, canal a secas para flatpak, `.deb` o `.rpm` —eso es el
**canal de distribución**

**Credencial de canal** (Rust `ChannelCredential`):
El `idsession` que la sede manda en la URL de arranque, y que repite en cada
mensaje del canal. **No es un identificador de transacción**: es lo único que
impide que otra página abierta en el mismo equipo use el canal. Es **opcional,
no ausente**: si la URL de arranque la trae, se exige en cada mensaje; si la
sede habla el protocolo 3 y no la trae, no se exige ninguna. Un valor que
llega y está mal formado se rechaza siempre, hable el protocolo que hable la
sede.
_Avoid_: id de sesión, token, identificador de transacción

**Conversación** (Rust `conversation`):
El ir y venir de mensajes sobre un canal ya abierto, con sus reglas: el eco
antes de nada, el `idsession` en cada mensaje, la espera y el sondeo del
resultado, y un solo trámite vivo a la vez.
_Avoid_: sesión de protocolo, diálogo, intercambio

**Llegada** (Rust `Arrival`):
Cuándo queda resuelto lo que se intercambia con la sede, sea la operación o un
rechazo, y lo dice el transporte al abrirse, no la URL: **esperada** si la sede
se conectará a un canal que queda escuchando para dar o recoger lo que toque
(`wss`, `service`), e **inmediata** si al abrir ya está resuelto, sea la
operación que se descarga o el rechazo que se sube (servidor intermedio). De
ella depende si la ventana de sede espera o actúa ya.
_Avoid_: modo de canal, canal sin puerto, puerto cero

**Cliente de canal** (Rust `channel_client`):
El cliente propio, escrito en Rust, con el que se prueba el canal: saluda por
`wss://`, manda el eco y comprueba los **caminos de rechazo que un cliente
conforme no puede provocar** —una credencial que no coincide, un canal abierto
sólo para rechazar, alguien que intenta hablar en claro—. No es el cliente de
nadie: existe para las pruebas.
_Avoid_: cliente de pruebas, mock del navegador, simulador de sede, banco de conformidad, suite
de conformidad

**Banco de conformidad**:
La puerta del CI: el `autoscript.js` **publicado**, fijado al tag `v1.9.2` y corriendo bajo Node,
con el que se comprueba que rfirma habla con el cliente real y no con una idea propia de él, en
proceso y con veredicto verde o rojo de `cargo test`. Es el otro trabajo, no el mismo que el del
**cliente de canal**: aquél cubre lo que el real no puede provocar, y éste cubre lo que el real
hace. No se copia al repositorio: se descarga a etiqueta fijada, con `sha256` y caché
(`just autoscript`), y vive repartido en `tests/conformance_*.rs`.
_Avoid_: tests de integración, e2e, banco de pruebas, suite de conformidad, cliente de canal

**Suite de conformidad** (`rfirma-conformance`):
La herramienta local que mide si una aplicación —AutoFirma o rFirma— cumple el protocolo. Es otro
bounded context, con su propio glosario (`CONTEXT-MAP.md`); en este sólo se nombra para no
confundirla con el banco de conformidad ni con el cliente de canal.
_Avoid_: banco de conformidad, cliente de canal

**Códec del protocolo** (Rust `ProtocolCodec`):
La traducción entre el texto que viaja por el canal y las estructuras con las
que se razona dentro: la URL de operación, la respuesta con sus campos
separados, y el formato exacto de un error.
_Avoid_: serializador, parser, marshalling

### Memoria de la aplicación

**Documento reciente** (`RecentDocument`):
Documento que la aplicación ha visto antes y ofrece para volver a él, sin
guardar una copia. Se **guarda** por su ruta canónica y se **referencia** desde
la ventana por un identificador opaco, del que no se reconstruye ninguna ruta
(ADR-0010). La fila enseña la ruta **donde se conoce** y sólo el nombre donde no
(ADR-0011): un documento que entra por el portal no tiene ruta original que
enseñar.
_Avoid_: historial, documento abierto, favorito

**Carpeta de destino** (Rust `DestinationFolder`):
Carpeta que la persona elige en Preferencias. Recibe el documento firmado con
el modo de destino «en esta carpeta», y con «junto al original» cuando el
original entra por el portal y no tiene carpeta propia. La aplicación no la
crea nunca: si no está, no está. La enseña por su ruta donde la conoce y por su
nombre donde no (ADR-0011).
_Avoid_: carpeta fija, ruta de salida

**Modo de destino** (`DestinationMode`):
Preferencia que decide dónde cae el documento firmado: «junto al documento
original» —el de omisión— o «en esta carpeta», la carpeta de destino. Bajo el
sandbox no se ofrece, porque ahí ningún original tiene carpeta propia (ADR-0011).
_Avoid_: modo de guardado, ubicación de salida

**Preferencia** (TS `Preferences` · Rust `Configuration`):
Ajuste que el usuario elige y que la aplicación se limita a obedecer: el idioma,
dónde guardar el documento firmado, los interruptores.
_Avoid_: configuración, opción, setting

**Estado** (Rust `State`):
Lo que la aplicación recuerda por su cuenta, sin que nadie se lo pida: los
documentos recientes, la última configuración de firma visible y el certificado
usado la última vez. Borrarlo no reconfigura nada.
_Avoid_: caché, historial, sesión, estado como lo que la aplicación informa de su instalación
—eso es el **diagnóstico**

**Certificado recordado** (`remembered`):
El certificado de la última firma, que la aplicación vuelve a proponer en la
siguiente. Es estado, no preferencia, y no responde por sí solo a una sede
(ADR-0010).
_Avoid_: certificado por defecto, certificado fijo, sticky

### Distribución

**Canal de distribución** (Rust `desktop::domain::channel::Channel`):
La forma en la que la aplicación llegó a la máquina: flatpak, `.deb`, `.rpm`, `windows` o una compilación de
desarrollo. Decide si corre dentro del **sandbox** y qué puede ver del sistema, así que es lo
primero que hay que saber para interpretar todo lo demás.
_Avoid_: canal a secas —es la conexión con la sede—, paquete, formato de instalación

**Sandbox**:
Confinamiento del sistema operativo en el que corre la aplicación cuando se
instala como flatpak: no ve el sistema de ficheros del anfitrión y toda entrada
y salida de documentos pasa por los portales, así que no conoce la ruta original
de un documento que entre por ahí (ADR-0004, ADR-0011). Los canales nativos
—`.deb`, `.rpm`— corren fuera de él.
_Avoid_: arenero, caja de arena, jaula, contenedor

**Señal** (`Signal`):
Una comprobación de la instalación, con su lectura y su veredicto. La **lectura** es qué se midió y
cuándo; el **veredicto** es el juicio que se pinta, y es una lista cerrada: correcto, atención,
incorrecto, no aplica y comprobando. «No aplica» —el sujeto no existe en esta máquina— y
«comprobando» no son medias tintas de «correcto». Que la lectura esté fresca o rancia es propiedad
de la lectura, no un veredicto más.
_Avoid_: check, chequeo, casilla, indicador, estado de la instalación

**Diagnóstico** (`status`):
El conjunto de señales: lo que la aplicación informa por su cuenta sobre cómo ha quedado instalada.
Es lo contrario de una **preferencia** —lo que la persona decide— y no es el **estado** —lo que la
aplicación recuerda—. De una señal puede colgar una **reparación**, que la arregla ahí mismo y
vuelve a medirla; eso no es un ajuste, es la respuesta al diagnóstico.
_Avoid_: estado de la aplicación, salud, healthcheck, autodiagnóstico

### Sistema constructivo de la interfaz

Las capas del catálogo de la interfaz, de la más atómica a la pantalla completa.
Cada pieza del catálogo pertenece a exactamente una.

**Primitivo** (`primitives`):
Elemento de interfaz sin dominio: no sabe nada de firmas, certificados ni
documentos, y sus props son genéricas. Se publica en el kit de diseño.
_Avoid_: átomo, componente base, control

**Dominio** (`domain`):
Pieza que pinta una noción del dominio con props puras: recibe datos hechos y
no carga nada por su cuenta. Se publica en el kit de diseño.
_Avoid_: entidad, molécula, pieza a secas

**Flujo** (`flows`):
Composición autónoma con la que se arma un recorrido de la aplicación: un paso,
un diálogo de un trayecto, una sección de un panel. Se publica en el kit de
diseño: es el material para diseñar recorridos nuevos.
_Avoid_: bloque —es el bloque trifásico—, widget, organismo

**Pantalla** (`screens`):
Ventana o página completa que compone flujos y piezas de dominio. Es referencia
local: no se publica en el kit de diseño.
_Avoid_: página, vista, layout

### Identidad del producto

**rFirma**:
El producto, tal y como se escribe en prosa y tal y como lo ve la persona
usuaria: el título de la ventana, el `Name=` del lanzador, el `<name>` del
metainfo, la documentación. La forma **`rfirma`**, todo en minúscula, es el
**identificador**: el binario, el `productName`, el nombre del paquete, el del
`.desktop` y el de las rutas. Es la regla de idioma del proyecto —prosa en
castellano, identificadores en inglés— aplicada a un caso que no contemplaba, y
la vigila `just check-version`.
_Avoid_: Rfirma, RFirma, RFIRMA, rFirma como identificador

**Versión** (Rust `Version`):
El número de la entrega, que vive en `rfirma-app/src-tauri/Cargo.toml`
—única fuente: Tauri v2 la sella dentro de los tres paquetes y la interfaz la
lee en tiempo de ejecución— y se replica en candado comprobado a `Cargo.lock`,
al metainfo y a la sección más reciente del CHANGELOG. Subirla arrastra además el sello de
`packaging/flatpak/sources.lock`, que guarda el `sha256` de `Cargo.lock`. La del `pom.xml` del puente **no** es esta: es un artefacto interno y
queda fuera del candado. Una **candidata** (`-rc.N`) publica sólo el flatpak.
_Avoid_: release, tag, número de build
