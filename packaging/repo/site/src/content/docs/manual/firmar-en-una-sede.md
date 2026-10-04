---
title: Firmar en una sede electrónica
description: Qué pasa al pulsar «Firmar» en una sede electrónica que pide AutoFirma y la abre rFirma, paso a paso, del aviso del navegador al documento firmado y enviado.
---

Muchas sedes electrónicas de la Administración piden AutoFirma para firmar un
trámite. rFirma atiende esas mismas peticiones: cuando la sede pulsa «Firmar»,
se abre la **ventana de sede** de rFirma, que dice quién pide la firma y qué se va
a firmar, y no firma nada hasta que lo consientes.

## Antes del primer trámite

La primera vez que se abre, rFirma propone **configurar el equipo** en dos pasos:

1. **La conexión segura entre el navegador y rFirma.** rFirma instala en tus
   navegadores un certificado propio, la CA local, para que la página de la sede
   pueda hablar con ella. Firefox necesita reiniciarse para tenerlo en cuenta.
2. **Usar rFirma por defecto.** Si tienes AutoFirma instalada, las sedes la abren
   a ella; con este paso las sedes abren rFirma.

Si omitiste la configuración, se completa después en la ventana **Estado de
rFirma**, desde el menú: «Firma en sedes» ofrece **Usar rFirma**, y
«Certificado de rFirma», **Instalar**.

## El aviso del navegador

La sede llama a rFirma con un enlace `afirma://`. Antes de abrirla, el navegador
pregunta si quieres abrir la aplicación externa que atiende esos enlaces; según
el sistema, la nombra rFirma o `xdg-open`. Acepta, y si el navegador ofrece
recordar la elección para esa sede, no volverá a preguntar.

Después, el navegador puede pedir un segundo permiso: que la página acceda a la
**red local**, que es por donde habla con rFirma, que está en tu propio equipo.
En Chrome sale como una franja bajo la barra de direcciones y en Firefox como un
panel junto a ella. Pulsa **Permitir**.

## La ventana de sede

Es una ventana pequeña, aparte de la ventana principal de rFirma, que acompaña
el trámite de principio a fin. Pasa por estos momentos:

### Conectando con la sede

rFirma espera a que la página de la sede se conecte. Si la conexión no llega, la
ventana lo dice con **«La petición no ha llegado»** y enseña cómo arreglarlo en
Chrome y en Firefox: casi siempre falta el permiso de red local del navegador o
la CA local de rFirma. Tras arreglarlo, vuelve a la sede y pulsa su botón de
reintentar. Más detalle en [Problemas frecuentes](/manual/problemas-frecuentes/).

Si la página de la sede usa una versión antigua de la pieza que habla con
rFirma, antes sale el aviso **«Esta página está desactualizada»**. Pulsa
Continuar para seguir con el trámite.

### Marcar dónde va la firma

Solo si la sede pide que la firma se vea en el PDF. La ventana crece para enseñar
el documento, y trazas con el ratón el recuadro de la firma. La sede decide qué
va dentro del recuadro.

### El consentimiento

El momento central del trámite, y **siempre aparece**, también cuando solo tienes
un certificado. La ventana dice:

- **Quién pide la firma**: la dirección de la página, por ejemplo «sede.ejemplo.es
  pide tu firma de un documento PDF». Si la página no se puede identificar, lo
  dice así.
- **Qué vas a firmar**: un documento PDF, un documento XML, una factura
  electrónica o un reto de autenticación. Del PDF da el título, las páginas y el
  tamaño, y si ya trae firmas, cuántas y si son válidas, con «Ver firmas» para
  revisarlas.
- **Si tu firma se añade a otras**: una cofirma va junto a las firmas que ya
  tiene el documento; una contrafirma va sobre ellas, y la ventana dice sobre
  cuáles.
- **Con qué certificado**: el desplegable lista los certificados de tus
  almacenes y tarjetas. Si la sede solo acepta algunos, la ventana avisa de que
  la sede ha limitado los certificados válidos. Los certificados caducados no se
  ofrecen nunca.

