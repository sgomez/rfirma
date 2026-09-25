# Un clic solo donde hay algo que decidir: la suite quita el PIN y rFirma de conformidad consiente lo que no deja elección

La mayoría de las comprobaciones necesitan una firma solo como medio para llegar a lo que miden, y
cada una abría el selector de certificado y el diálogo del PIN. Correr la suite pedía tener a
alguien delante toda la tanda.

**El PIN se quita en el almacén.** Las comprobaciones cuya asistencia es `clic` corren con un
almacén de un único certificado sin PIN: una base NSS sin contraseña en el perfil aislado, con los
tokens de SoftHSM fuera de su alcance. Es una condición de lanzamiento, igual para cualquier
cliente.

**El clic que queda lo quita rFirma, y solo una compilación de rFirma que no se publica.** Con la
feature de cargo `conformance-autoconsent`, rFirma lee `RFIRMA_CONFORMANCE_AUTOCONSENT=1` y, si el
paso que acaba de atender no deja nada que decidir, consiente por el mismo recorrido que el clic
—consentir, firmar sin secreto, entregar— y cierra la ventana como al acabar el desenlace. La
respuesta que viaja es la misma que con el clic; la ventana puede verse un instante. «Nada que
decidir» es una regla cerrada:

- consiente la selección de certificado (`selectcert`), la firma y los dos lotes, remoto y local,
  cuando tras los filtros de la sede queda **exactamente un** certificado;
- y, salvo en `selectcert`, que no abre el almacén (ADR-0025), cuando ese almacén **no pide
  secreto**;
- nunca consiente una firma con área visible que marcar, con firmas sin registrar en el documento
  (el aviso de la pantalla de consentimiento) ni que termine en el diálogo de guardar
  (`signandsave`);
- ni ningún otro paso: la confirmación del validador o del PDF certificado, guardar, cargar, la
  falta de certificado y los rechazos que se enseñan siguen esperando a la persona. Lo que se
  pregunte después de consentir —la contraseña de un PDF cifrado— se pregunta igual.

La suite enciende el interruptor en el entorno del cliente que lanza para cada trámite del tramo
`clic`, y lo quita en los demás, sea cual sea el cliente: AutoFirma no lo lee, y un rFirma sin la
feature tampoco. La suite sigue sin saber qué cliente corre.

## Consequences

- Una tanda completa se reparte en tres tramos (`ninguna`, `clic`, `persona`). Contra el rFirma de
  `just conformance-autoconsent`, el tramo `clic` solo para en los trámites que de verdad piden a
  alguien: varios certificados, un almacén con PIN, un rechazo que se enseña. Contra AutoFirma o un
  rFirma publicado, sigue siendo un clic por trámite.
- Las comprobaciones cuyo objeto es el PIN o elegir entre varios certificados corren con el almacén
  `token` o `several`, y ahí el consentimiento automático no actúa.
- Un rechazo que el cliente enseña en una ventana antes de contestar —los de parámetros, incluido
  el acceso a una dirección local, y los de guardar, cargar, seleccionar y lote— no llega a la sede
  hasta que alguien la cierra: esas comprobaciones siguen en el tramo `clic`, y siguen pidiendo el
  clic también a rFirma.
- Una comprobación cuyo objeto es cuánto tarda la persona en consentir no es un clic: su acción es
  `consent_after_waiting`, de asistencia `persona`, y el interruptor no se enciende para ella.
- El binario con la feature no llega a nadie: ninguna receta de publicación ni el manifiesto del
  flatpak la activan, `tests/autoconsent_stays_out_of_release.rs` lo vigila en las fuentes, y
  `packaging/verifica-contenido.sh` rechaza el paquete cuyo contenido nombre el interruptor. Sin la
  feature, el código no existe en el binario.

## Considered Options

- **Un clic por trámite, y la suite no esconde el consentimiento** (la versión anterior de este
  ADR). Descartada: la mayoría de los trámites del tramo `clic` no tienen nada que decidir —un
  certificado, sin PIN, sin aviso—, y pulsar Aceptar en cada uno no mide nada.
- **Un interruptor de lanzamiento en rFirma, en cualquier binario.** Descartada, y es la razón de la
  feature: una aplicación de firma publicada que firma sin preguntar si se lo pide una variable de
  entorno es un agujero. Compilado aparte, el interruptor no existe donde podría abusarse de él.
- **Encender el interruptor para toda la tanda.** Descartada: rFirma consentiría también en el
  tramo `persona`, donde la persona tiene que cancelar el consentimiento de un único certificado, y
  la comprobación mediría otra cosa.
- **Añadir `headless` o `mandatoryCertSelection=false` a la petición.** Descartada: cambia lo que
  envía la sede, y con ello lo que se mide. Además, rFirma muestra siempre el consentimiento al
  firmar, aunque la petición traiga `headless`.
- **Automatizar el escritorio para pulsar el clic** (`ydotool` sobre la ventana nueva). Descartada:
  es neutral respecto al cliente, pero depende del foco y del tiempo, y un Enter en la ventana
  equivocada falsea el resultado.
- **Lanzar AutoFirma siempre con `HeadLess=true`.** Descartada: cambia el comportamiento de un
  solo cliente, y con él lo que viaja: el rechazo de parámetros llega sin esperar a que nadie
  cierre su ventana.
- **Un perfil de lanzamiento `headless` por comprobación**, que añadía
  `-Des.gob.afirma.protocolinvocation.HeadLess=true` solo a las que lo declaraban, para que el
  rechazo del acceso local corriera sin nadie delante. Descartada: rFirma también enseña ese
  rechazo en su ventana y no lee la opción, así que la comprobación seguía necesitando un clic; y
  un perfil que solo afecta a un cliente mide cosas distintas en cada uno.
- **El certificado recordado con `sticky`.** Descartada: `sticky` es objeto de sus propias
  comprobaciones, y en rFirma solo preselecciona (ADR-0010).
