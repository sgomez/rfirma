# Dónde cae el documento firmado

El recorrido validado en el [ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md)
da por hecho que se firma y el documento aparece: sin diálogo por firma, con el destino
visible en el pie del panel antes de pulsar. El
[#22](https://github.com/sgomez/rfirma/issues/22) midió que bajo el sandbox de flatpak la
aplicación **no puede saber de qué carpeta salió el original** —`Documents.Info` y `.Lookup`
contestan `Not allowed in sandbox`— y que escribir un hermano del fichero que entrega el
portal deja un `.xdp-…` huérfano **sin dar error**. Eso mató el valor por omisión de
`docs/design/preferencias.md` y la degradación del
[ADR-0010](0010-memoria-entre-sesiones.md), que era ese mismo valor.

**El recorrido no se cambia por flatpak.** En macOS, en Windows y en un `.deb` el problema no
existe, y no se rebaja la experiencia de los tres para que se parezca a la del cuarto. Lo que
cambia es **dónde cae el fichero**, no cómo se firma.

Bajo el sandbox cae en la carpeta de documentos del usuario, declarando
**`--filesystem=xdg-documents`** en el manifiesto. Fuera del sandbox, cuando existan los
instaladores nativos, caerá junto al documento original.

## Por qué no las otras

**`--filesystem=home` no sirve, y no por caro: no funciona.** En `file-chooser.c` el portal
devuelve la ruta real únicamente cuando `xdp_app_info_is_host()`; en cualquier otro caso pasa
por `xdp_register_document` y contesta `/run/user/1000/doc/<id>/…`. Un flatpak no es «host»
tenga los permisos que tenga, así que conservar *junto al original* exigiría **saltarse el
portal**, que es lo que prohíbe el [ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md).
Conviene que quede escrito por esta razón y no por la del sandbox: «lo rechazamos por
seguridad» invita a reabrirlo, «no hace lo que creíamos» no.

**La carpeta de datos de la aplicación** (`~/.var/app/me.sgomez.rfirma/data/`) es escribible
sin ningún permiso, medido, y «Abrir la carpeta» funciona sobre ella. Se descarta porque
**al desinstalar el flatpak se borra**, y con ella los documentos firmados del usuario. Es la
misma familia de fallo que el `.xdp-…`: el fichero está, hasta el día que no.

**Preguntar la carpeta una sola vez** por portal es viable —el permiso de directorio se
concede con escritura por omisión y persiste en `~/.local/share/flatpak/db/documents`— y deja
el manifiesto limpio. Se descarta porque pone una pregunta donde el recorrido dice que no la
haya.

Esto **reabre a sabiendas** la decisión de #22 de no declarar ningún `--filesystem`. No es
`home` ni `host`: es la concesión más estrecha que resuelve el caso.

## La trampa que obliga a una regla

Si la carpeta declarada en `--filesystem` **no existe en el anfitrión**, dentro del sandbox
`mkdir` y la escritura contestan **OK**, el fichero se relee bien, y en el anfitrión no hay
nada; a la siguiente ejecución no queda ni rastro. Medido. Es el fallo silencioso otra vez.

De ahí la regla: **la carpeta de destino no se crea nunca.** Si al comprobarla no está, es que
no está de verdad, porque flatpak solo monta lo que ya existe. Y esa comprobación va **antes de
firmar**, como manda el ADR-0010, no al guardar.

## Consequences

- **Un solo modo en el desplegable bajo el sandbox.** *Junto al documento original* no aparece
  en flatpak: no existe ahí, y enseñarlo atenuado sería contarle al usuario nuestros problemas
  de empaquetado. Un ajuste cuya lista de valores depende del empaquetado no se puede explicar
  en una frase.
- **El destino se enseña por nombre, no por ruta**, en todos los canales. Bajo el sandbox la
  aplicación conoce la carpeta y escribe en ella, pero la única palabra que tiene de ella es su
  último segmento. Enseñar la ruta donde se puede y el nombre donde no es la misma incoherencia
  en pequeño. Esto corrige el pie «Se guardará en» de
  [`docs/design/panel-de-firma.md`](../design/panel-de-firma.md), que prometía una ruta.
- **`Cambiar`, en el pie del panel, vale solo para esa firma** y abre el diálogo de guardar.
  No toca la preferencia: cambiar una preferencia desde un pie de página, sin decirlo, manda la
  siguiente firma a un sitio que el usuario no recuerda haber elegido. La carpeta se cambia en
  Preferencias.
- **Conflicto de nombres lo resuelve la aplicación**: `contrato-firmado.pdf`, luego
  `contrato-firmado-2.pdf`, `-3`. Empieza en 2 porque el primero no lleva número, y no se apila
  un segundo sufijo si el original ya acababa en `-firmado`. Sin diálogo por firma no hay
  ningún «ya existe, ¿reemplazar?» del sistema que avise, así que sin numerar la segunda firma
  machacaría a la primera en silencio.
- **Si la carpeta no está, se avisa y no se degrada.** El pie sustituye el destino por
  «No se puede escribir en *Documents*» con el `Cambiar` al lado, y el botón de firmar **no se
  apaga**. Degradar en silencio a otro sitio devuelve un destino que el usuario no eligió, que
  es lo que este ADR quita; apagar el botón deja a alguien con el documento cargado, el
  certificado puesto y ninguna salida visible.
- **La degradación del ADR-0010 desaparece**, no se sustituye. Era «junto al original» y ese
  destino ya no existe.
- **Los instaladores nativos heredan una capacidad más.** *Guardar junto al original* se suma a
  lo que ya estaba apuntado que se gana al salir del sandbox, junto al módulo PKCS#11 del
  anfitrión y la matriz de WebKitGTK.

## Enmienda: dónde se abre el diálogo de abrir

Añadido después. El diálogo de abrir arranca en **la última carpeta usada**, y donde esa no se
puede saber, en **la carpeta de destino**.

Las dos mitades son necesarias porque los canales no saben lo mismo, y esa asimetría este ADR
ya la asume: la última viñeta de arriba apunta que los instaladores nativos heredan
capacidades que en el flatpak no existen. Esta es una más.

- **Fuera del sandbox** —deb, rpm, Windows, macOS— el diálogo devuelve una ruta de verdad, así
  que la carpeta de la que salió el documento se sabe, y se apunta.
- **Bajo el sandbox no se puede saber.** Lo que el portal devuelve es
  `/run/user/1000/doc/<id>/nombre.pdf`, cuyo directorio padre contiene un solo fichero y no es
  ninguna carpeta del usuario; preguntar por la real —`org.freedesktop.portal.Documents.Info` y
  `.Lookup`— contesta `Not allowed in sandbox`, y `--filesystem=home` tampoco la devolvería. Es
  la misma medición del apartado 4 de
  [`docs/research/flatpak-canal-unico.md`](../research/flatpak-canal-unico.md) que sostiene que
  *junto al original* no existe aquí.

El respaldo para ese caso es la carpeta de destino: la única carpeta del usuario que la
aplicación conoce y nombra en el flatpak. Resuelve lo que se quería de verdad —no empezar cada
vez en la lista de «Recientes» del sistema— y además deja a la vista lo ya firmado, que es lo
más probable que se quiera volver a abrir.

**Que el diálogo aparezca en distinto sitio según el canal no es la incoherencia que este ADR
rechaza.** Esa era enseñar la ruta donde se puede y el nombre donde no: la misma pantalla
contando cosas distintas, y el usuario sin forma de saber por qué. Dónde arranca un diálogo no
se lee ni se compara; se navega. Lo que sí sería incoherente es abrir en «Recientes» en un
canal donde se sabe hacerlo mejor.

Lo apuntado es **estado y no configuración** (ADR-0010): lo acumula la aplicación sola, vive en
`XDG_STATE_HOME` y **«Recordar mi actividad» se lo lleva** como se lleva los recientes y el
certificado. Una carpeta del anfitrión que sobreviviera a «Vaciar la lista» contaría por dónde
anduvo quien firmó antes.

Y sigue sin ser un sitio donde escribir: lo único que recibe esa ruta es el `set_directory` del
diálogo, y la única forma de nombrar dónde cae un fichero sigue siendo `CheckedFolder`. Si la
carpeta apuntada ya no está, se pasa a la de destino; si esa tampoco, no se pasa punto de
partida y abre donde el sistema quiera. **Ninguna de las dos se crea nunca.**

## Enmienda: cómo se elige la carpeta, y qué enseña el pie

Añadida con el hito v0.2 ([#123](https://github.com/sgomez/rfirma/issues/123)).
No cambia dónde cae el fichero; cambia con qué gesto se elige y qué se lee antes
de firmar.

### La carpeta se elige con un selector de directorio

En Preferencias, «Dónde se guarda el documento firmado» **deja de ser un
desplegable**. Lo era porque este ADR retiró *junto al documento original*, y lo
que quedó fue una lista de un solo elemento: un control que finge elegir, y el
único ajuste de la pantalla que mentía.

Pasa a ser una fila con el nombre de la carpeta y un botón **«Cambiar
carpeta…»** que abre el selector de directorio del sistema. La consecuencia «el
destino se enseña por nombre, no por ruta» **sigue en pie y ahora se cumple
mejor**: un directorio concedido por el portal llega como
`/run/user/1000/doc/<id>/Documentos`, cuyo último segmento *es* el nombre de la
carpeta, así que los cuatro canales pueden nombrarla igual sin que ninguno
dependa de conocer la ruta real. Es la opción «preguntar la carpeta una sola
vez» que este ADR describe más arriba, con el permiso que persiste en
`~/.local/share/flatpak/db/documents` — descartada entonces por poner una
pregunta en medio del recorrido, y admitida ahora porque la pregunta está en
Preferencias, que es donde se va a cambiar un ajuste.

**`Cambiar`, en el pie del panel, no cambia**: sigue abriendo el diálogo de
guardar y sigue valiendo solo para esa firma. Que los dos gestos sean distintos
no es la incoherencia que este ADR rechaza. El diálogo de guardar es el único
que fija **carpeta y nombre a la vez**, que es justo lo que hace falta para una
firma concreta; el ajuste persistente solo tiene que nombrar una carpeta, y para
eso está el selector de directorio. Uno decide una vez, el otro decide siempre.

### El pie enseña la carpeta y el nombre del fichero

«Se guardará en» pasa de enseñar solo la carpeta a enseñar **la última carpeta y
el nombre del fichero**: `…/Documentos/contrato-firmado.pdf`.

El nombre lo compone la aplicación —`-firmado`, con desempate `-2`, `-3`, como
fija este mismo ADR— y hasta ahora no se veía hasta después de firmar, cuando ya
no se podía hacer nada al respecto. Enseñarlo antes es lo que convierte la
numeración en información en vez de en sorpresa: quien va a cofirmar ve que va a
salir un `-2` y no que ha machacado el anterior.

El `…/` de delante dice que hay carpetas por encima **sin afirmar cuáles**. No
reabre la ruta que esta decisión prohíbe: bajo el sandbox la aplicación no las
conoce, y fuera de él no se enseñan igualmente, así que la marca significa lo
mismo en los cuatro canales. La regla de recorte —qué se conserva, qué se come
el `…`, y que la línea envuelve antes que cortarse— vive en el componente
**ruta de destino** de [`docs/design/design-system.md`](../design/design-system.md),
que es donde se puede escribir con el ejemplo delante.

El estado **destino no disponible** no cambia: el pie sustituye la línea entera
por «No se puede escribir en *Documentos*» con su `Cambiar`, y el botón de
firmar no se apaga.
