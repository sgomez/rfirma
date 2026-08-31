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

**Formato de firma**:
Estándar que define cómo se estructura y se incrusta una firma en un documento:
CAdES, PAdES, XAdES y FacturaE.
_Avoid_: tipo de firma, perfil de firma

**Firma visible**:
Recuadro que se estampa sobre una página del PDF para que la firma se vea al
abrir el documento. Es opcional y no aporta validez: la firma electrónica está
en la estructura del PDF, se dibuje o no. Su apariencia forma parte del
documento cuyo hash se firma, así que se decide antes de la prefirma.
_Avoid_: sello, marca de agua, firma gráfica

**Rúbrica**:
Imagen de la firma manuscrita del titular, escaneada, que puede mostrarse
dentro del recuadro de la firma visible. Es un adorno del recuadro, no la
firma: sin rúbrica la firma sigue siendo válida, y una rúbrica sin firma
electrónica no es nada.
_Avoid_: firma manuscrita (a secas), imagen de firma, sello

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
