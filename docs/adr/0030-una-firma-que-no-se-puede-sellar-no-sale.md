# Una firma que pide sello de tiempo y no se puede sellar no sale

Una sede pide sello de tiempo con `tsaURL` en los `extraParams`. AutoFirma
1.9.2 pretende sellar las tres familias, pero solo PAdES y XAdES lo consiguen,
y no las trata igual cuando el sello falla:

- **PAdES** propaga el error (`PdfTimestamper.java:275-279`). El puente sella
  a través de `PAdESTriPhaseSigner`, así que rFirma ya lo hereda.
- **CAdES** llama a `applyTimeStamp` después de firmar
  (`AOCAdESSigner.java:120`), y **nunca sella, ni con la TSA en servicio**.
  Si `TsaParams` no acepta la URL, un `catch (Exception)` devuelve la firma
  sin sello y sin dejar rastro (`:545-557`). Si la acepta, busca por reflexión
  `getTsaHashAlgorithm` en `CMSTimestamper`, que no lo tiene (es de
  `TsaParams`), y `addTimestamp` con un `GregorianCalendar`, cuando el
  parámetro es `Calendar` (`:565-571`). Salta `NoSuchMethodException`, y el
  `catch (Exception)` escribe un `SEVERE` y devuelve la firma sin sello
  (`:577-579`). Es el camino de **toda** firma CAdES con `tsaURL`, no solo el
  de una TSA que no contesta.
- **XAdES** sella con `XAdESTspUtil.timestampXAdES` y propaga el fallo de la
  TSA como `AOException`, pero si `TsaParams` no acepta la URL devuelve la
  firma sin sello y sin dejar rastro (`XAdESTspUtil.java:75-81`).

Los caminos que devuelven la firma sin sello son el BUG-23: la sede recibe
una firma sin el sello que pidió y no se entera. El procesador trifásico ni
siquiera llama al sellado, así que rFirma no sellaba nunca en CAdES ni en
XAdES.

## La regla

1. **Si la sede pide sello de tiempo y no se puede sellar, la firma falla.**
   La sede recibe `SAF_09` por el camino de cualquier fallo del puente, en vez
   de una firma sin sello. Lo que el código del original pretende es sellar;
   tragarse el error, o no llegar nunca a sellar, es el fallo (ADR-0026).
2. **Una URL de TSA que no se puede usar falla antes de firmar**: en la
   prefirma, antes de pedir el PIN. Una TSA que no contesta solo se descubre en
   la postfirma, después de firmar con el token.
3. **Se sella lo que el original pretende sellar: la operación `sign`**,
   incluido el ASiC-S, cuyo firmador delega en el de CAdES o en el de XAdES.
   En el ASiC-S se sella la firma antes de cerrar el contenedor. La cofirma y
   la contrafirma no sellan, tampoco en el original, y en ellas `tsaURL` no
   cuenta. La factura electrónica tampoco: la lista blanca de
   `AOFacturaESigner` (`:57-87, 224-228`) descarta `tsaURL` antes de delegar
   en XAdES, y la factura sale sin sello y sin error.
4. **En CAdES y en XAdES el sello es de firma, y `tsType` no cuenta.** El
   sello de documento es cosa del PDF; el original tampoco lee `tsType` fuera
   de PAdES.
5. **Una `tsaURL` vacía es no pedir sello.**

Lo aplica el puente, `SignatureTimestamp`, con las clases públicas del
original: `TsaParams`, `CMSTimestamper.addTimestamp` y
`XAdESTspUtil.timestampXAdES`. No se toca el código del original (ADR-0002).

## Consequences

- Una sede que pide sello y tiene su TSA en servicio recibe en PAdES y en
  XAdES lo mismo que de AutoFirma. En CAdES recibe una firma sellada, y de
  AutoFirma una sin sello: rFirma diverge del original **también** con la TSA
  sana, y una comprobación de conformidad que compare las dos en CAdES con
  `tsaURL` tiene que contar con ello. Una sede que pide sello con una TSA rota
  recibe `SAF_09` en vez de una firma sin sello: con AutoFirma esa sede ya
  estaba recibiendo firmas que no cumplían lo que pedía, sin saberlo.
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

**Sellar también la factura electrónica.** Partía de que `AOFacturaESigner`
delega en XAdES con los mismos parámetros, y no es así: su lista blanca no
admite `tsaURL`. Quitarlo es una decisión del original, no un error que se
trague, así que no hay intención que restituir. Sellarla haría fallar con
`SAF_09` una factura que el original firma, y cuando la TSA contesta, entregar
un sello que el original no pone. Descartada.

**Un código de error propio para el sello.** El texto de la ventana diría que
fue la TSA, pero la sede recibe el mismo `SAF_09` y el mensaje del puente ya lo
nombra. Se deja para cuando la ventana lo necesite.
