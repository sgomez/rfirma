# Se sigue al original salvo contradicción o riesgo grave: SHA-1 solo si la persona lo permite, XAdES explícita no

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
   SHA-512, con clave RSA o de curva elíptica, y SHA-1 solo con la preferencia
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
   - **Por defecto se rechaza**, en todo formato, en toda operación, en los dos
     lotes —local y remoto— y en la línea de órdenes. Se decide en rFirma,
     antes de llegar al puente, y antes de pedir certificado allí donde el
     algoritmo ya se conoce.
   - **Una preferencia, «Permitir SHA-1», apagada por defecto, lo permite** en
     los mismos puntos de entrada que el rechazo, y en toda operación. En el
     lote remoto vale para cualquier formato: la firma la construyen los
     servicios de la sede y rFirma solo firma la huella. Donde la construye el
     puente —firma suelta y lote local— vale en los formatos que no firman XML:
     CAdES y sus variantes, CMS, PKCS#1 y PAdES. Quien la
     activa no queda peor protegido que con AutoFirma, que es adonde volvería
     para acabar el trámite; quien no sabe nada de esto sigue protegido.
   - **La preferencia es una sola, para todas las sedes.** La ventana de sede
     no escribe nada (ADR-0010) y no hay estado por sede donde guardar una
     excepción.
   - **No se ofrece en el asistente de primer arranque.** La decisión tiene
     sentido cuando una sede la pide, no al instalar: quien nunca se encuentra
     una sede así no tiene por qué leerla.
   - **El rechazo explica el arreglo.** La ventana de sede dice a la persona
     que no es un fallo suyo, que puede permitir SHA-1 en Preferencias → Firma
     si confía en la sede, y que después tiene que volver a firmar desde la
     sede. Solo texto: ningún botón la lleva al ajuste. Bajo «Para quien
     mantiene la sede:», que pida SHA-256 o superior. El error de la línea de
     órdenes nombra el mismo ajuste. Después la sede recibe el código que ese
     punto de entrada da a un algoritmo que no se atiende: `SAF_03`, que nombra
     `algorithm`, en la firma suelta, como en el original.
   - **Con la preferencia activada, el consentimiento lo recuerda** con una
     línea informativa: la sede pide SHA-1 y la persona lo tiene permitido. No
     corta el trámite.
   - **En XAdES y FacturaE que firma el puente, SHA-1 se rechaza siempre**, y
     ese rechazo no sugiere la preferencia, porque no lo arreglaría. En el lote
     local, ese elemento falla como cualquier otro que no se puede firmar, y
     `stoponerror` decide si el lote sigue.
   - Un algoritmo que rFirma no reconoce no es SHA-1 y no lleva esa
     explicación.
5. **La XAdES explícita es otro caso de la excepción, y no tiene preferencia.**
   Donde AutoFirma firmaría la huella SHA-1 en lugar del documento, rFirma no
   firma: enseña el rechazo a la persona usuaria y, al cerrar la ventana, la
   sede recibe `SAF_06`. El algoritmo de la firma puede ser SHA-256 y aun así
   lo único que la ata al documento es un SHA-1, y el propio original la da
   por incorrecta: no es un caso feliz. La preferencia del punto 4 no la
   cubre, y su rechazo no la sugiere. En los demás casos `mode` se ignora,
   como en el original.
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
  con la preferencia activada. Cada traducción de un algoritmo a un mecanismo o
  a un nombre de huella nombra SHA-1 de forma explícita: un comodín que caiga
  en SHA-256 incumpliría el punto 3.
- El catálogo de `site/domain/protocol/algorithm.rs` sigue siendo por alias y
  es más laxo que el conjunto cerrado del original: acepta `SHA-256`,
  `SHA256withRSAandMGF1` o el URI de XMLDSig, que allí no están. Rechaza de
  menos, nunca firma con una huella distinta de la nombrada. Reconoce todas las
  grafías de SHA-1, y la preferencia decide si las atiende o las rechaza con su
  situación.
- El puente no está preparado para SHA-1 en XML: el JDK con el que se compila
  prohíbe SHA-1 en XMLDSig (`jdk.xml.dsig.secureValidationPolicy`), también al
  firmar. El lote remoto no pasa por el puente y no lo necesita. Si la
  preferencia se extiende algún día a XAdES y FacturaE en la firma suelta y
  el lote local, hace falta ese ajuste en el puente, acotado a la firma y sin
  relajar la validación.
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
  decide no hacer por defecto llevan la etiqueta `rfirma:adr-0023`: las que
  miden que el original firma con SHA-1 —`a_cades_signature_is_made_with_the_sha1_requested`,
  `a_xades_signature_is_made_with_the_sha1_requested`,
  `a_pades_signature_is_made_with_the_sha1_requested` y
  `a_local_batch_with_the_sha1_algorithm_is_signed_with_sha1`—, la XAdES explícita,
  `an_explicit_xades_signs_the_sha1_of_the_data`, que además lleva
  `manual:deprecated` porque el original la marca como obsoleta, y el
  certificado caducado de la regla 1,
  `expired_certificates_are_hidden_only_without_filters`. Con las preferencias
  por defecto, su NO CONFORME en rFirma cuenta como explicado.
- En el protocolo afirma, rFirma puede ser más permisiva que el original pero nunca
  más estricta sin un criterio que ningún documento legítimo pueda disparar; el caso
  de `checkSignatures` y el certificado caducado está en el ADR-0044.
- Si una versión posterior del original retira SHA-1 de su catálogo, o la
  XAdES explícita, este ADR se reescribe midiéndolo contra ese tag.
