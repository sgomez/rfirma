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
Recuadro que se estampa sobre una página del PDF para que la firma se vea al
abrir el documento. Es opcional y no aporta validez: la firma electrónica está
en la estructura del PDF, se dibuje o no. Su apariencia forma parte del
documento cuyo hash se firma, así que se decide antes de la prefirma. Dentro
del recuadro puede haber texto, la rúbrica del titular o las dos cosas; el
texto lo redacta rFirma y sigue al idioma de la aplicación.
_Avoid_: sello, marca de agua, firma gráfica

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

**Almacén de certificados**:
Conjunto de certificados disponibles para firmar en la máquina del usuario, ya
provengan del sistema operativo, de un fichero o de una tarjeta criptográfica.
_Avoid_: keystore, repositorio de certificados, llavero

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

### Memoria de la aplicación

**Documento reciente**:
Documento que la aplicación ha visto antes y ofrece para volver a él, sin
guardar una copia: solo dónde estaba y cómo pintar su fila.
_Avoid_: historial, documento abierto, favorito

**Carpeta de destino**:
Carpeta donde cae el documento firmado. La aplicación la enseña por su
**nombre**, nunca por su ruta, y no la crea: si no está, no está.
_Avoid_: carpeta fija, ruta de salida, junto al original

**Preferencia**:
Ajuste que el usuario elige y que la aplicación se limita a obedecer: el idioma,
dónde guardar el documento firmado, los interruptores.
_Avoid_: configuración, opción, setting

**Estado**:
Lo que la aplicación recuerda por su cuenta, sin que nadie se lo pida: los
documentos recientes, la última configuración de firma visible y el certificado
usado la última vez. Borrarlo no reconfigura nada.
_Avoid_: caché, historial, sesión
