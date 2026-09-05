# rfirma

Aplicación nativa de firma electrónica que sustituye la interfaz de **AutoFirma**
para ciudadanos y empresas que firman ante la Administración española. Este
documento es el glosario del dominio: define **qué es** cada término, no cómo
está implementado (eso vive en `docs/adr/`).

## Language

### Proceso de firma

**Firma trifásica**:
Procedimiento de firma partido en tres etapas —prefirma, firma y postfirma— de
modo que la clave privada nunca sale del dispositivo que la custodia.
_Avoid_: firma en tres pasos, firma distribuida, triphase

**Prefirma**:
Primera etapa: a partir del documento y del certificado del firmante se calculan
los datos que hay que firmar (típicamente un hash) y los metadatos necesarios
para reensamblar la firma después.
_Avoid_: pre-proceso, presign, preparación

**Firma**:
Segunda etapa: la operación criptográfica que aplica la clave privada sobre los
datos calculados en la prefirma. Es la única etapa que toca la clave privada.
_Avoid_: firmado, sign, cifrado del hash

**Postfirma**:
Tercera etapa: ensamblado del documento firmado final incorporando el resultado
de la firma en el formato de firma correspondiente.
_Avoid_: post-proceso, postsign, ensamblado

**Configuración de firma**:
Conjunto de parámetros con que rFirma pide la firma de un PDF: el subfiltro, el
recuadro y su contenido, y el motivo si lo hay. Es lo que distingue una firma de
otra a igualdad de documento y certificado. No incluye el certificado ni el
documento, que son entradas por su cuenta.
_Avoid_: extraParams, opciones de firma, perfil de firma

**Sello de sesión**:
Bloque que la prefirma devuelve y que la postfirma exige recibir idéntico:
lleva la configuración de firma tal y como quedó tras la prefirma, el instante
de la firma, la zona horaria y el algoritmo. rFirma lo transporta sin leerlo.
Existe porque la postfirma regenera el documento entero y cualquier diferencia
invalida la firma sin dar error.
_Avoid_: contexto de firma, sesión trifásica, sello de tiempo (es otra cosa)

**Formato de firma**:
Estándar que define cómo se estructura y se incrusta una firma en un documento:
CAdES, PAdES, XAdES y FacturaE.
_Avoid_: tipo de firma, perfil de firma

**Firma visible**:
Recuadro que se estampa sobre una o varias páginas del PDF para que la firma se
vea al abrir el documento. Es opcional y no aporta validez: la firma electrónica está
en la estructura del PDF, se dibuje o no. Su apariencia forma parte del
documento cuyo hash se firma, así que se decide antes de la prefirma. Dentro
del recuadro puede haber texto, la rúbrica del titular o las dos cosas; el
texto lo redacta rFirma y sigue al idioma de la aplicación.
_Avoid_: sello, marca de agua, firma gráfica

**Colocación**:
Dónde y en qué páginas se estampa el recuadro de la firma visible: un
rectángulo en espacio de usuario y el conjunto de páginas que lo llevan. No hay
colocación «vacía»: mientras no haya al menos una página sellada, no hay
recuadro en ninguna parte y no se puede firmar con firma visible. El conjunto
puede ser una página, algunas o todas, y el recuadro se dibuja idéntico en
todas ellas y en ninguna más, porque el PDF lleva un solo campo de firma con su
widget replicado. Se recuerda por documento: «las páginas 3, 7 y 9» no
significa nada en otro PDF.
_Avoid_: ancla, posición de la firma, página de firma

**Espacio de usuario**:
Sistema de coordenadas del propio PDF, en puntos, con el origen donde lo ponga
la MediaBox de la página. Es donde rFirma guarda el recuadro de la firma
visible: los píxeles del visor se derivan de él en cada pintada, nunca al
revés, porque un recuadro guardado en píxeles se desplaza sobre el documento
en cuanto cambia el zoom.
_Avoid_: coordenadas del PDF, puntos de pantalla, píxeles

**Rúbrica**:
Imagen de la firma manuscrita del titular, escaneada, que puede mostrarse
dentro del recuadro de la firma visible. Es un adorno del recuadro, no la
firma: sin rúbrica la firma sigue siendo válida, y una rúbrica sin firma
electrónica no es nada. Rúbrica es **siempre** una imagen: el texto que
acompaña al recuadro no es una rúbrica, es texto de la firma visible.
_Avoid_: firma manuscrita (a secas), imagen de firma, sello, rúbrica de texto

### Identidad y claves

**Certificado**:
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

**Tarjeta criptográfica**:
Dispositivo físico que custodia una clave no exportable y ejecuta la firma en su
interior, protegido por un PIN. El caso principal en España es el **DNIe**.
_Avoid_: smartcard, token, tarjeta inteligente

**Almacén**:
**Un** origen de certificados, no todos: una tarjeta criptográfica, el perfil de
Firefox, la base de datos de Chrome. Son varios a la vez y se abren por
separado, así que uno que no cargue no deja sin certificados a los demás. Cada
certificado sabe de cuál salió, y hace falta: el mismo certificado en dos
almacenes es indistinguible sin decirlo.
_Avoid_: keystore, repositorio de certificados, llavero, «el conjunto de
certificados de la máquina»

**Almacén NSS**:
El almacén de un navegador —el perfil de Firefox, la base de datos de Chrome—,
que es a la vez de donde salen certificados para firmar y **donde la aplicación
registra la CA local** para que ese navegador confíe en el servidor local. Es el
único tipo de almacén en el que rfirma escribe.
_Avoid_: nssdb, base de datos de certificados, almacén del navegador

