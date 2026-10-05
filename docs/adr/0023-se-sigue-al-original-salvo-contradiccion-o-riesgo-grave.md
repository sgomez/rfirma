# Se sigue al original salvo contradicción o riesgo grave: SHA-1 y la XAdES explícita, solo si la persona lo permite

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
SHA-1, en firma suelta y en lote; el `clientSigner.js` de Guadaltel, por
ejemplo, monta el acceso con certificado como un lote remoto CAdES de una sola
firma con `SHA1withRSA`. SHA-1 ya no es apto para generar firmas nuevas
(ETSI TS 119 312), y las colisiones de prefijo elegido son prácticas desde 2020.

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
2. **Del catálogo de algoritmos del original se atienden SHA-256, SHA-384 y
   SHA-512, con clave RSA o de curva elíptica, y SHA-1 solo con el permiso
   del punto 4.** MD5 y RIPEMD-160 siguen fuera, como allí.
3. **SHA-1 no se sustituye por SHA-256 por lo bajo.** En trifásico el servlet
   construye la prefirma con la huella que declaró y el `PK1` del token tiene
   que corresponderse con ella. Firmar con otra huella no da una firma mejor:
   da una firma inválida. Una degradación silenciosa aquí sería un fallo, no
   una defensa.
4. **SHA-1 es un caso de la excepción de seguridad, y la persona puede
   levantarla.** Con colisiones de prefijo elegido ya prácticas, quien prepara
   el documento —la sede o el propio firmante— puede fabricar dos documentos
   con la misma huella, y la firma vale para los dos: se pierde el no repudio.
   Quien lo permite no queda peor protegido que con AutoFirma, que es adonde
   volvería para acabar el trámite; quien no sabe nada de esto no firma con
   SHA-1 sin que se le pregunte.
   - **Vale para todo formato y toda operación**: CAdES y sus variantes, CMS,
     PKCS#1, PAdES, XAdES y FacturaE, en la firma suelta, el lote local, el lote
     remoto y la línea de órdenes. Se decide en rFirma, antes de llegar al
     puente, y antes de pedir certificado allí donde el algoritmo ya se conoce.
   - **Una preferencia, «Permitir SHA-1», apagada por defecto, lo permite
     siempre.** Es una sola, para todas las sedes: la ventana de sede no
     escribe nada (ADR-0010) y no hay estado por sede donde guardar una
     excepción.
   - **Sin la preferencia, la ventana de sede pregunta en vez de rechazar**,
     en la firma suelta, el lote local y el lote remoto. El consentimiento
     avisa de que la firma es poco segura, y su botón pasa a «Firmar solo esta
     vez», con la cuenta atrás aunque la persona la tenga desactivada; el
     trámite no se consiente solo aunque la sede lo pida. Consentir permite
     SHA-1 en esa operación y en ninguna otra: el permiso vive en la memoria
     del trámite, se olvida al acabar la operación y no toca la preferencia.
     Rechazar es cancelar el trámite, con la respuesta de siempre a la sede.
   - **Sin la preferencia, la línea de órdenes rechaza**, porque no tiene
     consentimiento donde preguntar, y su error nombra el ajuste.
   - **Con la preferencia activada, el consentimiento lo recuerda** con una
     línea informativa: la sede pide SHA-1 y la persona lo tiene permitido. No
     corta el trámite.
   - **No se ofrece en el asistente de primer arranque.** La decisión tiene
     sentido cuando una sede la pide, no al instalar: quien nunca se encuentra
     una sede así no tiene por qué leerla.
   - Un algoritmo que rFirma no reconoce no es SHA-1 y no lleva esa
     explicación.
5. **La XAdES explícita es el mismo caso que SHA-1, y la cubre el mismo
   permiso.** Lo único que ata la firma al documento es una huella SHA-1, el
   mismo riesgo que firmar con `SHA1withRSA`, aunque el algoritmo de la firma
   sea SHA-256. Con la preferencia o con «Firmar solo esta vez», rFirma hace
   lo que el original: sustituye el documento por su SHA-1 y marca el
   `MimeType` como `hash/sha1`. El consentimiento enseña el documento de la
   sede, no la huella. En los demás casos `mode` se ignora, como en el
   original.
