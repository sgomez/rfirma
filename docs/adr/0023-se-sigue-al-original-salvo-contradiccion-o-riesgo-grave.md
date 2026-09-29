# Se sigue al original salvo contradicción o riesgo grave: SHA-1 no, XAdES explícita no

rFirma es el cliente que la sede invoca en lugar de AutoFirma 1.9.2. Cada
petición que el original atiende y rFirma rechaza deja a una persona a mitad de
un trámite, y la manda de vuelta a AutoFirma. Este ADR fija cuándo rFirma puede
apartarse del original en un caso que allí sale bien, y aplica la regla a los
dos casos que la motivaron.

**SHA-1 como algoritmo.** AutoFirma publica doce nombres de algoritmo y no
acepta ninguno más (`UrlParametersToSign.SUPPORTED_SIGNATURE_ALGORITHMS`):
`SHA1`, `SHA256`, `SHA384` y `SHA512`, cada uno suelto, con `withRSA` y con
`withECDSA`. Para `op=batch` ni siquiera comprueba nada: el algoritmo viaja
dentro del `dat` y se pasa tal cual a los dos servlets
(`BatchSigner.getAlgorithmForXML`). Hay sedes en producción que todavía piden
SHA-1, en firma suelta y en lote. SHA-1 ya no es apto para generar firmas
nuevas (ETSI TS 119 312), y las colisiones de prefijo elegido son prácticas
desde 2020.

**La XAdES explícita.** Con `mode=explicit`, formato XAdES, operación `sign` (o
`signandsave`), sin `useManifest=true` y fuera de `XAdEStri`, AutoFirma sustituye
el documento por su SHA-1 y firma un `ds:Object` con `MimeType="hash/sha1"`
(`ProtocolInvocationLauncherSign.java:392-406, 886-893`). Lo que queda firmado
es solo la huella SHA-1. El propio original marca la función como
`@Deprecated`, avisa en el log de que dejará de soportarse y la describe como
«no es una firma correcta». Hay al menos una sede en producción que la pide.
Fuera de esas condiciones, el firmante XAdES ignora `mode`
(`Utils.java:414-416`) y firma el documento entero.

## La regla

1. **Se sigue a AutoFirma en los casos felices, salvo que el original se
   contradiga a sí mismo o haya un problema grave de seguridad.** Un cliente
   que rechaza lo que el original firma no protege a nadie: se queda sin
   usarse. La excepción se aplica solo donde se cumple, y con el mismo alcance
   que el caso del original que la justifica. **Un caso feliz es aquel en el
   que el original produce algo válido.** Un certificado caducado no lo es:
   `CertFilterManager` lo entrega cuando el filtro de la sede lo nombra
   explícitamente (por ejemplo, por su número de serie), pero la firma que
   resulta no tiene valor jurídico. Que el original lo entregue en ese caso no
   ata a rFirma, que no lo entrega nunca, admita o no el filtro de la sede al
   certificado caducado.
2. **Del catálogo de algoritmos del original se aceptan SHA-256, SHA-384 y
   SHA-512, con clave RSA o de curva elíptica, y nada más.** SHA-1 no, por el
   punto 4. MD5 y RIPEMD-160 siguen fuera, como allí.
3. **SHA-1 se rechaza; no se sustituye por SHA-256 por lo bajo.** En trifásico
   el servlet construye la prefirma con la huella que declaró y el `PK1` del
   token tiene que corresponderse con ella. Firmar con otra huella no da una
   firma mejor: da una firma inválida. Una degradación silenciosa aquí sería un
   fallo, no una defensa.
4. **SHA-1 es un caso de la excepción de seguridad.** Con colisiones de prefijo
   elegido ya prácticas, quien prepara el documento —la sede o el propio
   firmante— puede fabricar dos documentos con la misma huella, y la firma vale
   para los dos: se pierde el no repudio. Que la firma declare SHA-1 a la vista
   no protege a quien firma, ni a un tercero que la reciba sin validar su
   política. El rechazo alcanza a todo formato, a toda operación y a los dos
   lotes, local y remoto. Se decide en rFirma, antes de llegar al puente, y
   antes de pedir certificado allí donde el algoritmo ya se conoce. Tiene
   situación propia: la ventana de sede explica a la persona que no es un fallo
   suyo y, bajo «Para quien mantiene la sede:», que pida SHA-256 o superior.
   Después la sede recibe el código que ese punto de entrada da a un algoritmo
   que no se atiende: `SAF_03`, que nombra `algorithm`, en la firma suelta, como
   en el original. Un algoritmo que rFirma no reconoce no es SHA-1 y no lleva
   esa explicación.
5. **La XAdES explícita es otro caso de la excepción.** Donde AutoFirma
   firmaría la huella SHA-1 en lugar del documento, rFirma no firma: enseña el
   rechazo a la persona usuaria y, al cerrar la ventana, la sede recibe
   `SAF_06`. El riesgo es el del punto 4, y además escondido: el algoritmo de
   la firma puede ser SHA-256 y aun así lo único que la ata al documento es un
   SHA-1. En los demás casos `mode` se ignora, como en el original.
6. **Un filtro en el que el original no reconoce nada no es un filtro.** El
   original descarta las condiciones que no reconoce, y si no le queda
   ninguna, la expresión admite todos los certificados y, como ya cuenta como
   filtro, deja de ocultar los caducados: se contradice con su propia regla de
   no ofrecerlos sin filtro de la sede. rFirma quita esa expresión antes de
   llamar al motor. Si era la única, se listan los mismos certificados que sin
   filtro. Si era una de varias alternativas `filters.N`, deja de abrir el
   listado a todos.