Al pulsar **Firmar**, rFirma pide el PIN de la tarjeta o del almacén, si hace
falta, y firma. **Cancelar** devuelve a la sede la cancelación, y la sede decide
qué hacer con ella.

El botón de firmar tarda tres segundos en activarse, para que un Intro pulsado
por descuido no firme. Se apaga en Preferencias, con «Esperar 3 segundos antes
de firmar».

Una sede puede pedir que, si solo sirve un certificado, se use sin preguntar.
rFirma no lo hace salvo que lo permitas en Preferencias, con «Si solo sirve un
certificado y la sede lo permite, usarlo sin preguntar».

### Confirmar un aviso del documento

Si el PDF tiene algo que debes saber antes de firmar —está certificado y tu
firma invalidaría la anterior, o se modificó después de la última firma—, la
ventana lo pregunta antes del consentimiento. Continuar sigue con el trámite;
cancelar lo termina.

### Firmando y enviando

La ventana enseña con qué certificado se firma y, después, que la firma viaja a
la sede. Mientras rFirma firma se puede cancelar; cuando la firma ya va camino de
la sede, no hay nada que cancelar y no se ofrece. No cierres la ventana mientras
tanto.

### El desenlace

La ventana termina diciendo cómo ha ido:

- **Firmado y enviado**: la firma ya está en la sede. **rFirma no guarda copia**
  del documento ni lo añade a los recientes.
- **Has cancelado la firma.**
- **No se ha completado la petición**: la ventana dice qué ha fallado y qué
  puedes hacer, y trae un detalle técnico que se puede copiar para dárselo a
  quien mantiene la sede.

La ventana se cierra sola a los 15 segundos, salvo cuando enseña algo que pide
actuar.

### Sin certificado utilizable

Si no hay ningún certificado que elegir, en lugar del consentimiento la ventana
dice por qué:

- **«No tienes ningún certificado»**: hace falta uno instalado. Se consigue en
  la FNMT o en tu comunidad autónoma, y se instala desde la propia ventana con
  **Instalar un certificado…**
- **«La sede no acepta tu certificado»**: tienes certificados, pero ninguno de
  los que la sede admite. Instala otro, o cierra y sigue desde la sede.

Si acabas de conectar la tarjeta o de instalar el certificado con la ventana
abierta, vuelve a buscar desde la propia ventana. Salir abandona el trámite.

## Ceder los datos de identidad

Algunas sedes no piden una firma, sino **tus datos de identidad**, por ejemplo
para entrar. La ventana lo dice así —«sede.ejemplo.es pide tus datos de
identidad»— y explica qué se envía: tu nombre, tu NIF, el emisor del certificado y
su número de serie. Es el certificado público, sin firma: al pulsar **Enviar mis
datos** la sede recibe esos datos y nada más.

## Guardar y cargar ficheros

Una sede también puede pedir a rFirma que **guarde** un fichero en tu equipo o
que le **envíe** uno o varios ficheros tuyos. En ambos casos rFirma abre el
diálogo de ficheros del sistema y eres tú quien elige dónde guardar o qué enviar:
la sede solo propone un nombre. La ventana termina con «Guardado» o «Cargado».

Hay sedes que piden firmar un documento que eliges tú: primero sale el diálogo de
ficheros para elegirlo, y después el consentimiento de siempre. Y otras piden
firmar y guardar el resultado: después de firmar, el diálogo pregunta dónde
guardarlo.

## Los lotes

Una sede puede pedir varias firmas de una vez, en un **lote**, con un solo
consentimiento:

- En el **lote remoto**, los documentos se quedan en la sede y rFirma firma sin
  descargarlos, así que la ventana solo dice cuántas firmas son.
- En el **lote local**, la sede envía los documentos y la ventana lista cada uno
  con la operación que pide: firma, cofirma o contrafirma.

Al terminar, la ventana dice cuántas firmas del lote han llegado a la sede.
