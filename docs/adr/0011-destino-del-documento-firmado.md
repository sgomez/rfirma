# Dónde cae el documento firmado

El recorrido validado en el [ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md)
da por hecho que se firma y el documento aparece: sin diálogo por firma, con el destino
visible en el pie del panel antes de pulsar. **El recorrido no cambia con el canal.** Lo que
cambia es dónde cae el fichero, y eso lo decide **el documento, no el empaquetado**:

- Si el documento **no viene del portal** —`portal_id()` es `None`—, el firmado cae **junto
  al original**: es `document.parent()`, y nada más.
- Si viene del portal, cae en la **carpeta de destino**, la que la persona haya elegido en
  Preferencias.

Esta es la regla entera. No hay `FileAccess::{Portal, Direct}`, ni sondeo del entorno, ni
enum que diga en qué canal corremos: la capacidad ya vive en el código desde el
[#22](https://github.com/sgomez/rfirma/issues/22) —`PortalDocument` reconoce el enlace del
portal por el prefijo `/run/user/*/doc/`— y un enum que sondease `/.flatpak-info` sería una
segunda fuente de verdad para algo que el código ya sabe.

Y la versión por dato es **estrictamente más correcta**, no sólo más barata: un `.deb` puede
recibir una ruta del portal —el montaje FUSE existe también en el anfitrión y un gestor de
ficheros puede entregarla—, y ahí «no hay carpeta original» es la respuesta buena. Un sondeo
del entorno la daría mal.

## La ruta se enseña donde se conoce, y el nombre donde no

**Fuera del sandbox se enseña la ruta entera**, como cualquier aplicación de escritorio: en
el pie del panel, en la fila de recientes y en el mensaje de error. Dentro del flatpak se
enseña el nombre, porque ahí no hay otra cosa que enseñar.

La redacción anterior de este ADR decía «por nombre, nunca por ruta» y lo justificaba con
que enseñar la ruta donde se puede y el nombre donde no era «la misma incoherencia en
pequeño». Se rectifica, por dos razones:

- **El argumento de privacidad no se sostiene.** Se decía que `/home/<usuario>/` publica el
  nombre de usuario «en cualquier captura o registro». Como modelo de amenaza no aguanta:
  cualquier gestor de ficheros, editor o navegador enseña esa ruta todo el día. El motivo
  real de la guarda es otro: **bajo el sandbox la ruta real no se conoce, así que devolverla
  sería devolver una mentira**. Es corrección, no privacidad, y fuera del sandbox no hay
  ninguna mentira que impedir.
- **La asimetría entre canales está medida y es irreducible**
  ([#240](https://github.com/sgomez/rfirma/issues/240)). Fingir que no existe no la quita;
  sólo la paga el canal que no tenía el problema. Igualar por abajo es el mínimo común
  denominador, y es lo peor de las dos opciones.

**Cuánto de la ruta se pinta no lo decide este ADR**: lo decide el componente **ruta de
destino** de [`docs/design/design-system.md`](../design/design-system.md), que dice «la
última carpeta y el nombre, **nunca la ruta entera**», con el `…/` delante que indica que
hay carpetas por encima sin afirmar cuáles. Esa regla vale igual en los cuatro canales:
aquí se decide *qué* ruta se conoce, allí *cuánto* de ella cabe en una línea
([#242](https://github.com/sgomez/rfirma/issues/242)).

## La guarda vigila la ruta del portal, y es aproximada

La guarda se llama `the_portal_path_never_crosses_to_the_window` y mira **valores, no
texto**. La que hubo, `no_output_of_any_command_carries_a_host_path`, se retiró: su lista
negra —`PathBuf`, `&Path`, `path:`…, leída como texto sobre `commands/`— daba por malos
campos que fuera del sandbox son legítimos, y nunca impidió lo que todo el mundo creía que
impedía —un `pub folder: String` que en ejecución valga `/home/<usuario>/Documentos` pasaba
en verde—.

Lo que vigila es lo que sí es una mentira en **cualquier** canal: **la ruta del
portal (`/run/user/*/doc/`) no sale nunca a la ventana.** Ese directorio contiene un solo
fichero, no es ninguna carpeta del usuario, y enseñarlo es exactamente el fallo que la
guarda existe para evitar. Es una regla de **valor**, así que la comprueba una prueba que
construye cada vista desde su caso de uso con un enlace del portal, la serializa y recorre
el JSON campo a campo, por hondo que esté, en vez de hacer un `grep` sobre el fuente.

Y se dice en voz alta que **es aproximada**. Una guarda vendida como hermética que no lo es
enseña a no volver a mirar.

## La trampa que obliga a una regla

Si la carpeta declarada en `--filesystem` **no existe en el anfitrión**, dentro del sandbox
`mkdir` y la escritura contestan **OK**, el fichero se relee bien, y en el anfitrión no hay
nada; a la siguiente ejecución no queda ni rastro. Medido
([#27](https://github.com/sgomez/rfirma/issues/27)).

De ahí la regla: **la carpeta de destino no se crea nunca.** Si al comprobarla no está, es
que no está de verdad, porque flatpak solo monta lo que ya existe. Y esa comprobación va
**antes de firmar**, como manda el [ADR-0010](0010-memoria-entre-sesiones.md), no al
guardar.

## Cómo se elige la carpeta de destino

En Preferencias, «Dónde se guarda el documento firmado» es **una fila con el nombre de la
carpeta y un botón «Cambiar carpeta…»** que abre el selector de directorio del sistema. No
es un desplegable: lo fue mientras «junto al documento original» no existía y la lista tenía
un solo elemento —un control que fingía elegir—, y ahora que ese destino ha vuelto sigue sin
serlo, porque lo que se elige aquí es **una carpeta cualquiera**, no un modo.

Bajo el sandbox el permiso que concede el portal persiste en
`~/.local/share/flatpak/db/documents`, y el directorio llega como
`/run/user/1000/doc/<id>/Documentos`, cuyo último segmento *es* el nombre de la carpeta.

**`Cambiar`, en el pie del panel, vale sólo para esa firma** y abre el diálogo de guardar.
No toca la preferencia: cambiar una preferencia desde un pie de página, sin decirlo, manda
la siguiente firma a un sitio que el usuario no recuerda haber elegido. Que los dos gestos
sean distintos es deliberado — el diálogo de guardar es el único que fija **carpeta y nombre
a la vez**, que es lo que hace falta para una firma concreta; el ajuste persistente sólo
tiene que nombrar una carpeta. Uno decide una vez, el otro decide siempre.

## Dónde se abre el diálogo de abrir

Arranca en **la última carpeta usada**, y donde esa no se puede saber, en **la carpeta de
destino**.

Fuera del sandbox el diálogo devuelve una ruta de verdad, así que la carpeta de la que salió
el documento se sabe, y se apunta. Bajo el sandbox no: lo que el portal devuelve es
`/run/user/1000/doc/<id>/nombre.pdf`, y preguntar por la real
—`org.freedesktop.portal.Documents.Info` y `.Lookup`— contesta `Not allowed in sandbox`.

El respaldo es la carpeta de destino: la única carpeta del usuario que la aplicación conoce
y nombra en el flatpak. Resuelve lo que se quería de verdad —no empezar cada vez en la lista
de «Recientes» del sistema— y deja a la vista lo ya firmado, que es lo más probable que se
quiera volver a abrir.

Lo apuntado es **estado y no configuración** (ADR-0010): lo acumula la aplicación sola, vive
en `XDG_STATE_HOME` y **«Recordar mi actividad» se lo lleva**. Y sigue sin ser un sitio
donde escribir: lo único que recibe esa ruta es el `set_directory` del diálogo, y la única
forma de nombrar dónde cae un fichero sigue siendo `CheckedFolder`. Si la carpeta apuntada
ya no está, se pasa a la de destino; si esa tampoco, no se pasa punto de partida.
**Ninguna de las dos se crea nunca.**

## Por qué no las otras

**`--filesystem=home` no sirve, y no por caro: no funciona.** El
[#240](https://github.com/sgomez/rfirma/issues/240) lo midió tres capas por debajo del
permiso: `rfd` con el backend `gtk3` no abre un `GtkFileChooserDialog`, y
`GtkFileChooserNative` se enruta al portal en cuanto existe `/.flatpak-info` —la rama de
`GTK_USE_PORTAL` está muerta—, así que el diálogo devuelve `/run/user/…/doc/…` tenga el
permiso que tenga. Y aun con la ruta en la mano, la aplicación **no puede averiguar** la
carpeta del original. Queda cerrado por escrito en el
[ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md), que es donde viven los
permisos del manifiesto.

**La carpeta de datos de la aplicación** (`~/.var/app/me.sgomez.rfirma/data/`) es escribible
sin ningún permiso, medido, y «Abrir la carpeta» funciona sobre ella. Se descarta porque
**al desinstalar el flatpak se borra**, y con ella los documentos firmados del usuario. Es
la misma familia de fallo que el `.xdp-…` huérfano que el #22 midió: el fichero está, hasta
el día que no.

**Un `FolderLabel`, o una regla de «un solo segmento» aplicada en el backend.** No hay tipo
nuevo ni capa nueva de guarda: lo que sale es la ruta que se conoce, y cuánto se pinta lo
decide el sistema de diseño en el sitio donde se puede escribir con el ejemplo delante.

## Consequences

- **«Junto al documento original» vuelve al vocabulario.** Deja de ser un término prohibido
  del glosario y pasa a ser lo que ocurre por omisión en los canales nativos, sin ajuste que
  lo active.
- **`DestinationFolder` guarda la ruta entera en el fichero de configuración**, y deja de
  necesitar explicación: con la ruta real enseñándose en la interfaz, no hay ninguna
  asimetría entre disco y pantalla que justificar.
- **Conflicto de nombres lo resuelve la aplicación**: `contrato-firmado.pdf`, luego
  `contrato-firmado-2.pdf`, `-3`. Empieza en 2 porque el primero no lleva número, y no se
  apila un segundo sufijo si el original ya acababa en `-firmado`. Sin diálogo por firma no
  hay ningún «ya existe, ¿reemplazar?» del sistema que avise, así que sin numerar la segunda
  firma machacaría a la primera en silencio.
- **El pie enseña la carpeta y el nombre del fichero** antes de firmar:
  `…/Documentos/contrato-firmado.pdf`. El nombre lo compone la aplicación y hasta el
  [#123](https://github.com/sgomez/rfirma/issues/123) no se veía hasta después de firmar,
  cuando ya no se podía hacer nada. Enseñarlo antes es lo que convierte la numeración en
  información en vez de en sorpresa.
- **Si la carpeta no está, se avisa y no se degrada.** El pie sustituye el destino por «No
  se puede escribir en *Documentos*» —**nombrando la carpeta**— con el `Cambiar` al lado, y
  el botón de firmar **no se apaga**. Degradar en silencio a otro sitio devuelve un destino
  que el usuario no eligió, que es lo que este ADR quita; apagar el botón deja a alguien con
  el documento cargado, el certificado puesto y ninguna salida visible.
- **La degradación del ADR-0010 desaparece**, no se sustituye. Era «junto al original» como
  valor por omisión de un ajuste, y ahora eso no es un valor de ajuste: es lo que pasa
  cuando el documento tiene carpeta.
- Lo que cruza a la ventana sobre esto es **un booleano con nombre** en `ConfigurationView`
  (`can_save_next_to_original`), no el canal. Y quien lo calcula es un solo consumidor
  —`Environment`, la raíz de composición— con la pregunta por nombre,
  `dialogs_return_host_paths()`, no `is_flatpak()`: lo segundo invita a ramificar sobre él
  en veinte sitios.
