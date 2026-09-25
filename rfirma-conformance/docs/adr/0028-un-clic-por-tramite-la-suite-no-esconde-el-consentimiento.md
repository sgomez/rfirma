# Un clic por trámite: la suite quita el PIN y la elección que no existe, pero no las ventanas que se miden

Casi cincuenta comprobaciones necesitan una firma solo como medio para llegar a lo que miden, y
cada una abría el selector de certificado y el diálogo del PIN. Correr la suite pedía tener a
alguien delante toda la tanda.

**El PIN se quita en el almacén; la elección que no existe, en la petición.** Las firmas corren
con un almacén de un único certificado sin PIN: una base NSS sin contraseña en el perfil aislado,
con los tokens de SoftHSM fuera de su alcance. Es una condición de lanzamiento, igual para
cualquier cliente. Donde ese almacén deja un solo candidato tras los filtros, la petición lleva
`mandatoryCertSelection=false`, nunca `headless`: el cliente conforme resuelve al único candidato
sin preguntar (`CertFilterManager.java:145-154`, `AOKeyStoreDialog.java:726-729`), y si el trámite
no enseña ninguna otra ventana, la comprobación no tiene acción y va al tramo `ninguna`. rFirma
solo respeta el parámetro con la preferencia `honour_automatic_selection` encendida (ADR-0032):
el perfil aislado de la suite la activa, así que el informe mide rFirma con ella, que es el
comportamiento de fábrica del original. Queda un clic donde hay algo que elegir o que cerrar:
varios certificados, el PIN de un token, o una ventana que el cliente enseña antes de contestar.
La cola los agrupa en su tramo.

**Una acción de persona se retira solo si ningún cliente no conforme daría un resultado
distinguible por la sede.** Se queda mientras un cliente que se salta el diálogo, o que lo resuelve
de otro modo, conteste otra cosa por el cable o deje otra cosa en el disco: un destino ocupado que
se sobrescribe sin preguntar contesta `SAVE_OK` y cambia el fichero; un fallo de escritura que no
vuelve a pedir destino, `SAF_05`; un PDF certificado firmado sin aviso llega como firma; una
cofirma sin `dat` que no pide la firma, como un código de rechazo. Se retira cuando todo cliente
contesta lo mismo —que un PIN erróneo se vuelva a pedir no llega a la sede— o cuando otra
comprobación ya obliga a lo mismo. Lo que se puede provocar con la petición o el almacén se provoca
así.

## Consequences

- Una tanda completa se reparte en tres tramos (`ninguna`, `clic`, `persona`) y solo el primero
  corre sin nadie delante.
- La firma con la clave de un token PKCS#11 corre con el almacén `token`, y su PIN conocido se
  teclea en el mismo clic; elegir entre varios certificados, con `several`.
- `mandatoryCertSelection=false` no va en las comprobaciones que miden el selector: las que eligen
  entre varios certificados o filtran entre varios candidatos, las que lo cancelan, las que esperan
  con él abierto, las de `headless` y las que miden otro diálogo. Que el parámetro y `headless`
  resuelvan al único candidato sin preguntar lo exigen sus propias comprobaciones, en una selección
  y en una firma: un cliente que pregunta no vuelve solo y sale no conforme. En las demás del tramo
  `ninguna`, ese cliente agota la espera y la comprobación queda pendiente.
- Las comprobaciones del PDF con `headless` van al tramo `ninguna`: con un único candidato, ningún
  cliente enseña nada antes de contestar `SAF_50` o de firmar. La excepción es la del PDF con
  firmas no registradas que la sede permite, que sigue en `clic` porque rFirma enseña ese aviso en
  su consentimiento con la preferencia encendida.
- Dos comprobaciones que esperan el mismo `CANCEL` no son duplicadas si un cliente no conforme
  falla cada una de un modo distinto: la cancelación del cliente conforme es el resultado esperado,
  no lo que se mide. Lo demás de un diálogo se exige por lo que la petición provoca sin nadie
  delante: `headless`, la contraseña o el área en la petición.