6. **Un filtro en el que el original no reconoce nada no es un filtro.** El
   original descarta las condiciones que no reconoce, y si no le queda
   ninguna, la expresión admite todos los certificados y, como ya cuenta como
   filtro, deja de ocultar los caducados: se contradice con su propia regla de
   no ofrecerlos sin filtro de la sede. rFirma quita esa expresión antes de
   llamar al motor. Si era la única, se listan los mismos certificados que sin
   filtro. Si era una de varias alternativas `filters.N`, deja de abrir el
   listado a todos.

## Consequences

- `SignatureAlgorithm` tiene `SHA1withRSA` y `SHA1withECDSA`, y rFirma pide al
  token `CKM_SHA1_RSA_PKCS` o `CKM_ECDSA_SHA1` (y su equivalente en CNG) solo
  con el permiso del punto 4. Cada traducción de un algoritmo a un mecanismo o
  a un nombre de huella nombra SHA-1 de forma explícita: un comodín que caiga
  en SHA-256 incumpliría el punto 3.
- El catálogo de `site/domain/protocol/algorithm.rs` sigue siendo por alias y
  es más laxo que el conjunto cerrado del original: acepta `SHA-256`,
  `SHA256withRSAandMGF1` o el URI de XMLDSig, que allí no están. Rechaza de
  menos, nunca firma con una huella distinta de la nombrada. Reconoce todas las
  grafías de SHA-1, y el permiso decide si las atiende o pregunta.
- En la imagen nativa firma XML el proveedor XMLDSig del JDK, no el Santuario
  externo de la JVM, y su política de validación segura
  (`jdk.xml.dsig.secureValidationPolicy`) prohíbe SHA-1 también al firmar. El
  puente quita al arrancar las prohibiciones de la huella SHA-1, `rsa-sha1` y
  `ecdsa-sha1`, y deja el resto. El ciclo trifásico XAdES y FacturaE con
  `SHA1withRSA`, y la XAdES explícita, dan firmas que aceptan el validador del
  original y xmlsec. Lo que cambia es rFirma, que deja de rechazarlas.
- La guarda de la XAdES explícita reproduce las condiciones exactas del
  original, ahora para decidir cuándo se sustituye el documento por su huella;
  una cofirma, una contrafirma, `XAdEStri` o `useManifest=true` firman el
  documento entero.
- Las otras guardas de la firma de sede —la factura electrónica que ya está
  firmada y la contrafirma fuera de CAdES, CMS y XAdES— repiten el rechazo del
  propio AutoFirma, y no son desviaciones.
- Las comprobaciones de la suite de conformidad que solo pasan con el permiso
  del punto 4 llevan la etiqueta `rfirma:adr-0023`:
  `a_cades_signature_is_made_with_the_sha1_requested`,
  `a_xades_signature_is_made_with_the_sha1_requested`,
  `a_pades_signature_is_made_with_the_sha1_requested`,
  `a_local_batch_with_the_sha1_algorithm_is_signed_with_sha1` y la XAdES
  explícita, `an_explicit_xades_signs_the_sha1_of_the_data`, que además lleva
  `manual:deprecated` porque el original la marca como obsoleta. El perfil
  aislado de la suite activa la preferencia, como si la persona lo hubiera
  elegido, y la etiqueta dice por qué esas comprobaciones dependen de ella. El
  certificado caducado de la regla 1,
  `expired_certificates_are_hidden_only_without_filters`, lleva la misma
  etiqueta y su NO CONFORME en rFirma cuenta como explicado.
- En el protocolo afirma, rFirma puede ser más permisiva que el original pero nunca
  más estricta sin un criterio que ningún documento legítimo pueda disparar; el caso
  de `checkSignatures` y el certificado caducado está en el ADR-0044.
- Si una versión posterior del original retira SHA-1 de su catálogo, o la
  XAdES explícita, este ADR se reescribe midiéndolo contra ese tag.
