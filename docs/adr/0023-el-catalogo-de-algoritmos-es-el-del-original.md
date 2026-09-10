# El catálogo de algoritmos es el del original, SHA-1 incluido

AutoFirma 1.9.2 publica doce nombres de algoritmo y no acepta ninguno más
(`UrlParametersToSign.SUPPORTED_SIGNATURE_ALGORITHMS`): `SHA1`, `SHA256`,
`SHA384` y `SHA512`, cada uno suelto, con `withRSA` y con `withECDSA`. Para
`op=batch` ni siquiera comprueba nada: el algoritmo viaja dentro del `dat` y se
pasa tal cual a los dos servlets (`BatchSigner.getAlgorithmForXML`).

rFirma rechazaba SHA-1 en los cuatro sitios donde lee un algoritmo. La
restricción no venía de ningún requisito, no estaba en la lista de desviaciones
declaradas de `site/domain/protocol/mod.rs` y no tenía ADR: vivía como un
`return None` en `AskedAlgorithm::named`. El resultado, medido contra una sede
en producción que declara su lote con `algorithm="SHA1withRSA"`, es que rFirma
cierra la ventana y la sede recibe un `SAF_03` que nombra `algorithm`. La
persona vuelve a AutoFirma.

## La regla

1. **Se acepta lo que acepta el original, y nada más.** SHA-1, SHA-256, SHA-384
   y SHA-512, con clave RSA o de curva elíptica. MD5 y RIPEMD-160 siguen fuera,
   como allí.
2. **SHA-1 no se sustituye por SHA-256 por lo bajo.** En trifásico el servlet
   construye la prefirma con la huella que declaró y el `PK1` del token tiene
   que corresponderse con ella. Firmar con otra huella no da una firma mejor:
   da una firma inválida. Una degradación silenciosa aquí sería un fallo, no
   una defensa.
3. **La elección es de la sede, no de rFirma.** rFirma es el cliente que la
   sede invoca; qué huella tiene valor jurídico lo decide quien monta el
   trámite. Un cliente que rechaza lo que el original firma no protege a nadie:
   se queda sin usarse.

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
- Si una versión posterior del original retira SHA-1 de su catálogo, este ADR
  se reescribe midiéndolo contra ese tag.

## Considered Options

**Seguir rechazando SHA-1, documentándolo.** Era la única opción que no tocaba
la cadena de firma, y deja sin atender a las sedes en producción que lo piden,
que es exactamente el trabajo que rFirma existe para hacer. Descartada.

**Aceptarlo pero firmar con SHA-256.** Rompe la prefirma del servlet y produce
firmas que no validan. Descartada por incorrecta, no por política.

**Aceptarlo con un aviso en la pantalla de consentimiento.** Es defendible y no
está descartada por siempre: queda fuera de este ADR porque toca la interfaz y
su ficha de diseño, y el arreglo de compatibilidad no debe esperar a eso.
