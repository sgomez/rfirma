# La suite se optimiza para rFirma al encontrar, lanzar y conducir el cliente, y nunca al juzgarlo

La suite se declaraba agnóstica del cliente, y ya no lo es: la mano conduce a rFirma por AT-SPI,
cambia el tramo de una comprobación según el cliente y cada almacén le escribe sus preferencias.
Fingir lo contrario esconde esas decisiones y obliga a medir rFirma con pasos de más (instalar un
paquete, teclear su versión, esperar a que se preparen todos los perfiles).

**La línea.** La suite sabe qué cliente mide para **encontrarlo** (sus binarios candidatos, con su
biblioteca nativa), **lanzarlo** (entorno, preferencias y raíz de confianza de cada almacén),
**identificarlo** (clase, versión y canal, leídos del propio cliente) y **conducirlo** (la mano).
**Nunca para juzgarlo**: el resultado de una comprobación es el mismo para cualquier cliente. El
catálogo, el juez y la referencia no cambian según quién responda. Un cliente de terceros, AutoFirma
incluido, se sigue lanzando como hasta ahora.

**Lo que matiza ADR-0028.** Allí el perfil aislado era una condición de lanzamiento «igual para
cualquier cliente». Deja de serlo: el perfil es igual en lo que decide qué certificados ve el
cliente, y puede llevar preferencias propias de la clase de cliente. El consentimiento sigue sin
esconderse: ninguna preferencia ni receta salta lo que el cliente enseña antes de contestar, y
`headless` sigue siendo objeto de sus propias comprobaciones.

## Consequences

- El glosario de la suite deja de decir que no sabe qué cliente mide, y dice qué sabe y para qué.
- Las **etiquetas** se enseñan en cualquier informe, pero solo explican el NO CONFORME del cliente
  al que valen. Cambia lo que cuenta como explicado, no el resultado.
- Un informe guarda la identidad del cliente con que se creó (clase, origen y versión), no la ruta
  de su binario.
- Lo que una receta de cliente sabe se queda en ella: ni el juez ni el catálogo preguntan por la
  clase del cliente.

## Considered Options

- **Mantener la suite agnóstica.** Descartada: ya no lo es en la mano ni en el perfil, y la
  agnosticidad de palabra obligaba a medir rFirma como un binario cualquiera.
- **Dejar que el cliente influya también en el juicio** (resultados o expectativas por clase).
  Descartada: la suite mide el protocolo que pretende el código de AutoFirma (ADR-0026); un resultado
  que dependiera del cliente dejaría de ser comparable entre clientes y de poder validarse.
