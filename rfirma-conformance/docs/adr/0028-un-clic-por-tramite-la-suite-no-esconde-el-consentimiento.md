# Un clic por trámite: la suite quita el PIN, pero no esconde la elección del certificado

Casi cincuenta comprobaciones necesitan una firma solo como medio para llegar a lo que miden, y
cada una abría el selector de certificado y el diálogo del PIN. Correr la suite pedía tener a
alguien delante toda la tanda.

**El PIN se quita en el almacén; la elección del certificado no se quita.** Las comprobaciones
cuya asistencia es `clic` corren con un almacén de un único certificado sin PIN: una base NSS sin
contraseña en el perfil aislado, con los tokens de SoftHSM fuera de su alcance. Es una condición
de lanzamiento, igual para cualquier cliente. Queda un clic por trámite: el selector de AutoFirma
o el consentimiento de rFirma. La cola los agrupa en su tramo.

**La persona solo actúa donde el protocolo lo exige.** Una acción con nombre se queda si el
resultado que juzga el trámite depende de ella: la operación consiste en que la persona elija el
fichero o el destino, o su cancelación viaja como `CANCEL`, `SAF_43` o una firma invisible. Lo
que se puede provocar con la petición o el almacén se provoca así, y lo que la sede no puede
distinguir no se mide: qué diálogo de una misma operación se canceló, que un PIN erróneo se vuelva
a pedir o que un destino ya ocupado pida confirmación.

## Consequences

- Una tanda completa se reparte en tres tramos (`ninguna`, `clic`, `persona`) y solo el primero
  corre sin nadie delante.
- La firma con la clave de un token PKCS#11 corre con el almacén `token`, y su PIN conocido se
  teclea en el mismo clic; elegir entre varios certificados, con `several`.
- Cada operación mide su `CANCEL` una sola vez; otra cancelación solo se mide si lo que viaja es
  distinto (`SAF_43`, una firma invisible). Lo demás de un diálogo se exige por lo que la petición
  provoca sin nadie delante: `headless`, la contraseña o el área en la petición.
- Un rechazo que el cliente enseña en una ventana antes de contestar —los de parámetros, incluido
  el acceso a una dirección local, y los de guardar, cargar, seleccionar y lote— no llega
  a la sede hasta que alguien la cierra: esas comprobaciones van en el tramo `clic`, sin excepción.
  La suite lanza los dos clientes con las mismas opciones.

## Considered Options

- **Añadir `headless` o `mandatoryCertSelection=false` a la petición.** Descartada: cambia lo que
  envía la sede, y con ello lo que se mide. Además, rFirma muestra siempre el consentimiento al
  firmar, aunque la petición traiga `headless`.
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
- **Medir cada diálogo del original con su cancelación.** Era el catálogo anterior: pedir el
  documento, pedir la contraseña del PDF, avisar de un PDF certificado o de un destino ocupado,
  cada uno con su comprobación. Descartada: la sede recibe el mismo `CANCEL` sea cual sea el
  diálogo, así que la comprobación medía la ventana y no el protocolo.
- **El certificado recordado con `sticky`.** Descartada: `sticky` es objeto de sus propias
  comprobaciones, y en rFirma solo preselecciona (ADR-0010).