- Un rechazo que el cliente enseña en una ventana antes de contestar —los de parámetros, incluido
  el acceso a una dirección local, y los de guardar, cargar, seleccionar y lote— no llega
  a la sede hasta que alguien la cierra: esas comprobaciones van en el tramo `clic`, sin excepción.
  La suite lanza los dos clientes con las mismas opciones.

## Considered Options

- **Añadir `headless` a la petición.** Descartada: además del selector, se salta las
  confirmaciones —firmas previas, PDF certificado o protegido— (`ProtocolInvocationLauncherSign.java:432,
  793`), y con ellas lo que miden las comprobaciones de esos avisos; `headless` es objeto de sus
  propias comprobaciones.
- **Dejar la petición sin `mandatoryCertSelection=false`**, porque cambia lo que envía la sede y
  con ello lo que se mide. Descartada: con un solo candidato, el parámetro solo quita una elección
  que no existe; lo que vuelve a la sede es lo mismo, y lo que el parámetro sí cambia —que el
  selector no salga— lo exigen sus propias comprobaciones. Donde lo medido es el selector, no se
  añade.
- **Dejarlo fuera porque rFirma enseña siempre su consentimiento.** Descartada: rFirma respeta el
  parámetro con un único candidato si la persona enciende `honour_automatic_selection`
  (ADR-0032), y el perfil de la suite la enciende. Donde rFirma sigue preguntando con la
  preferencia —un PDF con firmas no registradas, o más de un candidato contando los caducados que
  admite un filtro explícito—, la comprobación se queda en `clic`.
- **Un interruptor de lanzamiento en rFirma que se salte el consentimiento.** Descartada: una
  aplicación de firma que firma sin preguntar si se lo pide una variable de entorno es un agujero,
  y AutoFirma no tiene equivalente. La preferencia del ADR-0032 no lo es: la enciende la persona y
  solo resuelve al único candidato, como el original.
- **Automatizar el escritorio para pulsar el clic** (`ydotool` sobre la ventana nueva). No
  descartada del todo, pero aplazada: es neutral respecto al cliente, pero depende del foco y del
  tiempo, y un Enter en la ventana equivocada falsea el resultado. Cabe como otro adaptador del
  testigo si los clics agrupados siguen pesando.
- **Lanzar AutoFirma siempre con `HeadLess=true`.** Descartada: cambia el comportamiento de un
  solo cliente, y con él lo que viaja: el rechazo de parámetros llega sin esperar a que nadie
  cierre su ventana.
- **Un perfil de lanzamiento `headless` por comprobación**, que añadía
  `-Des.gob.afirma.protocolinvocation.HeadLess=true` solo a las que lo declaraban, para que el
  rechazo del acceso local corriera sin nadie delante. Descartada: rFirma también enseña ese
  rechazo en su ventana y no lee la opción, así que la comprobación seguía necesitando un clic; y
  un perfil que solo afecta a un cliente mide cosas distintas en cada uno.
- **Retirar toda cancelación que llega como el mismo `CANCEL`.** Descartada: la sede no sabe qué
  diálogo se canceló cuando los dos clientes cancelan, pero el cliente no conforme no cancela.
  Retiraba la confirmación de un destino ocupado, el nuevo destino tras un fallo de escritura, el
  aviso de un PDF certificado, la contraseña de un PDF protegido sin `headless` y la cofirma sin
  `dat`, y en todas un cliente que se salta el diálogo contesta otra cosa.
- **Una comprobación por cada diálogo del original.** Descartada: el diálogo solo justifica la
  comprobación si saltárselo cambia lo que ve la sede. Que una firma sin `dat` pida el documento
  ya lo exige la comprobación que lo elige y completa, y un PIN erróneo que se vuelve a pedir no
  llega a la sede.
- **El certificado recordado con `sticky`.** Descartada: `sticky` es objeto de sus propias
  comprobaciones, y en rFirma solo preselecciona (ADR-0010).
