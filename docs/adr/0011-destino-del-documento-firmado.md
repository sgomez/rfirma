# Dónde cae el documento firmado

El recorrido validado en el [ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md)
da por hecho que se firma y el documento aparece: sin diálogo por firma, con el destino
visible en el pie del panel antes de pulsar. El
[#22](https://github.com/sgomez/rfirma/issues/22) midió que bajo el arenero de flatpak la
aplicación **no puede saber de qué carpeta salió el original** —`Documents.Info` y `.Lookup`
contestan `Not allowed in sandbox`— y que escribir un hermano del fichero que entrega el
portal deja un `.xdp-…` huérfano **sin dar error**. Eso mató el valor por omisión de
`docs/design/preferencias.md` y la degradación del
[ADR-0010](0010-memoria-entre-sesiones.md), que era ese mismo valor.

**El recorrido no se cambia por flatpak.** En macOS, en Windows y en un `.deb` el problema no
existe, y no se rebaja la experiencia de los tres para que se parezca a la del cuarto. Lo que
cambia es **dónde cae el fichero**, no cómo se firma.

Bajo el arenero cae en la carpeta de documentos del usuario, declarando
**`--filesystem=xdg-documents`** en el manifiesto. Fuera del arenero, cuando existan los
instaladores nativos, caerá junto al documento original.

## Por qué no las otras

**`--filesystem=home` no sirve, y no por caro: no funciona.** En `file-chooser.c` el portal
devuelve la ruta real únicamente cuando `xdp_app_info_is_host()`; en cualquier otro caso pasa
por `xdp_register_document` y contesta `/run/user/1000/doc/<id>/…`. Un flatpak no es «host»
tenga los permisos que tenga, así que conservar *junto al original* exigiría **saltarse el
portal**, que es lo que prohíbe el [ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md).
Conviene que quede escrito por esta razón y no por la del arenero: «lo rechazamos por
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

Si la carpeta declarada en `--filesystem` **no existe en el anfitrión**, dentro del arenero
`mkdir` y la escritura contestan **OK**, el fichero se relee bien, y en el anfitrión no hay
nada; a la siguiente ejecución no queda ni rastro. Medido. Es el fallo silencioso otra vez.

De ahí la regla: **la carpeta de destino no se crea nunca.** Si al comprobarla no está, es que
no está de verdad, porque flatpak solo monta lo que ya existe. Y esa comprobación va **antes de
firmar**, como manda el ADR-0010, no al guardar.

## Consequences

- **Un solo modo en el desplegable bajo el arenero.** *Junto al documento original* no aparece
  en flatpak: no existe ahí, y enseñarlo atenuado sería contarle al usuario nuestros problemas
  de empaquetado. Un ajuste cuya lista de valores depende del empaquetado no se puede explicar
  en una frase.
- **El destino se enseña por nombre, no por ruta**, en todos los canales. Bajo el arenero la
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
  lo que ya estaba apuntado que se gana al salir del arenero, junto al módulo PKCS#11 del
  anfitrión y la matriz de WebKitGTK.
