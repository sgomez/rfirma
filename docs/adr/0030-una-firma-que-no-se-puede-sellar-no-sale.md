# Una firma que pide sello de tiempo y no se puede sellar no sale

Una sede pide sello de tiempo con `tsaURL` en los `extraParams`. AutoFirma
1.9.2 sella las tres familias, pero no las trata igual cuando el sello falla:

- **PAdES** propaga el error (`PdfTimestamper.java:275-279`). El puente sella
  a través de `PAdESTriPhaseSigner`, así que rFirma ya lo hereda.
- **CAdES** llama a `applyTimeStamp` después de firmar
  (`AOCAdESSigner.java:120`). Si `TsaParams` no acepta la URL, un
  `catch (Exception)` devuelve la firma sin sello y sin dejar rastro
  (`:547-555`). Si la TSA no contesta, escribe un `SEVERE` y hace lo mismo
  (`:576-579`).
- **XAdES** sigue el mismo patrón (`XAdESTspUtil.java:75-84`).

Las dos últimas son el BUG-23: la sede recibe una firma sin el sello que pidió
y no se entera. El procesador trifásico ni siquiera llama al sellado, así que
rFirma no sellaba nunca en CAdES ni en XAdES.

## La regla

1. **Si la sede pide sello de tiempo y no se puede sellar, la firma falla.**
   La sede recibe `SAF_09` por el camino de cualquier fallo del puente, en vez
   de una firma sin sello. Lo que el código del original pretende es sellar;
   tragarse el error es el fallo (ADR-0026).
2. **Una URL de TSA que no se puede usar falla antes de firmar**: en la
   prefirma, antes de pedir el PIN. Una TSA que no contesta solo se descubre en
   la postfirma, después de firmar con el token.
3. **Se sella lo mismo que sella el original: la operación `sign`**, incluidos
   el ASiC-S y la factura electrónica, cuyo firmador delega en el de CAdES o en
   el de XAdES. En el ASiC-S se sella la firma antes de cerrar el contenedor.
   La cofirma y la contrafirma no sellan, tampoco en el original, y en ellas
   `tsaURL` no cuenta.
4. **En CAdES y en XAdES el sello es de firma, y `tsType` no cuenta.** El
   sello de documento es cosa del PDF; el original tampoco lee `tsType` fuera
   de PAdES.
5. **Una `tsaURL` vacía es no pedir sello.**

Lo aplica el puente, `SignatureTimestamp`, con las clases públicas del
original: `TsaParams`, `CMSTimestamper.addTimestamp` y
`XAdESTspUtil.timestampXAdES`. No se toca el código del original (ADR-0002).

## Consequences

- Una sede que pide sello y tiene su TSA en servicio recibe lo mismo que de
  AutoFirma. Una sede que pide sello con una TSA rota recibe `SAF_09` en vez
  de una firma sin sello: con AutoFirma esa sede ya estaba recibiendo firmas
  que no cumplían lo que pedía, sin saberlo.
- rFirma abre ahora una conexión a la URL que manda la sede, por `http`, por
  `https` o por `socket`, como el original. La imagen nativa ya admitía los
  dos primeros protocolos.
- XAdES canonicaliza el nodo que sella con XOM, y XOM carga por nombre sus
  tablas de caracteres y su lector de XPath: la imagen nativa los declara en
  sus metadatos. Sin ellos, el sello XAdES solo falla en la imagen, no en la
  JVM.
- Lo miden la grada A del puente, con una TSA falsa de BouncyCastle, y la
  grada C, con una TSA de OpenSSL en el bucle local.

## Considered Options

**Firmar sin sello y avisar a la persona.** Deja la decisión a quien firma,
pero la sede sigue recibiendo una firma que no es la que pidió, y quien firma
no sabe qué exige la sede. Descartada.

**Copiar AutoFirma también en BUG-23.** Sería conformidad literal con un fallo
que contradice lo que el propio código intenta hacer. Descartada, como todo bug
con intención clara (ADR-0026).

**Sellar también la cofirma y la contrafirma.** `CMSTimestamper` sellaría a
todos los firmantes del CMS, también a los que ya estaban, y el original no lo
hace. Descartada: cambiaría firmas ajenas.

**Un código de error propio para el sello.** El texto de la ventana diría que
fue la TSA, pero la sede recibe el mismo `SAF_09` y el mensaje del puente ya lo
nombra. Se deja para cuando la ventana lo necesite.
