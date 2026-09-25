# Un clic por trámite: la suite quita el PIN, pero no esconde la elección del certificado

Casi cincuenta comprobaciones necesitan una firma solo como medio para llegar a lo que miden, y
cada una abría el selector de certificado y el diálogo del PIN. Correr la suite pedía tener a
alguien delante toda la tanda.

**El PIN se quita en el almacén; la elección del certificado no se quita.** Las comprobaciones
cuya asistencia es `clic` corren con un almacén de un único certificado sin PIN: una base NSS sin
contraseña en el perfil aislado, con los tokens de SoftHSM fuera de su alcance. Es una condición
de lanzamiento, igual para cualquier cliente. Queda un clic por trámite: el selector de AutoFirma
o el consentimiento de rFirma. La cola los agrupa en su tramo.

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
- Dos comprobaciones que esperan el mismo `CANCEL` no son duplicadas si un cliente no conforme
  falla cada una de un modo distinto: la cancelación del cliente conforme es el resultado esperado,
  no lo que se mide. Lo demás de un diálogo se exige por lo que la petición provoca sin nadie
  delante: `headless`, la contraseña o el área en la petición.
- Un rechazo que el cliente enseña en una ventana antes de contestar —los de parámetros, incluido
  el acceso a una dirección local, y los de guardar, cargar, seleccionar y lote— no llega
  a la sede hasta que alguien la cierra: esas comprobaciones van en el tramo `clic`, sin excepción.
  La suite lanza los dos clientes con las mismas opciones.

## Considered Options

- **Añadir `headless` o `mandatoryCertSelection=false` a la petición.** Descartada: cambia lo que
  envía la sede, y con ello lo que se mide.
- **Un interruptor de lanzamiento en rFirma que se salte el consentimiento.** Descartada: una
  aplicación de firma que firma sin preguntar si se lo pide una variable de entorno es un agujero,
  y AutoFirma no tiene equivalente.
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