## Consequences

- `SignatureAlgorithm` no tiene variantes SHA-1, ni rFirma pide al token
  `CKM_SHA1_RSA_PKCS` o `CKM_ECDSA_SHA1`.
- El catálogo de `site/domain/protocol/algorithm.rs` sigue siendo por alias y
  es más laxo que el conjunto cerrado del original: acepta `SHA-256`,
  `SHA256withRSAandMGF1` o el URI de XMLDSig, que allí no están. Rechaza de
  menos, nunca firma con una huella distinta de la nombrada. Reconoce todas las
  grafías de SHA-1 para rechazarlas con su situación, no para atenderlas.
- El puente no está preparado para SHA-1 en XML: el JDK con el que se compila
  prohíbe SHA-1 en XMLDSig (`jdk.xml.dsig.secureValidationPolicy`), también al
  firmar. Si algún día se vuelve a aceptar SHA-1, XAdES y FacturaE necesitan
  ese ajuste en el puente además del cambio en rFirma.
- La guarda de la XAdES explícita (`refuse_explicit_xades`) reproduce las
  condiciones exactas del original; una cofirma, una contrafirma, `XAdEStri` o
  `useManifest=true` llegan al consentimiento.
- Los rechazos de SHA-1 y de la XAdES explícita tienen situación propia y pasan
  por la pantalla de rechazo antes de contestar, con una nota para quien
  mantiene la sede. Las otras dos guardas de la firma de sede —la factura
  electrónica que ya está firmada y la contrafirma fuera de CAdES, CMS y
  XAdES— van por el mismo camino: repiten el rechazo del propio AutoFirma, y no
  son desviaciones.
- Las comprobaciones de la suite de conformidad que exigen lo que este ADR
  decide no hacer llevan la etiqueta `rfirma:adr-0023`: las que miden que el
  original firma con SHA-1 —`a_cades_signature_is_made_with_the_sha1_requested`,
  `a_xades_signature_is_made_with_the_sha1_requested`,
  `a_pades_signature_is_made_with_the_sha1_requested` y
  `a_local_batch_with_the_sha1_algorithm_is_signed_with_sha1`—, la XAdES explícita,
  `an_explicit_xades_signs_the_sha1_of_the_data`, que además lleva
  `manual:deprecated` porque el original la marca como obsoleta, y el
  certificado caducado de la regla 1,
  `expired_certificates_are_hidden_only_without_filters`. Su NO CONFORME en
  rFirma cuenta como explicado.
- Si una versión posterior del original retira SHA-1 de su catálogo, o la
  XAdES explícita, este ADR se reescribe midiéndolo contra ese tag.

## Considered Options

**Aceptar SHA-1 como el original.** Atiende a las sedes que todavía lo piden, y
fue lo que rFirma hizo durante un tiempo con el argumento de que la firma
declara SHA-1 y quien la valida aplica su política. Descartada: el validador
que la recibe primero suele ser la misma sede que pidió SHA-1, y lo acepta; el
riesgo de colisión lo sufre quien firma o un tercero que la reciba después.
Además, en XAdES y FacturaE nunca llegó a funcionar: el puente fallaba con un
error técnico en crudo.

**Aceptar SHA-1 con un aviso en la pantalla de consentimiento.** Quien firma
pulsa «sí» para acabar el trámite, y el riesgo no es solo suyo: un aviso no
protege al tercero. Descartada por el mismo motivo que el aviso de la XAdES
explícita.

**Un ajuste de Preferencias, desactivado por defecto, que permita SHA-1.**
Aplazada, no descartada. Explica el riesgo donde se activa, pero quien lo
activa suele llegar con prisa desde un rechazo, y una vez activado vale para
todas las sedes. Se puede añadir encima del rechazo si muchas personas se
quedan sin trámite; entonces hace falta además el ajuste del puente de
Consequences.

**Rechazar SHA-1 sin explicarlo.** La persona ve un error que parece suyo o de
su certificado, y la sede no sabe qué cambiar. Descartada.

**Aceptar SHA-1 pero firmar con SHA-256.** Rompe la prefirma del servlet y
produce firmas que no validan. Descartada por incorrecta, no por política.

**Imitar la XAdES explícita.** Firmaría la huella SHA-1 en lugar del documento,
con la pérdida de no repudio del punto 4, en una función que el propio original
da por incorrecta. Descartada.

**Imitar la XAdES explícita con un aviso.** El daño lo sufre un tercero, o lo
aprovecha el propio firmante, y un aviso en la pantalla no protege a ninguno de
los dos. Descartada.

**Rechazar `mode=explicit` con XAdES en cualquier operación.** Era lo que hacía
rFirma: rechazaba cofirmas, contrafirmas, `XAdEStri` y `useManifest=true`, que
el original firma enteras. Rechazar un caso feliz sin el riesgo que justifica
la excepción la extiende más allá de su motivo. Descartada.

**Entregar a la sede un certificado caducado cuando su filtro lo admite, como
hace el original.** Es exactamente lo que la regla 1 permitiría si entregar un
certificado caducado fuera un caso feliz: no lo es, porque la firma que
resulta no tiene valor jurídico aunque el original la produzca sin protestar.
Descartada.
