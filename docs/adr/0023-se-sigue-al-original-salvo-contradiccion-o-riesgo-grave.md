# Se sigue al original salvo contradicción o riesgo grave: SHA-1 sí, XAdES explícita no

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
(`BatchSigner.getAlgorithmForXML`). rFirma rechazaba SHA-1 en los cuatro sitios
donde lee un algoritmo, sin requisito ni ADR detrás: vivía como un `return None`
en `AskedAlgorithm::named`. Medido contra una sede en producción que declara su
lote con `algorithm="SHA1withRSA"`, rFirma cerraba la ventana y la sede recibía
un `SAF_03` que nombra `algorithm`.

**La XAdES explícita.** Con `mode=explicit`, formato XAdES, operación `sign` (o
`signandsave`), sin `useManifest=true` y fuera de `XAdEStri`, AutoFirma sustituye
el documento por su SHA-1 y firma un `ds:Object` con `MimeType="hash/sha1"`
(`ProtocolInvocationLauncherSign.java:392-406, 886-893`). Lo que queda firmado
es solo la huella SHA-1. El propio original marca la función como
`@Deprecated`, avisa en el log de que dejará de soportarse y la describe como
«no es una firma correcta». No se conoce ninguna sede que la use. Fuera de esas
condiciones, el firmante XAdES ignora `mode` (`Utils.java:414-416`) y firma el
documento entero.

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
2. **Se acepta el catálogo de algoritmos del original, y nada más.** SHA-1,
   SHA-256, SHA-384 y SHA-512, con clave RSA o de curva elíptica. MD5 y
   RIPEMD-160 siguen fuera, como allí.
3. **SHA-1 no se sustituye por SHA-256 por lo bajo.** En trifásico el servlet
   construye la prefirma con la huella que declaró y el `PK1` del token tiene
   que corresponderse con ella. Firmar con otra huella no da una firma mejor:
   da una firma inválida. Una degradación silenciosa aquí sería un fallo, no
   una defensa.
4. **La elección del algoritmo es de la sede, no de rFirma.** Qué huella tiene
   valor jurídico lo decide quien monta el trámite, y la firma la declara a la
   vista: quien la valide ve SHA-1 y aplica su política.
5. **La XAdES explícita es el primer caso de la excepción.** Donde AutoFirma
   firmaría la huella SHA-1 en lugar del documento, rFirma no firma: enseña el
   rechazo a la persona usuaria y, al cerrar la ventana, la sede recibe
   `SAF_06`. Con colisiones de prefijo elegido ya prácticas, quien prepara el
   documento —la sede o el propio firmante— puede fabricar dos documentos con
   la misma huella, y la firma vale para los dos: se pierde el no repudio. A
   diferencia del punto 4, la firma no lo declara: su algoritmo puede ser
   SHA-256 y aun así lo único que la ata al documento es un SHA-1. En los
   demás casos `mode` se ignora, como en el original.

## Consequences

- `SignatureAlgorithm` gana `Sha1Rsa` y `Sha1Ecdsa`, con los mecanismos
  `CKM_SHA1_RSA_PKCS` y `CKM_ECDSA_SHA1`. Cualquier ranura PKCS#11 los ofrece;
  la que no, sale por el camino de `mechanismNotOffered` que ya existía.
- `identity/domain/ecdsa.rs` resume con la huella que nombra el algoritmo. Su
  `match` tenía un brazo por omisión a SHA-256: sin un brazo explícito para
  SHA-1, la firma de curva elíptica habría resumido con SHA-256 y salido
  inválida sin decir nada. Ese `_ =>` es la trampa de este cambio.
- El catálogo de `site/domain/protocol/algorithm.rs` sigue siendo por alias y
  es más laxo que el conjunto cerrado del original: acepta `SHA-256`,
  `SHA256withRSAandMGF1` o el URI de XMLDSig, que allí no están. Es una
  desviación anterior a este ADR y se mantiene: rechaza de menos, nunca firma
  con una huella distinta de la nombrada.
- La guarda de la XAdES explícita (`refuse_explicit_xades`) reproduce las
  condiciones exactas del original; una cofirma, una contrafirma, `XAdEStri` o
  `useManifest=true` llegan al consentimiento.
- El rechazo de la XAdES explícita tiene situación propia y pasa por la
  pantalla de rechazo antes de contestar, con una nota para quien mantiene la
  sede. Las otras dos guardas de la firma de sede —la factura electrónica que
  ya está firmada y la contrafirma fuera de CAdES, CMS y XAdES— van por el
  mismo camino: repiten el rechazo del propio AutoFirma, y no son desviaciones.
- Dos comprobaciones de la suite de conformidad exigen lo que este ADR decide
  no hacer y llevan la etiqueta `rfirma:adr-0023`: la XAdES explícita,
  `an_explicit_xades_signs_the_sha1_of_the_data`, que además lleva
  `manual:deprecated` porque el original la marca como obsoleta, y el
  certificado caducado de la regla 1,
  `expired_certificates_are_hidden_only_without_filters`. Su NO CONFORME en
  rFirma cuenta como explicado.
- Si una versión posterior del original retira SHA-1 de su catálogo, o la
  XAdES explícita, este ADR se reescribe midiéndolo contra ese tag.

## Considered Options

**Seguir rechazando SHA-1, documentándolo.** Era la única opción que no tocaba
la cadena de firma, y deja sin atender a las sedes en producción que lo piden,
que es exactamente el trabajo que rFirma existe para hacer. Descartada.

**Aceptar SHA-1 pero firmar con SHA-256.** Rompe la prefirma del servlet y
produce firmas que no validan. Descartada por incorrecta, no por política.

**Aceptar SHA-1 con un aviso en la pantalla de consentimiento.** Es defendible
y no está descartada por siempre: queda fuera de este ADR porque toca la
interfaz y su ficha de diseño, y el arreglo de compatibilidad no debe esperar a
eso.

**Imitar la XAdES explícita.** Firmaría la huella SHA-1 en lugar del documento,
con la pérdida de no repudio del punto 5, en una función que el propio original
da por incorrecta y que ninguna sede conocida usa. Descartada.

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
