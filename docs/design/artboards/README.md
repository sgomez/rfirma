# Artboards importados de Claude Design

**Este directorio se mantiene hasta cerrar la v1.0.** Lo decidió el
[#80](https://github.com/sgomez/rfirma/issues/80#issuecomment-5521081522) el 03/09/2026,
enmendando lo que decía antes —que era una importación de un solo uso y se borraba al
terminar ese issue—. Se ha usado para decidir en la v0.2, la v0.3 y la 0.3.1, y es la copia
1-1 que permite revisar la interfaz **sin cuenta de Claude**, que es el motivo por el que se
importó a un repositorio público. Se ha usado también en la v0.4. **No preguntes si hay que
borrarlo: no, hasta la v1.0.**

Son los diecinueve artboards del canvas de Claude Design «Autofirma de escritorio
en Rust», bajados literalmente, más el `canvas.json` que los ordena y los titula.
Tres de ellos —`Main`, `EstadoExito` y `PreferenciasPantalla`— se rehicieron el
02/09/2026 con las decisiones de v0.2 del
[#123](https://github.com/sgomez/rfirma/issues/123); ver «Lo que cambió en v0.2»
al final. Ese mismo día `Main` volvió a cambiar y nació `EstadoPaginasSinSello`
con las decisiones de v0.3 del [#155](https://github.com/sgomez/rfirma/issues/155);
ver «Lo que cambió en v0.3». Los cambios de la v0.4 —el
[#250](https://github.com/sgomez/rfirma/issues/250)— tocan diez de los catorce y
no crean ninguno; ver «Lo que cambió en v0.4». La v0.5 —el
[#317](https://github.com/sgomez/rfirma/issues/317)— es la primera que **crea
artboards nuevos** desde la v0.3: cinco, en una página propia, más dos de la
ventana principal tocados de rebote; ver «Lo que cambió en v0.5».
Están aquí para que la transcripción a JSX se pueda hacer y revisar **sin
cuenta de Claude**, y porque el repositorio es público y su interfaz no puede
estar especificada detrás de un servicio con acceso restringido.

La referencia **normativa** de cada pantalla son las fichas de `docs/design/`,
como dice `docs/agents/prototyping.md`; el lienzo es la fuente primaria de la
decisión que las sostiene, y este directorio su copia legible sin cuenta.
Cuando llegue la v1.0 habrá que decidir de nuevo si sigue haciendo falta.

## Qué es cada fichero

`canvas.json` numera los estados y los reparte en **tres** páginas. El orden de
la página «Recorrido de firma» es el de la ficha `ventana-principal.md`; la
página «Ventana de sede · v0.5» va aparte porque es otra ventana:

| # | Artboard | Estado |
| - | -------- | ------ |
| 1 | `EstadoVacio` | Vacío, con el menú de la cabecera **dibujado abierto** |
| 2 | `EstadoDocumentoCargado` | Documento cargado, sin certificado |
| 2b | `EstadoElegirCertificado` | Eligiendo entre varios certificados |
| 3 | `EstadoCargandoCertificados` | Buscando certificados, y el diálogo de secreto del almacén cuando la sesión se abre **antes** de listar |
| 4 | `EstadoSinCertificados` | Sin certificados, con salida a instalar uno |
| 5 | `Main` | **Colocando** la firma visible — el nudo del recorrido, con el pie del destino y el bloque de colocación de v0.3, el botón de sellar de la 0.3.1 y la franja de notificación de la v0.4 |
| 6 | `EstadoPin` | Pidiendo el secreto del almacén — PIN o contraseña, según la clase de almacén |
| 7 | `EstadoPinIncorrecto` | Secreto incorrecto |
| 8 | `EstadoFirmando` | Firmando, con las tres fases |
| 9 | `EstadoExito` | Firmado — el resumen, sin la ficha 14 |
| 10 | `EstadoErrorFirma` | Error de firma, en el pie del panel |
| 5b | `EstadoPaginasSinSello` | Antes de firmar: las páginas donde el recuadro no cabe |
| — | `PreferenciasPantalla` | Preferencias, a pantalla completa, con los certificados en fichero de la v0.4 |
| — | `EstadoAcercaDe` | Diálogo de «acerca de», con el «cómo actualizar» de la v0.4 |
| S1 | `SedeEspera` | Ventana de sede: esperando el canal, y las dos recetas de reparación cuando no se abre |
| S2 | `SedeConsentimiento` | Ventana de sede: el consentimiento — quién pide, qué se firma y con qué certificado |
| S3 | `SedeFirmando` | Ventana de sede: firmando y devolviendo la firma a la sede |
| S4 | `SedeDesenlace` | Ventana de sede: firmado, cancelado o petición rechazada |
| S5 | `SedeSinCertificado` | Ventana de sede: sin ningún certificado, o con todos excluidos por la sede |

Los cinco de `Sede*` viven en la página **«Ventana de sede · v0.5»** y su ficha
es [`ventana-de-sede.md`](../ventana-de-sede.md), **una sola para los cinco**:
es una ventana con una secuencia, no cinco pantallas
([#332](https://github.com/sgomez/rfirma/issues/332)). Miden 720 × 600 px, no
1180 × 700: la ventana es de 520 × 420 y se dibuja centrada sobre un lienzo que
representa el escritorio, para que se vea su tamaño real.

No se ha importado `firmar-fichero-local.dc.html`: `canvas.json` lo aparta en
la página «Otros» y lo marca como ajeno al recorrido.

## Cómo leerlos

Un `.dc.html` es un **artboard**, no un componente. Lleva andamiaje que no va a
la aplicación y que el ID-43 del #80 manda quitar: la etiqueta `<x-dc>`, el
`<helmet>`, la prop `{{tema}}`, el bloque `data-dc-script` y `support.js` (que
no se ha importado: sin él los ficheros no se renderizan solos, y no hacen
falta para transcribir).

**El `<helmet>` no es la fuente del sistema de diseño.** Es una copia
comprimida y le faltan tokens; manda el bundle versionado (ID-47). **Todos** los
ficheros lo llevan byte a byte idéntico al de **`_helmet.part`**, que es de
donde se copia al redactar uno nuevo —nunca de un `get_file` del proyecto de
Claude Design, cuya copia se queda atrás—, y `comprueba.sh` lo verifica contra
ese fichero. Compararlos solo entre sí no valía: trece ficheros de acuerdo
entre ellos dan verde con el sistema de diseño equivocado entero.

## `EstadoElegirCertificado` no viene del canvas original

Los otros doce se bajaron del canvas tal cual. Este se **añadió después**, el
01/09/2026, porque el recorrido no tenía pantalla para elegir entre varios
certificados: con más de uno el panel enseñaba «Elegir certificado» y ese botón
se limitaba a volver a buscar, así que no había forma de elegir ninguno.

Es el único que **se puede pulsar**: abre el desplegable, se desplaza y se
elige, y lleva tres palancas —estado inicial, cuántos certificados hay y si se
listan los que no sirven— para poder decidir mirando en vez de suponiendo. Lo
que se decidió con él:

- **Desplegable superpuesto**, y no un acordeón en flujo ni un diálogo: la lista
  flota sobre el panel, así que la firma visible y el botón de firmar no se
  mueven al abrirla.
- **Un certificado caducado o revocado se lista, dice por qué y no se deja
  elegir** (`disabled`). Que falte de la lista no le explica nada a quien viene
  a firmar justo con él.
- **La fila lleva el almacén** —`DNI · emisor · almacén`—, porque el mismo
  certificado en el perfil de Firefox y en `~/.pki/nssdb` es indistinguible sin
  él. El disparador cerrado no lo lleva: elegido ya no desambigua nada. En la
  v0.4 las dos filas de almacén «Tarjeta» pasan a «Instalado en rFirma» y a
  «Chrome»: siguen siendo tres clases distintas, que es lo que sostiene la
  columna.
- **Sin preselección la primera vez**: elegir con qué identidad se firma no lo
  hace la aplicación, y el orden de la lista solo dice en qué orden cargaron los
  módulos.
- **El certificado se recuerda al firmar con él**, no al elegirlo en la lista.

Sustituye al botón `Cambiar` de la tarjeta de certificado que declaraba
`panel-de-firma.md`: el disparador del desplegable ya es el sitio donde se
cambia.

## Lo que hay que decidir al transcribir

Una cosa que el canvas da por buena y el código no puede sostener tal cual.
No la resuelvas por tu cuenta: es un cambio de ficha (ID-44).

1. **El panel enseña datos que hoy nadie calcula**: «27 páginas · 2,4 MB», «Ya
   lleva 1 firma: la tuya será una cofirma» y, con la palanca «Ficha 14»
   levantada, «2 firmas» y la lista de firmas. El código pasa el tamaño y las
   firmas como desconocidos, y detectar si un PDF ya viene firmado está fuera
   del alcance del #81. El tamaño **sí** se recupera en el resumen: lo conoce
   `finish_signing` y hoy `SignedDocumentView` lo descarta.

Los otros dos puntos que había aquí —«Junto al documento original» como destino,
y la ausencia de «Recordar mi actividad» y «Vaciar la lista»— los resolvió
`PreferenciasPantalla`, y ya no hay nada que decidir.

## Lo que cambió en v0.2

Decidido en la conversación del
[#123](https://github.com/sgomez/rfirma/issues/123) y dibujado el 02/09/2026.

**`PreferenciasPantalla` sustituye a `EstadoPreferencias`**, que se ha borrado.
El diálogo de 480 px pasa a ocupar toda la ventana bajo la cabecera, que se
queda intacta con su estado de documento: sigue siendo un diálogo, no una
navegación. Índice de secciones a la izquierda —Firma, Privacidad,
Apariencia—, columna de contenido en medio y **pie fijo con `Cerrar`**, que así
no se pierde al desplazarse. Los ajustes se siguen guardando al cambiarlos: no
hay «Guardar» ni «Cancelar». El destino deja de ser un desplegable de una sola
opción y pasa a un selector de directorio con `Cambiar carpeta…`.

Tres palancas: **Ficha 5b** saca cada uno de los dos avisos de error que hoy se
tragan —el de guardar un ajuste y el de vaciar la lista—, **Apagar Recordar mi
actividad** saca la confirmación destructiva, que sigue siendo un diálogo
pequeño encima porque el borrado es irreversible, y **Maquetación** decide el
ancho de la columna.

**`Main` estrena el pie del destino**: carpeta *más* nombre de fichero, con la
última carpeta precedida de `…/` y el nombre recortado por el medio. La carpeta
entera, la extensión y la cola —`-firmado` y su número de desempate— no se
recortan nunca; la línea envuelve a dos renglones antes que cortarse. La palanca
**Pie · destino** recorre los cinco casos, incluido el destino no disponible en
el que el botón de firmar **no** se apaga; **Pie · recorte** lleva dos
deslizadores, uno para el nombre y otro para la carpeta.

**`EstadoExito` estrena el resumen sin la ficha 14**: nombre, tamaño,
encabezado `Resumen` con la insignia `PAdES` sola guardando el sitio, y tres
botones verticales —`Abrir el PDF` primario, `Abrir la carpeta` secundario,
`Volver a firmar` fantasma—. `Firmar otro documento` se retira: la bandeja ya
abre y acepta arrastre. La palanca **Ficha 14** enseña qué ocupará ese hueco en
v1.0.

**Los otros ocho artboards que llevan pie se barrieron a la vez**: ninguno
enseña ya `~/Documentos/…`, que es la ruta entera que el componente **ruta de
destino** del sistema de diseño no admite en el pie —regla de cuánto se pinta,
no de qué ruta se conoce, que es lo que decide el ADR-0011—. Los suyos
son estáticos —solo `Main` lleva las palancas—, pero siguen la misma regla:
carpeta atenuada con `…/` delante, nombre en color de texto, y la línea
envolviendo en vez de cortarse.

[#80]: https://github.com/sgomez/rfirma/issues/80

## Lo que cambió en v0.3

Decidido en el [#155](https://github.com/sgomez/rfirma/issues/155) —que absorbió
al [#154](https://github.com/sgomez/rfirma/issues/154)— y dibujado el
02/09/2026. Se supone siempre la firma visible activada. No hay pantallas
nuevas: cambia `Main` y nace un diálogo.

**`Main` estrena el bloque «Colocación»**, que sustituye a la línea «Página 3 ·
arrástralo para colocarlo». Tres opciones en radio —`Solo 1 página` con el
número en el pie, `Estas páginas` con un campo en formato de impresión
(`1,2-3,10-20`), y `Todas las páginas (27)`— más la frase de la limitación
cuando hay más de una. Bajo la hoja aparece una pastilla con tres caras
—colocar, sellar esta página, quitar el sello— y los tiradores del recuadro
funcionan, con un tamaño mínimo.

**Es el segundo artboard que se puede pulsar**, después de
`EstadoElegirCertificado`. La palanca «Colocación» salta a los ocho casos del
recorrido; desde ahí, los radios, la pastilla y las flechas de página funcionan
de verdad. La palanca «tecleado» recorre los tres errores del rango, que
**apagan el botón de firmar** en vez de recortar en silencio (ID-22); «zoom»
recorre 50 %, 100 % y 300 % para comprobar que los tiradores no escalan con la
hoja; «tamaño» enseña el mínimo útil.

**`EstadoPaginasSinSello` es nuevo**: el diálogo que avisa, antes de firmar, de
las páginas donde el recuadro no cabe. Palanca «Cuántas se caen»: 1, 3 y 12 de
13 elegidas, más 3 de 27 con todas elegidas. **Las páginas no se nombran una a
una**, se dice el total *n* de *m*, y *m* es el conjunto elegido y no el
documento.

**Se descartaron por el camino** una tira de miniaturas bajo el visor, una
etiqueta colgando del recuadro, y los cuatro campos de medidas en puntos de la
ficha 6. Los porqués están en
[panel-de-firma.md](../panel-de-firma.md) y
[visor-de-documento.md](../visor-de-documento.md).

**Y `Main` estrena la palanca «Vista previa»**, que decide qué se ve **dentro**
del recuadro (ficha 7, [#156](https://github.com/sgomez/rfirma/issues/156)):
fiel, recalculando, moviendo, y no se ha podido dibujar. La regla que lo
sostiene es que **o es el sello de verdad, o no hay recuadro** — no se enseña
nunca una aproximación—, y lo que hace que se cumpla es que **sin certificado el
bloque entero de firma visible está apagado y en gris**, que es lo que
`EstadoDocumentoCargado` ya dibujaba sin que ninguna ficha lo recogiera. Al
cerrar el #156 se corrigió ahí el interruptor, que se pintaba encendido dentro
del bloque apagado, y la línea «Página 3 · arrástralo para colocarlo», que además
contradecía al #155, pasó a «Se activa al elegir certificado».

Esto **enmienda el ID-44**, que dejaba el recuadro vacío: su argumento era
correcto para lo que se sabía entonces, y el sondeo
[#115](https://github.com/sgomez/rfirma/issues/115) le quitó la premisa —no hay
que maquetar el sello, se le pide a quien lo dibuja—. Hubo dos artboards de
trabajo, `VistaPreviaRecuadro` y `VistaPreviaDentro`, en una página aparte; **se
han borrado al fundirlos aquí**, para no dejar dos sitios donde mirar la misma
pantalla.

## Lo que cambió en v0.3.1

Decidido el 03/09/2026 y dibujado el mismo día. **No hay artboards nuevos**: los
dos que nacieron para decidir —`SellarEstaPagina` y `DisparoDelSello`, en sus
dos páginas propias— se fundieron aquí y **se borraron**, que es la regla.
Cambian `Main`, `EstadoElegirCertificado` y `EstadoDocumentoCargado`.

**Los catorce pasan de 1000 a 700 px de alto.** La ventana abría a 1440×900 con
un mínimo de 700, sin mirar la pantalla: en un portátil de 1366×768 nacía más
alta que el escritorio y el pie del panel quedaba fuera, y el mínimo impedía
encogerla para recuperarlo. Pasa a 1280×720 con mínimo 1100×560, y **no se
añade código de monitores**: el gestor de ventanas ya coloca, lo que había que
hacer era dejar de atarle las manos.

**`Main` estrena el botón de sellar en el panel**, dentro del bloque
«Colocación», con sus tres caras. Venía de una pastilla bajo la hoja que iba
*en flujo* dentro del desplazamiento: al ampliar, la hoja crecía y el botón se
iba de la vista. Se descartaron la pastilla flotante —habría competido por el
mismo hueco que el estado del sello— y meterlo en la botonera de paginación, que
está centrada y bailaría de sitio con cada cara, además de mezclar navegación
con una acción que modifica el documento.

**Y la pastilla renace flotante y dedicada al sello**, en el hueco sobre la
botonera donde ya vivía el aviso del visor. Mide lo mismo tenga botón o no: sin
altura fija pega saltos al pasar de «congelado» a «sin componer». No dice nada
de colocación, porque la etiqueta del botón del panel ya lo cuenta.

**Fuera el bloque «Vista previa» del panel**, con su insignia de estado, y fuera
los tres mensajes de colocación con su botón «ir a la página». El sello en
directo sobre la hoja **no se toca**.

**El recuadro estrena cinco casillas** —`Firmante`, `Emisor`, `Fecha`,
`Rúbrica`, `Motivo`— y un solo **párrafo** separado por puntos en vez de cuatro
líneas. La ofuscación se muda al `CN`, que es donde la aplica AutoFirma: estaba
puesta en una línea «DNI» aparte mientras el `CN` se estampaba en claro, y como
los certificados españoles llevan el DNI dentro del `CN`, tapaba lo que ya se
leía arriba. Además `layer2FontSize` pasa a 0, con lo que la letra crece y
encoge con el recuadro en vez de quedarse en 12 pt como tope.

**`EstadoElegirCertificado` estrena grupos**: `Disponibles` y `No utilizables`,
cada uno alfabético con `localeCompare("es")` y el almacén desempatando. La
palanca «Orden» conserva el de hoy —el que responden los módulos PKCS#11— para
poder comparar.

## Lo que cambió en v0.4

Decidido en el [#250](https://github.com/sgomez/rfirma/issues/250) —cuyo mapa es
el [#217](https://github.com/sgomez/rfirma/issues/217)— y dibujado el
04/09/2026. **No hay artboards nuevos y no hubo ninguna página de trabajo**:
todo entra en artboards que ya existían. Cambian diez —`Main`,
`PreferenciasPantalla`, `EstadoAcercaDe`, `EstadoPin`, `EstadoPinIncorrecto`,
`EstadoCargandoCertificados`, `EstadoElegirCertificado`, `EstadoFirmando`,
`EstadoSinCertificados` y `EstadoErrorFirma`—, y los catorce pierden una regla
del `<helmet>`.

**`Main` estrena la franja de notificación**, bajo la cabecera: descartable, con
una frase y **una sola acción** —«Cómo actualizar», que abre *Acerca de*—, y
41 px de alto en una ventana cuyo mínimo son 560. Lo decidido **no es dónde va
el aviso de versión sino dónde notifica rFirma**: la franja es el patrón, y el
aviso es su primer inquilino. Se dibujaron cuatro colocaciones y se juzgaron con
la ventana **ocupada** —documento cargado, pie de destino, nombre largo—, porque
un aviso que sólo se ve con la ventana vacía no decide nada. **Se descartaron**
una insignia en el botón de menú, que no se ve hasta abrir el menú y por tanto
no notifica, y una línea en el pie, que estrenaba una barra de estado entera
—rFirma no tiene ninguna— para una frase que casi siempre no está. Las tres
descartadas **se han borrado del artboard**; la palanca sobrevive convertida en
palanca de **estado** de dos posiciones, «hay versión nueva» y «al día».

**`PreferenciasPantalla` gana tres cosas.** Una **sección «Certificados en
fichero»** con una lista y dos gestos, «Añadir…» y «Quitar» —del fichero no se
recuerda nada, ni la ruta, así que la fila identifica al **certificado**—, con
sus tres palancas: cuatro instalados, ninguno, y el rechazo de una clave
elíptica al instalar, en un renglón y sin explicación técnica. El **ajuste del
aviso de versión**, en *Privacidad*, que **está siempre**. Y **«junto al
original» pasa a ser condicional** al entorno que sabe devolver la ruta real:
los dos estados llevan `min-height` de 200 px para que las secciones de debajo
no salten.

**`EstadoAcercaDe` estrena «cómo actualizar»**: tres pestañas de canal y **un
solo bloque de órdenes** con «Copiar», presente en los dos estados de versión,
de modo que lo único que cambia entre ellos es la línea de arriba y miden lo
mismo por construcción. Lo que se enseña son las órdenes de alta del repositorio
y no un botón de descarga, así que el mecanismo se autoliquida; y no hay ningún
enlace pulsable, porque `opener:deny-open-url` sigue denegado. El diálogo pasa
de 460 a **520 px** y el nombre de 32 a **28 px**.

**`EstadoPin` y `EstadoPinIncorrecto` estrenan la palanca «Clase de almacén»**,
con las tres situaciones: módulo PKCS#11 (**«PIN»**, antes de listar,
`Continuar`), perfil de Firefox (**«contraseña»**, antes de listar, `Continuar`)
y `.p12` instalado (**«contraseña»**, al firmar, `Firmar`). **Fuera el contador
de reintentos**, que era estructuralmente imposible: la información de token de
PKCS#11 no lo trae ni con una tarjeta real. **Fuera todas las pistas.** Y fuera
la jerga: el diálogo no nombra la clase de módulo ni la etiqueta del token.

**`EstadoCargandoCertificados` estrena el diálogo de sesión**, que era la mitad
menos visible del cambio y no se veía en ninguna pantalla: con Firefox y
contraseña maestra se pide **aquí**, con la pantalla todavía buscando y sin
lista detrás. Es el diálogo de «6» copiado literal.

**Las tarjetas salen del dibujo, no sólo del código.** En
`EstadoElegirCertificado` no queda ningún almacén «Tarjeta»; `EstadoFirmando`
dice «Firmando» a secas y pierde «No retires la tarjeta hasta que termine»;
`EstadoErrorFirma` pasa de «La tarjeta se ha desconectado» a «El certificado ha
dejado de estar disponible», que cubre el perfil que se cierra y el `.p12` que
se retira sin nombrar hardware —el detalle técnico copiable no cambia, porque es
lo que devuelve PKCS#11 en los tres casos—; y `EstadoSinCertificados` pierde el
remedio del lector y cambia «Otro módulo…» por «Añadir un certificado…».

**Los catorce pierden `.rf-field--error .rf-hint::before`**, el `"! "` delante
del texto de ayuda de un campo en error. En castellano la exclamación abre con
`¡`, así que un `!` suelto se lee como una exclamación mal cerrada. El error
sigue marcado sin color, con el borde de 2 px y la negrita de la ayuda. Se ha
quitado también de `_helmet.part`, del bundle del sistema de diseño y de los dos
documentos que lo prescribían.

**Y una regla de redacción que atraviesa los diez**: si borrar una frase no
cambia lo que la persona puede hacer, la frase sobra. Por ahí se ha ido una
docena —las tranquilizadoras («no se guarda en ningún sitio», «la clave privada
no sale de tu ordenador»), las que narran el mecanismo, las que explican lo
evidente y los remedios obvios—. La única que se salvó, recortada, es «La
carpeta no se crea nunca».

## Lo que cambió en v0.5

Decidido en el [#317](https://github.com/sgomez/rfirma/issues/317) —cuyo mapa es
el [#308](https://github.com/sgomez/rfirma/issues/308)— y dibujado el
05/09/2026. **Cinco artboards nuevos y ninguna página de trabajo**: la ventana de
sede no existía en ningún sitio, así que no había artboard donde meterla con una
palanca. De rebote cambian dos de la ventana principal, `EstadoPin` y
`EstadoElegirCertificado`. Los diecinueve conservan el mismo `<helmet>`.

**La ventana de sede es una ventana, no una pantalla más de la aplicación.** Es
un diálogo de **520 × 420 px** sin cabecera, sin menú, sin recientes y sin pie de
destino, dibujado centrado sobre un lienzo de 720 × 600 que hace de escritorio
para que se vea su tamaño real. No sustituye nunca lo que hubiera en la ventana
principal: es un trámite ajeno y corto, y sugerir que hay más dentro invita a
buscar cosas que no están. Los cinco artboards son **cinco momentos de la misma
ventana**, y por eso tienen **una sola ficha**,
[`ventana-de-sede.md`](../ventana-de-sede.md), y no cinco
([#332](https://github.com/sgomez/rfirma/issues/332)).

**`SedeConsentimiento` es el que justifica el hito.** Lo que hoy enseña
AutoFirma cuando lo llama una sede es el selector de certificados a secas: no
dice **quién** pide, ni **qué** se firma, ni que haya una sede detrás. Eso se
dibuja como opción de la palanca `forma` —«hoy · selector de certificado»— para
poder compararlo, y **`confirmacion` es el `default`**. El certificado **no se
reinventa**: es el mismo desplegable de «2b · Elegir certificado», con sus
mismas clases, su agrupación `Disponibles` / `No utilizables` y su lista de
232 px. Cuatro situaciones, y la quinta que llegó a existir —«cero tras el filtro
de la sede»— **se borró de aquí y se mudó entera** a `SedeSinCertificado`: no era
una variante del consentimiento sino otra situación, con otra salida, y el caso
vive en un solo sitio.

**`SedeFirmando` dibuja el hueco que hoy queda vacío.** Entre aceptar y volver a
la sede no aparece nada, y esa opción de la palanca —«hoy · la ventana no
aparece»— es literalmente el rectángulo vacío de 520 × 420. **No es «8 ·
Firmando»**: allí se listan las tres fases —prefirma, firma, posfirma— porque la
persona ha pedido un fichero y el reparto trifásico explica por qué tarda; aquí
no hay destino que enseñar y las fases son estado interno del motor. Se enseñan
los dos momentos que sí le importan a quien firma —«Firmando», con su
certificado, y «Enviando la firma a la sede»— y la barra avanza de verdad entre
los dos, 45 % y 88 %, porque si marcaran lo mismo las dos opciones se verían
iguales.

**`SedeEspera` es texto, no capturas.** El aviso del navegador se describe por su
forma —«la franja bajo la barra de direcciones», «el panel junto a la barra»—
citando el botón `Permitir`, y el camino de reparación **no diagnostica**: rFirma
no puede saber si el permiso se denegó, así que es un conmutador entre dos
recetas y la persona elige la suya. La dirección de Chrome **se copia, no se
pulsa**. La frase obligatoria vive en el pie —«tras permitir, vuelve a la sede y
pulsa Reintentar»—, porque `Reintentar` es un botón **de la sede** y esta ventana
no lo tiene.

**`SedeDesenlace` se cierra sola a los 15 s, no a los 5.** Con 5 no daba tiempo a
leer, y que hiciera falta más tiempo era justo la prueba de que sobraba texto.
El caso que decide es «rechazo × se cierra sola»: cerrarse sola reproduce el
síntoma que el aviso venía a evitar. El rechazo lleva un **detalle copiable**,
que es lo único accionable, y el argumento para enseñarlo no es que la persona
pueda arreglarlo —no puede—, sino que acaba de arrancarse un programa en su
equipo a petición de una web.

**`EstadoElegirCertificado` llevaba un recorte desde antes de esta tanda.** El
panel del desplegable colgaba del disparador dentro de la columna lateral, que
desplaza 740 px de contenido en 466 px visibles: su `overflow: auto` **cortaba
63 px de los 232** de la lista, justo donde empieza el pie de «Se guardará en /
Firmar documento», y lo hacía **en la opción por defecto** del artboard. El panel
sube a la raíz del artboard —el equivalente dibujado del **portal** que usará la
implementación— anclado con las medidas del disparador. Se descartó bajar el
`max-height` de la lista, que es mutilar el componente para tapar el fallo, y
quitarle el `overflow` a la columna, que tiene que seguir desplazándose. La regla
queda escrita como componente en el [sistema de
diseño](../design-system.md): **un desplegable se ve entero aunque se salga de la
ventana**.

**`EstadoPin` estrena `autofocus` en el campo del secreto**, con el foco
dibujado. Es una decisión que va al spec y no sólo al dibujo: hoy no está
soportado, y el diálogo tiene una sola entrada, así que en los tres almacenes el
gesto siguiente es siempre teclear. El PIN de la ventana de sede **no tiene
artboard propio**: es «6 · Pidiendo PIN» tal cual, y la palanca de contexto que
se llegó a dibujar se retiró entera.

**Y la regla de redacción de la v0.4 se aprieta una vuelta más.** Ya no es sólo
«si borrar una frase no cambia lo que la persona puede hacer, sobra»: son **tres
familias que se borran, no se acortan** — lo obvio (si el botón de al lado ya lo
dice), lo que **infunde temor** («entregar mis datos», «dice venir de») y lo que
**acusa** («no dice de dónde viene», «el fallo es de»). Por ahí se han ido una
veintena de frases de las cinco pantallas, y la regla queda escrita con sus
ejemplos —antes → después— en el [sistema de diseño](../design-system.md). Lo
que **no** se recorta: la distinción entre «no tienes ninguno» y «la sede excluyó
los tuyos», la instrucción accionable del pie de `SedeEspera`, y la regla dura de
no enumerar jamás lo que la sede descartó.
