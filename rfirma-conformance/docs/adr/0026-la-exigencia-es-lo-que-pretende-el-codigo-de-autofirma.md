# La exigencia es lo que pretende el código de AutoFirma; el manual solo rebaja

Cada comprobación dice lo que debe responder un cliente, y alguien tiene que decidir de dónde sale
ese «debe». La suite toma AutoFirma 1.9.2 como el protocolo, pero AutoFirma tiene bugs: hay
caminos en los que su código prepara una respuesta y nunca llega a darla. Una operación
desconocida en `sign`, por ejemplo, tiene su `SAF_04` preparado; `getOperation` devuelve `null`,
se pide el certificado y el `switch` acaba en una excepción que sale como `SAF_09`
(`ProtocolInvocationLauncherSign.java:700, 857-860`). Y el manual del integrador, la otra fuente
a mano, se contradice con el código y consigo mismo.

**La exigencia es lo que el código de AutoFirma 1.9.2 pretende responder, cuando el propio código
deja clara la intención**: un código SAF preparado, una validación escrita, un mensaje. Si
AutoFirma no llega a hacerlo por un bug suyo, la exigencia no cambia: AutoFirma sale NO CONFORME,
la comprobación declara su ficha `BUG-NN` y la referencia lo recoge. Donde el código no muestra intención distinta de lo
que hace, la exigencia es lo que hace.

**El manual no crea exigencias, pero sí las rebaja.** Si el manual y el código no coinciden,
manda el código, y la diferencia se anota en `docs/afirma/1.9.2/`. Lo que el manual declara sin
soporte se mide, pero no cuenta como fallo: CMS, XMLDSig, ODF y OOXML se mantienen «por
retrocompatibilidad», con «su uso desaconsejado» y sin soporte (MCF, §8, pág. 97), y sus
comprobaciones llevan `deprecated = true`. `NONE` no está en esa lista: el manual lo documenta como
firma PKCS#1 sin formato y lo admite en el `format` del lote.

## Consequences

- Un enunciado puede decir algo que AutoFirma no hace. No es un error del catálogo si su fuente
  cita dónde el código muestra la intención y la comprobación declara el bug, que la consola enseña
  en cualquier informe.
- La frontera entre «bug» y «comportamiento» se decide con el código delante, cita incluida. Sin
  una intención visible en el código, lo que hace AutoFirma es el protocolo, aunque parezca raro.
- Un formato que el manual desaconseja se sigue midiendo: su comprobación se marca como formato
  deprecado, la consola lo enseña en cualquier informe y su NO CONFORME se cuenta aparte de los
  fallos del cliente. Que rFirma no lo soporte no es una desviación deliberada ni un fallo.

## Considered Options

- **La exigencia es lo que AutoFirma responde, bugs incluidos.** Descartada: obligaría a rFirma a
  imitar una excepción no capturada, y convertiría cada bug del original en protocolo.
- **El manual también cuenta como prueba de intención.** Descartada: el manual se contradice
  consigo mismo (el lote con `stopOnError`, §6.5.3 frente a §6.5.5; `filters.0` en §6.6 frente a
  `filters.1` en §7.3), y cada errata suya pasaría a ser una exigencia.
- **La exigencia es lo que AutoFirma responde, salvo donde se contradice a sí mismo.** Era la regla
  anterior. Descartada por estrecha: un bug no es una contradicción, y dejaba sin regla el caso más
  frecuente.
- **Un formato que el manual desaconseja deja de medirse y sus comprobaciones se borran.** Era la
  regla anterior. Descartada: AutoFirma los sigue soportando, y borrarlas escondía una diferencia
  real entre los dos clientes; marcadas, la diferencia se ve sin pasar por fallo de rFirma.
