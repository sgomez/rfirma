# La envoltura XAdES la declara la sede; el nombre del formato solo la suple

En AutoFirma 1.9.2 el nombre del formato elige el firmador, pero no su
envoltura. `XAdES`, `XAdES Detached`, `XAdES Enveloped` y `XAdES Enveloping`
van los cuatro a `AOXAdESSigner` (`AOSignerFactory.java:60-63`). El firmador
lee la envoltura de un solo sitio: el parámetro `format` de los `extraParams`,
con `XAdES Enveloping` por defecto (`XAdESSigner.java:265-266`). Ese parámetro
es el que usan las sedes para pedir una firma separada o dentro de su XML, y
la única vía para `XAdES Externally Detached`, que no tiene nombre de formato.

Como nadie copia el nombre del formato a ese parámetro, `format=XAdES Detached`
sin declarar nada más firma `Enveloping` (BUG-32). La suite exige lo que ese
nombre pretende (ADR-0026).

## La regla

1. **Si la sede declara en los `extraParams` una de las cuatro envolturas del
   original, esa es la que se firma**, se llame como se llame el formato. Es lo
   que hace AutoFirma y lo que la sede calibró contra él. Cuando se repite la
   clave, cuenta la última declaración, como en unas `Properties` de Java.
2. **Si no declara ninguna, la pone el nombre del formato.** Esta es la
   desviación que corrige BUG-32: `XAdES Detached` firma separada y
   `XAdES Enveloped`, dentro del documento. Sobre datos que no son XML, esta
   última se rechaza con `SAF_29`.
3. **XAdES ASiC-S y FacturaE imponen su envoltura.** Su firmador del original
   la fija sin atender a la sede (`AOXAdESASiCSSigner.java:67`,
   `AOFacturaESigner.java:91`), así que en ellos la declaración de la sede no
   cuenta.

Lo aplica `signing::adapters::ffi`, al escribir el bloque de `extraParams` que
cruza al puente.

## Consequences

- Una sede que pide `format=XAdES` y la envoltura en los `extraParams`, que es
  la forma más común, recibe lo mismo que de AutoFirma. Una sede que manda un
  valor que no es una de las cuatro envolturas cae en el punto 2, igual que si
  no hubiera declarado nada.
- Una sede que declara una envoltura que contradice el nombre del formato
  recibe la de los `extraParams`. Es lo que obtenía de AutoFirma, y no se
  corrige: esa sede ya está calibrada contra ese resultado.
- Lo mide la grada C con la Externally Detached con su `uri`, la Detached que
  impone la política AGE y el `SAF_29` de un Enveloped sobre datos que no son
  XML.

## Considered Options

**El nombre del formato manda siempre.** Era lo que hacía rFirma: escribía la
envoltura del nombre encima de la de la sede. Firmaba `Enveloping` a toda sede
que pide `format=XAdES` con su envoltura aparte, y dejaba sin salida
`Externally Detached`. Descartada porque rompe el caso más común.

**Copiar AutoFirma también en BUG-32.** Si no se declara nada, siempre
`Enveloping`. Sería una conformidad literal con un fallo que el propio nombre
del formato delata. Descartada, como todo bug con intención clara
(ADR-0026).
