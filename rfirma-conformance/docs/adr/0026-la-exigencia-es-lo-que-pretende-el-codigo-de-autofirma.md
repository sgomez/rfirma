# La exigencia es lo que pretende el código de AutoFirma; el manual solo excluye

Cada comprobación dice lo que debe responder un cliente, y alguien tiene que decidir de dónde sale
ese «debe». La suite toma AutoFirma 1.9.2 como el protocolo, pero AutoFirma tiene bugs: hay
caminos en los que su código prepara una respuesta y nunca llega a darla. Una operación
desconocida en `sign`, por ejemplo, tiene su `SAF_04` preparado; `getOperation` devuelve `null`,
se pide el certificado y el `switch` acaba en una excepción que sale como `SAF_09`
(`ProtocolInvocationLauncherSign.java:700, 857-860`). Y el manual del integrador, la otra fuente
a mano, se contradice con el código y consigo mismo.

**La exigencia es lo que el código de AutoFirma 1.9.2 pretende responder, cuando el propio código
deja clara la intención**: un código SAF preparado, una validación escrita, un mensaje. Si
AutoFirma no llega a hacerlo por un bug suyo, la exigencia no cambia: AutoFirma sale NO CONFORME y
la referencia lo recoge con su ficha `BUG-NN`. Donde el código no muestra intención distinta de lo
que hace, la exigencia es lo que hace.

**El manual no crea exigencias, pero sí las excluye.** Si el manual y el código no coinciden,
manda el código, y la diferencia se anota en `docs/afirma/1.9.2/`. Lo que el manual declara sin
soporte no se exige: CMS, XMLDSig, ODF y OOXML se mantienen «por retrocompatibilidad», con «su uso
desaconsejado» y sin soporte (MCF, §8, pág. 97), y no tienen comprobaciones. `NONE` no está en esa
lista: el manual lo documenta como firma PKCS#1 sin formato y lo admite en el `format` del lote.

## Consequences

- Un enunciado puede decir algo que AutoFirma no hace. No es un error del catálogo si su fuente
  cita dónde el código muestra la intención y la referencia lleva el bug.
- La frontera entre «bug» y «comportamiento» se decide con el código delante, cita incluida. Sin
  una intención visible en el código, lo que hace AutoFirma es el protocolo, aunque parezca raro.
- Un formato que el manual retire deja de medirse: sus comprobaciones se borran, no se marcan.

## Considered Options

- **La exigencia es lo que AutoFirma responde, bugs incluidos.** Descartada: obligaría a rFirma a
  imitar una excepción no capturada, y convertiría cada bug del original en protocolo.
- **El manual también cuenta como prueba de intención.** Descartada: el manual se contradice
  consigo mismo (el lote con `stopOnError`, §6.5.3 frente a §6.5.5; `filters.0` en §6.6 frente a
  `filters.1` en §7.3), y cada errata suya pasaría a ser una exigencia.
- **La exigencia es lo que AutoFirma responde, salvo donde se contradice a sí mismo.** Era la regla
  anterior. Descartada por estrecha: un bug no es una contradicción, y dejaba sin regla el caso más
  frecuente.
