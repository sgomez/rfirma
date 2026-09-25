# La sede verifica las firmas sin dependencias externas, con su propia C14N inclusiva

Las firmas CAdES, PAdES y XAdES se juzgaban por su forma: una firma con los atributos firmados mal
calculados o con un SignedInfo mal canonicalizado salía CONFORME. La condición
`the-signature-verifies` las verifica con la clave del certificado devuelto, y XAdES necesita
canonicalización XML (C14N).

**La sede verifica con Node y nada más**: `node:crypto` para las firmas y los resúmenes, y una C14N
propia en `lib/c14n.mjs`. Solo sabe lo que produce AutoFirma: C14N 1.0 inclusiva, con o sin
comentarios, y las transformaciones `enveloped-signature` y el filtro XPath
`not(ancestor-or-self::ds:Signature)`. Lo demás (C14N exclusiva o 1.1, otros XPath, Base64,
referencias externas) no se da por fallo: la condición sale NO OBSERVABLE con el motivo.

## Consequences

- La sede sigue sin dependencias que instalar: corre donde corra Node, en el portátil y en el
  runner, igual que antes.
- La C14N propia se contrasta con el banco de referencia de `testdata/reference/`, firmado por el
  original 1.9.2 y validado por su oráculo: las siete firmas XML verifican, y una alterada en un
  byte no.
- Si un cliente firma con una canonicalización que la sede no sabe hacer, esa parte sale NO
  OBSERVABLE, nunca NO CONFORME: el hueco es de la sede, no del cliente.
- Las partes se combinan como un AND de tres valores, entre firmantes y dentro de cada uno
  (Reference, SignedInfo, messageDigest, firma): un fallo real da NO CONFORME aunque otra parte no
  se pueda medir; si nada falla y algo no se mide, NO OBSERVABLE; solo si todo verifica, CONFORME.
  Lo que la sede no sabe medir no esconde lo que sí mide y encuentra roto.

## Considered Options

- **`xmlsec1`.** Descartada: no está instalado en este equipo ni en el runner del CI, sería la
  primera dependencia de sistema de la sede y su salida se interpreta como texto. Verifica más de
  lo que AutoFirma produce, pero ese «más» no lo pide ninguna comprobación.
- **Una biblioteca de npm (`xml-crypto`, `xmldsigjs`).** Descartada: la sede no tiene
  `package.json` ni dependencias, y meter un árbol de npm para una transformación de doscientas
  líneas cuesta más de lo que ahorra.
- **Dejar XAdES sin verificar.** Descartada: la C14N inclusiva cabe en un módulo pequeño y el banco
  de referencia basta para contrastarla.