**CA local**:
Certificado que rfirma genera en la máquina de la persona y registra en sus
almacenes NSS. No identifica a nadie ni firma documentos: su único trabajo es
firmar el certificado del servidor local. Es lo que se queda dentro del
navegador y puede sobrevivir a la desinstalación, así que su caducidad es la
red.
_Avoid_: ancla, ancla de confianza, CA raíz, certificado raíz

**Certificado del servidor local**:
El que rfirma presenta en cada saludo TLS del servidor local, firmado por la CA
local. No se guarda en ningún sitio: se genera al arrancar y vive lo que vive el
proceso.
_Avoid_: hoja, certificado de servidor, certificado TLS

### Invocación

**Petición de firma**:
Solicitud, originada normalmente en una sede electrónica abierta en el
navegador, que pide firmar unos datos concretos con un certificado que el
usuario debe elegir.
_Avoid_: request, encargo, trabajo de firma

**Sede electrónica**:
Sitio web de la Administración que origina la petición de firma y recibe el
documento firmado.
_Avoid_: portal, cliente web, tercero

**Canal**:
La conexión `wss://` que la sede abre contra el servidor local, y lo que hace
falta para sostenerla: escuchar en el *loopback*, el saludo TLS y comprobar de
dónde viene la petición. Lo que lo cierra es la **credencial de canal**, abajo.
_Avoid_: socket, conexión, túnel

**Credencial de canal**:
El `idsession` que la sede sortea y manda en la URL de arranque, y que repite en
cada mensaje del canal. **No es un identificador de transacción**: es lo único
que impide que otra página abierta en el mismo equipo use el canal. Un valor mal
formado se rechaza; nunca se ignora, porque un canal sin credencial es un canal
sin cerradura.
_Avoid_: id de sesión, token, identificador de transacción

**Conversación**:
El ir y venir de mensajes sobre un canal ya abierto, con sus reglas: el eco
antes de nada, el `idsession` en cada mensaje, la espera y el sondeo del
resultado, y un solo trámite vivo a la vez.
_Avoid_: sesión de protocolo, diálogo, intercambio

**Cliente de canal**:
El cliente propio, escrito en Rust, con el que se prueba el canal: saluda por
`wss://`, manda el eco y comprueba los **caminos de rechazo que un cliente
conforme no puede provocar** —una credencial que no coincide, un canal abierto
sólo para rechazar, alguien que intenta hablar en claro—. No es el cliente de
nadie: existe para las pruebas.
_Avoid_: cliente de pruebas, mock del navegador, simulador de sede

**Banco de conformidad**:
El `autoscript.js` **publicado**, fijado al tag `v1.9.2` y corriendo bajo Node,
con el que se comprueba que rfirma habla con el cliente real y no con una idea
propia de él. Es el otro trabajo, no el mismo que el del **cliente de canal**:
aquél cubre lo que el real no puede provocar, y éste cubre lo que el real hace.
No se copia al repositorio: se descarga a etiqueta fijada, con `sha256` y caché
(`just autoscript`), y vive en `tests/conformance_bench.rs`.
_Avoid_: tests de integración, e2e, banco de pruebas

**Códec del protocolo**:
La traducción entre el texto que viaja por el canal y las estructuras con las
que se razona dentro: la URL de operación, la respuesta con sus campos
separados, y el formato exacto de un error.
_Avoid_: serializador, parser, marshalling

### Memoria de la aplicación

**Documento reciente**:
Documento que la aplicación ha visto antes y ofrece para volver a él, sin
guardar una copia. Se **guarda** por su ruta canónica y se **referencia** desde
la ventana por un identificador opaco, del que no se reconstruye ninguna ruta
(ADR-0010). La fila enseña la ruta **donde se conoce** y sólo el nombre donde no
(ADR-0011): un documento que entra por el portal no tiene ruta original que
enseñar.
_Avoid_: historial, documento abierto, favorito

**Carpeta de destino**:
Carpeta donde cae el documento firmado cuando el original entra por el portal
y no tiene carpeta propia. La aplicación no la crea nunca: si no está, no está.
La enseña por su ruta donde la conoce y por su nombre donde no (ADR-0011).
_Avoid_: carpeta fija, ruta de salida

**Preferencia**:
Ajuste que el usuario elige y que la aplicación se limita a obedecer: el idioma,
dónde guardar el documento firmado, los interruptores.
_Avoid_: configuración, opción, setting

**Estado**:
Lo que la aplicación recuerda por su cuenta, sin que nadie se lo pida: los
documentos recientes, la última configuración de firma visible y el certificado
usado la última vez. Borrarlo no reconfigura nada.
_Avoid_: caché, historial, sesión

### Distribución

**Sandbox**:
Confinamiento del sistema operativo en el que corre la aplicación cuando se
instala como flatpak: no ve el sistema de ficheros del anfitrión y toda entrada
y salida de documentos pasa por los portales, así que no conoce la ruta original
de un documento que entre por ahí (ADR-0004, ADR-0011). Los canales nativos
—`.deb`, `.rpm`— corren fuera de él.
_Avoid_: arenero, caja de arena, jaula, contenedor

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

**Versión**:
El número de la entrega, que vive en `rfirma-app/src-tauri/tauri.conf.json`
—única fuente, porque es el que el bundler sella dentro de los tres paquetes— y
se replica en candado comprobado a `package.json`, a `Cargo.toml`, a
`Cargo.lock` y al metainfo. Subirla arrastra además el sello de
`packaging/flatpak/sources.lock`, que guarda el `sha256` de `Cargo.lock`. La del `pom.xml` del puente **no** es esta: es un artefacto interno y
queda fuera del candado. Una **candidata** (`-rc.N`) publica sólo el flatpak.
_Avoid_: release, tag, número de build
