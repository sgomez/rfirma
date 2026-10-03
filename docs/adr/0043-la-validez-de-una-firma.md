# La validez de una firma: tres valores, gana el peor problema y el sello de tiempo prueba la fecha

El original juzga el documento entero, no cada firma. Sus validadores devuelven
una lista plana de resultados, y ninguno dice de qué firmante es. rFirma
enseña una **validez** por firma, y la misma en todas partes: `verify -v`, el
resumen, el aviso de firmas previas y «¿Firmar de todos modos?». Para eso la
calcula con los validadores por firmante del propio original
(`ValidatePdfSignature.validateSign`, `ValidateBinarySignature.verifySign` por
SignerInfo y `ValidateXMLSignature.validateSign` por `ds:Signature`). Lo que el
original solo sabe decir del documento va aparte, como **hallazgo del
documento**, y nunca se cuelga de una firma. Este ADR fija en qué puntos ese
juicio se aparta del original, y por qué, aplicando la regla del ADR-0023.

## La regla

1. **La validez es una lista cerrada de tres valores: válida, caducada y no
   válida.** Una firma ilegible, sin certificado de firma o con un `/SubFilter`
   que rFirma no reconoce es *no válida*, con su motivo. No se omite y no
   tiene un valor propio.
2. **El aviso de perfil longevo del original no cuenta** (`SIGN_PROFILE_NOT_CHECKED`
   en `ValidatePdfSignature`). Una firma T, LT o LTA se juzga como cualquier
   otra, por su integridad y por la vigencia de su certificado. Ese aviso solo
   dice que el original no comprueba los datos de larga duración, nada de la
   firma. Se sigue usando para detectar el `/SubFilter` no reconocido.
3. **La vigencia del certificado se compara con el instante actual, como en
   el original, salvo que la firma lleve el sello de tiempo de una TSA.** En
   ese caso se compara con la fecha del sello. Una firma sellada cuando su
   certificado estaba en vigor es *válida* aunque el certificado haya caducado
   después. El sello es lo único que fecha una firma: la fecha `/M` la declara
   el firmante, y una respuesta OCSP guardada habla del certificado, no de la
   firma.
4. **Si una firma tiene varios problemas, se queda el más grave.** El original
   solo marca la firma posterior a una firma de certificación sin cambios
   permitidos si no hay ningún otro resultado, y su `decisive` deja que un
   aviso pese más que un KO. Con eso, una cofirma prohibida y con el
   certificado caducado saldría *caducada*, y quedaría oculto lo que la hace
   no válida.
5. **En CAdES se comprueba siempre la integridad, aunque el certificado haya
   caducado.** `ValidateBinarySignature` mira primero el certificado, y si
   está caducado ya no comprueba la criptografía. Una firma alterada saldría
   como un simple «caducada». Es la excepción de riesgo del ADR-0023.
6. **En XAdES cuentan todos los certificados del KeyInfo**, como en la
   validación de documento del original, y el motivo nombra el certificado que
   ha caducado. `validateSign(Element, …)` no comprueba la vigencia, así que
   se añade la comprobación.
7. **Con varias firmas de certificación se usa la última, como en el
   original.** La condición `rev <= 0` del original se decide con un PDF de
   prueba de dos firmas corrientes. Si el original lo da por KO, rFirma se
   aparta por contradicción.

## Considered Options

- **Un cuarto valor, «no se ha podido comprobar del todo».** Se descartó porque
  describe una limitación del validador, no algo que la persona pueda hacer
  con la firma. Además daba veredictos peores que los reales: una firma LT con
  el certificado en vigor salía así en vez de válida, y una con el certificado
  caducado dejaba de contar como problema.
- **Copiar la lista de documento del original y repartirla entre las firmas.**
  Se descartó porque esos resultados no dicen de qué firmante son. Un hallazgo
  del documento colgado de la última firma la pinta no válida sin que lo sea.
- **Seguir al original en los puntos 4 y 5.** Se descartó porque, en los dos,
  enseñar *caducada* esconde un problema que hace la firma no válida.
